use palette::Srgb;
use themalingadingdong::generate::{GenerateConfig, generate, parse_hex};
use themalingadingdong::interpolation::{generate_accent_hues, interpolate_lightness};

#[test]
fn test_interpolate_lightness_generates_correct_count() {
    let dark = Srgb::new(0.1f32, 0.1, 0.12);
    let light = Srgb::new(0.9f32, 0.9, 0.88);

    let colors = interpolate_lightness(dark, light, 8);
    assert_eq!(colors.len(), 8);
}

#[test]
fn test_interpolate_lightness_endpoints() {
    let dark = Srgb::new(0.1f32, 0.1, 0.1);
    let light = Srgb::new(0.9f32, 0.9, 0.9);

    let colors = interpolate_lightness(dark, light, 8);

    // First color should be similar to dark
    assert!((colors[0].red - 0.1).abs() < 0.1);

    // Last color should be similar to light
    assert!((colors[7].red - 0.9).abs() < 0.1);
}

#[test]
fn test_interpolate_lightness_monotonic() {
    let dark = Srgb::new(0.1f32, 0.1, 0.12);
    let light = Srgb::new(0.9f32, 0.9, 0.88);

    let colors = interpolate_lightness(dark, light, 8);

    // Luminance should be monotonically increasing
    for i in 1..colors.len() {
        let lum_prev =
            0.2126 * colors[i - 1].red + 0.7152 * colors[i - 1].green + 0.0722 * colors[i - 1].blue;
        let lum_curr = 0.2126 * colors[i].red + 0.7152 * colors[i].green + 0.0722 * colors[i].blue;
        assert!(
            lum_curr >= lum_prev - 0.01,
            "Luminance should be non-decreasing: {} < {}",
            lum_curr,
            lum_prev
        );
    }
}

#[test]
fn test_generate_accent_hues_correct_count() {
    let accents = generate_accent_hues(25.0, 0.7, 0.12, 8);
    assert_eq!(accents.len(), 8);
}

#[test]
fn test_generate_accent_hues_different_colors() {
    let accents = generate_accent_hues(0.0, 0.7, 0.15, 8);

    // All colors should be different
    for i in 0..accents.len() {
        for j in (i + 1)..accents.len() {
            let dist = ((accents[i].red - accents[j].red).powi(2)
                + (accents[i].green - accents[j].green).powi(2)
                + (accents[i].blue - accents[j].blue).powi(2))
            .sqrt();
            assert!(dist > 0.01, "Colors {} and {} should be different", i, j);
        }
    }
}

#[test]
fn test_parse_hex_with_hash() {
    let color = parse_hex("#1a1a2e").unwrap();
    assert_eq!(color.red, 0x1a);
    assert_eq!(color.green, 0x1a);
    assert_eq!(color.blue, 0x2e);
}

#[test]
fn test_parse_hex_without_hash() {
    let color = parse_hex("eaeaea").unwrap();
    assert_eq!(color.red, 0xea);
    assert_eq!(color.green, 0xea);
    assert_eq!(color.blue, 0xea);
}

#[test]
fn test_generate_creates_scheme() {
    let config = GenerateConfig {
        background: Srgb::new(26u8, 26, 46),
        foreground: Srgb::new(234u8, 234, 234),
        accent_hue: 25.0,
        target_contrast: 75.0,
        extended_contrast: 60.0,
        accent_chroma: 0.25,
        extended_chroma: 0.20,
        name: "Test Scheme".to_string(),
        author: Some("Test Author".to_string()),
    };

    let result = generate(&config);

    assert_eq!(result.scheme.name, "Test Scheme");
    assert_eq!(result.scheme.author, "Test Author");
    // Slug now includes variant suffix
    assert!(result.scheme.slug.starts_with("test-scheme"));
}

#[test]
fn test_generate_has_all_base16_colors() {
    let config = GenerateConfig::default();
    let scheme = generate(&config).scheme;

    // Check base00-base0F exist
    for i in 0..16 {
        let name = format!("base0{:X}", i);
        assert!(
            scheme.palette.contains_key(&name),
            "Missing color: {}",
            name
        );
    }
}

#[test]
fn test_generate_has_extended_colors() {
    let config = GenerateConfig::default();
    let scheme = generate(&config).scheme;

    // Check base10-base17 exist (base24)
    for i in 0..8 {
        let name = format!("base1{:X}", i);
        assert!(
            scheme.palette.contains_key(&name),
            "Missing extended color: {}",
            name
        );
    }
}

#[test]
fn test_generate_dark_variant() {
    let config = GenerateConfig {
        background: Srgb::new(26u8, 26, 46), // Dark background
        foreground: Srgb::new(234u8, 234, 234),
        ..Default::default()
    };

    let scheme = generate(&config).scheme;
    // SchemeVariant doesn't implement PartialEq, so check via Debug format
    assert!(
        format!("{:?}", scheme.variant).contains("Dark"),
        "Expected Dark variant for dark background"
    );
}

#[test]
fn test_generate_light_variant() {
    let config = GenerateConfig {
        background: Srgb::new(250u8, 250, 250), // Light background
        foreground: Srgb::new(30u8, 30, 30),
        ..Default::default()
    };

    let scheme = generate(&config).scheme;
    // SchemeVariant doesn't implement PartialEq, so check via Debug format
    assert!(
        format!("{:?}", scheme.variant).contains("Light"),
        "Expected Light variant for light background"
    );
}
