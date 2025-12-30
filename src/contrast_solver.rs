//! Contrast solver using Brent's method to find OKLCH lightness for target APCA contrast.

use argmin::core::{CostFunction, Error, Executor};
use argmin::solver::brent::BrentOpt;
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
