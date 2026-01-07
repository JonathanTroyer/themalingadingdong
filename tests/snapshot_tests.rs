//! Snapshot tests for palette generation.
//!
//! Uses insta for YAML snapshot testing to capture full palette generation.
//! Assumes current generation logic is correct - these tests detect regressions.

use palette::Srgb;
use serde::Serialize;
use themalingadingdong::curves::InterpolationConfig;
use themalingadingdong::generate::{GenerateConfig, generate};
use themalingadingdong::generated::{
    CUSP_LUT, ECCENTRICITY_CAM16_LUT, ECCENTRICITY_LUT, GAMMA_LUT, HK_HUE_LUT,
};

/// Serializable palette representation for snapshots.
#[derive(Debug, Serialize)]
struct PaletteSnapshot {
    name: String,
    variant: String,
    base00: String,
    base01: String,
    base02: String,
    base03: String,
    base04: String,
    base05: String,
    base06: String,
    base07: String,
    base08: String,
    base09: String,
    base0a: String,
    base0b: String,
    base0c: String,
    base0d: String,
    base0e: String,
    base0f: String,
    base10: String,
    base11: String,
    base12: String,
    base13: String,
    base14: String,
    base15: String,
    base16: String,
    base17: String,
}

impl PaletteSnapshot {
    fn from_scheme(scheme: &tinted_builder::Base16Scheme) -> Self {
        let get_hex = |name: &str| {
            scheme
                .palette
                .get(name)
                .map(|c| format!("#{}{}{}", c.hex.0, c.hex.1, c.hex.2))
                .unwrap_or_else(|| "missing".to_string())
        };

        Self {
            name: scheme.name.clone(),
            variant: format!("{:?}", scheme.variant),
            base00: get_hex("base00"),
            base01: get_hex("base01"),
            base02: get_hex("base02"),
            base03: get_hex("base03"),
            base04: get_hex("base04"),
            base05: get_hex("base05"),
            base06: get_hex("base06"),
            base07: get_hex("base07"),
            base08: get_hex("base08"),
            base09: get_hex("base09"),
            base0a: get_hex("base0A"),
            base0b: get_hex("base0B"),
            base0c: get_hex("base0C"),
            base0d: get_hex("base0D"),
            base0e: get_hex("base0E"),
            base0f: get_hex("base0F"),
            base10: get_hex("base10"),
            base11: get_hex("base11"),
            base12: get_hex("base12"),
            base13: get_hex("base13"),
            base14: get_hex("base14"),
            base15: get_hex("base15"),
            base16: get_hex("base16"),
            base17: get_hex("base17"),
        }
    }
}

fn default_config() -> GenerateConfig {
    GenerateConfig {
        hue_overrides: [None; 8],
        min_contrast: 75.0,
        extended_min_contrast: 60.0,
        max_lightness_adjustment: 2.0,
        interpolation: InterpolationConfig::default(),
        ..Default::default()
    }
}

// ============================================================================
// Standard palettes
// ============================================================================

#[test]
fn snapshot_dark_palette() {
    let config = GenerateConfig {
        background: Srgb::new(0x1a_u8, 0x1a, 0x2e), // #1a1a2e
        foreground: Srgb::new(0xea_u8, 0xea, 0xea), // #eaeaea
        name: "Dark Palette".to_string(),
        author: Some("Snapshot Test".to_string()),
        ..default_config()
    };

    let result = generate(&config);
    let snapshot = PaletteSnapshot::from_scheme(&result.scheme);
    insta::assert_yaml_snapshot!("dark_palette", snapshot);
}

#[test]
fn snapshot_light_palette() {
    let config = GenerateConfig {
        background: Srgb::new(0xf0_u8, 0xf0, 0xf0), // #f0f0f0
        foreground: Srgb::new(0x20_u8, 0x20, 0x20), // #202020
        name: "Light Palette".to_string(),
        author: Some("Snapshot Test".to_string()),
        ..default_config()
    };

    let result = generate(&config);
    let snapshot = PaletteSnapshot::from_scheme(&result.scheme);
    insta::assert_yaml_snapshot!("light_palette", snapshot);
}

// ============================================================================
// Pathologic palettes - edge cases and stress tests
// ============================================================================

#[test]
fn snapshot_pure_black_background() {
    let config = GenerateConfig {
        background: Srgb::new(0x00_u8, 0x00, 0x00), // #000000
        foreground: Srgb::new(0xff_u8, 0xff, 0xff), // #ffffff
        name: "Pure Black".to_string(),
        author: Some("Snapshot Test".to_string()),
        ..default_config()
    };

    let result = generate(&config);
    let snapshot = PaletteSnapshot::from_scheme(&result.scheme);
    insta::assert_yaml_snapshot!("pure_black_background", snapshot);
}

