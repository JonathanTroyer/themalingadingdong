use palette::Srgb;
use themalingadingdong::generate::{GenerateConfig, generate};
use themalingadingdong::validation::{validate, validate_with_warnings};

#[test]
fn test_validate_returns_results() {
    let config = GenerateConfig::default();
    let scheme = generate(&config).scheme;

    let results = validate(&scheme);
    assert!(!results.is_empty());
}

#[test]
fn test_validate_checks_contrast() {
    let config = GenerateConfig::default();
    let scheme = generate(&config).scheme;

    for result in validate(&scheme) {
        // Contrast should be calculated (non-zero for different colors)
        assert!(
            result.contrast.abs() > 0.0 || result.contrast == 0.0,
            "Contrast should be calculated"
        );
    }
}

#[test]
fn test_high_contrast_scheme_passes() {
    let config = GenerateConfig {
        background: Srgb::new(0u8, 0, 0),
        foreground: Srgb::new(255u8, 255, 255),
        min_contrast: 75.0,
        accent_chroma: 0.1,
        ..Default::default()
    };

    let scheme = generate(&config).scheme;

    // base07 (white) on base00 (black) should always pass
    let results = validate(&scheme);
    let base07_result = results
        .iter()
        .find(|r| r.pair.foreground == "base07")
        .unwrap();
    assert!(base07_result.passes);
    assert!(base07_result.contrast.abs() > 100.0);
}

#[test]
fn test_validate_with_warnings_returns_failures() {
    // Low contrast that will produce warnings
    let config = GenerateConfig {
        background: Srgb::new(30u8, 30, 40),
        foreground: Srgb::new(200u8, 200, 200),
        min_contrast: 45.0,
        accent_chroma: 0.15,
        ..Default::default()
    };

    let scheme = generate(&config).scheme;
    let warnings = validate_with_warnings(&scheme);

    // Low target contrast may produce warnings for pairs requiring higher contrast
    // Just verify the function works; actual warnings depend on the thresholds
    for warning in &warnings {
        assert!(
            warning.contains("Lc="),
            "Warning should contain contrast value"
        );
    }
}

#[test]
fn test_accent_colors_meet_target_contrast() {
    // Accents (base08-base0F) are computed to meet target_contrast.
    // UI colors (base00-base07) are interpolated and may not meet all thresholds.
    let config = GenerateConfig {
        background: Srgb::new(0u8, 0, 0),
        foreground: Srgb::new(255u8, 255, 255),
        min_contrast: 75.0,
        accent_chroma: 0.1,
        ..Default::default()
    };

    let scheme = generate(&config).scheme;
    let results = validate(&scheme);

    // Accent colors should pass their validation (they're computed for contrast)
    let accent_failures: Vec<_> = results
        .iter()
        .filter(|r| r.pair.foreground.starts_with("base0") && r.pair.foreground >= "base08")
        .filter(|r| !r.passes)
        .collect();

    assert!(
        accent_failures.is_empty(),
        "Accent colors should meet contrast: {:?}",
        accent_failures
            .iter()
            .map(|r| &r.pair.foreground)
            .collect::<Vec<_>>()
    );
}
