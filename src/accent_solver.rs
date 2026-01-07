//! COBYLA-based accent color optimization.
//!
//! Optimizes (J', M) pairs for accent colors using constrained optimization
//! to balance lightness uniformity against colorfulness preservation.

use std::time::Instant;

use argmin::core::{CostFunction, Error, Executor, State};
use cobyla::CobylaSolver;
use palette::Srgb;
use rayon::prelude::*;
use tracing::{debug, info, warn};

use crate::apca::{contrast_from_luminances, srgb_f32_to_luminance, srgb_to_luminance};
use crate::config::AccentOptSettings;
use crate::gamut_map::{cusp_at_hue, gamut_map, max_colorfulness_at};
use crate::hellwig::HellwigJmh;
use crate::interpolation::srgb_to_u8;

/// Result for a single hue optimization.
#[derive(Debug, Clone)]
pub struct HueOptResult {
    /// The hue (degrees, 0-360)
    pub hue: f32,
    /// Optimized lightness (J')
    pub j: f32,
    /// Optimized colorfulness (M)
    pub m: f32,
    /// Original colorfulness before gamut mapping
    pub original_m: f32,
    /// Final gamut-mapped sRGB color
    pub color: Srgb<f32>,
    /// Post-gamut-mapping lightness
    pub post_clamp_j: f32,
    /// APCA contrast against background
    pub achieved_contrast: f64,
    /// Whether all constraints were satisfied
    pub met_constraints: bool,
    /// Warning message if constraints couldn't be satisfied
    pub warning: Option<String>,
    /// Whether J is within bounds after gamut mapping
    pub j_in_bounds: bool,
    /// Whether M is within bounds after gamut mapping
    pub m_in_bounds: bool,
    /// The M lower bound for this optimization
    pub m_lower_bound: f32,
    /// The M upper bound for this optimization
    pub m_upper_bound: f32,
}

/// Result for all accent hues.
#[derive(Debug, Clone)]
pub struct AccentOptResult {
    /// Per-hue optimization results
    pub hue_results: Vec<HueOptResult>,
    /// Total optimization time in milliseconds
    pub elapsed_ms: u64,
}

/// Cost function for COBYLA optimization of a single hue.
///
/// Minimizes weighted combination of contrast gap and uniformity while enforcing:
/// - Box constraints on J' and M (hard constraints)
/// - Gamut constraint: M <= max achievable at J, hue
struct AccentProblem {
    /// Pre-computed background luminance (shared across all hues)
    bg_lum: f64,
    /// Fixed hue for this optimization
    hue: f32,
    /// Target lightness
    target_j: f32,
    /// Target colorfulness
    target_m: f32,
    /// Max deviation from target_j
    delta_j: f32,
    /// Max deviation from target_m
    delta_m: f32,
    /// Weight for J vs M uniformity (0=M priority, 1=J priority)
    j_weight: f32,
    /// Weight for contrast vs uniformity (0=uniformity, 1=contrast)
    contrast_weight: f32,
    /// Minimum contrast requirement
    min_contrast: f64,
}

impl AccentProblem {
    /// Compute contrast for given (J', M) after gamut mapping.
    #[inline]
    fn contrast_at(&self, j: f64, m: f64) -> f64 {
        let color = HellwigJmh::new(j as f32, m as f32, self.hue);
        let mapped = gamut_map(color);
        let srgb = mapped.into_srgb();
        let fg_lum = srgb_f32_to_luminance(srgb);
        contrast_from_luminances(fg_lum, self.bg_lum).abs()
    }

    /// Compute weighted objective distance from targets.
    fn objective(&self, j: f64, m: f64) -> f64 {
        let w = self.j_weight as f64;
        let j_term = ((j - self.target_j as f64) / self.delta_j as f64).powi(2);
        let m_term = ((m - self.target_m as f64) / self.delta_m as f64).powi(2);
        w * j_term + (1.0 - w) * m_term
    }
}

impl CostFunction for AccentProblem {
    type Param = Vec<f64>;
    type Output = Vec<f64>;

