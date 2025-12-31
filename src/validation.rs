//! Palette validation with APCA contrast checking.

use float_cmp::approx_eq;
use palette::{IntoColor, Oklch, Srgb};
use tinted_builder::Base16Scheme;

use crate::apca::{Threshold, apca_contrast, thresholds};

/// A color pair that should be validated for contrast.
#[derive(Debug, Clone)]
pub struct ValidationPair {
    pub foreground: &'static str,
    pub background: &'static str,
    pub threshold: Threshold,
}

/// Result of validating a single color pair.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub pair: ValidationPair,
    pub contrast: f64,
    pub passes: bool,
    /// OKLCH values of the foreground color (only for accent colors base08-base17).
    pub fg_oklch: Option<Oklch>,
}

/// Get the default validation pairs for a base16/24 scheme.
pub fn default_validation_pairs() -> Vec<ValidationPair> {
    let mut pairs = Vec::new();

    // Foreground colors (base06-base07) on dark backgrounds (base00-base01)
    for fg in ["base06", "base07"] {
        for bg in ["base00", "base01"] {
            pairs.push(ValidationPair {
                foreground: fg,
                background: bg,
                threshold: thresholds::BODY_TEXT_MIN,
            });
        }
    }

    // Accent colors (base08-base0F) on backgrounds (base00-base01)
    // Solver optimizes against base00; base01 shown for reference
    for fg in [
        "base08", "base09", "base0A", "base0B", "base0C", "base0D", "base0E", "base0F",
    ] {
        for bg in ["base00", "base01"] {
            pairs.push(ValidationPair {
                foreground: fg,
                background: bg,
                threshold: thresholds::CONTENT_TEXT,
            });
        }
    }

    // Extended accent colors (base10-base17) on backgrounds (base00-base01)
    for fg in [
        "base10", "base11", "base12", "base13", "base14", "base15", "base16", "base17",
    ] {
        for bg in ["base00", "base01"] {
            pairs.push(ValidationPair {
                foreground: fg,
                background: bg,
                threshold: thresholds::CONTENT_TEXT,
            });
        }
    }

    pairs
}

/// Check if a color name is an accent color (base08-base0F or base10-base17).
fn is_accent_color(name: &str) -> bool {
    matches!(
        name,
        "base08"
            | "base09"
            | "base0A"
            | "base0B"
            | "base0C"
            | "base0D"
            | "base0E"
            | "base0F"
            | "base10"
            | "base11"
            | "base12"
            | "base13"
            | "base14"
            | "base15"
            | "base16"
            | "base17"
    )
}

/// Validate a scheme and return results for all pairs.
pub fn validate(scheme: &Base16Scheme) -> Vec<ValidationResult> {
    default_validation_pairs()
        .into_iter()
        .map(|pair| {
            let fg_color = scheme.palette.get(pair.foreground);
            let bg_color = scheme.palette.get(pair.background);

            match (fg_color, bg_color) {
                (Some(fg), Some(bg)) => {
                    let fg_srgb = Srgb::new(fg.rgb.0, fg.rgb.1, fg.rgb.2);
                    let bg_srgb = Srgb::new(bg.rgb.0, bg.rgb.1, bg.rgb.2);
                    let contrast = apca_contrast(fg_srgb, bg_srgb);
                    let abs_contrast = contrast.abs();
                    let threshold = pair.threshold.min_lc;
                    // Pass if contrast >= threshold (with epsilon matching display precision)
                    let passes = abs_contrast > threshold
                        || approx_eq!(f64, abs_contrast, threshold, epsilon = 0.5);

                    // Compute OKLCH for accent colors
                    let fg_oklch = if is_accent_color(pair.foreground) {
                        let fg_f32: Srgb<f32> = fg_srgb.into_format();
                        Some(fg_f32.into_color())
                    } else {
                        None
                    };

                    ValidationResult {
                        pair,
                        contrast,
                        passes,
                        fg_oklch,
                    }
                }
                _ => ValidationResult {
                    pair,
                    contrast: 0.0,
                    passes: false,
                    fg_oklch: None,
                },
            }
        })
        .collect()
}

/// Validate a scheme and return warnings for any failing color pairs.
pub fn validate_with_warnings(scheme: &Base16Scheme) -> Vec<String> {
    validate(scheme)
        .into_iter()
        .filter(|r| !r.passes)
        .map(|r| {
            format!(
                "{} on {}: Lc={:.1} (required: {:.0} for {})",
                r.pair.foreground,
                r.pair.background,
                r.contrast.abs(),
                r.pair.threshold.min_lc,
                r.pair.threshold.description
            )
        })
        .collect()
}
