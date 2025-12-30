use approx::assert_relative_eq;
use palette::Srgb;
use themalingadingdong::contrast_solver::solve_lightness_for_contrast;

#[test]
fn test_solve_for_dark_background() {
    let dark_bg = Srgb::new(26u8, 26, 46);
    let result = solve_lightness_for_contrast(dark_bg, 60.0, 25.0, 0.12);

    assert!(result.is_exact, "Should achieve target exactly");
    assert_relative_eq!(result.achieved_contrast, 60.0, epsilon = 1.0);
}

#[test]
fn test_solve_for_light_background() {
    let light_bg = Srgb::new(250u8, 250, 250);
    let result = solve_lightness_for_contrast(light_bg, 60.0, 25.0, 0.12);

    assert!(result.is_exact, "Should achieve target exactly");
    assert_relative_eq!(result.achieved_contrast, 60.0, epsilon = 1.0);
}

#[test]
fn test_higher_contrast_needs_more_lightness_diff() {
    let dark_bg = Srgb::new(0u8, 0, 0);

    let result_60 = solve_lightness_for_contrast(dark_bg, 60.0, 180.0, 0.1);
    let result_75 = solve_lightness_for_contrast(dark_bg, 75.0, 180.0, 0.1);

    // Higher contrast target should require higher lightness on dark bg
    assert!(result_75.lightness > result_60.lightness);
}

#[test]
fn test_unreachable_contrast_returns_best_effort() {
    let dark_bg = Srgb::new(10u8, 10, 10);
    // Very high chroma + very high contrast = likely unreachable
    let result = solve_lightness_for_contrast(dark_bg, 100.0, 295.0, 0.4);

    // Should not panic, should return something reasonable
    assert!(result.lightness > 0.0 && result.lightness < 1.0);
    // May or may not be exact depending on gamut
    if !result.is_exact {
        assert!(result.warning.is_some());
    }
}
