use themalingadingdong::curves::{
    CurveConfig, CurveType, compute_sample_positions, evaluate_curve,
};

#[test]
fn test_linear_curve() {
    let config = CurveConfig::default();
    assert_eq!(evaluate_curve(&config, 0.0), 0.0);
    assert_eq!(evaluate_curve(&config, 0.5), 0.5);
    assert_eq!(evaluate_curve(&config, 1.0), 1.0);
}

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

#[test]
fn test_sample_positions() {
    let config = CurveConfig::default();
    let positions = compute_sample_positions(8, &config);
    assert_eq!(positions.len(), 8);
    assert_eq!(positions[0], 0.0);
    assert_eq!(positions[7], 1.0);
}

#[test]
fn test_curve_type_cycle() {
    let start = CurveType::Linear;
    let mut current = start;
    for _ in 0..7 {
        current = current.next();
    }
    assert_eq!(current, start);
}
