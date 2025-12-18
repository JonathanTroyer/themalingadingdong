use approx::assert_relative_eq;
use palette::Srgb;
use themalingadingdong::apca::apca_contrast;

#[test]
fn test_black_on_white() {
    let black = Srgb::new(0u8, 0, 0);
    let white = Srgb::new(255u8, 255, 255);

    let lc = apca_contrast(black, white);

    // Black on white should be approximately 105 Lc
    assert_relative_eq!(lc, 105.0, epsilon = 3.0);
    assert!(lc > 0.0, "Dark text on light bg should be positive");
}

#[test]
fn test_white_on_black() {
    let black = Srgb::new(0u8, 0, 0);
    let white = Srgb::new(255u8, 255, 255);

    let lc = apca_contrast(white, black);

    // White on black should be approximately -105 Lc
    assert_relative_eq!(lc, -105.0, epsilon = 3.0);
    assert!(lc < 0.0, "Light text on dark bg should be negative");
}

#[test]
fn test_same_color_zero_contrast() {
    let gray = Srgb::new(128u8, 128, 128);

    let lc = apca_contrast(gray, gray);

    // Same color should have zero contrast
    assert_relative_eq!(lc, 0.0, epsilon = 1.0);
}

#[test]
fn test_mid_gray_on_white() {
    let gray = Srgb::new(128u8, 128, 128);
    let white = Srgb::new(255u8, 255, 255);

    let lc = apca_contrast(gray, white);

    // Mid-gray on white should have moderate contrast
    assert!(lc > 30.0, "Should have at least UI component contrast");
    assert!(lc < 80.0, "Should be less than body text contrast");
}

#[test]
fn test_mid_gray_on_black() {
    let gray = Srgb::new(128u8, 128, 128);
    let black = Srgb::new(0u8, 0, 0);

    let lc = apca_contrast(gray, black);

    // Mid-gray on black should have moderate negative contrast
    assert!(lc < -30.0, "Should have at least UI component contrast");
    assert!(lc > -80.0, "Should be less than body text contrast");
}

#[test]
fn test_polarity_asymmetry() {
    // APCA is polarity-sensitive: swapping fg/bg gives different absolute values
    let dark = Srgb::new(30u8, 30, 30);
    let light = Srgb::new(220u8, 220, 220);

    let lc_dark_on_light = apca_contrast(dark, light);
    let lc_light_on_dark = apca_contrast(light, dark);

    // Both should be significant contrast
    assert!(lc_dark_on_light.abs() > 60.0);
    assert!(lc_light_on_dark.abs() > 60.0);

    // They should have opposite signs
    assert!(lc_dark_on_light > 0.0);
    assert!(lc_light_on_dark < 0.0);

    // APCA is asymmetric, so absolute values won't be equal
    // (dark text on light bg typically has higher absolute Lc)
}

#[test]
fn test_colored_text() {
    // Test with actual colors, not just grayscale
    let red = Srgb::new(200u8, 50, 50);
    let white = Srgb::new(255u8, 255, 255);

    let lc = apca_contrast(red, white);

    // Red on white should have decent contrast
    assert!(lc > 30.0, "Red on white should be readable");
}

#[test]
fn test_low_contrast_warning_case() {
    // A case that would fail body text (Lc 75) but pass UI components (Lc 30)
    let light_gray = Srgb::new(180u8, 180, 180);
    let white = Srgb::new(255u8, 255, 255);

    let lc = apca_contrast(light_gray, white);

    // Should be below body text minimum (75) but above UI components (30)
    assert!(lc.abs() < 75.0, "Light gray on white should fail body text");
    assert!(
        lc.abs() > 30.0,
        "Light gray on white should pass UI components"
    );
}
