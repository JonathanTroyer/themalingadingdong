//! Contrast solver using Brent's method to find OKLCH lightness for target APCA contrast.

use argmin::core::{CostFunction, Error, Executor};
use argmin::solver::brent::{BrentOpt, BrentRoot};
use palette::{IntoColor, Oklch, Srgb};

use crate::apca::apca_contrast;
use crate::interpolation::{oklch_lightness, srgb_to_u8};

/// Result of solving for lightness with fallback support.
#[derive(Debug, Clone)]
pub struct SolveResult {
    /// The lightness value found (OKLCH L, 0.0-1.0)
    pub lightness: f32,
    /// The actual APCA contrast achieved
    pub achieved_contrast: f64,
    /// Whether the exact target was achieved (vs. best-effort)
    pub is_exact: bool,
    /// Warning message if contrast target couldn't be achieved
    pub warning: Option<String>,
}

/// Cost function for APCA contrast optimization.
///
/// Minimizes `|apca_contrast(L) - target_lc|` to find the lightness
/// that achieves the target contrast (or gets as close as possible).
struct ContrastCost {
    bg: Srgb<u8>,
    target_lc: f64,
    hue: f32,
    chroma: f32,
}

impl ContrastCost {
    /// Compute the APCA contrast for a given lightness value.
    fn contrast_at(&self, l: f64) -> f64 {
        let oklch = Oklch::new(l as f32, self.chroma, self.hue);
        let linear_srgb: palette::LinSrgb<f32> = oklch.into_color();
        let fg = Srgb::from_linear(linear_srgb);
        let fg_u8 = srgb_to_u8(fg);
        apca_contrast(fg_u8, self.bg).abs()
    }
}

impl CostFunction for ContrastCost {
    type Param = f64;
    type Output = f64;

    fn cost(&self, l: &Self::Param) -> Result<Self::Output, Error> {
        let contrast = self.contrast_at(*l);
        Ok((contrast - self.target_lc).abs())
    }
}

/// Solve for OKLCH lightness that achieves target APCA contrast against background.
///
/// Uses Brent's minimization method to find the lightness that minimizes
/// `|achieved_contrast - target_contrast|`. This always succeeds:
/// - If the target is achievable, returns the exact solution (error ≈ 0)
/// - If the target is unreachable, returns the closest achievable contrast
///
/// # Arguments
///
/// * `bg` - Background color in sRGB
/// * `target_lc` - Target APCA contrast (Lc value, typically 30-90)
/// * `hue` - Fixed hue for the foreground color (degrees, 0-360)
/// * `chroma` - Fixed chroma for the foreground color
///
/// # Returns
///
/// A `SolveResult` containing the optimal lightness and whether it was exact.
///
/// # Example
///
/// ```
/// use palette::Srgb;
/// use themalingadingdong::contrast_solver::solve_lightness_for_contrast;
///
/// let dark_bg = Srgb::new(26u8, 26, 46);  // #1a1a2e
/// let result = solve_lightness_for_contrast(dark_bg, 60.0, 25.0, 0.12);
/// assert!(result.lightness > 0.5);  // Light colors needed for dark background
/// assert!(result.is_exact);         // 60 Lc should be achievable
/// ```
pub fn solve_lightness_for_contrast(
    bg: Srgb<u8>,
    target_lc: f64,
    hue: f32,
    chroma: f32,
) -> SolveResult {
    // Determine search bounds based on background luminance
    let is_dark_bg = oklch_lightness(bg) < 0.5;
    let (low, high) = if is_dark_bg {
        // Dark background: need lighter foreground
        (0.1, 0.99)
    } else {
        // Light background: need darker foreground
        (0.01, 0.9)
    };

    let cost = ContrastCost {
        bg,
        target_lc,
        hue,
        chroma,
    };

    // Use Brent's method for 1D bounded minimization
    let solver = BrentOpt::new(low, high);

    let result = Executor::new(cost, solver)
        .configure(|state| state.max_iters(50))
        .run();

    match result {
        Ok(res) => {
            let lightness = res.state.best_param.unwrap_or(0.5) as f32;
            let error = res.state.best_cost;

            // Recompute achieved contrast at the solution point
            let oklch = Oklch::new(lightness, chroma, hue);
            let linear_srgb: palette::LinSrgb<f32> = oklch.into_color();
            let fg = Srgb::from_linear(linear_srgb);
            let fg_u8 = srgb_to_u8(fg);
            let achieved_contrast = apca_contrast(fg_u8, bg).abs();

            // Consider it "exact" if within 0.5 Lc of target
            let is_exact = error < 0.5;

            let warning = if is_exact {
                None
            } else {
                Some(format!(
                    "Hue {:.0}: target Lc {:.0} unreachable, achieved Lc {:.1}",
                    hue, target_lc, achieved_contrast
                ))
            };

            SolveResult {
                lightness,
                achieved_contrast,
                is_exact,
                warning,
            }
        }
        Err(_) => {
            // Fallback: use midpoint of search range
            let fallback_l = ((low + high) / 2.0) as f32;
            let oklch = Oklch::new(fallback_l, chroma, hue);
            let linear_srgb: palette::LinSrgb<f32> = oklch.into_color();
            let fg = Srgb::from_linear(linear_srgb);
            let fg_u8 = srgb_to_u8(fg);
            let achieved_contrast = apca_contrast(fg_u8, bg).abs();

            SolveResult {
                lightness: fallback_l,
                achieved_contrast,
                is_exact: false,
                warning: Some(format!(
                    "Hue {:.0}: optimization failed, using fallback Lc {:.1}",
                    hue, achieved_contrast
                )),
            }
        }
    }
}

