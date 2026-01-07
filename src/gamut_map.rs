//! Ray tracing gamut mapping in HellwigJmh space.
//!
//! Provides perceptually-accurate gamut mapping that preserves hue
//! when projecting out-of-gamut colors toward the achromatic axis.

use std::cell::RefCell;

use crate::generated::CUSP_LUT;
use crate::hellwig::HellwigJmh;

/// Maximum Newton-Raphson iterations before fallback to triangle estimate.
const MAX_NEWTON_ITERS: usize = 5;

/// Convergence threshold for Newton-Raphson (in sRGB channel units).
const NEWTON_TOLERANCE: f32 = 0.001;

/// Minimum J' for safe gamut mapping (avoid numerical instability near black).
const MIN_SAFE_J: f32 = 5.0;

/// Maximum J' for safe gamut mapping (avoid numerical instability near white).
const MAX_SAFE_J: f32 = 98.0;

/// J' resolution for cache buckets (0.1 = 1000 buckets for 0-100 range).
const J_RESOLUTION: f32 = 0.1;

/// Number of J' buckets in the cache.
const J_BUCKETS: usize = 1000;

/// Hue resolution for cache buckets (0.1 = 3600 buckets for 0-360 range).
const HUE_RESOLUTION: f32 = 0.1;

/// Number of hue buckets in the cache (0.1 degree precision).
const HUE_BUCKETS: usize = 3600;

/// Thread-local cache for gamut boundary M values.
///
/// Stores the maximum in-gamut colorfulness M for each (J', hue) pair.
/// Gamut boundaries are mathematically constant, so no invalidation is needed.
/// Uses Vec<Vec<...>> to avoid stack overflow during initialization.
struct GamutCache {
    data: Vec<Vec<Option<f32>>>,
}

impl GamutCache {
    fn new() -> Self {
        Self {
            data: vec![vec![None; HUE_BUCKETS]; J_BUCKETS],
        }
    }

    fn j_to_bucket(j: f32) -> usize {
        ((j / J_RESOLUTION) as usize).min(J_BUCKETS - 1)
    }

    fn hue_to_bucket(hue: f32) -> usize {
        ((hue.rem_euclid(360.0) / HUE_RESOLUTION) as usize) % HUE_BUCKETS
    }

    fn get(&self, j: f32, hue: f32) -> Option<f32> {
        let j_idx = Self::j_to_bucket(j);
        let h_idx = Self::hue_to_bucket(hue);
        self.data[j_idx][h_idx]
    }

    fn put(&mut self, j: f32, hue: f32, m_max: f32) {
        let j_idx = Self::j_to_bucket(j);
        let h_idx = Self::hue_to_bucket(hue);
        self.data[j_idx][h_idx] = Some(m_max);
    }

    fn clear(&mut self) {
        for j_slice in self.data.iter_mut() {
            for entry in j_slice.iter_mut() {
                *entry = None;
            }
        }
    }
}

thread_local! {
    static GAMUT_CACHE: RefCell<GamutCache> = RefCell::new(GamutCache::new());
}

/// Clear the thread-local gamut cache.
///
/// Useful for testing or when you need deterministic behavior.
pub fn clear_gamut_cache() {
    GAMUT_CACHE.with(|c| c.borrow_mut().clear());
}

/// Get the center J' value for the bucket containing the given J'.
#[inline]
fn bucket_center_j(j: f32) -> f32 {
    let bucket = GamutCache::j_to_bucket(j);
    (bucket as f32 + 0.5) * J_RESOLUTION
}

/// Get the center hue value for the bucket containing the given hue.
#[inline]
fn bucket_center_hue(hue: f32) -> f32 {
    let bucket = GamutCache::hue_to_bucket(hue);
    (bucket as f32 + 0.5) * HUE_RESOLUTION
}

/// Cusp data for a single hue: (J'_cusp, M_cusp).
#[derive(Debug, Clone, Copy)]
pub struct Cusp {
    pub j: f32,
    pub m: f32,
}

/// Look up cusp for a given hue using linear interpolation.
#[inline]
pub fn cusp_at_hue(hue_deg: f32) -> Cusp {
    let hue = hue_deg.rem_euclid(360.0);
    let idx = (hue as usize) % 360;
    let frac = hue - idx as f32;

    let c0 = CUSP_LUT[idx];
    let c1 = CUSP_LUT[(idx + 1) % 360];

    Cusp {
        j: c0.0 + (c1.0 - c0.0) * frac,
        m: c0.1 + (c1.1 - c0.1) * frac,
    }
}

