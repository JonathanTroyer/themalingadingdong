//! Contrast solver using Brent's method to find HellwigJmh lightness for target APCA contrast.

use argmin::core::{CostFunction, Error, Executor};
use argmin::solver::brent::{BrentOpt, BrentRoot};
use palette::Srgb;

use crate::apca::apca_contrast;
use crate::hellwig::{HellwigJmh, hellwig_lightness, post_clamp_lightness};

/// Result of solving for lightness with fallback support.
#[derive(Debug, Clone)]
pub struct SolveResult {
    /// The lightness value found (HellwigJmh J', 0-100)
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
/// Minimizes `|apca_contrast(J') - target_lc|` to find the lightness
/// that achieves the target contrast (or gets as close as possible).
struct ContrastCost {
    bg: Srgb<u8>,
    target_lc: f64,
    hue: f32,
    colorfulness: f32,
}

impl ContrastCost {
    /// Compute the APCA contrast for a given lightness value.
    fn contrast_at(&self, j: f64) -> f64 {
        let hellwig = HellwigJmh::new(j as f32, self.colorfulness, self.hue);
        let fg_u8 = hellwig.into_srgb_u8();
        apca_contrast(fg_u8, self.bg).abs()
    }
}

impl CostFunction for ContrastCost {
    type Param = f64;
    type Output = f64;

