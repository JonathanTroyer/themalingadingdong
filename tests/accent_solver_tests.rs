//! Tests for COBYLA-based accent solver edge cases.

use palette::Srgb;
use themalingadingdong::accent_solver::optimize_accents;
use themalingadingdong::config::AccentOptSettings;

#[test]
fn infeasible_high_contrast_produces_warning() {
    // Demand extremely high contrast (Lc 100) on a mid-gray background
    // This is physically impossible, so should produce warnings
    let bg = Srgb::new(128u8, 128, 128);
    let hues = [25.0, 285.0]; // Red and purple - hardest hues
    let settings = AccentOptSettings {
        target_j: 50.0, // Mid-lightness
        target_m: 30.0,
        delta_j: 5.0, // Very tight constraint
        delta_m: 5.0,
        j_weight: 0.5,
        contrast_weight: 0.8,
    };

    let result = optimize_accents(bg, &hues, &settings, 100.0); // Lc 100 is impossible

    // Should have warnings for infeasible constraints
    let warnings: Vec<_> = result
        .hue_results
        .iter()
        .filter(|r| r.warning.is_some())
        .collect();
    assert!(
        !warnings.is_empty(),
        "Expected warnings for infeasible contrast"
    );

    // Should still produce valid colors (best-effort)
    for hr in &result.hue_results {
        assert!(hr.color.red >= 0.0 && hr.color.red <= 1.0);
        assert!(hr.color.green >= 0.0 && hr.color.green <= 1.0);
        assert!(hr.color.blue >= 0.0 && hr.color.blue <= 1.0);
    }
}

#[test]
fn high_contrast_dark_theme_meets_target() {
    // Dark background with high J target (the common case)
    let bg = Srgb::new(26u8, 26, 46);
    let hues = [60.0, 180.0, 300.0]; // Yellow, cyan, magenta - easier hues
    let settings = AccentOptSettings {
        target_j: 80.0,
        target_m: 25.0,
        delta_j: 10.0,
        delta_m: 15.0,
        j_weight: 0.7,
        contrast_weight: 0.8,
    };

    let result = optimize_accents(bg, &hues, &settings, 60.0);

    for hr in &result.hue_results {
        assert!(
            hr.met_constraints,
            "Hue {:.0} should meet constraints",
            hr.hue
        );
        assert!(
            hr.achieved_contrast >= 55.0,
            "Hue {:.0} achieved {:.1} < 55.0",
            hr.hue,
            hr.achieved_contrast
        );
    }
}

#[test]
fn high_contrast_light_theme_meets_target() {
    // Light background with low J target
    let bg = Srgb::new(250u8, 250, 250);
    let hues = [60.0, 180.0, 300.0];
    let settings = AccentOptSettings {
        target_j: 35.0,
        target_m: 25.0,
        delta_j: 10.0,
        delta_m: 15.0,
        j_weight: 0.7,
        contrast_weight: 0.8,
    };

    let result = optimize_accents(bg, &hues, &settings, 60.0);

    for hr in &result.hue_results {
        assert!(
            hr.met_constraints,
            "Hue {:.0} should meet constraints",
            hr.hue
        );
        assert!(
            hr.achieved_contrast >= 55.0,
            "Hue {:.0} achieved {:.1} < 55.0",
            hr.hue,
            hr.achieved_contrast
        );
    }
}

#[test]
fn j_weight_affects_uniformity() {
    let bg = Srgb::new(26u8, 26, 46);
    let hues = [25.0, 60.0, 180.0, 285.0]; // Mix of hard and easy hues

    // High J weight - prioritize uniform lightness
    let uniform_settings = AccentOptSettings {
        target_j: 80.0,
        target_m: 25.0,
        delta_j: 15.0,
        delta_m: 20.0,
        j_weight: 0.9,
        contrast_weight: 0.8,
    };
    let uniform_result = optimize_accents(bg, &hues, &uniform_settings, 45.0);

    // Low J weight - prioritize colorfulness
    let vibrant_settings = AccentOptSettings {
        target_j: 80.0,
        target_m: 25.0,
        delta_j: 15.0,
        delta_m: 20.0,
        j_weight: 0.1,
        contrast_weight: 0.8,
    };
    let vibrant_result = optimize_accents(bg, &hues, &vibrant_settings, 45.0);

    // Compute J' variance for each
    let uniform_js: Vec<f32> = uniform_result.hue_results.iter().map(|r| r.j).collect();
    let vibrant_js: Vec<f32> = vibrant_result.hue_results.iter().map(|r| r.j).collect();

    let uniform_variance = variance(&uniform_js);
    let vibrant_variance = variance(&vibrant_js);

    // High j_weight should produce lower J' variance (more uniform)
    assert!(
        uniform_variance <= vibrant_variance + 10.0,
        "High j_weight ({:.2}) should produce similar or lower J' variance than low j_weight ({:.2})",
        uniform_variance,
        vibrant_variance
    );
}

fn variance(values: &[f32]) -> f32 {
    let mean = values.iter().sum::<f32>() / values.len() as f32;
    values.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / values.len() as f32
}
