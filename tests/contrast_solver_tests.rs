use approx::assert_relative_eq;
use palette::Srgb;
use themalingadingdong::contrast_solver::{
    find_uniform_post_clamp_lightness, solve_lightness_for_contrast,
};
use themalingadingdong::interpolation::DEFAULT_BASE16_HUES;

#[test]
fn test_solve_for_dark_background() {
    let dark_bg = Srgb::new(26u8, 26, 46);
    // HellwigJmh: hue=25°, colorfulness=15 (was OKLCH chroma 0.12)
    let result = solve_lightness_for_contrast(dark_bg, 60.0, 25.0, 15.0);

    assert!(result.is_exact, "Should achieve target exactly");
    assert_relative_eq!(result.achieved_contrast, 60.0, epsilon = 1.0);
}

#[test]
fn test_solve_for_light_background() {
    let light_bg = Srgb::new(250u8, 250, 250);
    // HellwigJmh: hue=25°, colorfulness=15 (was OKLCH chroma 0.12)
    let result = solve_lightness_for_contrast(light_bg, 60.0, 25.0, 15.0);

    assert!(result.is_exact, "Should achieve target exactly");
    assert_relative_eq!(result.achieved_contrast, 60.0, epsilon = 1.0);
}

#[test]
fn test_higher_contrast_needs_more_lightness_diff() {
    let dark_bg = Srgb::new(0u8, 0, 0);
    // HellwigJmh: hue=180°, colorfulness=15 (was OKLCH chroma 0.1)
    let result_60 = solve_lightness_for_contrast(dark_bg, 60.0, 180.0, 15.0);
    let result_75 = solve_lightness_for_contrast(dark_bg, 75.0, 180.0, 15.0);

    // Higher contrast target should require higher lightness on dark bg
    assert!(result_75.lightness > result_60.lightness);
}

#[test]
fn test_unreachable_contrast_returns_best_effort() {
    let dark_bg = Srgb::new(10u8, 10, 10);
    // Very high colorfulness + very high contrast = likely unreachable
    // HellwigJmh: hue=295°, colorfulness=50 (was OKLCH chroma 0.4)
    let result = solve_lightness_for_contrast(dark_bg, 100.0, 295.0, 50.0);

    // Should not panic, should return something reasonable
    // HellwigJmh lightness range is 0-100 (not 0-1 like OKLCH)
    assert!(result.lightness > 0.0 && result.lightness < 100.0);
    // May or may not be exact depending on gamut
    if !result.is_exact {
        assert!(result.warning.is_some());
    }
}

// Tests for find_uniform_post_clamp_lightness

#[test]
fn test_post_clamp_uniform_dark_bg() {
    let dark_bg = Srgb::new(26u8, 26, 46);
    let hues: Vec<f32> = DEFAULT_BASE16_HUES.to_vec();

    let result = find_uniform_post_clamp_lightness(dark_bg, &hues, 25.0, 60.0, 2.0);

    // All hues should meet minimum contrast
    for hr in &result.hue_results {
        assert!(
            hr.achieved_contrast >= 59.5 || !hr.met_minimum_contrast,
            "Hue {}: contrast {} should be >= 60 or have warning",
            hr.hue,
            hr.achieved_contrast
        );
    }

    // Post-clamp J' values should be relatively uniform
    let post_clamp_js: Vec<f32> = result.hue_results.iter().map(|r| r.post_clamp_j).collect();
    let max_j = post_clamp_js.iter().cloned().fold(f32::MIN, f32::max);
    let min_j = post_clamp_js.iter().cloned().fold(f32::MAX, f32::min);
    let j_range = max_j - min_j;

    // Range should be reasonably small (less than 10 J' units)
    assert!(
        j_range < 10.0,
        "Post-clamp J' range {} is too large (max={}, min={})",
        j_range,
        max_j,
        min_j
    );
}