    fn cost(&self, j: &Self::Param) -> Result<Self::Output, Error> {
        let contrast = self.contrast_at(*j);
        Ok((contrast - self.target_lc).abs())
    }
}

/// Solve for HellwigJmh lightness that achieves target APCA contrast against background.
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
/// * `colorfulness` - Fixed colorfulness for the foreground color (HellwigJmh M)
///
/// # Returns
///
/// A `SolveResult` containing the optimal lightness (J', 0-100) and whether it was exact.
///
/// # Example
///
/// ```
/// use palette::Srgb;
/// use themalingadingdong::contrast_solver::solve_lightness_for_contrast;
///
/// let dark_bg = Srgb::new(26u8, 26, 46);  // #1a1a2e
/// let result = solve_lightness_for_contrast(dark_bg, 60.0, 25.0, 25.0);
/// assert!(result.lightness > 50.0);  // Light colors needed for dark background
/// assert!(result.is_exact);          // 60 Lc should be achievable
/// ```
pub fn solve_lightness_for_contrast(
    bg: Srgb<u8>,
    target_lc: f64,
    hue: f32,
    colorfulness: f32,
) -> SolveResult {
    // Determine search bounds based on background luminance
    // HellwigJmh lightness is 0-100 scale
    let is_dark_bg = hellwig_lightness(bg) < 50.0;
    let (low, high) = if is_dark_bg {
        // Dark background: need lighter foreground
        (10.0, 99.0)
    } else {
        // Light background: need darker foreground
        (1.0, 90.0)
    };

    let cost = ContrastCost {
        bg,
        target_lc,
        hue,
        colorfulness,
    };

    // Use Brent's method for 1D bounded minimization
    let solver = BrentOpt::new(low, high);

    let result = Executor::new(cost, solver)
        .configure(|state| state.max_iters(50))
        .run();

    match result {
        Ok(res) => {
            let lightness = res.state.best_param.unwrap_or(50.0) as f32;
            let error = res.state.best_cost;

            // Recompute achieved contrast at the solution point
            let hellwig = HellwigJmh::new(lightness, colorfulness, hue);
            let fg_u8 = hellwig.into_srgb_u8();
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
            let fallback_j = ((low + high) / 2.0) as f32;
            let hellwig = HellwigJmh::new(fallback_j, colorfulness, hue);
            let fg_u8 = hellwig.into_srgb_u8();
            let achieved_contrast = apca_contrast(fg_u8, bg).abs();

            SolveResult {
                lightness: fallback_j,
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
    /// The base lightness value for all hues (HellwigJmh J', 0-100)
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
    /// Final lightness (base + adjustment), HellwigJmh J' scale (0-100)
    pub lightness: f32,
    /// Adjustment applied to base lightness (typically -2 to +2 on 0-100 scale)
    pub adjustment: f32,
    /// The APCA contrast achieved
    pub achieved_contrast: f64,
    /// Whether minimum contrast was achieved
    pub met_minimum: bool,
    /// Warning message if minimum couldn't be achieved
    pub warning: Option<String>,
}

/// Compute APCA contrast for a color with given HellwigJmh parameters against a background.
pub fn contrast_at_lightness(bg: Srgb<u8>, lightness: f32, colorfulness: f32, hue: f32) -> f64 {
    let hellwig = HellwigJmh::new(lightness, colorfulness, hue);
    let fg_u8 = hellwig.into_srgb_u8();
    apca_contrast(fg_u8, bg).abs()
}

/// Cost function for finding uniform lightness using BrentRoot.
///
/// Returns `worst_contrast(J') - target` so that BrentRoot finds J' where
/// the worst-case hue exactly meets the target contrast.
struct WorstContrastCost {
    bg: Srgb<u8>,
    hues: Vec<f32>,
    colorfulness: f32,
    target: f64,
}

impl WorstContrastCost {
    /// Compute the minimum contrast across all hues at a given lightness.
    fn worst_contrast_at(&self, j: f64) -> f64 {
        self.hues
            .iter()
            .map(|&hue| contrast_at_lightness(self.bg, j as f32, self.colorfulness, hue))
            .fold(f64::INFINITY, f64::min)
    }
}

impl CostFunction for WorstContrastCost {
    type Param = f64;
    type Output = f64;

    fn cost(&self, j: &Self::Param) -> Result<Self::Output, Error> {
        let worst = self.worst_contrast_at(*j);
        // Return difference from target: root is where worst_contrast = target
        Ok(worst - self.target)
    }
}

/// Cost function for maximizing worst-case contrast (fallback when target unreachable).
struct MaxWorstContrastCost {
    bg: Srgb<u8>,
    hues: Vec<f32>,
    colorfulness: f32,
}

impl MaxWorstContrastCost {
    fn worst_contrast_at(&self, j: f64) -> f64 {
        self.hues
            .iter()
            .map(|&hue| contrast_at_lightness(self.bg, j as f32, self.colorfulness, hue))
            .fold(f64::INFINITY, f64::min)
    }
}

impl CostFunction for MaxWorstContrastCost {
    type Param = f64;
    type Output = f64;

    fn cost(&self, j: &Self::Param) -> Result<Self::Output, Error> {
        // Negate to maximize (BrentOpt minimizes)
        Ok(-self.worst_contrast_at(*j))
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
/// * `colorfulness` - Colorfulness for all accent colors (HellwigJmh M)
/// * `min_contrast` - Minimum APCA contrast target (Lc value)
/// * `max_adjustment` - Maximum per-hue lightness adjustment allowed (0-10 on J' scale)
///
/// # Returns
///
/// A `UniformLightnessResult` with base lightness and per-hue results.
pub fn find_uniform_lightness(
    bg: Srgb<u8>,
    hues: &[f32],
    colorfulness: f32,
    min_contrast: f64,
    max_adjustment: f32,
) -> UniformLightnessResult {
    if hues.is_empty() {
        return UniformLightnessResult {
            base_lightness: 50.0,
            hue_results: vec![],
            all_met_minimum: true,
        };
    }

    // HellwigJmh lightness is 0-100 scale
    let is_dark_bg = hellwig_lightness(bg) < 50.0;
    let (low, high) = if is_dark_bg {
        (10.0, 99.0)
    } else {
        (1.0, 90.0)
    };

    let cost = WorstContrastCost {
        bg,
        hues: hues.to_vec(),
        colorfulness,
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
            Ok(res) => res.state.best_param.unwrap_or(50.0) as f32,
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
            colorfulness,
        };
        let solver = BrentOpt::new(low, high);
        let result = Executor::new(max_cost, solver)
            .configure(|state| state.max_iters(50))
            .run();

        match result {
            Ok(res) => res.state.best_param.unwrap_or(50.0) as f32,
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
            let base_contrast = contrast_at_lightness(bg, base_lightness, colorfulness, hue);

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
                let mut best_j = base_lightness;
                let mut best_contrast = base_contrast;

                // Binary search for minimum adjustment needed
                let mut adj = 0.0;
                let step = max_adjustment / 10.0;
                while adj <= max_adjustment {
                    let test_j = (base_lightness + adjustment_dir * adj).clamp(1.0, 99.0);
                    let test_contrast = contrast_at_lightness(bg, test_j, colorfulness, hue);
                    if test_contrast > best_contrast {
                        best_j = test_j;
                        best_contrast = test_contrast;
                    }
                    if test_contrast >= min_contrast {
                        break;
                    }
                    adj += step;
                }

                let adjustment = best_j - base_lightness;
                let met_minimum = best_contrast >= min_contrast;

                HueResult {
                    hue,
                    lightness: best_j,
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

/// Result of uniform post-clamp lightness optimization.
#[derive(Debug, Clone)]
pub struct PostClampResult {
    /// The input J' value to use for all hues (before gamut mapping)
    pub input_lightness: f32,
    /// Target post-clamp J' (mean of all hues at input_lightness)
    pub target_post_clamp_j: f32,
    /// Per-hue results with post-clamp info
    pub hue_results: Vec<PostClampHueResult>,
    /// Whether all hues achieved both consistent J' and minimum contrast
    pub all_goals_met: bool,
}

/// Result for a single hue in post-clamp lightness optimization.
#[derive(Debug, Clone)]
pub struct PostClampHueResult {
    /// The hue value (degrees, 0-360)
    pub hue: f32,
    /// Input J' fed into HellwigJmh (may differ from base if adjusted)
    pub input_j: f32,
    /// J' after gamut mapping (the "actual" perceived lightness)
    pub post_clamp_j: f32,
    /// Deviation from target: post_clamp_j - target_post_clamp_j
    pub j_deviation: f32,
    /// The APCA contrast achieved
    pub achieved_contrast: f64,
    /// Whether minimum contrast was achieved
    pub met_minimum_contrast: bool,
    /// Whether color is within sRGB gamut
    pub is_in_gamut: bool,
    /// Warning message if goals couldn't be achieved
    pub warning: Option<String>,
}

/// Cost function for finding gamut ceiling (max post-clamp J').
/// BrentOpt minimizes, so we negate to maximize.
struct GamutCeilingCost {
    colorfulness: f32,
    hue: f32,
}

impl CostFunction for GamutCeilingCost {
    type Param = f64;
    type Output = f64;

    fn cost(&self, input_j: &Self::Param) -> Result<Self::Output, Error> {
        let post_clamp_j = post_clamp_lightness(*input_j as f32, self.colorfulness, self.hue);
        // Negate to maximize (BrentOpt minimizes)
        Ok(-(post_clamp_j as f64))
    }
}

/// Find the maximum achievable post-clamp J' for a hue at given colorfulness.
///
/// This is the "gamut ceiling" - the highest perceived lightness achievable
/// before saturation limits further increases.
fn find_gamut_ceiling(colorfulness: f32, hue: f32) -> f32 {
    let cost = GamutCeilingCost { colorfulness, hue };
    let solver = BrentOpt::new(1.0, 99.0);

    let result = Executor::new(cost, solver)
        .configure(|state| state.max_iters(50))
        .run();

    match result {
        Ok(res) => {
            let optimal_input_j = res.state.best_param.unwrap_or(99.0) as f32;
            // Return the actual post-clamp J' at optimal point
            post_clamp_lightness(optimal_input_j, colorfulness, hue)
        }
        Err(_) => {
            // Fallback: evaluate at high J' input
            post_clamp_lightness(99.0, colorfulness, hue)
        }
    }
}

/// Cost function for finding contrast floor (where contrast = min_lc).
struct ContrastFloorCost {
    bg: Srgb<u8>,
    colorfulness: f32,
    hue: f32,
    min_contrast: f64,
}

impl CostFunction for ContrastFloorCost {
    type Param = f64;
    type Output = f64;

    fn cost(&self, input_j: &Self::Param) -> Result<Self::Output, Error> {
        let contrast = contrast_at_lightness(self.bg, *input_j as f32, self.colorfulness, self.hue);
        // Return difference from target: root is where contrast = min_lc
        Ok(contrast - self.min_contrast)
    }
}

/// Find the minimum post-clamp J' that achieves minimum contrast.
///
/// Returns (input_j, post_clamp_j) where the contrast exactly meets min_lc,
/// or None if the contrast target is impossible for this hue.
fn find_contrast_floor(
    bg: Srgb<u8>,
    colorfulness: f32,
    hue: f32,
    min_contrast: f64,
    is_dark_bg: bool,
) -> Option<(f32, f32)> {
    let (low, high) = (1.0, 99.0);

    let cost = ContrastFloorCost {
        bg,
        colorfulness,
        hue,
        min_contrast,
    };

    // Check if root exists
    let f_low = contrast_at_lightness(bg, low as f32, colorfulness, hue) - min_contrast;
    let f_high = contrast_at_lightness(bg, high as f32, colorfulness, hue) - min_contrast;

    if f_low * f_high > 0.0 {
        // No root in range - check if we're always above or below target
        if f_low > 0.0 && f_high > 0.0 {
            // Always above target - use lowest J' that still meets contrast
            let input_j = if is_dark_bg { low as f32 } else { high as f32 };
            let post_clamp_j = post_clamp_lightness(input_j, colorfulness, hue);
            return Some((input_j, post_clamp_j));
        }
        // Always below target - impossible
        return None;
    }

    let solver = BrentRoot::new(low, high, 1e-6);
    let result = Executor::new(cost, solver)
        .configure(|state| state.max_iters(50))
        .run();

    match result {
        Ok(res) => {
            let input_j = res.state.best_param.unwrap_or(50.0) as f32;
            let post_clamp_j = post_clamp_lightness(input_j, colorfulness, hue);
            Some((input_j, post_clamp_j))
        }
        Err(_) => None,
    }
}

/// Cost function for finding input J' that produces target post-clamp J'.
struct TargetPostClampCost {
    colorfulness: f32,
    hue: f32,
    target_j: f32,
}

impl CostFunction for TargetPostClampCost {
    type Param = f64;
    type Output = f64;

    fn cost(&self, input_j: &Self::Param) -> Result<Self::Output, Error> {
        let post_clamp_j = post_clamp_lightness(*input_j as f32, self.colorfulness, self.hue);
        Ok((post_clamp_j - self.target_j) as f64)
    }
}

/// Find the input J' that produces a target post-clamp J'.
///
/// For in-gamut colors, input_j ≈ target_j.
/// For out-of-gamut colors, input_j > target_j (pushed out, clamped back).
fn find_input_for_target_post_clamp(target_j: f32, colorfulness: f32, hue: f32) -> f32 {
    let cost = TargetPostClampCost {
        colorfulness,
        hue,
        target_j,
    };

    // Search from target_j upward (may need higher input to hit target after clamping)
    let low = 1.0_f64;
    let high = 99.0_f64;

    // Check bounds
    let f_low = post_clamp_lightness(low as f32, colorfulness, hue) - target_j;
    let f_high = post_clamp_lightness(high as f32, colorfulness, hue) - target_j;

    if f_low * f_high > 0.0 {
        // Target outside achievable range
        if f_low > 0.0 {
            // Target is below minimum achievable
            return low as f32;
        } else {
            // Target is above maximum achievable
            return high as f32;
        }
    }

    let solver = BrentRoot::new(low, high, 1e-6);
    let result = Executor::new(cost, solver)
        .configure(|state| state.max_iters(50))
        .run();

    match result {
        Ok(res) => res.state.best_param.unwrap_or(target_j as f64) as f32,
        Err(_) => target_j, // Fallback: assume in-gamut
    }
}

/// Find uniform post-clamp lightness for all hues.
///
/// Uses a bounds-based algorithm to find a consistent post-gamut-mapping J'
/// across all hues while ensuring minimum APCA contrast is met.
///
/// Algorithm:
/// 1. Find gamut ceiling (max achievable post-clamp J') for each hue
/// 2. Find contrast floor (min post-clamp J' meeting min_lc) for each hue
/// 3. Target = max(all floors) to ensure all hues meet contrast
/// 4. Find input J' for each hue that produces the target post-clamp J'
///
/// # Arguments
///
/// * `bg` - Background color in sRGB
/// * `hues` - Array of hue values (degrees, 0-360)
/// * `colorfulness` - Colorfulness for all accent colors (HellwigJmh M)
/// * `min_contrast` - Minimum APCA contrast target (Lc value)
/// * `_max_adjustment` - Unused (kept for API compatibility)
///
/// # Returns
///
/// A `PostClampResult` with the optimized input lightness and per-hue details.
pub fn find_uniform_post_clamp_lightness(
    bg: Srgb<u8>,
    hues: &[f32],
    colorfulness: f32,
    min_contrast: f64,
    _max_adjustment: f32,
) -> PostClampResult {
    if hues.is_empty() {
        return PostClampResult {
            input_lightness: 50.0,
            target_post_clamp_j: 50.0,
            hue_results: vec![],
            all_goals_met: true,
        };
    }

    let is_dark_bg = hellwig_lightness(bg) < 50.0;

    // Phase 1: Find gamut ceiling and contrast floor for each hue
    let mut ceilings: Vec<f32> = Vec::with_capacity(hues.len());
    let mut floors: Vec<Option<(f32, f32)>> = Vec::with_capacity(hues.len());

    for &hue in hues {
        let ceiling = find_gamut_ceiling(colorfulness, hue);
        let floor = find_contrast_floor(bg, colorfulness, hue, min_contrast, is_dark_bg);
        ceilings.push(ceiling);
        floors.push(floor);
    }

    // Phase 2: Determine feasible range and target post-clamp J'
    let upper_bound = ceilings.iter().cloned().fold(f32::INFINITY, f32::min); // Lowest ceiling

    // For floors, extract the post-clamp J' values (second element of tuple)
    let valid_floors: Vec<f32> = floors
        .iter()
        .filter_map(|f| f.map(|(_, post_j)| post_j))
        .collect();

    let lower_bound = if valid_floors.is_empty() {
        0.0
    } else {
        valid_floors
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max) // Highest floor
    };

    // Determine target post-clamp J' - use minimum of valid range (highest floor)
    let (target_post_clamp_j, feasible) = if lower_bound <= upper_bound {
        (lower_bound, true)
    } else {
        // Infeasible: constraints conflict. Use the lowest ceiling.
        (upper_bound, false)
    };

    // Phase 3: Find input J' for each hue to achieve target post-clamp J'
    let hue_results: Vec<PostClampHueResult> = hues
        .iter()
        .zip(floors.iter())
        .map(|(&hue, floor_opt)| {
            // Find input J' that produces target post-clamp J'
            let input_j = find_input_for_target_post_clamp(target_post_clamp_j, colorfulness, hue);

            // Compute actual results at this input J'
            let hellwig = HellwigJmh::new(input_j, colorfulness, hue);
            let clamped = hellwig.into_srgb_u8();
            let post_clamp = HellwigJmh::from_srgb_u8(clamped);
            let achieved_contrast = apca_contrast(clamped, bg).abs();
            let is_in_gamut = hellwig.is_in_gamut();

            let met_minimum = achieved_contrast >= min_contrast - 0.5; // Small tolerance
            let j_deviation = post_clamp.lightness - target_post_clamp_j;

            let warning = if !met_minimum {
                Some(format!(
                    "Hue {:.0}°: minimum Lc {:.0} unreachable, achieved {:.1}",
                    hue, min_contrast, achieved_contrast
                ))
            } else if !feasible && floor_opt.is_none() {
                Some(format!(
                    "Hue {:.0}°: contrast target conflicts with gamut ceiling",
                    hue
                ))
            } else {
                None
            };

            PostClampHueResult {
                hue,
                input_j,
                post_clamp_j: post_clamp.lightness,
                j_deviation,
                achieved_contrast,
                met_minimum_contrast: met_minimum,
                is_in_gamut,
                warning,
            }
        })
        .collect();

    // Find a representative input_lightness (use the one for first hue)
    let input_lightness = hue_results.first().map(|r| r.input_j).unwrap_or(50.0);

    let all_goals_met = hue_results
        .iter()
        .all(|r| r.met_minimum_contrast && r.j_deviation.abs() < 1.0);

    PostClampResult {
        input_lightness,
        target_post_clamp_j,
        hue_results,
        all_goals_met,
    }
}