/// Result of uniform lightness optimization for a set of accent hues.
#[derive(Debug, Clone)]
pub struct UniformLightnessResult {
    /// The base lightness value for all hues (OKLCH L, 0.0-1.0)
    pub base_lightness: f32,
    /// Per-hue results with any micro-adjustments
    pub hue_results: Vec<HueResult>,
    /// Whether all hues achieved minimum contrast
    pub all_met_minimum: bool,
}

/// Result for a single hue in uniform lightness optimization.
#[derive(Debug, Clone)]
pub struct HueResult {
    /// The hue value (degrees, 0-360)
    pub hue: f32,
    /// Final lightness (base + adjustment)
    pub lightness: f32,
    /// Adjustment applied to base lightness (typically -0.02 to +0.02)
    pub adjustment: f32,
    /// The APCA contrast achieved
    pub achieved_contrast: f64,
    /// Whether minimum contrast was achieved
    pub met_minimum: bool,
    /// Warning message if minimum couldn't be achieved
    pub warning: Option<String>,
}

/// Compute APCA contrast for a color with given OKLCH parameters against a background.
pub fn contrast_at_lightness(bg: Srgb<u8>, lightness: f32, chroma: f32, hue: f32) -> f64 {
    let oklch = Oklch::new(lightness, chroma, hue);
    let linear_srgb: palette::LinSrgb<f32> = oklch.into_color();
    let fg = Srgb::from_linear(linear_srgb);
    let fg_u8 = srgb_to_u8(fg);
    apca_contrast(fg_u8, bg).abs()
}

/// Cost function for finding uniform lightness using BrentRoot.
///
/// Returns `worst_contrast(L) - target` so that BrentRoot finds L where
/// the worst-case hue exactly meets the target contrast.
struct WorstContrastCost {
    bg: Srgb<u8>,
    hues: Vec<f32>,
    chroma: f32,
    target: f64,
}

impl WorstContrastCost {
    /// Compute the minimum contrast across all hues at a given lightness.
    fn worst_contrast_at(&self, l: f64) -> f64 {
        self.hues
            .iter()
            .map(|&hue| contrast_at_lightness(self.bg, l as f32, self.chroma, hue))
            .fold(f64::INFINITY, f64::min)
    }
}

impl CostFunction for WorstContrastCost {
    type Param = f64;
    type Output = f64;

    fn cost(&self, l: &Self::Param) -> Result<Self::Output, Error> {
        let worst = self.worst_contrast_at(*l);
        // Return difference from target: root is where worst_contrast = target
        Ok(worst - self.target)
    }
}

/// Cost function for maximizing worst-case contrast (fallback when target unreachable).
struct MaxWorstContrastCost {
    bg: Srgb<u8>,
    hues: Vec<f32>,
    chroma: f32,
}

impl MaxWorstContrastCost {
    fn worst_contrast_at(&self, l: f64) -> f64 {
        self.hues
            .iter()
            .map(|&hue| contrast_at_lightness(self.bg, l as f32, self.chroma, hue))
            .fold(f64::INFINITY, f64::min)
    }
}

impl CostFunction for MaxWorstContrastCost {
    type Param = f64;
    type Output = f64;

    fn cost(&self, l: &Self::Param) -> Result<Self::Output, Error> {
        // Negate to maximize (BrentOpt minimizes)
        Ok(-self.worst_contrast_at(*l))
    }
}

