//! HellwigJmh color interpolation and utilities.

use palette::Srgb;

#[cfg(debug_assertions)]
use tracing::instrument;

use crate::accent_solver::optimize_accents;
use crate::config::AccentOptSettings;
use crate::curves::{InterpolationConfig, evaluate_curve};
use crate::hellwig::HellwigJmh;

/// Default hues for base16 accent colors (base08-base0F).
///
/// These values are based on analysis of popular base16 themes and follow
/// ANSI terminal color semantics:
/// - base08: Red
/// - base09: Orange
/// - base0A: Yellow
/// - base0B: Green
/// - base0C: Cyan
/// - base0D: Blue
/// - base0E: Purple
/// - base0F: Magenta
pub const DEFAULT_BASE16_HUES: [f32; 8] = [
    25.0,  // base08: Red
    55.0,  // base09: Orange
    90.0,  // base0A: Yellow
    145.0, // base0B: Green
    180.0, // base0C: Cyan
    250.0, // base0D: Blue
    285.0, // base0E: Purple
    335.0, // base0F: Magenta
];

/// Build the final hue array from defaults with optional overrides.
///
/// # Arguments
///
/// * `overrides` - Array of optional hue overrides. `None` values use the default.
///
/// # Example
///
/// ```
/// use themalingadingdong::interpolation::{build_hues_with_overrides, DEFAULT_BASE16_HUES};
///
/// // Override just base08 (red) to be more pink
/// let mut overrides = [None; 8];
/// overrides[0] = Some(340.0);
/// let hues = build_hues_with_overrides(&overrides);
/// assert_eq!(hues[0], 340.0);  // overridden
/// assert_eq!(hues[1], DEFAULT_BASE16_HUES[1]); // unchanged
/// ```
pub fn build_hues_with_overrides(overrides: &[Option<f32>; 8]) -> [f32; 8] {
    let mut hues = DEFAULT_BASE16_HUES;
    for (i, override_hue) in overrides.iter().enumerate() {
        if let Some(h) = override_hue {
            hues[i] = *h;
        }
    }
    hues
}

/// Result of generating an accent color with contrast optimization.
#[derive(Debug, Clone)]
pub struct AccentResult {
    /// The generated sRGB color
    pub color: Srgb<f32>,
    /// The hue used (degrees, 0-360)
    pub hue: f32,
    /// The input lightness for this hue (HellwigJmh J', 0-100)
    pub lightness: f32,
    /// The post-clamp lightness (actual perceived J' after gamut mapping)
    pub post_clamp_lightness: f32,
    /// Deviation of post-clamp J' from target (for uniform lightness mode)
    pub j_deviation: f32,
    /// The APCA contrast achieved
    pub achieved_contrast: f64,
    /// Whether minimum contrast was achieved
    pub met_minimum: bool,
    /// Whether color was gamut mapped (colorfulness reduced)
    pub was_gamut_mapped: bool,
    /// Whether M is within bounds after gamut mapping
    pub m_in_bounds: bool,
    /// Warning if minimum couldn't be achieved for this hue
    pub warning: Option<String>,
}

/// Interpolate between two colors in HellwigJmh space.
///
/// Interpolates all three components (lightness, colorfulness, hue) using
/// HellwigJmh's perceptually uniform space for smooth color transitions.
/// This is a convenience wrapper around `interpolate_with_curves` using linear curves.
///
/// # Arguments
///
/// * `start` - Starting color (typically the background)
/// * `end` - Ending color (typically the foreground)
/// * `steps` - Number of colors to generate (inclusive of start and end)
///
/// # Example
///
/// ```
/// use palette::Srgb;
/// use themalingadingdong::interpolation::interpolate_lightness;
///
/// let dark = Srgb::new(0.1f32, 0.1, 0.12);
/// let light = Srgb::new(0.9f32, 0.9, 0.88);
///
/// let colors = interpolate_lightness(dark, light, 8);
/// assert_eq!(colors.len(), 8);
/// ```
pub fn interpolate_lightness(start: Srgb<f32>, end: Srgb<f32>, steps: usize) -> Vec<Srgb<f32>> {
    interpolate_with_curves(start, end, steps, &InterpolationConfig::default())
}

