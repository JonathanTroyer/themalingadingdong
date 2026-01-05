//! Palette validation with APCA contrast checking.

use float_cmp::approx_eq;
use palette::Srgb;
use tinted_builder::Base16Scheme;

use crate::apca::{Threshold, apca_contrast, thresholds};
use crate::hellwig::HellwigJmh;

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
    /// HellwigJmh values of the foreground color (only for accent colors base08-base17).
    pub fg_hellwig: Option<HellwigJmh>,
}

/// Get required validation pairs (must pass for scheme to be valid).
///
/// - UI colors (base06-base07) must pass on both base00 and base01
/// - Accent colors only need to pass on base00 (solver targets base00 only)
pub fn required_validation_pairs() -> Vec<ValidationPair> {
    let mut pairs = Vec::new();

    // UI colors must pass on both backgrounds
    for fg in ["base06", "base07"] {
        for bg in ["base00", "base01"] {
            pairs.push(ValidationPair {
                foreground: fg,
                background: bg,
                threshold: thresholds::BODY_TEXT_MIN,
            });
        }
    }

    // Accent colors only need to pass on base00
    for fg in [
        "base08", "base09", "base0A", "base0B", "base0C", "base0D", "base0E", "base0F",
    ] {
        pairs.push(ValidationPair {
            foreground: fg,
            background: "base00",
            threshold: thresholds::CONTENT_TEXT,
        });
    }

    // Extended accent colors only need to pass on base00
    for fg in [
        "base10", "base11", "base12", "base13", "base14", "base15", "base16", "base17",
    ] {
        pairs.push(ValidationPair {
            foreground: fg,
            background: "base00",
            threshold: thresholds::CONTENT_TEXT,
        });
    }

    pairs
}

/// Get reference validation pairs (informational, shown but not required).
///
/// These show contrast against base01 for accent colors.
pub fn reference_validation_pairs() -> Vec<ValidationPair> {
    let mut pairs = Vec::new();

    // Accent colors on base01 (reference only)
    for fg in [
        "base08", "base09", "base0A", "base0B", "base0C", "base0D", "base0E", "base0F",
    ] {
        pairs.push(ValidationPair {
            foreground: fg,
            background: "base01",
            threshold: thresholds::CONTENT_TEXT,
        });
    }

    // Extended accent colors on base01 (reference only)
    for fg in [
        "base10", "base11", "base12", "base13", "base14", "base15", "base16", "base17",
    ] {
        pairs.push(ValidationPair {
            foreground: fg,
            background: "base01",
            threshold: thresholds::CONTENT_TEXT,
        });
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

/// Combined validation results with required and reference checks separated.
#[derive(Debug, Clone)]
pub struct ValidationResults {
    /// Results that must pass for scheme to be valid.
    pub required: Vec<ValidationResult>,
    /// Results shown for reference only (informational).
    pub reference: Vec<ValidationResult>,
}

/// Validate a scheme and return separated required/reference results.
pub fn validate(scheme: &Base16Scheme) -> ValidationResults {
    let validate_pairs = |pairs: Vec<ValidationPair>| -> Vec<ValidationResult> {
        pairs
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

                        // Compute HellwigJmh for accent colors
                        let fg_hellwig = if is_accent_color(pair.foreground) {
                            Some(HellwigJmh::from_srgb_u8(fg_srgb))
                        } else {
                            None
                        };

                        ValidationResult {
                            pair,
                            contrast,
                            passes,
                            fg_hellwig,
                        }
                    }
                    _ => ValidationResult {
                        pair,
                        contrast: 0.0,
                        passes: false,
                        fg_hellwig: None,
                    },
                }
            })
            .collect()
    };

    ValidationResults {
        required: validate_pairs(required_validation_pairs()),
        reference: validate_pairs(reference_validation_pairs()),
    }
}

/// Validate a scheme and return warnings for any failing required pairs.
pub fn validate_with_warnings(scheme: &Base16Scheme) -> Vec<String> {
    validate(scheme)
        .required
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