/// Find uniform lightness for all hues to meet minimum contrast.
///
/// Uses BrentRoot to find the lightness where the worst-case hue meets the target.
/// Falls back to BrentOpt to maximize worst-case contrast if target is unreachable.
///
/// # Arguments
///
/// * `bg` - Background color in sRGB
/// * `hues` - Array of hue values (degrees, 0-360)
/// * `chroma` - Chroma for all accent colors
/// * `min_contrast` - Minimum APCA contrast target (Lc value)
/// * `max_adjustment` - Maximum per-hue lightness adjustment allowed
///
/// # Returns
///
/// A `UniformLightnessResult` with base lightness and per-hue results.
pub fn find_uniform_lightness(
    bg: Srgb<u8>,
    hues: &[f32],
    chroma: f32,
    min_contrast: f64,
    max_adjustment: f32,
) -> UniformLightnessResult {
    if hues.is_empty() {
        return UniformLightnessResult {
            base_lightness: 0.5,
            hue_results: vec![],
            all_met_minimum: true,
        };
    }

    let is_dark_bg = oklch_lightness(bg) < 0.5;
    let (low, high) = if is_dark_bg { (0.1, 0.99) } else { (0.01, 0.9) };

    let cost = WorstContrastCost {
        bg,
        hues: hues.to_vec(),
        chroma,
        target: min_contrast,
    };

    // Check if root exists: f(low) and f(high) must have opposite signs
    let f_low = cost.worst_contrast_at(low) - min_contrast;
    let f_high = cost.worst_contrast_at(high) - min_contrast;

    let base_lightness = if f_low * f_high < 0.0 {
        // Root exists: use BrentRoot to find it
        let solver = BrentRoot::new(low, high, 1e-6);
        let result = Executor::new(cost, solver)
            .configure(|state| state.max_iters(50))
            .run();

        match result {
            Ok(res) => res.state.best_param.unwrap_or(0.5) as f32,
            Err(_) => {
                // Fallback: use the bound that gives higher contrast
                if is_dark_bg { high as f32 } else { low as f32 }
            }
        }
    } else if f_low >= 0.0 && f_high >= 0.0 {
        // Target already exceeded at both bounds: use bound closest to neutral
        if is_dark_bg { low as f32 } else { high as f32 }
    } else {
        // Target unreachable: maximize worst-case contrast using BrentOpt
        let max_cost = MaxWorstContrastCost {
            bg,
            hues: hues.to_vec(),
            chroma,
        };
        let solver = BrentOpt::new(low, high);
        let result = Executor::new(max_cost, solver)
            .configure(|state| state.max_iters(50))
            .run();

        match result {
            Ok(res) => res.state.best_param.unwrap_or(0.5) as f32,
            Err(_) => {
                if is_dark_bg {
                    high as f32
                } else {
                    low as f32
                }
            }
        }
    };

    // Generate per-hue results with optional micro-adjustments
    let hue_results: Vec<HueResult> = hues
        .iter()
        .map(|&hue| {
            let base_contrast = contrast_at_lightness(bg, base_lightness, chroma, hue);

            if base_contrast >= min_contrast {
                // Already meets minimum at base lightness
                HueResult {
                    hue,
                    lightness: base_lightness,
                    adjustment: 0.0,
                    achieved_contrast: base_contrast,
                    met_minimum: true,
                    warning: None,
                }
            } else {
                // Try micro-adjustment to reach minimum
                let adjustment_dir = if is_dark_bg { 1.0 } else { -1.0 };
                let mut best_l = base_lightness;
                let mut best_contrast = base_contrast;

                // Binary search for minimum adjustment needed
                let mut adj = 0.0;
                let step = max_adjustment / 10.0;
                while adj <= max_adjustment {
                    let test_l = (base_lightness + adjustment_dir * adj).clamp(0.01, 0.99);
                    let test_contrast = contrast_at_lightness(bg, test_l, chroma, hue);
                    if test_contrast > best_contrast {
                        best_l = test_l;
                        best_contrast = test_contrast;
                    }
                    if test_contrast >= min_contrast {
                        break;
                    }
                    adj += step;
                }

                let adjustment = best_l - base_lightness;
                let met_minimum = best_contrast >= min_contrast;

                HueResult {
                    hue,
                    lightness: best_l,
                    adjustment,
                    achieved_contrast: best_contrast,
                    met_minimum,
                    warning: if met_minimum {
                        None
                    } else {
                        Some(format!(
                            "Hue {:.0}°: minimum Lc {:.0} unreachable, achieved {:.1}",
                            hue, min_contrast, best_contrast
                        ))
                    },
                }
            }
        })
        .collect();

    let all_met_minimum = hue_results.iter().all(|r| r.met_minimum);

    UniformLightnessResult {
        base_lightness,
        hue_results,
        all_met_minimum,
    }
}