/// Estimate gamut boundary M using triangle approximation.
///
/// Returns the maximum M for a given J' and hue, based on linear
/// interpolation between black, cusp, and white.
#[inline]
fn triangle_estimate(j: f32, cusp: Cusp) -> f32 {
    if j <= cusp.j {
        cusp.m * (j / cusp.j.max(0.001))
    } else {
        cusp.m * ((100.0 - j) / (100.0 - cusp.j).max(0.001))
    }
}

/// Find the limiting RGB channel and its bound for an out-of-gamut color.
///
/// Returns (channel_index, bound, signed_distance) where:
/// - channel_index: 0=R, 1=G, 2=B
/// - bound: 0.0 or 1.0
/// - signed_distance: how far the channel is from its bound (positive = out of gamut)
fn find_limiting_channel(r: f32, g: f32, b: f32) -> (usize, f32, f32) {
    let channels = [r, g, b];

    let mut worst_idx = 0;
    let mut worst_dist = 0.0f32;
    let mut worst_bound = 0.0f32;

    for (i, &c) in channels.iter().enumerate() {
        let dist_to_0 = -c; // Positive if c < 0
        let dist_to_1 = c - 1.0; // Positive if c > 1

        if dist_to_0 > worst_dist {
            worst_idx = i;
            worst_dist = dist_to_0;
            worst_bound = 0.0;
        }
        if dist_to_1 > worst_dist {
            worst_idx = i;
            worst_dist = dist_to_1;
            worst_bound = 1.0;
        }
    }

    (worst_idx, worst_bound, worst_dist)
}

/// Get a single RGB channel from HellwigJmh.
fn get_channel(j: f32, m: f32, h: f32, channel: usize) -> f32 {
    let srgb = HellwigJmh::new(j, m, h).into_srgb();
    match channel {
        0 => srgb.red,
        1 => srgb.green,
        _ => srgb.blue,
    }
}

/// Numerical derivative dc/dM for a specific RGB channel.
fn dc_dm(j: f32, m: f32, h: f32, channel: usize) -> f32 {
    const EPS: f32 = 0.0001;
    (get_channel(j, m + EPS, h, channel) - get_channel(j, m - EPS, h, channel)) / (2.0 * EPS)
}

/// Newton-Raphson refinement to find exact gamut boundary.
///
/// Starting from triangle estimate, refines M until the limiting
/// RGB channel exactly hits its bound.
fn newton_refine(j: f32, h: f32, m_initial: f32) -> f32 {
    let mut m = m_initial;

    for _iter in 0..MAX_NEWTON_ITERS {
        let srgb = HellwigJmh::new(j, m, h).into_srgb();
        let (channel, bound, distance) = find_limiting_channel(srgb.red, srgb.green, srgb.blue);

        // Check convergence
        if distance.abs() < NEWTON_TOLERANCE {
            return m;
        }

        // If color is in gamut (distance <= 0), we've gone too far
        if distance <= 0.0 {
            return m;
        }

        // Newton step: M_new = M - f(M) / f'(M)
        // where f(M) = c(J', M, h) - bound
        let derivative = dc_dm(j, m, h, channel);

        // Avoid division by zero
        if derivative.abs() < 1e-10 {
            tracing::warn!(j, m, h, "Newton-Raphson derivative near zero");
            return m;
        }

        let c_current = match channel {
            0 => srgb.red,
            1 => srgb.green,
            _ => srgb.blue,
        };

        let step = (c_current - bound) / derivative;
        m -= step;

        // Clamp to valid range
        m = m.clamp(0.0, 150.0);
    }

    tracing::warn!(j, m_initial, h, "Newton-Raphson failed to converge");
    m
}