#[test]
fn snapshot_pure_white_background() {
    let config = GenerateConfig {
        background: Srgb::new(0xff_u8, 0xff, 0xff), // #ffffff
        foreground: Srgb::new(0x00_u8, 0x00, 0x00), // #000000
        name: "Pure White".to_string(),
        author: Some("Snapshot Test".to_string()),
        ..default_config()
    };

    let result = generate(&config);
    let snapshot = PaletteSnapshot::from_scheme(&result.scheme);
    insta::assert_yaml_snapshot!("pure_white_background", snapshot);
}

#[test]
fn snapshot_saturated_purple_background() {
    let config = GenerateConfig {
        background: Srgb::new(0x2d_u8, 0x00, 0x4d), // #2d004d - deep purple
        foreground: Srgb::new(0xf0_u8, 0xe0, 0xff), // #f0e0ff - light lavender
        name: "Saturated Purple".to_string(),
        author: Some("Snapshot Test".to_string()),
        ..default_config()
    };

    let result = generate(&config);
    let snapshot = PaletteSnapshot::from_scheme(&result.scheme);
    insta::assert_yaml_snapshot!("saturated_purple_background", snapshot);
}

#[test]
fn snapshot_saturated_red_background() {
    let config = GenerateConfig {
        background: Srgb::new(0x4d_u8, 0x00, 0x00), // #4d0000 - deep red
        foreground: Srgb::new(0xff_u8, 0xe0, 0xe0), // #ffe0e0 - light pink
        name: "Saturated Red".to_string(),
        author: Some("Snapshot Test".to_string()),
        ..default_config()
    };

    let result = generate(&config);
    let snapshot = PaletteSnapshot::from_scheme(&result.scheme);
    insta::assert_yaml_snapshot!("saturated_red_background", snapshot);
}

#[test]
fn snapshot_low_contrast_input() {
    let config = GenerateConfig {
        background: Srgb::new(0x40_u8, 0x40, 0x40), // #404040
        foreground: Srgb::new(0x60_u8, 0x60, 0x60), // #606060 - only 32 steps difference
        name: "Low Contrast Input".to_string(),
        author: Some("Snapshot Test".to_string()),
        ..default_config()
    };

    let result = generate(&config);
    let snapshot = PaletteSnapshot::from_scheme(&result.scheme);
    insta::assert_yaml_snapshot!("low_contrast_input", snapshot);
}

#[test]
fn snapshot_custom_hue_overrides() {
    let config = GenerateConfig {
        background: Srgb::new(0x1a_u8, 0x1a, 0x2e),
        foreground: Srgb::new(0xea_u8, 0xea, 0xea),
        hue_overrides: [
            Some(0.0),   // base08: pure red
            Some(30.0),  // base09: orange
            Some(60.0),  // base0A: yellow
            Some(120.0), // base0B: green
            Some(180.0), // base0C: cyan
            Some(240.0), // base0D: blue
            Some(270.0), // base0E: purple
            Some(330.0), // base0F: magenta
        ],
        name: "Custom Hues".to_string(),
        author: Some("Snapshot Test".to_string()),
        ..default_config()
    };

    let result = generate(&config);
    let snapshot = PaletteSnapshot::from_scheme(&result.scheme);
    insta::assert_yaml_snapshot!("custom_hue_overrides", snapshot);
}

#[test]
fn snapshot_high_colorfulness() {
    use themalingadingdong::config::AccentOptSettings;
    let config = GenerateConfig {
        background: Srgb::new(0x1a_u8, 0x1a, 0x2e),
        foreground: Srgb::new(0xea_u8, 0xea, 0xea),
        accent_opt: AccentOptSettings {
            target_m: 45.0, // much higher than default 25
            ..Default::default()
        },
        extended_accent_opt: AccentOptSettings {
            target_m: 55.0, // much higher than default 35
            ..Default::default()
        },
        name: "High Colorfulness".to_string(),
        author: Some("Snapshot Test".to_string()),
        ..default_config()
    };

    let result = generate(&config);
    let snapshot = PaletteSnapshot::from_scheme(&result.scheme);
    insta::assert_yaml_snapshot!("high_colorfulness", snapshot);
}

