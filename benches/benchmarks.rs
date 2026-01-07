//! Performance benchmarks for palette generation.
//!
//! Measures the hot paths:
//! - Full palette generation
//! - HellwigJmh color space conversions
//! - APCA contrast calculations
//! - Accent solver COBYLA optimization
//! - Gamut mapping operations
//! - Interpolation

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use palette::Srgb;
use themalingadingdong::accent_solver::optimize_accents;
use themalingadingdong::apca::apca_contrast;
use themalingadingdong::config::AccentOptSettings;
use themalingadingdong::curves::{CurveType, InterpolationConfig};
use themalingadingdong::gamut_map::{gamut_map, max_colorfulness_at};
use themalingadingdong::generate::{GenerateConfig, generate};
use themalingadingdong::hellwig::HellwigJmh;
use themalingadingdong::interpolation::interpolate_with_curves;

/// Benchmark full palette generation with default config.
fn bench_palette_generation(c: &mut Criterion) {
    let config = GenerateConfig::default();

    c.bench_function("palette_generation", |b| {
        b.iter(|| generate(black_box(&config)))
    });
}

/// Benchmark HellwigJmh forward conversion (sRGB -> JMh) for 256 colors.
fn bench_hellwig_from_srgb(c: &mut Criterion) {
    // Generate 256 test colors spanning the color space
    let colors: Vec<Srgb<u8>> = (0u8..=255)
        .map(|i: u8| {
            // Create a variety of colors by cycling through RGB
            let r = i;
            let g = i.wrapping_mul(97);
            let b = i.wrapping_mul(193);
            Srgb::new(r, g, b)
        })
        .collect();

    c.bench_function("hellwig_from_srgb_256", |b| {
        b.iter(|| {
            for color in &colors {
                black_box(HellwigJmh::from_srgb_u8(*color));
            }
        })
    });
}

/// Benchmark HellwigJmh reverse conversion (JMh -> sRGB) for 256 colors.
fn bench_hellwig_into_srgb(c: &mut Criterion) {
    // Pre-convert colors to HellwigJmh
    let jmh_colors: Vec<HellwigJmh> = (0u8..=255)
        .map(|i: u8| {
            let r = i;
            let g = i.wrapping_mul(97);
            let b = i.wrapping_mul(193);
            HellwigJmh::from_srgb_u8(Srgb::new(r, g, b))
        })
        .collect();

    c.bench_function("hellwig_into_srgb_256", |b| {
        b.iter(|| {
            for jmh in &jmh_colors {
                black_box(jmh.into_srgb_u8());
            }
        })
    });
}

/// Benchmark APCA contrast calculation for 256 foreground colors against a fixed background.
fn bench_apca_contrast(c: &mut Criterion) {
    let background = Srgb::new(26u8, 26, 46); // Default dark background

    // Generate 256 foreground colors
    let foregrounds: Vec<Srgb<u8>> = (0u8..=255)
        .map(|i: u8| {
            let r = i;
            let g = i.wrapping_mul(97);
            let b = i.wrapping_mul(193);
            Srgb::new(r, g, b)
        })
        .collect();

    c.bench_function("apca_contrast_256", |b| {
        b.iter(|| {
            for fg in &foregrounds {
                black_box(apca_contrast(*fg, background));
            }
        })
    });
}

/// Benchmark gamut mapping for 256 colors with various J'/M/hue values.
fn bench_gamut_map(c: &mut Criterion) {
    // Generate 256 test colors with varying J', M, and hue
    // Mix of in-gamut and out-of-gamut colors
    let colors: Vec<HellwigJmh> = (0u8..=255)
        .map(|i: u8| {
            // Vary lightness (J') from 10 to 90
            let j = 10.0 + (i as f32 / 255.0) * 80.0;
            // Vary colorfulness (M) from 0 to 100 (some will be out-of-gamut)
            let m = (i.wrapping_mul(73) as f32 / 255.0) * 100.0;
            // Cycle through hues
            let h = (i.wrapping_mul(193) as f32 / 255.0) * 360.0;
            HellwigJmh::new(j, m, h)
        })
        .collect();

    c.bench_function("gamut_map_256", |b| {
        b.iter(|| {
            for color in &colors {
                black_box(gamut_map(*color));
            }
        })
    });
}

/// Benchmark finding max colorfulness for various J'/hue combinations.
fn bench_max_colorfulness_at(c: &mut Criterion) {
    // Generate 256 (J', hue) pairs spanning the color space
    let test_params: Vec<(f32, f32)> = (0u8..=255)
        .map(|i: u8| {
            // Vary lightness (J') from 10 to 90
            let j = 10.0 + (i as f32 / 255.0) * 80.0;
            // Cycle through hues
            let h = (i.wrapping_mul(193) as f32 / 255.0) * 360.0;
            (j, h)
        })
        .collect();

    c.bench_function("max_colorfulness_at_256", |b| {
        b.iter(|| {
            for &(j, hue) in &test_params {
                black_box(max_colorfulness_at(j, hue));
            }
        })
    });
}

/// Benchmark interpolation with B-spline curve (8 steps).
fn bench_interpolate_bspline_8(c: &mut Criterion) {
    let start = Srgb::new(0.1f32, 0.1, 0.12);
    let end = Srgb::new(0.9f32, 0.9, 0.88);
    let mut config = InterpolationConfig::default();
    config.lightness.curve_type = CurveType::BSpline;
    config.lightness.control_points = Some(vec![
        (0.0, 0.0),
        (0.25, 0.1),
        (0.5, 0.5),
        (0.75, 0.9),
        (1.0, 1.0),
    ]);

    c.bench_function("interpolate_bspline_8", |b| {
        b.iter(|| {
            black_box(interpolate_with_curves(
                black_box(start),
                black_box(end),
                black_box(8),
                black_box(&config),
            ))
        })
    });
}

/// Benchmark accent solver COBYLA optimization for 8 standard hues.
///
/// Measures the end-to-end optimization including:
/// - Feasibility checking
/// - Initial guess computation
/// - COBYLA optimization (parallel across hues)
/// - Gamut mapping and contrast validation
fn bench_accent_solver(c: &mut Criterion) {
    let background = Srgb::new(26u8, 26, 46);
    let hues = [25.0, 60.0, 120.0, 180.0, 240.0, 285.0, 320.0, 350.0];
    let settings = AccentOptSettings::default();
    let min_contrast = 60.0;

    c.bench_function("accent_solver_8_hues", |b| {
        b.iter(|| black_box(optimize_accents(background, &hues, &settings, min_contrast)))
    });
}

criterion_group!(
    benches,
    bench_palette_generation,
    bench_hellwig_from_srgb,
    bench_hellwig_into_srgb,
    bench_apca_contrast,
    bench_gamut_map,
    bench_max_colorfulness_at,
    bench_accent_solver,
    bench_interpolate_bspline_8,
);

criterion_main!(benches);
