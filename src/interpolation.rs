//! HellwigJmh color interpolation and utilities.

use palette::Srgb;

use crate::contrast_solver::find_uniform_post_clamp_lightness;
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
    /// Whether color is within sRGB gamut
    pub is_in_gamut: bool,
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

    // Convert to HellwigJmh for perceptually uniform interpolation
    let start_u8 = srgb_to_u8(start);
    let end_u8 = srgb_to_u8(end);
    let start_hellwig = HellwigJmh::from_srgb_u8(start_u8);
    let end_hellwig = HellwigJmh::from_srgb_u8(end_u8);

    (0..steps)
        .map(|i| {
            let linear_t = i as f32 / (steps - 1) as f32;

            // Apply different curves to each component
            let t_j = evaluate_curve(&curves.lightness, linear_t);
            let t_m = evaluate_curve(&curves.chroma, linear_t);
            let t_h = evaluate_curve(&curves.hue, linear_t);

            // Interpolate each component separately
            let j = lerp(start_hellwig.lightness, end_hellwig.lightness, t_j);
            let m = lerp(start_hellwig.colorfulness, end_hellwig.colorfulness, t_m);
            let h = lerp_hue(start_hellwig.hue, end_hellwig.hue, t_h);

            let interpolated = HellwigJmh::new(j, m, h);
            interpolated.into_srgb()
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

/// Generate accent colors with uniform lightness optimization.
///
/// All hues share a near-uniform lightness while meeting minimum contrast.
/// This produces more visually cohesive accent colors compared to per-hue
/// exact contrast optimization.
///
/// # Arguments
///
/// * `hues` - Array of hue values (from `build_hues_with_overrides()`)
/// * `colorfulness` - Colorfulness for all accents (HellwigJmh M)
/// * `min_contrast` - Minimum APCA Lc value (floor, not exact target)
/// * `max_adjustment` - Maximum per-hue lightness adjustment allowed (0-10 on J' scale)
/// * `background` - Background color for contrast calculation
///
/// # Returns
///
/// Vector of `AccentResult` with uniform-lightness colors and any warnings.
///
/// # Example
///
/// ```
/// use palette::Srgb;
/// use themalingadingdong::interpolation::{build_hues_with_overrides, generate_accents_uniform};
///
/// let bg = Srgb::new(26u8, 26, 46);
/// let hues = build_hues_with_overrides(&[None; 8]);
/// let accents = generate_accents_uniform(&hues, 25.0, 60.0, 2.0, bg);
/// assert_eq!(accents.len(), 8);
/// ```
pub fn generate_accents_uniform(
    hues: &[f32],
    colorfulness: f32,
    min_contrast: f64,
    max_adjustment: f32,
    background: Srgb<u8>,
) -> Vec<AccentResult> {
    let result = find_uniform_post_clamp_lightness(
        background,
        hues,
        colorfulness,
        min_contrast,
        max_adjustment,
    );

    result
        .hue_results
        .into_iter()
        .map(|hr| {
            let hellwig = HellwigJmh::new(hr.input_j, colorfulness, hr.hue);
            let color = hellwig.into_srgb();

            AccentResult {
                color,
                hue: hr.hue,
                lightness: hr.input_j,
                post_clamp_lightness: hr.post_clamp_j,
                j_deviation: hr.j_deviation,
                achieved_contrast: hr.achieved_contrast,
                met_minimum: hr.met_minimum_contrast,
                is_in_gamut: hr.is_in_gamut,
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

/// Get HellwigJmh lightness (perceptually uniform, 0-100).
pub fn hellwig_lightness(color: Srgb<u8>) -> f32 {
    HellwigJmh::from_srgb_u8(color).lightness
}

/// Convert sRGB to HellwigJmh components (lightness, colorfulness, hue).
pub fn srgb_to_hellwig(color: Srgb<u8>) -> (f32, f32, f32) {
    let hellwig = HellwigJmh::from_srgb_u8(color);
    (hellwig.lightness, hellwig.colorfulness, hellwig.hue)
}
