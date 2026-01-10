//! APCA (Accessible Perceptual Contrast Algorithm) implementation.
//!
//! Calculates perceptual contrast between foreground and background colors
//! following the APCA-W3 specification for WCAG 3.0.

use crate::generated::{GAMMA_LUT, GAMMA_LUT_F32, GAMMA_LUT_F32_SIZE};
use palette::Srgb;

/// APCA luminance coefficients for sRGB D65
const COEF_R: f64 = 0.2126729;
const COEF_G: f64 = 0.7151522;
const COEF_B: f64 = 0.0721750;

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
    let y = COEF_R * GAMMA_LUT[color.red as usize]
        + COEF_G * GAMMA_LUT[color.green as usize]
        + COEF_B * GAMMA_LUT[color.blue as usize];

    // Low-luminance soft clamp
    if y < LOW_Y_THRESHOLD {
        y + (LOW_Y_THRESHOLD - y).powf(LOW_Y_EXPONENT)
    } else {
        y
    }
}

/// Look up gamma-corrected value from f32 LUT with linear interpolation.
#[inline]
fn gamma_f32(x: f32) -> f64 {
    let x = x.clamp(0.0, 1.0);
    let scale = (GAMMA_LUT_F32_SIZE - 1) as f32;
    let idx_f = x * scale;
    let idx = idx_f as usize;

    if idx >= GAMMA_LUT_F32_SIZE - 1 {
        GAMMA_LUT_F32[GAMMA_LUT_F32_SIZE - 1]
    } else {
        let t = (idx_f - idx as f32) as f64;
        GAMMA_LUT_F32[idx] * (1.0 - t) + GAMMA_LUT_F32[idx + 1] * t
    }
}

/// Convert an sRGB f32 color (0.0-1.0) to APCA luminance (Y).
///
/// Uses a 4096-entry LUT with linear interpolation for fast, accurate
/// gamma correction without u8 precision loss.
#[inline]
pub fn srgb_f32_to_luminance(color: Srgb<f32>) -> f64 {
    let y = COEF_R * gamma_f32(color.red)
        + COEF_G * gamma_f32(color.green)
        + COEF_B * gamma_f32(color.blue);

    // Low-luminance soft clamp
    if y < LOW_Y_THRESHOLD {
        y + (LOW_Y_THRESHOLD - y).powf(LOW_Y_EXPONENT)
    } else {
        y
    }
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

/// Compute APCA contrast from pre-computed luminance values.
/// Use when background luminance is fixed across many foreground evaluations.
pub fn contrast_from_luminances(y_fg: f64, y_bg: f64) -> f64 {
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

/// APCA contrast thresholds for different use cases.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Threshold {
    pub min_lc: f64,
    pub description: &'static str,
}

/// Predefined APCA thresholds
pub mod thresholds {
    use super::Threshold;

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
}
