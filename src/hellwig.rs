//! Hellwig-Fairchild color appearance model.
//!
//! Wraps palette's CAM16 with:
//! - Improved eccentricity (Munsell-fitted Fourier series)
//! - Helmholtz-Kohlrausch effect for perceptual brightness accuracy
//!
//! Based on Hellwig & Fairchild 2022 papers.

use std::f32::consts::PI;
use std::sync::LazyLock;

use palette::cam16::{BakedParameters, Cam16Jmh, Parameters, StaticWp};
use palette::convert::IntoColorUnclamped;
use palette::white_point::D65;
use palette::{IntoColor, Srgb, Xyz};

use crate::gamut_map::gamut_map;
use crate::generated::{ECCENTRICITY_CAM16_LUT, ECCENTRICITY_LUT, HK_HUE_LUT};

/// Default viewing conditions for sRGB D65 display viewing.
///
/// - adapting_luminance: 64/π × 0.2 cd/m² (gray world assumption)
/// - background_luminance: 20% of Yw=100
/// - surround: average
/// - D65 white point
pub static DEFAULT_PARAMS: LazyLock<BakedParameters<StaticWp<D65>, f32>> = LazyLock::new(|| {
    let adapting_luminance = 64.0 / PI * 0.2;
    Parameters::default_static_wp(adapting_luminance).bake()
});

/// Linear interpolation lookup in a 360-entry hue LUT.
#[inline]
fn lut_lookup(lut: &[f32; 360], hue_rad: f32) -> f32 {
    let hue_deg = hue_rad.to_degrees().rem_euclid(360.0);
    let idx = (hue_deg as usize) % 360; // Handle floating-point edge case at 360.0
    let frac = hue_deg - idx as f32;
    let a = lut[idx];
    let b = lut[(idx + 1) % 360];
    a + (b - a) * frac
}

/// Improved eccentricity using Munsell-fitted Fourier series.
///
/// From Hellwig & Fairchild 2022, provides better hue uniformity than CAM16.
/// Uses precomputed LUT with linear interpolation for efficiency.
///
/// # Arguments
/// * `hue_rad` - Hue angle in radians
///
/// # Returns
/// Eccentricity factor (typically ~1.0, range approximately 0.7-1.3)
#[inline]
pub fn eccentricity(hue_rad: f32) -> f32 {
    lut_lookup(&ECCENTRICITY_LUT, hue_rad)
}

/// Original CAM16 eccentricity function.
///
/// Used to calculate the correction ratio between Hellwig and CAM16.
/// Uses precomputed LUT with linear interpolation for efficiency.
#[inline]
fn eccentricity_cam16(hue_rad: f32) -> f32 {
    lut_lookup(&ECCENTRICITY_CAM16_LUT, hue_rad)
}

/// Hue angle dependency for Helmholtz-Kohlrausch effect.
///
/// Determines how much the HK effect varies with hue.
/// Based on Hellwig & Fairchild 2022 paper on H-K effect.
/// Uses precomputed LUT with linear interpolation for efficiency.
///
/// # Arguments
/// * `hue_rad` - Hue angle in radians
///
/// # Returns
/// HK adjustment factor (range approximately 0.4-1.2)
#[inline]
pub fn hue_angle_dependency(hue_rad: f32) -> f32 {
    lut_lookup(&HK_HUE_LUT, hue_rad)
}

/// Hellwig-Fairchild JMh color with HK effect.
///
/// A perceptually-accurate color appearance model with:
/// - Improved eccentricity (Munsell-fitted)
/// - Helmholtz-Kohlrausch brightness adjustment
///
/// Designed as a drop-in alternative to OKLCH for color manipulation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HellwigJmh {
    /// Lightness with HK adjustment (J_hk), range ~0-101.56
    pub lightness: f32,
    /// Colorfulness (M), typically 0-105
    pub colorfulness: f32,
    /// Hue angle in degrees (0-360)
    pub hue: f32,
}

impl HellwigJmh {
    /// Create a new Hellwig-Fairchild color.
    ///
    /// # Arguments
    /// * `lightness` - Lightness with HK effect (~0-101.56)
    /// * `colorfulness` - Colorfulness (0-105)
    /// * `hue` - Hue angle in degrees (0-360)
    #[inline]
    pub fn new(lightness: f32, colorfulness: f32, hue: f32) -> Self {
        Self {
            lightness,
            colorfulness,
            hue,
        }
    }

    /// Convert from sRGB to Hellwig-Fairchild JMh.
    ///
    /// Applies eccentricity correction and HK effect.
    pub fn from_srgb(srgb: Srgb<f32>) -> Self {
        let xyz: Xyz<D65, f32> = srgb.into_linear().into_color();
        let cam16 = Cam16Jmh::from_xyz(xyz, *DEFAULT_PARAMS);

        let hue_rad = cam16.hue.into_radians();

        // Apply eccentricity correction
        let e_ratio = eccentricity(hue_rad) / eccentricity_cam16(hue_rad);
        let colorfulness = cam16.colorfulness * e_ratio;

        // Apply HK effect: J_hk = J + f(h) * C^0.587
        // Chroma C = M * 35 / a_w (a_w ≈ 100 for default params)
        let chroma = colorfulness * 35.0 / 100.0;
        let lightness = cam16.lightness + hue_angle_dependency(hue_rad) * chroma.powf(0.587);

        Self {
            lightness,
            colorfulness,
            hue: cam16.hue.into_positive_degrees(),
        }
    }