    fn cost(&self, params: &Self::Param) -> Result<Self::Output, Error> {
        let j = params[0];
        let m = params[1];

        // Uniformity term: weighted J/M distance from targets
        let uniformity = self.objective(j, m);

        // Contrast gap term: normalized squared distance below minimum contrast
        let contrast = self.contrast_at(j, m);
        let contrast_gap = ((self.min_contrast - contrast) / self.min_contrast)
            .max(0.0)
            .powi(2);

        // OBJECTIVE: weighted combination (contrast_weight controls priority)
        let cw = self.contrast_weight as f64;
        let objective = cw * contrast_gap + (1.0 - cw) * uniformity;

        // HARD CONSTRAINTS (COBYLA treats positive values as satisfied)
        // J box constraints
        let j_lower = j - (self.target_j - self.delta_j) as f64;
        let j_upper = (self.target_j + self.delta_j) as f64 - j;

        // M box constraints
        let m_lower = m - (self.target_m - self.delta_m).max(0.0) as f64;
        let m_upper = (self.target_m + self.delta_m) as f64 - m;

        // Gamut constraint: M must be <= max achievable at this J'
        let m_max = max_colorfulness_at(j as f32, self.hue) as f64;
        let gamut_constraint = m_max - m;

        Ok(vec![
            objective,
            j_lower,
            j_upper,
            m_lower,
            m_upper,
            gamut_constraint,
        ])
    }
}

/// Find feasible starting point for optimization using cusp data.
fn initial_guess(hue: f32, settings: &AccentOptSettings) -> (f64, f64) {
    let cusp = cusp_at_hue(hue);

    // Start at target J' if feasible, otherwise use cusp J'
    let j = if settings.target_j >= 5.0 && settings.target_j <= 95.0 {
        settings.target_j
    } else {
        cusp.j.clamp(20.0, 80.0)
    };

    // Start at target M if in gamut, otherwise scale down (0.95 to stay close to boundary)
    let m_max = max_colorfulness_at(j, hue);
    let m = settings.target_m.min(m_max * 0.95);

    (j as f64, m as f64)
}

/// Check if M lower bound is achievable within J bounds for a given hue.
///
/// Returns (is_feasible, max_achievable_m) where:
/// - is_feasible: true if gamut allows M >= target_m - delta_m
/// - max_achievable_m: maximum M achievable within J bounds
fn check_m_feasibility(hue: f32, settings: &AccentOptSettings) -> (bool, f32) {
    let j_min = settings.target_j - settings.delta_j;
    let j_max = settings.target_j + settings.delta_j;
    let m_required = (settings.target_m - settings.delta_m).max(0.0);

    // Find max achievable M within J bounds (sample at 1-unit intervals)
    let mut max_m = 0.0f32;
    let j_start = j_min.max(0.0) as i32;
    let j_end = j_max.min(100.0) as i32;
    for j in j_start..=j_end {
        max_m = max_m.max(max_colorfulness_at(j as f32, hue));
    }

    (max_m >= m_required, max_m)
}

/// Optimize accent colors for all hues using COBYLA.
///
/// Pre-computes background luminance once and runs per-hue optimization.
/// Returns best-effort results even when constraints are infeasible.
///
/// # Arguments
///
/// * `background` - Background color for contrast calculation
/// * `hues` - Slice of hue values (degrees, 0-360)
/// * `settings` - Optimization settings (targets, tolerances, weight)
/// * `min_contrast` - Minimum APCA contrast requirement (Lc)
///
/// # Example
///
/// ```
/// use palette::Srgb;
/// use themalingadingdong::accent_solver::optimize_accents;
/// use themalingadingdong::config::AccentOptSettings;
///
/// let bg = Srgb::new(26u8, 26, 46);
/// let hues = [25.0, 60.0, 120.0, 180.0, 240.0, 285.0, 320.0, 350.0];
/// let settings = AccentOptSettings::default();
/// let result = optimize_accents(bg, &hues, &settings, 60.0);
/// assert_eq!(result.hue_results.len(), 8);
/// ```
pub fn optimize_accents(
    background: Srgb<u8>,
    hues: &[f32],
    settings: &AccentOptSettings,
    min_contrast: f64,
) -> AccentOptResult {
    let start = Instant::now();

    // Pre-compute background luminance ONCE for all hues
    let bg_lum = srgb_to_luminance(background);

    // Parallel optimization across hues (typically 8 hues, scales well on multi-core)
    let hue_results: Vec<HueOptResult> = hues
        .par_iter()
        .map(|&hue| optimize_single_hue(bg_lum, hue, settings, min_contrast))
        .collect();

    let elapsed_ms = start.elapsed().as_millis() as u64;
    info!(
        hues = hues.len(),
        elapsed_ms, "Accent optimization complete"
    );

    AccentOptResult {
        hue_results,
        elapsed_ms,
    }
}

