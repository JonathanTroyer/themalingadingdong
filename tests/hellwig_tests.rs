use approx::assert_relative_eq;
use palette::Srgb;
use themalingadingdong::gamut_map::cusp_at_hue;
use themalingadingdong::generated::{ECCENTRICITY_CAM16_LUT, ECCENTRICITY_LUT, HK_HUE_LUT};
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

// ============================================================================
// LUT interpolation accuracy tests
// ============================================================================

/// Analytical eccentricity (Hellwig Munsell-fitted Fourier series)
fn eccentricity_analytical(hue_rad: f32) -> f32 {
    let h = hue_rad;
    let h2 = 2.0 * h;
    let h3 = 3.0 * h;
    let h4 = 4.0 * h;

    -0.0582 * h.cos() - 0.0258 * h2.cos() - 0.1347 * h3.cos() + 0.0289 * h4.cos()
        - 0.1475 * h.sin()
        - 0.0308 * h2.sin()
        + 0.0385 * h3.sin()
        + 0.0096 * h4.sin()
        + 1.0
}

/// Analytical CAM16 eccentricity
fn eccentricity_cam16_analytical(hue_rad: f32) -> f32 {
    0.25 * ((hue_rad + 2.0).cos() + 3.8)
}

/// Analytical HK hue angle dependency
fn hk_hue_analytical(hue_rad: f32) -> f32 {
    let h = hue_rad;
    let h2 = 2.0 * h;

    -0.160 * h.cos() + 0.132 * h2.cos() - 0.405 * h.sin() + 0.080 * h2.sin() + 0.792
}

/// Linear interpolation lookup (mirrors hellwig.rs lut_lookup)
fn lut_lookup(lut: &[f32; 360], hue_rad: f32) -> f32 {
    let hue_deg = hue_rad.to_degrees().rem_euclid(360.0);
    let idx = (hue_deg as usize) % 360;
    let frac = hue_deg - idx as f32;
    let a = lut[idx];
    let b = lut[(idx + 1) % 360];
    a + (b - a) * frac
}

#[test]
fn eccentricity_lut_interpolation_error() {
    let mut max_abs_error = 0.0f32;
    let mut max_rel_error = 0.0f32;
    let mut worst_hue = 0.0f32;

    // Sample at 0.1° intervals (3600 samples)
    for i in 0..3600 {
        let hue_deg = i as f32 * 0.1;
        let hue_rad = hue_deg.to_radians();

        let lut_value = lut_lookup(&ECCENTRICITY_LUT, hue_rad);
        let analytical = eccentricity_analytical(hue_rad);

        let abs_error = (lut_value - analytical).abs();
        let rel_error = abs_error / analytical.abs();

        if abs_error > max_abs_error {
            max_abs_error = abs_error;
            worst_hue = hue_deg;
        }
        max_rel_error = max_rel_error.max(rel_error);
    }

    println!(
        "ECCENTRICITY_LUT: max abs error = {:.6} at {:.1}°, max rel error = {:.4}%",
        max_abs_error,
        worst_hue,
        max_rel_error * 100.0
    );

    assert!(
        max_abs_error < 0.001,
        "Eccentricity LUT interpolation error too high: {:.6}",
        max_abs_error
    );
}

#[test]
fn eccentricity_cam16_lut_interpolation_error() {
    let mut max_abs_error = 0.0f32;
    let mut max_rel_error = 0.0f32;
    let mut worst_hue = 0.0f32;

    for i in 0..3600 {
        let hue_deg = i as f32 * 0.1;
        let hue_rad = hue_deg.to_radians();

        let lut_value = lut_lookup(&ECCENTRICITY_CAM16_LUT, hue_rad);
        let analytical = eccentricity_cam16_analytical(hue_rad);

        let abs_error = (lut_value - analytical).abs();
        let rel_error = abs_error / analytical.abs();

        if abs_error > max_abs_error {
            max_abs_error = abs_error;
            worst_hue = hue_deg;
        }
        max_rel_error = max_rel_error.max(rel_error);
    }

    println!(
        "ECCENTRICITY_CAM16_LUT: max abs error = {:.6} at {:.1}°, max rel error = {:.4}%",
        max_abs_error,
        worst_hue,
        max_rel_error * 100.0
    );

    assert!(
        max_abs_error < 0.0001,
        "CAM16 eccentricity LUT interpolation error too high: {:.6}",
        max_abs_error
    );
}

