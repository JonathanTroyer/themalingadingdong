//! OKLCH lightness interpolation and color utilities.

use palette::{IntoColor, Oklch, Srgb};

use crate::contrast_solver::solve_lightness_for_contrast;

/// Result of generating an accent color with per-hue contrast solving.
#[derive(Debug, Clone)]
pub struct AccentResult {
    /// The generated sRGB color
    pub color: Srgb<f32>,
    /// The hue used (degrees, 0-360)
    pub hue: f32,
    /// The lightness solved for this specific hue
    pub lightness: f32,
    /// The APCA contrast achieved
    pub achieved_contrast: f64,
    /// Whether target contrast was achieved exactly
    pub is_exact: bool,
    /// Warning if target couldn't be achieved for this hue
    pub warning: Option<String>,
}

/// Interpolate between two colors by varying only lightness.
///
/// Keeps hue and chroma from the darker color (lower lightness),
/// and generates `steps` colors with evenly spaced lightness values.
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
    if steps == 0 {
        return vec![];
    }
    if steps == 1 {
        return vec![start];
    }

    // Convert to OKLCH
    let start_oklch: Oklch<f32> = start.into_linear().into_color();
    let end_oklch: Oklch<f32> = end.into_linear().into_color();

    // Determine which is darker (lower lightness)
    let (darker, _lighter) = if start_oklch.l < end_oklch.l {
        (start_oklch, end_oklch)
    } else {
        (end_oklch, start_oklch)
    };

    // Use hue and chroma from the darker color
    let hue = darker.hue;
    let chroma = darker.chroma;

    // Handle the case where start is lighter than end
    let (l_start, l_end) = (start_oklch.l, end_oklch.l);

    (0..steps)
        .map(|i| {
            let t = i as f32 / (steps - 1) as f32;
            let l = l_start + (l_end - l_start) * t;

            let oklch = Oklch::new(l, chroma, hue);
            let linear_srgb: palette::LinSrgb<f32> = oklch.into_color();
            Srgb::from_linear(linear_srgb)
        })
        .collect()
}

/// Generate accent colors by rotating hue at constant lightness and chroma.
///
/// # Arguments
///
/// * `start_hue` - Starting hue in degrees (0-360)
/// * `lightness` - Constant lightness for all accents (0.0-1.0)
/// * `chroma` - Constant chroma for all accents (typically 0.1-0.2)
/// * `count` - Number of accent colors to generate
///
/// # Example
///
/// ```
/// use themalingadingdong::interpolation::generate_accent_hues;
///
/// let accents = generate_accent_hues(25.0, 0.7, 0.12, 8);
/// assert_eq!(accents.len(), 8);
/// ```
pub fn generate_accent_hues(
    start_hue: f32,
    lightness: f32,
    chroma: f32,
    count: usize,
) -> Vec<Srgb<f32>> {
    if count == 0 {
        return vec![];
    }

    let hue_step = 360.0 / count as f32;

    (0..count)
        .map(|i| {
            let hue = (start_hue + i as f32 * hue_step) % 360.0;
            let oklch = Oklch::new(lightness, chroma, hue);
            let linear_srgb: palette::LinSrgb<f32> = oklch.into_color();
            Srgb::from_linear(linear_srgb)
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

/// Generate accent colors with per-hue contrast solving.
///
/// This is Step 2 of the two-step accent generation process: for each hue,
/// solve for the lightness that achieves target contrast against the background.
///
/// # Arguments
///
/// * `hues` - Array of hue values (from `generate_hues()`)
/// * `chroma` - Chroma for all accents
/// * `target_contrast` - Target APCA Lc value
/// * `background` - Background color for contrast calculation
///
/// # Returns
///
/// Vector of `AccentResult` with color, metadata, and any warnings.
///
/// # Example
///
/// ```
/// use palette::Srgb;
/// use themalingadingdong::interpolation::{generate_hues, generate_accents_for_contrast};
///
/// let bg = Srgb::new(26u8, 26, 46);
/// let hues = generate_hues(25.0, 8);
/// let accents = generate_accents_for_contrast(&hues, 0.12, 60.0, bg);
/// assert_eq!(accents.len(), 8);
/// ```
pub fn generate_accents_for_contrast(
    hues: &[f32],
    chroma: f32,
    target_contrast: f64,
    background: Srgb<u8>,
) -> Vec<AccentResult> {
    hues.iter()
        .map(|&hue| {
            let solve_result =
                solve_lightness_for_contrast(background, target_contrast, hue, chroma);

            let oklch = Oklch::new(solve_result.lightness, chroma, hue);
            let linear_srgb: palette::LinSrgb<f32> = oklch.into_color();
            let color = Srgb::from_linear(linear_srgb);

            AccentResult {
                color,
                hue,
                lightness: solve_result.lightness,
                achieved_contrast: solve_result.achieved_contrast,
                is_exact: solve_result.is_exact,
                warning: solve_result.warning,
            }
        })
        .collect()
}

/// Clamp an sRGB color to valid range [0, 1] for each channel.
///
/// OKLCH colors can produce out-of-gamut sRGB values, so clamping is necessary.
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

/// Get OKLCH lightness (perceptually uniform, 0.0-1.0).
pub fn oklch_lightness(color: Srgb<u8>) -> f32 {
    let srgb = srgb_to_f32(color);
    let oklch: Oklch<f32> = srgb.into_linear().into_color();
    oklch.l
}
