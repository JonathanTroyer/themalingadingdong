//! Curve configuration and interpolation types for easing functions.

use enterpolation::{Signal, bspline::BSpline};
use serde::{Deserialize, Serialize};

/// Available curve/easing types for interpolation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurveType {
    /// Linear interpolation (no easing)
    #[default]
    Linear,
    /// Smooth S-curve (Hermite interpolation)
    Smoothstep,
    /// Smoother S-curve (Ken Perlin's improved version)
    Smootherstep,
    /// Ease-in (accelerating from start)
    SmoothStart,
    /// Ease-out (decelerating to end)
    SmoothEnd,
    /// Sigmoid curve with configurable steepness
    Sigmoid,
    /// Custom B-spline with control points
    BSpline,
}

impl CurveType {
    /// Get the next curve type in sequence.
    pub fn next(self) -> Self {
        match self {
            Self::Linear => Self::Smoothstep,
            Self::Smoothstep => Self::Smootherstep,
            Self::Smootherstep => Self::SmoothStart,
            Self::SmoothStart => Self::SmoothEnd,
            Self::SmoothEnd => Self::Sigmoid,
            Self::Sigmoid => Self::BSpline,
            Self::BSpline => Self::Linear,
        }
    }

    /// Get the previous curve type in sequence.
    pub fn prev(self) -> Self {
        match self {
            Self::Linear => Self::BSpline,
            Self::Smoothstep => Self::Linear,
            Self::Smootherstep => Self::Smoothstep,
            Self::SmoothStart => Self::Smootherstep,
            Self::SmoothEnd => Self::SmoothStart,
            Self::Sigmoid => Self::SmoothEnd,
            Self::BSpline => Self::Sigmoid,
        }
    }

    /// Display name for the curve type.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Linear => "Linear",
            Self::Smoothstep => "Smoothstep",
            Self::Smootherstep => "Smootherstep",
            Self::SmoothStart => "Ease In",
            Self::SmoothEnd => "Ease Out",
            Self::Sigmoid => "Sigmoid",
            Self::BSpline => "B-Spline",
        }
    }

    /// Whether this curve type uses the strength parameter.
    pub fn uses_strength(self) -> bool {
        matches!(self, Self::Sigmoid)
    }
}

/// Configuration for a single interpolation curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CurveConfig {
    /// The curve type to use
    #[serde(rename = "type")]
    pub curve_type: CurveType,
    /// Strength/steepness parameter (for sigmoid, 0.1-5.0)
    pub strength: f32,
    /// Custom control points for B-spline (t, value pairs)
    pub control_points: Option<Vec<(f32, f32)>>,
}

impl Default for CurveConfig {
    fn default() -> Self {
        Self {
            curve_type: CurveType::Linear,
            strength: 1.0,
            control_points: None,
        }
    }
}

/// Complete interpolation configuration with separate curves for L/C/H.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InterpolationConfig {
    /// Curve for lightness interpolation
    pub lightness: CurveConfig,
    /// Curve for chroma interpolation
    pub chroma: CurveConfig,
    /// Curve for hue interpolation
    pub hue: CurveConfig,
}

impl Default for InterpolationConfig {
    fn default() -> Self {
        Self {
            lightness: CurveConfig {
                curve_type: CurveType::Smoothstep,
                ..Default::default()
            },
            chroma: CurveConfig::default(),
            hue: CurveConfig::default(),
        }
    }
}

/// Evaluate a curve at parameter t (0.0 to 1.0).
///
/// Returns the mapped t value after applying the easing function.
pub fn evaluate_curve(config: &CurveConfig, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);

    match config.curve_type {
        CurveType::Linear => t,
        CurveType::Smoothstep => smoothstep(t),
        CurveType::Smootherstep => smootherstep(t),
        CurveType::SmoothStart => smooth_start(t),
        CurveType::SmoothEnd => smooth_end(t),
        CurveType::Sigmoid => sigmoid(t, config.strength),
        CurveType::BSpline => evaluate_bspline(config, t),
    }
}

/// Smoothstep easing function (Hermite interpolation).
/// Creates an S-curve that starts and ends smoothly.
fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Smootherstep easing function (Ken Perlin's improved version).
/// Even smoother transitions at the boundaries.
fn smootherstep(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Ease-in (smooth start) - accelerates from rest.
fn smooth_start(t: f32) -> f32 {
    t * t
}

/// Ease-out (smooth end) - decelerates to rest.
fn smooth_end(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

/// Sigmoid function with configurable steepness.
/// Maps [0,1] -> [0,1] with an S-curve shape.
fn sigmoid(t: f32, strength: f32) -> f32 {
    // Centered sigmoid: steepness controls how sharp the S-curve is
    // strength: 0.1-1.0 = gentle curves, 1.0-5.0 = progressively sharper
    let k = strength.max(0.1) * 6.0; // Scale for usable range
    let x = (t - 0.5) * k;
    let raw = 1.0 / (1.0 + (-x).exp());

    // Normalize to exactly [0, 1] range
    let min_val = 1.0 / (1.0 + (k * 0.5).exp());
    let max_val = 1.0 / (1.0 + (-k * 0.5).exp());
    (raw - min_val) / (max_val - min_val)
}

/// Evaluate custom spline with control points using B-spline interpolation.
fn evaluate_bspline(config: &CurveConfig, t: f32) -> f32 {
    let Some(points) = &config.control_points else {
        return t; // Fallback to linear if no control points
    };

    if points.len() < 2 {
        return t;
    }

    // Extract values from control points (we use normalized domain 0-1)
    let values: Vec<f64> = points.iter().map(|(_, v)| f64::from(*v)).collect();
    let degree = (values.len() - 1).min(3); // Degree up to 3, but not more than points-1

    // Build B-spline with clamped mode (curve passes through endpoints)
    let result = BSpline::builder()
        .clamped()
        .elements(values)
        .equidistant::<f64>()
        .degree(degree)
        .normalized()
        .dynamic()
        .build();

    match result {
        Ok(spline) => spline.eval(f64::from(t)) as f32,
        Err(_) => t, // Fallback on error
    }
}

/// Compute sample positions based on curve configuration.
/// Returns the output t values for each step (where colors will be sampled).
pub fn compute_sample_positions(steps: usize, curve: &CurveConfig) -> Vec<f32> {
    if steps == 0 {
        return vec![];
    }
    if steps == 1 {
        return vec![0.0];
    }

    (0..steps)
        .map(|i| {
            let linear_t = i as f32 / (steps - 1) as f32;
            evaluate_curve(curve, linear_t)
        })
        .collect()
}