/// Optimize a single hue using COBYLA.
fn optimize_single_hue(
    bg_lum: f64,
    hue: f32,
    settings: &AccentOptSettings,
    min_contrast: f64,
) -> HueOptResult {
    // Check M feasibility before optimization
    let (is_m_feasible, max_achievable_m) = check_m_feasibility(hue, settings);
    let m_lower = (settings.target_m - settings.delta_m).max(0.0);

    if !is_m_feasible {
        // Gamut boundary prevents achieving M lower bound - skip optimization
        warn!(
            hue,
            max_achievable_m, m_lower, "Hue infeasible: gamut limit below M lower bound"
        );
        // Use best possible values: target J and max achievable M
        return build_hue_result(
            bg_lum,
            hue,
            settings.target_j,
            max_achievable_m,
            min_contrast,
            settings,
            Some(format!(
                "Hue {:.0}: gamut limit {:.1} < M bound {:.1}",
                hue, max_achievable_m, m_lower
            )),
        );
    }

    let (j_init, m_init) = initial_guess(hue, settings);

    debug!(
        hue,
        j_init,
        m_init,
        target_j = settings.target_j,
        target_m = settings.target_m,
        delta_j = settings.delta_j,
        delta_m = settings.delta_m,
        min_contrast,
        "Starting COBYLA optimization"
    );

    let problem = AccentProblem {
        bg_lum,
        hue,
        target_j: settings.target_j,
        target_m: settings.target_m,
        delta_j: settings.delta_j,
        delta_m: settings.delta_m,
        j_weight: settings.j_weight,
        contrast_weight: settings.contrast_weight,
        min_contrast,
    };

    // Check initial contrast to understand feasibility
    let init_contrast = problem.contrast_at(j_init, m_init);
    debug!(hue, init_contrast, "Initial guess contrast");

    let solver = CobylaSolver::new(vec![j_init, m_init]);

    let result = Executor::new(problem, solver)
        .configure(|mut state| {
            state.rhoend = 1e-6; // Tighter convergence tolerance
            state.max_iters(200).iprint(0) // Higher limit as safety net
        })
        .run();

    match result {
        Ok(res) => {
            let fallback = vec![j_init, m_init];
            let best = res.state.get_best_param().unwrap_or(&fallback);
            let j = best[0] as f32;
            let m = best[1] as f32;

            debug!(hue, j, m, "COBYLA converged");
            build_hue_result(bg_lum, hue, j, m, min_contrast, settings, None)
        }
        Err(e) => {
            warn!(hue, error = %e, "COBYLA optimization failed, using initial guess");
            let j = j_init as f32;
            let m = m_init as f32;

            build_hue_result(
                bg_lum,
                hue,
                j,
                m,
                min_contrast,
                settings,
                Some(format!("COBYLA failed: {}", e)),
            )
        }
    }
}