#[test]
fn test_post_clamp_uniform_light_bg() {
    let light_bg = Srgb::new(250u8, 250, 250);
    let hues: Vec<f32> = DEFAULT_BASE16_HUES.to_vec();

    let result = find_uniform_post_clamp_lightness(light_bg, &hues, 25.0, 60.0, 2.0);

    // All hues should meet minimum contrast
    for hr in &result.hue_results {
        assert!(
            hr.achieved_contrast >= 59.5 || !hr.met_minimum_contrast,
            "Hue {}: contrast {} should be >= 60 or have warning",
            hr.hue,
            hr.achieved_contrast
        );
    }

    // Post-clamp J' values should be relatively uniform
    let post_clamp_js: Vec<f32> = result.hue_results.iter().map(|r| r.post_clamp_j).collect();
    let max_j = post_clamp_js.iter().cloned().fold(f32::MIN, f32::max);
    let min_j = post_clamp_js.iter().cloned().fold(f32::MAX, f32::min);
    let j_range = max_j - min_j;

    // Range should be reasonably small (less than 10 J' units)
    assert!(
        j_range < 10.0,
        "Post-clamp J' range {} is too large (max={}, min={})",
        j_range,
        max_j,
        min_j
    );
}

#[test]
fn test_post_clamp_higher_contrast_more_uniform() {
    let dark_bg = Srgb::new(26u8, 26, 46);
    let hues: Vec<f32> = DEFAULT_BASE16_HUES.to_vec();

    // Compare results at different contrast levels
    let result_60 = find_uniform_post_clamp_lightness(dark_bg, &hues, 25.0, 60.0, 2.0);
    let result_75 = find_uniform_post_clamp_lightness(dark_bg, &hues, 25.0, 75.0, 2.0);

    // Higher contrast should require higher base lightness for dark bg
    assert!(
        result_75.input_lightness > result_60.input_lightness,
        "Higher contrast should need higher lightness: {} vs {}",
        result_75.input_lightness,
        result_60.input_lightness
    );
}

#[test]
fn test_post_clamp_empty_hues() {
    let dark_bg = Srgb::new(26u8, 26, 46);
    let hues: Vec<f32> = vec![];

    let result = find_uniform_post_clamp_lightness(dark_bg, &hues, 25.0, 60.0, 2.0);

    assert!(result.hue_results.is_empty());
    assert!(result.all_goals_met);
}

#[test]
fn test_post_clamp_deviation_tracking() {
    let dark_bg = Srgb::new(26u8, 26, 46);
    let hues: Vec<f32> = DEFAULT_BASE16_HUES.to_vec();

    let result = find_uniform_post_clamp_lightness(dark_bg, &hues, 25.0, 60.0, 2.0);

    // Verify j_deviation is correctly computed
    for hr in &result.hue_results {
        let expected_deviation = hr.post_clamp_j - result.target_post_clamp_j;
        assert_relative_eq!(hr.j_deviation, expected_deviation, epsilon = 0.01);
    }
}

#[test]
fn test_high_colorfulness_uniform_j() {
    // This test specifically targets the bug where high colorfulness
    // caused J' values to vary widely (80.5 to 91.9, 11-unit spread)
    let dark_bg = Srgb::new(26u8, 26, 46);
    let hues: Vec<f32> = DEFAULT_BASE16_HUES.to_vec();

    // M=45 was the problematic case
    let result = find_uniform_post_clamp_lightness(dark_bg, &hues, 45.0, 75.0, 2.0);

    // Collect post-clamp J' values
    let post_clamp_js: Vec<f32> = result.hue_results.iter().map(|r| r.post_clamp_j).collect();
    let max_j = post_clamp_js.iter().cloned().fold(f32::MIN, f32::max);
    let min_j = post_clamp_js.iter().cloned().fold(f32::MAX, f32::min);
    let j_range = max_j - min_j;

    // With the bounds-based algorithm, J' range should be ≤3 units
    assert!(
        j_range <= 3.0,
        "High colorfulness J' range {} is too large (max={}, min={}, target={}). Hue results: {:?}",
        j_range,
        max_j,
        min_j,
        result.target_post_clamp_j,
        result
            .hue_results
            .iter()
            .map(|r| (r.hue, r.post_clamp_j))
            .collect::<Vec<_>>()
    );

    // All hues should still meet minimum contrast
    let all_meet_contrast = result
        .hue_results
        .iter()
        .all(|r| r.achieved_contrast >= 74.5);
    assert!(
        all_meet_contrast,
        "Some hues don't meet minimum contrast: {:?}",
        result
            .hue_results
            .iter()
            .map(|r| (r.hue, r.achieved_contrast))
            .collect::<Vec<_>>()
    );
}