#[test]
fn snapshot_low_colorfulness() {
    use themalingadingdong::config::AccentOptSettings;
    let config = GenerateConfig {
        background: Srgb::new(0x1a_u8, 0x1a, 0x2e),
        foreground: Srgb::new(0xea_u8, 0xea, 0xea),
        accent_opt: AccentOptSettings {
            target_m: 10.0, // much lower than default 25
            ..Default::default()
        },
        extended_accent_opt: AccentOptSettings {
            target_m: 15.0, // much lower than default 35
            ..Default::default()
        },
        name: "Low Colorfulness".to_string(),
        author: Some("Snapshot Test".to_string()),
        ..default_config()
    };

    let result = generate(&config);
    let snapshot = PaletteSnapshot::from_scheme(&result.scheme);
    insta::assert_yaml_snapshot!("low_colorfulness", snapshot);
}

#[test]
fn snapshot_strict_contrast() {
    let config = GenerateConfig {
        background: Srgb::new(0x1a_u8, 0x1a, 0x2e),
        foreground: Srgb::new(0xea_u8, 0xea, 0xea),
        min_contrast: 90.0,          // very strict
        extended_min_contrast: 80.0, // stricter than default
        name: "Strict Contrast".to_string(),
        author: Some("Snapshot Test".to_string()),
        ..default_config()
    };

    let result = generate(&config);
    let snapshot = PaletteSnapshot::from_scheme(&result.scheme);
    insta::assert_yaml_snapshot!("strict_contrast", snapshot);
}

#[test]
fn snapshot_relaxed_contrast() {
    let config = GenerateConfig {
        background: Srgb::new(0x1a_u8, 0x1a, 0x2e),
        foreground: Srgb::new(0xea_u8, 0xea, 0xea),
        min_contrast: 45.0,          // very relaxed
        extended_min_contrast: 30.0, // very relaxed
        name: "Relaxed Contrast".to_string(),
        author: Some("Snapshot Test".to_string()),
        ..default_config()
    };

    let result = generate(&config);
    let snapshot = PaletteSnapshot::from_scheme(&result.scheme);
    insta::assert_yaml_snapshot!("relaxed_contrast", snapshot);
}

#[test]
fn snapshot_solarized_dark_like() {
    let config = GenerateConfig {
        background: Srgb::new(0x00_u8, 0x2b, 0x36), // #002b36 - Solarized base03
        foreground: Srgb::new(0x83_u8, 0x94, 0x96), // #839496 - Solarized base0
        name: "Solarized Dark Like".to_string(),
        author: Some("Snapshot Test".to_string()),
        ..default_config()
    };

    let result = generate(&config);
    let snapshot = PaletteSnapshot::from_scheme(&result.scheme);
    insta::assert_yaml_snapshot!("solarized_dark_like", snapshot);
}

#[test]
fn snapshot_solarized_light_like() {
    let config = GenerateConfig {
        background: Srgb::new(0xfd_u8, 0xf6, 0xe3), // #fdf6e3 - Solarized base3
        foreground: Srgb::new(0x65_u8, 0x7b, 0x83), // #657b83 - Solarized base00
        name: "Solarized Light Like".to_string(),
        author: Some("Snapshot Test".to_string()),
        ..default_config()
    };

    let result = generate(&config);
    let snapshot = PaletteSnapshot::from_scheme(&result.scheme);
    insta::assert_yaml_snapshot!("solarized_light_like", snapshot);
}

// ============================================================================
// LUT snapshots - compile-time generated lookup tables
// ============================================================================

#[test]
fn snapshot_gamma_lut() {
    let values: Vec<String> = GAMMA_LUT.iter().map(|v| format!("{:.10}", v)).collect();
    insta::assert_yaml_snapshot!("gamma_lut", values);
}

#[test]
fn snapshot_eccentricity_lut() {
    let values: Vec<String> = ECCENTRICITY_LUT
        .iter()
        .map(|v| format!("{:.10}", v))
        .collect();
    insta::assert_yaml_snapshot!("eccentricity_lut", values);
}

#[test]
fn snapshot_eccentricity_cam16_lut() {
    let values: Vec<String> = ECCENTRICITY_CAM16_LUT
        .iter()
        .map(|v| format!("{:.10}", v))
        .collect();
    insta::assert_yaml_snapshot!("eccentricity_cam16_lut", values);
}

#[test]
fn snapshot_hk_hue_lut() {
    let values: Vec<String> = HK_HUE_LUT.iter().map(|v| format!("{:.10}", v)).collect();
    insta::assert_yaml_snapshot!("hk_hue_lut", values);
}

#[test]
fn snapshot_cusp_lut() {
    let values: Vec<String> = CUSP_LUT
        .iter()
        .map(|(j, m)| format!("({:.6}, {:.6})", j, m))
        .collect();
    insta::assert_yaml_snapshot!("cusp_lut", values);
}