/// Build HueOptResult from optimized (J', M) values.
fn build_hue_result(
    bg_lum: f64,
    hue: f32,
    j: f32,
    m: f32,
    min_contrast: f64,
    settings: &AccentOptSettings,
    mut warning: Option<String>,
) -> HueOptResult {
    // Store original M before gamut mapping
    let original_m = m;

    // Apply gamut mapping
    let color = HellwigJmh::new(j, m, hue);
    let mapped = gamut_map(color);
    let srgb = mapped.into_srgb();

    // Compute actual contrast
    let fg_lum = srgb_to_luminance(srgb_to_u8(srgb));
    let achieved_contrast = contrast_from_luminances(fg_lum, bg_lum).abs();

    // Compute bounds
    let j_lower = settings.target_j - settings.delta_j;
    let j_upper = settings.target_j + settings.delta_j;
    let m_lower = (settings.target_m - settings.delta_m).max(0.0);
    let m_upper = settings.target_m + settings.delta_m;

    // Check post-gamut-mapping bounds
    let j_in_bounds = mapped.lightness >= j_lower && mapped.lightness <= j_upper;
    let m_in_bounds = mapped.colorfulness >= m_lower && mapped.colorfulness <= m_upper;
    let contrast_met = achieved_contrast >= min_contrast;

    // met_constraints now includes J/M bounds
    let met_constraints = j_in_bounds && m_in_bounds && contrast_met;

    debug!(
        hue,
        j,
        m,
        post_clamp_j = mapped.lightness,
        post_clamp_m = mapped.colorfulness,
        achieved_contrast,
        min_contrast,
        j_in_bounds,
        m_in_bounds,
        met_constraints,
        "Hue optimization result"
    );

    // Generate warnings for constraint violations (M bounds take priority)
    if !m_in_bounds && warning.is_none() {
        warn!(
            hue,
            m = mapped.colorfulness,
            m_lower,
            m_upper,
            "M outside bounds after gamut mapping"
        );
        warning = Some(format!(
            "Hue {:.0}: M={:.1} outside [{:.1}, {:.1}]",
            hue, mapped.colorfulness, m_lower, m_upper
        ));
    } else if !contrast_met && warning.is_none() {
        warn!(
            hue,
            achieved = achieved_contrast,
            required = min_contrast,
            "Contrast below minimum within bounds"
        );
        warning = Some(format!(
            "Hue {:.0}: Lc {:.1} < {:.1} (best within bounds)",
            hue, achieved_contrast, min_contrast
        ));
    }

    HueOptResult {
        hue,
        j,
        m: mapped.colorfulness,
        original_m,
        color: srgb,
        post_clamp_j: mapped.lightness,
        achieved_contrast,
        met_constraints,
        warning,
        j_in_bounds,
        m_in_bounds,
        m_lower_bound: m_lower,
        m_upper_bound: m_upper,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optimize_accents_returns_correct_count() {
        let bg = Srgb::new(26u8, 26, 46);
        let hues = [25.0, 60.0, 120.0, 180.0, 240.0, 285.0, 320.0, 350.0];
        let settings = AccentOptSettings::default();
        let result = optimize_accents(bg, &hues, &settings, 60.0);
        assert_eq!(result.hue_results.len(), 8);
    }

    #[test]
    fn optimize_accents_meets_contrast() {
        let bg = Srgb::new(26u8, 26, 46);
        let hues = [60.0, 180.0, 300.0]; // Easy hues
        let settings = AccentOptSettings::default();
        let result = optimize_accents(bg, &hues, &settings, 45.0); // Low contrast target

        for hr in &result.hue_results {
            assert!(
                hr.achieved_contrast >= 40.0, // Allow some slack
                "Hue {:.0} achieved {:.1} < 40.0",
                hr.hue,
                hr.achieved_contrast
            );
        }
    }

    #[test]
    fn optimize_accents_colors_in_gamut() {
        let bg = Srgb::new(26u8, 26, 46);
        let hues = [25.0, 60.0, 120.0, 180.0, 240.0, 285.0, 320.0, 350.0];
        let settings = AccentOptSettings::default();
        let result = optimize_accents(bg, &hues, &settings, 60.0);

        for hr in &result.hue_results {
            let color = hr.color;
            assert!(
                color.red >= 0.0 && color.red <= 1.0,
                "Hue {} red out of gamut: {}",
                hr.hue,
                color.red
            );
            assert!(
                color.green >= 0.0 && color.green <= 1.0,
                "Hue {} green out of gamut: {}",
                hr.hue,
                color.green
            );
            assert!(
                color.blue >= 0.0 && color.blue <= 1.0,
                "Hue {} blue out of gamut: {}",
                hr.hue,
                color.blue
            );
        }
    }
}