#[test]
fn hk_hue_lut_interpolation_error() {
    let mut max_abs_error = 0.0f32;
    let mut max_rel_error = 0.0f32;
    let mut worst_hue = 0.0f32;

    for i in 0..3600 {
        let hue_deg = i as f32 * 0.1;
        let hue_rad = hue_deg.to_radians();

        let lut_value = lut_lookup(&HK_HUE_LUT, hue_rad);
        let analytical = hk_hue_analytical(hue_rad);

        let abs_error = (lut_value - analytical).abs();
        let rel_error = abs_error / analytical.abs();

        if abs_error > max_abs_error {
            max_abs_error = abs_error;
            worst_hue = hue_deg;
        }
        max_rel_error = max_rel_error.max(rel_error);
    }

    println!(
        "HK_HUE_LUT: max abs error = {:.6} at {:.1}°, max rel error = {:.4}%",
        max_abs_error,
        worst_hue,
        max_rel_error * 100.0
    );

    assert!(
        max_abs_error < 0.001,
        "HK hue LUT interpolation error too high: {:.6}",
        max_abs_error
    );
}

#[test]
fn cusp_lut_produces_valid_gamut_boundaries() {
    // For cusp LUT, we can't easily compute analytical values at runtime.
    // Instead, verify that cusp values produce colors near the gamut boundary.

    let mut max_m_error = 0.0f32;
    let mut worst_hue = 0.0f32;

    for i in 0..3600 {
        let hue_deg = i as f32 * 0.1;
        let cusp = cusp_at_hue(hue_deg);

        // Create a color at the cusp
        let at_cusp = HellwigJmh::new(cusp.j, cusp.m, hue_deg);

        // It should be very close to the gamut boundary
        // (either just in or just out due to interpolation)
        let srgb = at_cusp.into_srgb_unclamped();

        // Find how far the most extreme channel is from [0, 1]
        let max_deviation = [srgb.red, srgb.green, srgb.blue]
            .iter()
            .map(|&c| {
                if c < 0.0 {
                    -c
                } else if c > 1.0 {
                    c - 1.0
                } else {
                    0.0
                }
            })
            .fold(0.0f32, f32::max);

        if max_deviation > max_m_error {
            max_m_error = max_deviation;
            worst_hue = hue_deg;
        }
    }

    println!(
        "CUSP_LUT: max boundary deviation = {:.6} at {:.1}°",
        max_m_error, worst_hue
    );

    // Cusp should be within 0.02 of the boundary (2% RGB tolerance)
    assert!(
        max_m_error < 0.02,
        "Cusp LUT boundary deviation too high: {:.6} at {:.1}°",
        max_m_error,
        worst_hue
    );
}

#[test]
fn cusp_lut_interpolation_is_smooth() {
    // Verify cusp values don't have discontinuities between adjacent entries
    let mut max_j_jump = 0.0f32;
    let mut max_m_jump = 0.0f32;
    let mut worst_hue_j = 0.0f32;
    let mut worst_hue_m = 0.0f32;

    let mut prev = cusp_at_hue(0.0);

    for i in 1..=3600 {
        let hue_deg = i as f32 * 0.1;
        let curr = cusp_at_hue(hue_deg);

        let j_jump = (curr.j - prev.j).abs();
        let m_jump = (curr.m - prev.m).abs();

        if j_jump > max_j_jump {
            max_j_jump = j_jump;
            worst_hue_j = hue_deg;
        }
        if m_jump > max_m_jump {
            max_m_jump = m_jump;
            worst_hue_m = hue_deg;
        }

        prev = curr;
    }

    println!(
        "CUSP_LUT continuity: max J' jump = {:.4} at {:.1}°, max M jump = {:.4} at {:.1}°",
        max_j_jump, worst_hue_j, max_m_jump, worst_hue_m
    );

    // At 0.1° steps, jumps should be small
    assert!(
        max_j_jump < 0.5,
        "Cusp J' discontinuity too large: {:.4} at {:.1}°",
        max_j_jump,
        worst_hue_j
    );
    assert!(
        max_m_jump < 0.5,
        "Cusp M discontinuity too large: {:.4} at {:.1}°",
        max_m_jump,
        worst_hue_m
    );
}