/// Interpolate between two colors in HellwigJmh space with configurable curves.
///
/// Uses separate curve configurations for lightness, colorfulness, and hue components,
/// allowing non-linear easing for each channel independently.
///
/// # Arguments
///
/// * `start` - Starting color (typically the background)
/// * `end` - Ending color (typically the foreground)
/// * `steps` - Number of colors to generate (inclusive of start and end)
/// * `curves` - Interpolation configuration with curves for J, M, h
///
/// # Example
///
/// ```
/// use palette::Srgb;
/// use themalingadingdong::curves::{InterpolationConfig, CurveConfig, CurveType};
/// use themalingadingdong::interpolation::interpolate_with_curves;
///
/// let dark = Srgb::new(0.1f32, 0.1, 0.12);
/// let light = Srgb::new(0.9f32, 0.9, 0.88);
///
/// let mut curves = InterpolationConfig::default();
/// curves.lightness.curve_type = CurveType::Smoothstep;
///
/// let colors = interpolate_with_curves(dark, light, 8, &curves);
/// assert_eq!(colors.len(), 8);
/// ```
#[cfg_attr(debug_assertions, instrument(skip(start, end, curves), fields(steps)))]
pub fn interpolate_with_curves(
    start: Srgb<f32>,
    end: Srgb<f32>,
    steps: usize,
    curves: &InterpolationConfig,
) -> Vec<Srgb<f32>> {
    if steps == 0 {
        return vec![];
    }
    if steps == 1 {
        return vec![start];
    }

    let start_hellwig = HellwigJmh::from_srgb_u8(srgb_to_u8(start));
    let end_hellwig = HellwigJmh::from_srgb_u8(srgb_to_u8(end));

    (0..steps)
        .map(|i| {
            let linear_t = i as f32 / (steps - 1) as f32;

            let t_j = evaluate_curve(&curves.lightness, linear_t);
            let t_m = evaluate_curve(&curves.chroma, linear_t);
            let t_h = evaluate_curve(&curves.hue, linear_t);

            let j = lerp(start_hellwig.lightness, end_hellwig.lightness, t_j);
            let m = lerp(start_hellwig.colorfulness, end_hellwig.colorfulness, t_m);
            let h = lerp_hue(start_hellwig.hue, end_hellwig.hue, t_h);

            HellwigJmh::new(j, m, h).into_srgb()
        })
        .collect()
}

/// Linear interpolation helper.
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Hue interpolation with shortest-path handling.
fn lerp_hue(a: f32, b: f32, t: f32) -> f32 {
    let diff = b - a;
    let adjusted_diff = if diff > 180.0 {
        diff - 360.0
    } else if diff < -180.0 {
        diff + 360.0
    } else {
        diff
    };
    (a + adjusted_diff * t).rem_euclid(360.0)
}

/// Generate accent colors by rotating hue at constant lightness and colorfulness.
///
/// # Arguments
///
/// * `start_hue` - Starting hue in degrees (0-360)
/// * `lightness` - Constant lightness for all accents (HellwigJmh J', 0-100)
/// * `colorfulness` - Constant colorfulness for all accents (HellwigJmh M)
/// * `count` - Number of accent colors to generate
///
/// # Example
///
/// ```
/// use themalingadingdong::interpolation::generate_accent_hues;
///
/// let accents = generate_accent_hues(25.0, 70.0, 25.0, 8);
/// assert_eq!(accents.len(), 8);
/// ```
pub fn generate_accent_hues(
    start_hue: f32,
    lightness: f32,
    colorfulness: f32,
    count: usize,
) -> Vec<Srgb<f32>> {
    if count == 0 {
        return vec![];
    }

    let hue_step = 360.0 / count as f32;

    (0..count)
        .map(|i| {
            let hue = (start_hue + i as f32 * hue_step) % 360.0;
            let hellwig = HellwigJmh::new(lightness, colorfulness, hue);
            hellwig.into_srgb()
        })
        .collect()
}

