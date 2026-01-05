//! APCA (Accessible Perceptual Contrast Algorithm) implementation.
//!
//! Calculates perceptual contrast between foreground and background colors
//! following the APCA-W3 specification for WCAG 3.0.

use palette::Srgb;

/// APCA luminance coefficients for sRGB D65
const COEF_R: f64 = 0.2126729;
const COEF_G: f64 = 0.7151522;
const COEF_B: f64 = 0.0721750;

/// Gamma exponent for sRGB inverse companding
const GAMMA: f64 = 2.4;

/// Threshold for low-luminance soft clamp
const LOW_Y_THRESHOLD: f64 = 0.022;
const LOW_Y_EXPONENT: f64 = 1.414;

/// APCA contrast calculation constants
const SCALE: f64 = 1.14;
const OFFSET: f64 = 0.027;
const THRESHOLD: f64 = 0.1;

/// Exponents for light background (dark text on light bg)
const EXP_BG_LIGHT: f64 = 0.56;
const EXP_FG_LIGHT: f64 = 0.57;

/// Exponents for dark background (light text on dark bg)
const EXP_BG_DARK: f64 = 0.65;
const EXP_FG_DARK: f64 = 0.62;

/// Convert an sRGB color to APCA luminance (Y).
pub fn srgb_to_luminance(color: Srgb<u8>) -> f64 {
    let r_lin = (color.red as f64 / 255.0).powf(GAMMA);
    let g_lin = (color.green as f64 / 255.0).powf(GAMMA);
    let b_lin = (color.blue as f64 / 255.0).powf(GAMMA);

    let mut y = COEF_R * r_lin + COEF_G * g_lin + COEF_B * b_lin;

    // Low-luminance soft clamp
    if y < LOW_Y_THRESHOLD {
        y += (LOW_Y_THRESHOLD - y).powf(LOW_Y_EXPONENT);
    }

    y
}

/// Calculate APCA contrast (Lc) between foreground and background colors.
///
/// Returns the Lc value:
/// - Positive values indicate dark text on light background
/// - Negative values indicate light text on dark background
/// - Typical range: -108 to +105
///
/// # Example
///
/// ```
/// use palette::Srgb;
/// use themalingadingdong::apca::apca_contrast;
///
/// let black = Srgb::new(0u8, 0, 0);
/// let white = Srgb::new(255u8, 255, 255);
///
/// // Black text on white background
/// let lc = apca_contrast(black, white);
/// assert!(lc > 100.0);
///
/// // White text on black background
/// let lc = apca_contrast(white, black);
/// assert!(lc < -100.0);
/// ```
pub fn apca_contrast(fg: Srgb<u8>, bg: Srgb<u8>) -> f64 {
    let y_fg = srgb_to_luminance(fg);
    let y_bg = srgb_to_luminance(bg);

    let c = if y_bg > y_fg {
        // Light background, dark text (positive contrast)
        SCALE * (y_bg.powf(EXP_BG_LIGHT) - y_fg.powf(EXP_FG_LIGHT))
    } else {
        // Dark background, light text (negative contrast)
        SCALE * (y_bg.powf(EXP_BG_DARK) - y_fg.powf(EXP_FG_DARK))
    };

    // Apply threshold and offset
    if c.abs() < THRESHOLD {
        0.0
    } else if c > 0.0 {
        (c - OFFSET) * 100.0
    } else {
        (c + OFFSET) * 100.0
    }
}

/// Invert APCA to find foreground luminance (Y) for a target contrast.
///
/// Given a background luminance and target Lc, computes the foreground Y
/// that would achieve that contrast. This is useful for establishing
/// search bounds when solving for lightness.
///
/// # Arguments
///
/// * `bg_y` - Background luminance (Y), typically from `srgb_to_luminance()`
/// * `target_lc` - Target APCA contrast (absolute value, e.g., 75.0)
/// * `is_dark_bg` - True if background is dark (Y < 0.18), false if light
///
/// # Returns
///
/// `Some(fg_y)` if the target is achievable, `None` if physically impossible.
///
/// # Note
///
/// This inversion ignores the low-luminance soft clamp for simplicity,
/// so results near Y=0 may be approximate.
pub fn invert_apca_for_y(bg_y: f64, target_lc: f64, is_dark_bg: bool) -> Option<f64> {
    let lc_abs = target_lc.abs();

    if is_dark_bg {
        // Dark bg formula: Lc = SCALE * (Y_bg^EXP_BG_DARK - Y_fg^EXP_FG_DARK + OFFSET) * 100
        // Solving for Y_fg:
        // Y_fg^EXP_FG_DARK = Y_bg^EXP_BG_DARK + OFFSET - lc_abs / (SCALE * 100)
        let rhs = bg_y.powf(EXP_BG_DARK) + OFFSET - lc_abs / (SCALE * 100.0);

        if rhs <= 0.0 {
            return None; // Target unreachable
        }

        Some(rhs.powf(1.0 / EXP_FG_DARK))
    } else {
        // Light bg formula: Lc = SCALE * (Y_bg^EXP_BG_LIGHT - Y_fg^EXP_FG_LIGHT - OFFSET) * 100
        // Solving for Y_fg:
        // Y_fg^EXP_FG_LIGHT = Y_bg^EXP_BG_LIGHT - OFFSET - lc_abs / (SCALE * 100)
        let rhs = bg_y.powf(EXP_BG_LIGHT) - OFFSET - lc_abs / (SCALE * 100.0);

        if rhs <= 0.0 {
            return None; // Target unreachable
        }

        Some(rhs.powf(1.0 / EXP_FG_LIGHT))
    }
}

/// APCA contrast thresholds for different use cases.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Threshold {
    pub min_lc: f64,
    pub description: &'static str,
}

/// Predefined APCA thresholds
pub mod thresholds {
    use super::Threshold;

    /// Body text (preferred level) - Lc 90
    pub const BODY_TEXT: Threshold = Threshold {
        min_lc: 90.0,
        description: "Body text (preferred)",
    };

    /// Body text (minimum level) - Lc 75
    pub const BODY_TEXT_MIN: Threshold = Threshold {
        min_lc: 75.0,
        description: "Body text (minimum)",
    };

    /// Content text (non-body) - Lc 60
    pub const CONTENT_TEXT: Threshold = Threshold {
        min_lc: 60.0,
        description: "Content text",
    };

    /// Headlines and large text - Lc 45
    pub const HEADLINES: Threshold = Threshold {
        min_lc: 45.0,
        description: "Headlines",
    };

    /// UI components and controls - Lc 30
    pub const UI_COMPONENTS: Threshold = Threshold {
        min_lc: 30.0,
        description: "UI components",
    };
}
