use themalingadingdong::curves::{CurveConfig, CurveType, evaluate_curve};

#[test]
fn test_smoothstep_boundaries() {
    let config = CurveConfig {
        curve_type: CurveType::Smoothstep,
        ..Default::default()
    };
    assert!((evaluate_curve(&config, 0.0) - 0.0).abs() < 0.001);
    assert!((evaluate_curve(&config, 1.0) - 1.0).abs() < 0.001);
    // Midpoint should be 0.5 for symmetric curve
    assert!((evaluate_curve(&config, 0.5) - 0.5).abs() < 0.001);
}

#[test]
fn test_sigmoid_boundaries() {
    let config = CurveConfig {
        curve_type: CurveType::Sigmoid,
        strength: 1.0,
        ..Default::default()
    };
    assert!((evaluate_curve(&config, 0.0) - 0.0).abs() < 0.01);
    assert!((evaluate_curve(&config, 1.0) - 1.0).abs() < 0.01);
}