    /// Convert from Hellwig-Fairchild JMh to sRGB.
    ///
    /// Reverses eccentricity correction and HK effect.
    /// Note: Out-of-gamut colors will be clamped.
    pub fn into_srgb(self) -> Srgb<f32> {
        let srgb = self.into_srgb_unclamped();
        Srgb::new(
            srgb.red.clamp(0.0, 1.0),
            srgb.green.clamp(0.0, 1.0),
            srgb.blue.clamp(0.0, 1.0),
        )
    }

    /// Convert from Hellwig-Fairchild JMh to sRGB without clamping.
    ///
    /// Out-of-gamut colors may have values outside [0, 1].
    /// Use `is_in_gamut()` to check before using these values directly.
    pub fn into_srgb_unclamped(self) -> Srgb<f32> {
        let hue_rad = self.hue.to_radians();

        // Reverse HK effect
        let chroma = self.colorfulness * 35.0 / 100.0;
        let lightness_base = self.lightness - hue_angle_dependency(hue_rad) * chroma.powf(0.587);

        // Reverse eccentricity correction
        let e_ratio = eccentricity_cam16(hue_rad) / eccentricity(hue_rad);
        let colorfulness = self.colorfulness * e_ratio;

        let cam16 = Cam16Jmh::new(lightness_base, colorfulness, self.hue);
        let xyz = cam16.into_xyz(*DEFAULT_PARAMS);
        Srgb::from_linear(xyz.into_color_unclamped())
    }

    /// Convert from sRGB u8 to Hellwig-Fairchild JMh.
    pub fn from_srgb_u8(srgb: Srgb<u8>) -> Self {
        let srgb_f32 = Srgb::new(
            srgb.red as f32 / 255.0,
            srgb.green as f32 / 255.0,
            srgb.blue as f32 / 255.0,
        );
        Self::from_srgb(srgb_f32)
    }

    /// Convert to sRGB u8, clamping out-of-gamut values.
    pub fn into_srgb_u8(self) -> Srgb<u8> {
        let srgb = self.into_srgb();
        Srgb::new(
            (srgb.red.clamp(0.0, 1.0) * 255.0).round() as u8,
            (srgb.green.clamp(0.0, 1.0) * 255.0).round() as u8,
            (srgb.blue.clamp(0.0, 1.0) * 255.0).round() as u8,
        )
    }

    /// Check if this color is within sRGB gamut.
    ///
    /// Returns true if the color can be represented in sRGB without clipping.
    /// Uses ULP-based comparison for boundary precision.
    pub fn is_in_gamut(&self) -> bool {
        use float_cmp::approx_eq;

        /// Check if a channel value is within valid sRGB bounds [0, 1].
        /// Uses ULP comparison to handle floating-point precision at boundaries.
        #[inline]
        fn is_channel_in_bounds(c: f32) -> bool {
            (c > 0.0 || approx_eq!(f32, c, 0.0, ulps = 2))
                && (c < 1.0 || approx_eq!(f32, c, 1.0, ulps = 2))
        }

        let srgb = self.into_srgb_unclamped();
        is_channel_in_bounds(srgb.red)
            && is_channel_in_bounds(srgb.green)
            && is_channel_in_bounds(srgb.blue)
    }

    /// Convert to sRGB with perceptual gamut mapping.
    ///
    /// Unlike `into_srgb()` which may produce out-of-gamut values,
    /// this method maps colors to the gamut boundary while preserving hue.
    pub fn into_srgb_gamut_mapped(self) -> Srgb<f32> {
        gamut_map(self).into_srgb()
    }

    /// Convert to sRGB u8 with perceptual gamut mapping.
    ///
    /// Uses ray-tracing gamut mapping to preserve hue when projecting
    /// out-of-gamut colors toward the achromatic axis.
    pub fn into_srgb_u8_gamut_mapped(self) -> Srgb<u8> {
        let srgb = self.into_srgb_gamut_mapped();
        Srgb::new(
            (srgb.red.clamp(0.0, 1.0) * 255.0).round() as u8,
            (srgb.green.clamp(0.0, 1.0) * 255.0).round() as u8,
            (srgb.blue.clamp(0.0, 1.0) * 255.0).round() as u8,
        )
    }
}

/// Get HellwigJmh lightness for an sRGB color.
///
/// Convenience function for quick lightness extraction.
/// Returns J' (lightness with HK effect), range ~0-101.56.
pub fn hellwig_lightness(color: Srgb<u8>) -> f32 {
    HellwigJmh::from_srgb_u8(color).lightness
}
