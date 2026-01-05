use approx::assert_relative_eq;
use palette::Srgb;
use themalingadingdong::hellwig::{HellwigJmh, eccentricity, hue_angle_dependency};

#[test]
fn eccentricity_stays_in_range() {
    for hue_deg in (0..360).step_by(5) {
        let hue_rad = (hue_deg as f32).to_radians();
        let e = eccentricity(hue_rad);
        assert!(
            e > 0.6 && e < 1.4,
            "eccentricity at {}° = {} is out of expected range [0.6, 1.4]",
            hue_deg,
            e
        );
    }
}

#[test]
fn eccentricity_is_continuous() {
    let mut prev = eccentricity(0.0);
    for hue_deg in 1..=360 {
        let hue_rad = (hue_deg as f32).to_radians();
        let curr = eccentricity(hue_rad);
        let diff = (curr - prev).abs();
        assert!(
            diff < 0.05,
            "eccentricity discontinuity at {}°: diff = {}",
            hue_deg,
            diff
        );
        prev = curr;
    }
}

#[test]
fn hue_dependency_stays_in_range() {
    for hue_deg in (0..360).step_by(5) {
        let hue_rad = (hue_deg as f32).to_radians();
        let f = hue_angle_dependency(hue_rad);
        assert!(
            f > 0.2 && f < 1.5,
            "HK factor at {}° = {} is out of expected range [0.2, 1.5]",
            hue_deg,
            f
        );
    }
}

#[test]
fn hue_dependency_is_continuous() {
    let mut prev = hue_angle_dependency(0.0);
    for hue_deg in 1..=360 {
        let hue_rad = (hue_deg as f32).to_radians();
        let curr = hue_angle_dependency(hue_rad);
        let diff = (curr - prev).abs();
        assert!(
            diff < 0.05,
            "HK factor discontinuity at {}°: diff = {}",
            hue_deg,
            diff
        );
        prev = curr;
    }
}

#[test]
fn roundtrip_preserves_saturated_colors() {
    let test_colors = [
        Srgb::new(1.0f32, 0.0, 0.0), // Red
        Srgb::new(0.0f32, 1.0, 0.0), // Green
        Srgb::new(0.0f32, 0.0, 1.0), // Blue
        Srgb::new(1.0f32, 1.0, 0.0), // Yellow
        Srgb::new(1.0f32, 0.0, 1.0), // Magenta
        Srgb::new(0.0f32, 1.0, 1.0), // Cyan
    ];

    for original in test_colors.iter() {
        let hellwig = HellwigJmh::from_srgb(*original);
        let result = hellwig.into_srgb();

        assert_relative_eq!(original.red, result.red, epsilon = 0.02);
        assert_relative_eq!(original.green, result.green, epsilon = 0.02);
        assert_relative_eq!(original.blue, result.blue, epsilon = 0.02);
    }
}

#[test]
fn roundtrip_preserves_muted_colors() {
    let test_colors = [
        Srgb::new(0.3f32, 0.2, 0.4),
        Srgb::new(0.7f32, 0.5, 0.3),
        Srgb::new(0.2f32, 0.6, 0.5),
        Srgb::new(0.8f32, 0.7, 0.9),
    ];

    for original in test_colors.iter() {
        let hellwig = HellwigJmh::from_srgb(*original);
        let result = hellwig.into_srgb();

        assert_relative_eq!(original.red, result.red, epsilon = 0.01);
        assert_relative_eq!(original.green, result.green, epsilon = 0.01);
        assert_relative_eq!(original.blue, result.blue, epsilon = 0.01);
    }
}

#[test]
fn roundtrip_preserves_grays() {
    let grays = [0.0f32, 0.25, 0.5, 0.75, 1.0];

    for gray in grays.iter() {
        let original = Srgb::new(*gray, *gray, *gray);
        let hellwig = HellwigJmh::from_srgb(original);
        let result = hellwig.into_srgb();

        assert_relative_eq!(original.red, result.red, epsilon = 0.01);
        assert_relative_eq!(original.green, result.green, epsilon = 0.01);
        assert_relative_eq!(original.blue, result.blue, epsilon = 0.01);
    }
}

#[test]
fn black_has_zero_lightness() {
    let black = Srgb::new(0.0f32, 0.0, 0.0);
    let hellwig = HellwigJmh::from_srgb(black);

    assert!(
        hellwig.lightness < 0.01,
        "Black lightness = {} (expected < 0.01)",
        hellwig.lightness
    );
}

#[test]
fn white_has_high_lightness() {
    let white = Srgb::new(1.0f32, 1.0, 1.0);
    let hellwig = HellwigJmh::from_srgb(white);

    assert!(
        hellwig.lightness > 95.0,
        "White lightness = {} (expected > 95)",
        hellwig.lightness
    );
}

#[test]
fn gray_has_low_colorfulness() {
    for gray in [0.25f32, 0.5, 0.75].iter() {
        let color = Srgb::new(*gray, *gray, *gray);
        let hellwig = HellwigJmh::from_srgb(color);

        assert!(
            hellwig.colorfulness < 2.0,
            "Gray {} colorfulness = {} (expected < 2.0)",
            gray,
            hellwig.colorfulness
        );
    }
}

#[test]
fn saturated_colors_have_high_colorfulness() {
    let saturated = [
        Srgb::new(1.0f32, 0.0, 0.0),
        Srgb::new(0.0f32, 1.0, 0.0),
        Srgb::new(0.0f32, 0.0, 1.0),
    ];

    for color in saturated.iter() {
        let hellwig = HellwigJmh::from_srgb(*color);
        assert!(
            hellwig.colorfulness > 30.0,
            "Saturated color colorfulness = {} (expected > 30)",
            hellwig.colorfulness
        );
    }
}

#[test]
fn u8_conversion_roundtrip() {
    let original = Srgb::new(128u8, 64, 192);
    let hellwig = HellwigJmh::from_srgb_u8(original);
    let result = hellwig.into_srgb_u8();

    // Allow +/- 1 due to rounding
    assert!((original.red as i16 - result.red as i16).abs() <= 1);
    assert!((original.green as i16 - result.green as i16).abs() <= 1);
    assert!((original.blue as i16 - result.blue as i16).abs() <= 1);
}

#[test]
fn new_constructor_works() {
    let hellwig = HellwigJmh::new(50.0, 30.0, 180.0);

    assert_relative_eq!(hellwig.lightness, 50.0);
    assert_relative_eq!(hellwig.colorfulness, 30.0);
    assert_relative_eq!(hellwig.hue, 180.0);
}

#[test]
fn hue_wraps_correctly() {
    // Test that hue values are normalized to 0-360
    let color = Srgb::new(1.0f32, 0.0, 0.0); // Red
    let hellwig = HellwigJmh::from_srgb(color);

    assert!(
        hellwig.hue >= 0.0 && hellwig.hue < 360.0,
        "Hue {} is outside [0, 360)",
        hellwig.hue
    );
}
