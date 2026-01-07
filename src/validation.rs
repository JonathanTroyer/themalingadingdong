//! Palette validation with APCA contrast checking.

use float_cmp::approx_eq;
use palette::Srgb;
use tinted_builder::Base16Scheme;
use tracing::warn;

#[cfg(debug_assertions)]
use tracing::instrument;

use crate::apca::{Threshold, apca_contrast, thresholds};
use crate::hellwig::HellwigJmh;
use crate::interpolation::AccentResult;

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
    /// Whether the accent color was gamut mapped (only for accent colors base08-base17).
    pub was_gamut_mapped: bool,
    /// Whether M is within bounds after gamut mapping (only for accent colors base08-base17).
    pub m_in_bounds: bool,
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

/// Accent color names (base08-base0F and base10-base17).
const ACCENT_COLORS: [&str; 16] = [
    "base08", "base09", "base0A", "base0B", "base0C", "base0D", "base0E", "base0F", "base10",
    "base11", "base12", "base13", "base14", "base15", "base16", "base17",
];

/// Check if a color name is an accent color (base08-base0F or base10-base17).
fn is_accent_color(name: &str) -> bool {
    ACCENT_COLORS.contains(&name)
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
#[cfg_attr(debug_assertions, instrument(skip(scheme), fields(scheme_name = %scheme.name)))]
pub fn validate(scheme: &Base16Scheme) -> ValidationResults {
    validate_with_accent_data(scheme, &[], &[])
}

/// Validate a scheme with accent result data for gamut mapping detection.
pub fn validate_with_accent_data(
    scheme: &Base16Scheme,
    base_accent_results: &[AccentResult],
    extended_accent_results: &[AccentResult],
) -> ValidationResults {
    // Helper to get AccentResult for a color name
    let get_accent_result = |fg_name: &str| -> Option<&AccentResult> {
        match fg_name {
            "base08" => base_accent_results.first(),
            "base09" => base_accent_results.get(1),
            "base0A" => base_accent_results.get(2),
            "base0B" => base_accent_results.get(3),
            "base0C" => base_accent_results.get(4),
            "base0D" => base_accent_results.get(5),
            "base0E" => base_accent_results.get(6),
            "base0F" => base_accent_results.get(7),
            "base10" => extended_accent_results.first(),
            "base11" => extended_accent_results.get(1),
            "base12" => extended_accent_results.get(2),
            "base13" => extended_accent_results.get(3),
            "base14" => extended_accent_results.get(4),
            "base15" => extended_accent_results.get(5),
            "base16" => extended_accent_results.get(6),
            "base17" => extended_accent_results.get(7),
            _ => None,
        }
    };

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
                        let passes = abs_contrast > threshold
                            || approx_eq!(f64, abs_contrast, threshold, epsilon = 0.5);

                        let fg_hellwig = if is_accent_color(pair.foreground) {
                            Some(HellwigJmh::from_srgb_u8(fg_srgb))
                        } else {
                            None
                        };

                        let accent_result = get_accent_result(pair.foreground);
                        let was_gamut_mapped =
                            accent_result.map(|r| r.was_gamut_mapped).unwrap_or(false);
                        let m_in_bounds = accent_result.map(|r| r.m_in_bounds).unwrap_or(true);

                        ValidationResult {
                            pair,
                            contrast,
                            passes,
                            fg_hellwig,
                            was_gamut_mapped,
                            m_in_bounds,
                        }
                    }
                    _ => {
                        warn!(
                            fg = pair.foreground,
                            bg = pair.background,
                            "missing color in palette"
                        );
                        ValidationResult {
                            pair,
                            contrast: 0.0,
                            passes: false,
                            fg_hellwig: None,
                            was_gamut_mapped: false,
                            m_in_bounds: true,
                        }
                    }
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