/// Map an out-of-gamut color to the sRGB gamut boundary.
///
/// Preserves hue by projecting toward the achromatic axis (M=0).
/// Uses cached gamut boundary lookup for efficiency.
///
/// # Algorithm
///
/// 1. Check if color is in gamut - return unchanged if so
/// 2. Handle edge cases (extreme J') with achromatic fallback
/// 3. Look up cached maximum colorfulness for (J', hue)
/// 4. Verify result is in gamut, reduce M if needed (bucket-center approximation)
/// 5. Return color with M clamped to boundary
pub fn gamut_map(color: HellwigJmh) -> HellwigJmh {
    // Fast path: check if already in gamut
    if color.is_in_gamut() {
        return color;
    }

    // Edge cases: extreme lightness - fall back to achromatic
    if color.lightness < MIN_SAFE_J || color.lightness > MAX_SAFE_J {
        tracing::warn!(j = %color.lightness, "Extreme lightness, falling back to achromatic");
        return HellwigJmh::new(color.lightness.clamp(0.0, 100.0), 0.0, color.hue);
    }

    // Use cached boundary lookup (computed at bucket center)
    let mut m_boundary = max_colorfulness_at(color.lightness, color.hue);
    m_boundary = m_boundary.min(color.colorfulness);

    // Verify result is in gamut (bucket-center value may be slightly off for edge queries)
    let mut result = HellwigJmh::new(color.lightness, m_boundary, color.hue);
    if !result.is_in_gamut() {
        // Binary search to find safe M for actual coordinates
        let mut lo = 0.0;
        let mut hi = m_boundary;
        while hi - lo > 0.01 {
            let mid = (lo + hi) / 2.0;
            if HellwigJmh::new(color.lightness, mid, color.hue).is_in_gamut() {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        result = HellwigJmh::new(color.lightness, lo, color.hue);
    }

    result
}

/// Find the maximum in-gamut M for a given J' and hue.
///
/// Useful for optimization and constraint checking.
/// Uses ULP-aware gamut checking from `is_in_gamut()` for accurate boundary detection.
/// Results are cached in a thread-local 2D cache for performance.
///
/// Computes boundaries at bucket centers to ensure deterministic results
/// regardless of query order within the same bucket.
pub fn max_colorfulness_at(j: f32, hue: f32) -> f32 {
    // Edge cases
    if !(MIN_SAFE_J..=MAX_SAFE_J).contains(&j) {
        return 0.0;
    }

    // Check cache first
    if let Some(m_max) = GAMUT_CACHE.with(|c| c.borrow().get(j, hue)) {
        return m_max;
    }

    // Use bucket centers for deterministic computation
    let j_center = bucket_center_j(j);
    let hue_center = bucket_center_hue(hue);

    // Compute the boundary at bucket center
    let cusp = cusp_at_hue(hue_center);
    let estimate = triangle_estimate(j_center, cusp);

    // Check if estimate is in gamut
    let test = HellwigJmh::new(j_center, estimate, hue_center);
    let m_center = if test.is_in_gamut() {
        // Estimate is conservative - we might be able to go higher
        // Use binary search to find exact boundary
        let mut lo = estimate;
        let mut hi = estimate * 1.5;

        while hi - lo > 0.01 {
            let mid = (lo + hi) / 2.0;
            if HellwigJmh::new(j_center, mid, hue_center).is_in_gamut() {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        lo
    } else {
        newton_refine(j_center, hue_center, estimate)
    };

    // Note: m_center is computed at bucket center, so it may be slightly out of gamut
    // for edge values in the bucket. This is acceptable (error < 0.05 in J' and hue).
    let m_max = m_center;

    // Store in cache
    GAMUT_CACHE.with(|c| c.borrow_mut().put(j, hue, m_max));
    m_max
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn cusp_lookup_interpolates() {
        let c0 = cusp_at_hue(0.0);
        let c1 = cusp_at_hue(1.0);
        let c_half = cusp_at_hue(0.5);

        // Halfway point should be interpolated
        assert_relative_eq!(c_half.j, (c0.j + c1.j) / 2.0, epsilon = 0.01);
        assert_relative_eq!(c_half.m, (c0.m + c1.m) / 2.0, epsilon = 0.01);
    }

    #[test]
    fn cusp_wraps_at_360() {
        let c0 = cusp_at_hue(0.0);
        let c360 = cusp_at_hue(360.0);

        assert_relative_eq!(c0.j, c360.j, epsilon = 0.01);
        assert_relative_eq!(c0.m, c360.m, epsilon = 0.01);
    }

    #[test]
    fn in_gamut_unchanged() {
        let color = HellwigJmh::new(50.0, 10.0, 180.0);
        assert!(color.is_in_gamut());

        let mapped = gamut_map(color);
        assert_relative_eq!(mapped.lightness, color.lightness, epsilon = 0.001);
        assert_relative_eq!(mapped.colorfulness, color.colorfulness, epsilon = 0.001);
        assert_relative_eq!(mapped.hue, color.hue, epsilon = 0.001);
    }

    #[test]
    fn out_of_gamut_reduces_colorfulness() {
        let color = HellwigJmh::new(50.0, 100.0, 25.0); // Very saturated red
        assert!(!color.is_in_gamut());

        let mapped = gamut_map(color);
        assert!(mapped.is_in_gamut());
        assert!(mapped.colorfulness < color.colorfulness);
    }

    #[test]
    fn preserves_lightness() {
        for hue in (0..360).step_by(30) {
            let color = HellwigJmh::new(60.0, 120.0, hue as f32);
            let mapped = gamut_map(color);

            assert!(
                (mapped.lightness - color.lightness).abs() < 0.1,
                "Lightness changed from {} to {} at hue {}",
                color.lightness,
                mapped.lightness,
                hue
            );
        }
    }

    #[test]
    fn preserves_hue() {
        for hue in (0..360).step_by(30) {
            let color = HellwigJmh::new(60.0, 120.0, hue as f32);
            let mapped = gamut_map(color);

            assert!(
                (mapped.hue - color.hue).abs() < 0.1,
                "Hue changed from {} to {}",
                color.hue,
                mapped.hue
            );
        }
    }

    #[test]
    fn extreme_j_falls_back_to_achromatic() {
        let dark = HellwigJmh::new(2.0, 50.0, 180.0);
        let mapped = gamut_map(dark);
        assert_relative_eq!(mapped.colorfulness, 0.0, epsilon = 0.001);

        let light = HellwigJmh::new(99.5, 50.0, 180.0);
        let mapped = gamut_map(light);
        assert_relative_eq!(mapped.colorfulness, 0.0, epsilon = 0.001);
    }

    #[test]
    fn max_colorfulness_reasonable() {
        // At the cusp hue for red (~25-30°), max M should be high
        for hue in (0..360).step_by(30) {
            let max_m = max_colorfulness_at(50.0, hue as f32);
            assert!(max_m > 0.0, "max_colorfulness_at(50, {}) = {}", hue, max_m);

            // Verify the returned M is close to in-gamut (may be slightly out due to bucket centering)
            // Use bucket center values for the test since that's what max_colorfulness_at computes for
            let j_center = bucket_center_j(50.0);
            let hue_center = bucket_center_hue(hue as f32);
            let test = HellwigJmh::new(j_center, max_m, hue_center);
            assert!(
                test.is_in_gamut() || max_m < 0.5,
                "max_colorfulness_at(50, {}) = {} is out of gamut at bucket center ({}, {})",
                hue,
                max_m,
                j_center,
                hue_center
            );
        }
    }

    #[test]
    fn cache_hit_on_repeated_query() {
        clear_gamut_cache();
        let m1 = max_colorfulness_at(50.0, 180.0);
        let m2 = max_colorfulness_at(50.0, 180.0);
        assert_eq!(m1, m2);
    }

    #[test]
    fn cache_quantizes_nearby_values() {
        clear_gamut_cache();
        // Values within the same bucket (J_RESOLUTION = 0.1) should return the same result
        let m1 = max_colorfulness_at(50.0, 180.0);
        let m2 = max_colorfulness_at(50.05, 180.0); // Same J bucket (both map to bucket 500)
        assert_eq!(m1, m2);
    }

    #[test]
    fn max_colorfulness_deterministic_across_bucket() {
        // Different exact values in the same bucket should produce identical results
        // because we compute at bucket centers
        clear_gamut_cache();
        let m1 = max_colorfulness_at(50.01, 250.03);
        clear_gamut_cache();
        let m2 = max_colorfulness_at(50.08, 250.08); // Same bucket, different exact values
        assert_eq!(m1, m2, "Same bucket should produce same result");
    }

    #[test]
    fn cache_clear_works() {
        clear_gamut_cache();
        let _ = max_colorfulness_at(50.0, 180.0);
        clear_gamut_cache();
        // After clear, cache should be empty - this should recompute
        // We can't directly test this without hit/miss tracking, but we can verify
        // the function still returns correct values after clear
        let m = max_colorfulness_at(50.0, 180.0);
        assert!(m > 0.0);
    }

    #[test]
    fn cache_handles_hue_wrapping() {
        clear_gamut_cache();
        // Hue 0 and 360 should map to the same bucket
        let m1 = max_colorfulness_at(50.0, 0.0);
        let m2 = max_colorfulness_at(50.0, 360.0);
        assert_eq!(m1, m2);
    }
}
