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
use palette::white_point::D65;
use palette::{IntoColor, Srgb, Xyz};

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

/// Improved eccentricity using Munsell-fitted Fourier series.
///
/// From Hellwig & Fairchild 2022, provides better hue uniformity than CAM16.
///
/// # Arguments
/// * `hue_rad` - Hue angle in radians
///
/// # Returns
/// Eccentricity factor (typically ~1.0, range approximately 0.7-1.3)
#[inline]
pub fn eccentricity(hue_rad: f32) -> f32 {
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

/// Original CAM16 eccentricity function.
///
/// Used to calculate the correction ratio between Hellwig and CAM16.
#[inline]
fn eccentricity_cam16(hue_rad: f32) -> f32 {
    0.25 * ((hue_rad + 2.0).cos() + 3.8)
}

/// Hue angle dependency for Helmholtz-Kohlrausch effect.
///
/// Determines how much the HK effect varies with hue.
/// Based on Hellwig & Fairchild 2022 paper on H-K effect.
///
/// # Arguments
/// * `hue_rad` - Hue angle in radians
///
/// # Returns
/// HK adjustment factor (range approximately 0.4-1.2)
#[inline]
pub fn hue_angle_dependency(hue_rad: f32) -> f32 {
    let h = hue_rad;
    let h2 = 2.0 * h;

    -0.160 * h.cos() + 0.132 * h2.cos() - 0.405 * h.sin() + 0.080 * h2.sin() + 0.792
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
    pub fn into_srgb(self) -> Srgb<f32> {
        let hue_rad = self.hue.to_radians();

        // Reverse HK effect
        let chroma = self.colorfulness * 35.0 / 100.0;
        let lightness_base = self.lightness - hue_angle_dependency(hue_rad) * chroma.powf(0.587);

        // Reverse eccentricity correction
        let e_ratio = eccentricity_cam16(hue_rad) / eccentricity(hue_rad);
        let colorfulness = self.colorfulness * e_ratio;

        let cam16 = Cam16Jmh::new(lightness_base, colorfulness, self.hue);
        let xyz = cam16.into_xyz(*DEFAULT_PARAMS);
        Srgb::from_linear(xyz.into_color())
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
    pub fn is_in_gamut(&self) -> bool {
        let srgb = self.into_srgb();
        srgb.red >= 0.0
            && srgb.red <= 1.0
            && srgb.green >= 0.0
            && srgb.green <= 1.0
            && srgb.blue >= 0.0
            && srgb.blue <= 1.0
    }

    /// Convert to sRGB with gamut clamping and return the post-clamp HellwigJmh.
    ///
    /// This gives the "actual" color appearance after gamut mapping.
    /// Useful for measuring the true perceived lightness of a color
    /// that may be out of sRGB gamut.
    pub fn gamut_mapped(self) -> Self {
        Self::from_srgb_u8(self.into_srgb_u8())
    }
}

/// Get HellwigJmh lightness for an sRGB color.
///
/// Convenience function for quick lightness extraction.
/// Returns J' (lightness with HK effect), range ~0-101.56.
pub fn hellwig_lightness(color: Srgb<u8>) -> f32 {
    HellwigJmh::from_srgb_u8(color).lightness
}

/// Compute post-clamp lightness for given Hellwig parameters.
///
/// Creates a HellwigJmh color, converts to sRGB (with clamping), and returns
/// the lightness of the clamped color. This gives the "actual" perceived
/// lightness after gamut mapping.
///
/// # Arguments
/// * `lightness` - Input lightness (J' scale, 0-100)
/// * `colorfulness` - Colorfulness (M)
/// * `hue` - Hue angle in degrees
///
/// # Returns
/// The post-clamp lightness (J' of the gamut-mapped color)
pub fn post_clamp_lightness(lightness: f32, colorfulness: f32, hue: f32) -> f32 {
    HellwigJmh::new(lightness, colorfulness, hue)
        .gamut_mapped()
        .lightness
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn eccentricity_range() {
        for hue_deg in (0..360).step_by(10) {
            let hue_rad = (hue_deg as f32).to_radians();
            let e = eccentricity(hue_rad);
            assert!(
                e > 0.5 && e < 1.5,
                "eccentricity at {}° = {} (expected 0.5-1.5)",
                hue_deg,
                e
            );
        }
    }

    #[test]
    fn hue_dependency_range() {
        for hue_deg in (0..360).step_by(10) {
            let hue_rad = (hue_deg as f32).to_radians();
            let f = hue_angle_dependency(hue_rad);
            assert!(
                f > 0.2 && f < 1.5,
                "HK factor at {}° = {} (expected 0.2-1.5)",
                hue_deg,
                f
            );
        }
    }

    #[test]
    fn roundtrip_chromatic() {
        let original = Srgb::new(0.5f32, 0.3, 0.8);
        let hellwig = HellwigJmh::from_srgb(original);
        let result = hellwig.into_srgb();

        assert_relative_eq!(original.red, result.red, epsilon = 0.01);
        assert_relative_eq!(original.green, result.green, epsilon = 0.01);
        assert_relative_eq!(original.blue, result.blue, epsilon = 0.01);
    }

    #[test]
    fn roundtrip_gray() {
        let original = Srgb::new(0.5f32, 0.5, 0.5);
        let hellwig = HellwigJmh::from_srgb(original);
        let result = hellwig.into_srgb();

        assert_relative_eq!(original.red, result.red, epsilon = 0.01);
        assert_relative_eq!(original.green, result.green, epsilon = 0.01);
        assert_relative_eq!(original.blue, result.blue, epsilon = 0.01);
    }

    #[test]
    fn black_has_zero_lightness() {
        let black = Srgb::new(0.0f32, 0.0, 0.0);
        let hellwig = HellwigJmh::from_srgb(black);
        assert!(hellwig.lightness < 0.01);
    }

    #[test]
    fn white_has_high_lightness() {
        let white = Srgb::new(1.0f32, 1.0, 1.0);
        let hellwig = HellwigJmh::from_srgb(white);
        assert!(hellwig.lightness > 95.0);
    }

    #[test]
    fn gray_has_low_colorfulness() {
        let gray = Srgb::new(0.5f32, 0.5, 0.5);
        let hellwig = HellwigJmh::from_srgb(gray);
        assert!(
            hellwig.colorfulness < 2.0,
            "Gray colorfulness = {} (expected < 2.0)",
            hellwig.colorfulness
        );
    }
}
