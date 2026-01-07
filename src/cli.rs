//! CLI argument parsing and command handling.

use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use serde::Serialize;

use crate::config::{
    AccentOptSettings, ColorConfig, ContrastConfig, HueOverrides, ThemeConfig, ThemeMetadata,
};
use crate::curves::{CurveConfig, CurveType, InterpolationConfig};

/// Output variant selection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum VariantArg {
    /// Auto-detect from background luminance
    #[default]
    Auto,
    /// Force dark variant
    Dark,
    /// Force light variant
    Light,
    /// Generate both variants (requires --output)
    Both,
}

/// CLI-compatible curve type enum.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum, Serialize)]
pub enum CurveTypeArg {
    /// Linear interpolation (no easing)
    #[default]
    Linear,
    /// Smooth S-curve (Hermite)
    Smoothstep,
    /// Smoother S-curve (Ken Perlin)
    Smootherstep,
    /// Ease-in (accelerating start)
    SmoothStart,
    /// Ease-out (decelerating end)
    SmoothEnd,
    /// Configurable S-curve (use with --lightness-strength)
    Sigmoid,
}

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

/// Base24 palette generator using HellwigJmh color space with APCA validation.
#[derive(Parser, Debug, Serialize)]
#[command(name = "themalingadingdong")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Background color (base00) in hex format (#RRGGBB or RRGGBB)
    #[arg(
        short,
        long,
        default_value_if("interactive", "true", "#000000"),
        required_unless_present_any = ["interactive", "config"]
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,

    /// Foreground color (base07) in hex format (#RRGGBB or RRGGBB)
    #[arg(
        short,
        long,
        default_value_if("interactive", "true", "#FFFFFF"),
        required_unless_present_any = ["interactive", "config"]
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreground: Option<String>,

    /// Minimum APCA contrast for accent colors (floor, not exact target)
    /// Colors will achieve at least this contrast while maintaining uniform lightness.
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_contrast: Option<f64>,

    /// Minimum APCA contrast for extended accent colors base10-base17
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extended_min_contrast: Option<f64>,

    /// Maximum per-hue lightness adjustment (0-10 J' units, default 2.0)
    /// Small adjustments help difficult hues reach minimum contrast.
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_lightness_adjustment: Option<f32>,

    // Individual hue overrides (base08-base0F)
    // Default values come from DEFAULT_BASE16_HUES lookup table
    /// Override hue for base08 (Red). Default: 25 degrees
    #[arg(long, value_name = "DEGREES")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hue_08: Option<f32>,

    /// Override hue for base09 (Orange). Default: 55 degrees
    #[arg(long, value_name = "DEGREES")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hue_09: Option<f32>,

    /// Override hue for base0A (Yellow). Default: 90 degrees
    #[arg(long, value_name = "DEGREES")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hue_0a: Option<f32>,

    /// Override hue for base0B (Green). Default: 145 degrees
    #[arg(long, value_name = "DEGREES")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hue_0b: Option<f32>,

    /// Override hue for base0C (Cyan). Default: 180 degrees
    #[arg(long, value_name = "DEGREES")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hue_0c: Option<f32>,

    /// Override hue for base0D (Blue). Default: 250 degrees
    #[arg(long, value_name = "DEGREES")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hue_0d: Option<f32>,

    /// Override hue for base0E (Purple). Default: 285 degrees
    #[arg(long, value_name = "DEGREES")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hue_0e: Option<f32>,

    /// Override hue for base0F (Magenta). Default: 335 degrees
    #[arg(long, value_name = "DEGREES")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hue_0f: Option<f32>,

    /// Scheme name
    #[arg(
        long,
        default_value_if("interactive", "true", "My Theme"),
        required_unless_present_any = ["interactive", "config"]
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Author name
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Output file (stdout if not specified, required for --variant both)
    #[arg(short, long)]
    #[serde(skip)]
    pub output: Option<std::path::PathBuf>,

    /// Output variant: auto (detect from background), dark, light, or both
    #[arg(long, value_enum, default_value_t = VariantArg::Auto)]
    #[serde(skip)]
    pub variant: VariantArg,

    /// Fail on validation errors instead of auto-adjusting
    #[arg(long)]
    #[serde(skip)]
    pub no_adjust: bool,

    /// Launch interactive TUI for previewing and editing the palette
    #[arg(short, long)]
    #[serde(skip)]
    pub interactive: bool,

    // Curve configuration
    /// Lightness interpolation curve type
    #[arg(long, value_enum)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lightness_curve: Option<CurveTypeArg>,

    /// Lightness curve strength (for sigmoid, 1.0-10.0)
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lightness_strength: Option<f32>,

    /// Chroma interpolation curve type
    #[arg(long, value_enum)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chroma_curve: Option<CurveTypeArg>,

    /// Hue interpolation curve type
    #[arg(long, value_enum)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hue_curve: Option<CurveTypeArg>,

    /// Load configuration from TOML file
    #[arg(long, value_name = "FILE")]
    #[serde(skip)]
    pub config: Option<PathBuf>,

    /// Save current configuration to TOML file
    #[arg(long, value_name = "FILE")]
    #[serde(skip)]
    pub save_config: Option<PathBuf>,

    /// Log file path (default: themalingadingdong.log)
    #[arg(long, value_name = "FILE")]
    #[serde(skip)]
    pub log_file: Option<PathBuf>,

    /// Log level: trace, debug, info, warn, error (default: info)
    #[arg(long, value_name = "LEVEL", default_value = "info")]
    #[serde(skip)]
    pub log_level: String,

    // Accent optimization settings (for future COBYLA solver)
    /// Target lightness (J') for accent colors
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_j: Option<f32>,

    /// Target colorfulness (M) for accent colors
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_m: Option<f32>,

    /// Max lightness deviation from target (box constraint)
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta_j: Option<f32>,

    /// Max colorfulness deviation from target (box constraint)
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta_m: Option<f32>,

    /// Weight for uniformity vs vibrancy (0=vibrancy, 1=uniformity)
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub j_weight: Option<f32>,

    /// Weight for contrast in optimization (0=ignore, 1=prioritize)
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contrast_weight: Option<f32>,
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
