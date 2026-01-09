//! CLI argument parsing and command handling.

pub use crate::cli_args::*;

use crate::config::{
    AccentOptSettings, ColorConfig, ContrastConfig, HueOverrides, ThemeConfig, ThemeMetadata,
};
use crate::curves::{CurveConfig, CurveType, InterpolationConfig};

impl From<CurveTypeArg> for CurveType {
    fn from(arg: CurveTypeArg) -> Self {
        match arg {
            CurveTypeArg::Linear => CurveType::Linear,
            CurveTypeArg::Smoothstep => CurveType::Smoothstep,
            CurveTypeArg::Smootherstep => CurveType::Smootherstep,
            CurveTypeArg::SmoothStart => CurveType::SmoothStart,
            CurveTypeArg::SmoothEnd => CurveType::SmoothEnd,
            CurveTypeArg::Sigmoid => CurveType::Sigmoid,
        }
    }
}

impl Cli {
    /// Build the hue overrides array from CLI flags.
    pub fn hue_overrides(&self) -> [Option<f32>; 8] {
        [
            self.hue_08,
            self.hue_09,
            self.hue_0a,
            self.hue_0b,
            self.hue_0c,
            self.hue_0d,
            self.hue_0e,
            self.hue_0f,
        ]
    }

    /// Build InterpolationConfig from CLI arguments, using defaults where not specified.
    pub fn interpolation_config(&self) -> InterpolationConfig {
        let defaults = InterpolationConfig::default();
        InterpolationConfig {
            lightness: CurveConfig {
                curve_type: self
                    .lightness_curve
                    .map(Into::into)
                    .unwrap_or(defaults.lightness.curve_type),
                strength: self
                    .lightness_strength
                    .unwrap_or(defaults.lightness.strength),
                control_points: None,
            },
            chroma: CurveConfig {
                curve_type: self
                    .chroma_curve
                    .map(Into::into)
                    .unwrap_or(defaults.chroma.curve_type),
                strength: defaults.chroma.strength,
                control_points: None,
            },
            hue: CurveConfig {
                curve_type: self
                    .hue_curve
                    .map(Into::into)
                    .unwrap_or(defaults.hue.curve_type),
                strength: defaults.hue.strength,
                control_points: None,
            },
        }
    }

    /// Build AccentOptSettings from CLI arguments, using defaults where not specified.
    pub fn accent_opt_settings(&self) -> AccentOptSettings {
        let defaults = AccentOptSettings::default();
        AccentOptSettings {
            target_j: self.target_j.unwrap_or(defaults.target_j),
            target_m: self.target_m.unwrap_or(defaults.target_m),
            delta_j: self.delta_j.unwrap_or(defaults.delta_j),
            delta_m: self.delta_m.unwrap_or(defaults.delta_m),
            j_weight: self.j_weight.unwrap_or(defaults.j_weight),
            contrast_weight: self.contrast_weight.unwrap_or(defaults.contrast_weight),
        }
    }

    /// Convert flat CLI args to nested ThemeConfig for Figment merging.
    ///
    /// Only fields that are explicitly set on the CLI will be included in the
    /// returned config (via `skip_serializing_if`), allowing proper layering
    /// with TOML file settings.
    pub fn to_config_overrides(&self) -> ThemeConfig {
        let defaults = ThemeConfig::default();

        // Build hue overrides only if any hue is specified
        let hue_overrides = {
            let arr = self.hue_overrides();
            if arr.iter().any(|h| h.is_some()) {
                Some(HueOverrides::from_array(arr))
            } else {
                None
            }
        };

        // Build interpolation config, using defaults for unspecified curves
        let curves = InterpolationConfig {
            lightness: CurveConfig {
                curve_type: self
                    .lightness_curve
                    .map(Into::into)
                    .unwrap_or(defaults.curves.lightness.curve_type),
                strength: self
                    .lightness_strength
                    .unwrap_or(defaults.curves.lightness.strength),
                control_points: None,
            },
            chroma: CurveConfig {
                curve_type: self
                    .chroma_curve
                    .map(Into::into)
                    .unwrap_or(defaults.curves.chroma.curve_type),
                strength: defaults.curves.chroma.strength,
                control_points: None,
            },
            hue: CurveConfig {
                curve_type: self
                    .hue_curve
                    .map(Into::into)
                    .unwrap_or(defaults.curves.hue.curve_type),
                strength: defaults.curves.hue.strength,
                control_points: None,
            },
        };

        // Build optimization settings, using defaults for unspecified values
        let optimization = AccentOptSettings {
            target_j: self.target_j.unwrap_or(defaults.optimization.target_j),
            target_m: self.target_m.unwrap_or(defaults.optimization.target_m),
            delta_j: self.delta_j.unwrap_or(defaults.optimization.delta_j),
            delta_m: self.delta_m.unwrap_or(defaults.optimization.delta_m),
            j_weight: self.j_weight.unwrap_or(defaults.optimization.j_weight),
            contrast_weight: self
                .contrast_weight
                .unwrap_or(defaults.optimization.contrast_weight),
        };

        ThemeConfig {
            theme: ThemeMetadata {
                name: self.name.clone().unwrap_or_default(),
                author: self.author.clone(),
                variant: None,
            },
            colors: ColorConfig {
                background: self.background.clone(),
                foreground: self.foreground.clone(),
                hue_overrides,
            },
            curves,
            contrast: ContrastConfig {
                minimum: self.min_contrast.unwrap_or(defaults.contrast.minimum),
                extended_minimum: self
                    .extended_min_contrast
                    .unwrap_or(defaults.contrast.extended_minimum),
                max_adjustment: self
                    .max_lightness_adjustment
                    .unwrap_or(defaults.contrast.max_adjustment),
            },
            extended_optimization: defaults.extended_optimization.clone(),
            optimization,
        }
    }
}