/// Generate evenly-spaced hues starting from a given hue.
///
/// This is Step 1 of the two-step accent generation process.
/// Future enhancements could make hue spacing perceptually uniform
/// or avoid problematic gamut boundary hues.
///
/// # Arguments
///
/// * `start_hue` - Starting hue in degrees (0-360)
/// * `count` - Number of hues to generate
///
/// # Returns
///
/// Vector of hue values in degrees, evenly spaced around the color wheel.
///
/// # Example
///
/// ```
/// use themalingadingdong::interpolation::generate_hues;
///
/// let hues = generate_hues(25.0, 8);
/// assert_eq!(hues.len(), 8);
/// assert_eq!(hues[0], 25.0);  // First hue is the starting hue
/// ```
pub fn generate_hues(start_hue: f32, count: usize) -> Vec<f32> {
    if count == 0 {
        return vec![];
    }

    let hue_step = 360.0 / count as f32;

    (0..count)
        .map(|i| (start_hue + i as f32 * hue_step) % 360.0)
        .collect()
}

/// Generate accent colors with COBYLA optimization.
///
/// Optimizes (J', M) pairs for each hue using constrained optimization
/// to balance lightness uniformity against colorfulness preservation.
///
/// # Arguments
///
/// * `hues` - Array of hue values (from `build_hues_with_overrides()`)
/// * `settings` - Optimization settings (targets, tolerances, weight)
/// * `min_contrast` - Minimum APCA Lc value (floor, not exact target)
/// * `background` - Background color for contrast calculation
///
/// # Returns
///
/// Vector of `AccentResult` with optimized colors and any warnings.
///
/// # Example
///
/// ```
/// use palette::Srgb;
/// use themalingadingdong::config::AccentOptSettings;
/// use themalingadingdong::interpolation::{build_hues_with_overrides, generate_accents_uniform};
///
/// let bg = Srgb::new(26u8, 26, 46);
/// let hues = build_hues_with_overrides(&[None; 8]);
/// let settings = AccentOptSettings::default();
/// let accents = generate_accents_uniform(&hues, &settings, 60.0, bg);
/// assert_eq!(accents.len(), 8);
/// ```
#[cfg_attr(debug_assertions, instrument(skip(hues, settings, background), fields(hue_count = hues.len())))]
pub fn generate_accents_uniform(
    hues: &[f32],
    settings: &AccentOptSettings,
    min_contrast: f64,
    background: Srgb<u8>,
) -> Vec<AccentResult> {
    let result = optimize_accents(background, hues, settings, min_contrast);

    result
        .hue_results
        .into_iter()
        .map(|hr| {
            AccentResult {
                color: hr.color,
                hue: hr.hue,
                lightness: hr.j,
                post_clamp_lightness: hr.post_clamp_j,
                j_deviation: hr.j - settings.target_j,
                achieved_contrast: hr.achieved_contrast,
                met_minimum: hr.met_constraints,
                was_gamut_mapped: hr.original_m > hr.m + 0.8, // tolerance for numerical noise
                m_in_bounds: hr.m_in_bounds,
                warning: hr.warning,
            }
        })
        .collect()
}

/// Clamp an sRGB color to valid range [0, 1] for each channel.
///
/// HellwigJmh colors can produce out-of-gamut sRGB values, so clamping is necessary.
fn clamp_srgb(color: Srgb<f32>) -> Srgb<f32> {
    Srgb::new(
        color.red.clamp(0.0, 1.0),
        color.green.clamp(0.0, 1.0),
        color.blue.clamp(0.0, 1.0),
    )
}

/// Convert sRGB f32 to u8 with clamping for out-of-gamut colors.
pub fn srgb_to_u8(color: Srgb<f32>) -> Srgb<u8> {
    let c = clamp_srgb(color);
    Srgb::new(
        (c.red * 255.0).round() as u8,
        (c.green * 255.0).round() as u8,
        (c.blue * 255.0).round() as u8,
    )
}

/// Convert sRGB u8 to f32.
pub fn srgb_to_f32(color: Srgb<u8>) -> Srgb<f32> {
    Srgb::new(
        color.red as f32 / 255.0,
        color.green as f32 / 255.0,
        color.blue as f32 / 255.0,
    )
}

/// Convert sRGB to hex string (without # prefix).
pub fn srgb_to_hex(color: Srgb<u8>) -> String {
    format!("{:02x}{:02x}{:02x}", color.red, color.green, color.blue)
}

/// Convert sRGB to HellwigJmh components (lightness, colorfulness, hue).
pub fn srgb_to_hellwig(color: Srgb<u8>) -> (f32, f32, f32) {
    let hellwig = HellwigJmh::from_srgb_u8(color);
    (hellwig.lightness, hellwig.colorfulness, hellwig.hue)
}
