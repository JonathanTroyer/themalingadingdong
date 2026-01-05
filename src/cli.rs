//! CLI argument parsing and command handling.

use std::path::PathBuf;

use clap::{Parser, ValueEnum};

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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
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
#[derive(Parser, Debug)]
#[command(name = "themalingadingdong")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Background color (base00) in hex format (#RRGGBB or RRGGBB)
    #[arg(
        short,
        long,
        default_value_if("interactive", "true", "#000000"),
        required_unless_present = "interactive"
    )]
    pub background: Option<String>,

    /// Foreground color (base07) in hex format (#RRGGBB or RRGGBB)
    #[arg(
        short,
        long,
        default_value_if("interactive", "true", "#FFFFFF"),
        required_unless_present = "interactive"
    )]
    pub foreground: Option<String>,

    /// Minimum APCA contrast for accent colors (floor, not exact target)
    /// Colors will achieve at least this contrast while maintaining uniform lightness.
    #[arg(long, default_value_t = 75.0)]
    pub min_contrast: f64,

    /// Minimum APCA contrast for extended accent colors base10-base17
    #[arg(long, default_value_t = 60.0)]
    pub extended_min_contrast: f64,

    /// Maximum per-hue lightness adjustment (0-10 J' units, default 2.0)
    /// Small adjustments help difficult hues reach minimum contrast.
    #[arg(long, default_value_t = 2.0)]
    pub max_lightness_adjustment: f32,

    /// Colorfulness for accent colors (HellwigJmh M, 0-50 typical)
    #[arg(long, default_value_t = 25.0)]
    pub accent_colorfulness: f32,

    /// Colorfulness for extended accent colors base10-base17 (HellwigJmh M, 0-50 typical)
    #[arg(long, default_value_t = 35.0)]
    pub extended_colorfulness: f32,

    // Individual hue overrides (base08-base0F)
    // Default values come from DEFAULT_BASE16_HUES lookup table
    /// Override hue for base08 (Red). Default: 25°
    #[arg(long, value_name = "DEGREES")]
    pub hue_08: Option<f32>,

    /// Override hue for base09 (Orange). Default: 55°
    #[arg(long, value_name = "DEGREES")]
    pub hue_09: Option<f32>,

    /// Override hue for base0A (Yellow). Default: 90°
    #[arg(long, value_name = "DEGREES")]
    pub hue_0a: Option<f32>,

    /// Override hue for base0B (Green). Default: 145°
    #[arg(long, value_name = "DEGREES")]
    pub hue_0b: Option<f32>,

    /// Override hue for base0C (Cyan). Default: 180°
    #[arg(long, value_name = "DEGREES")]
    pub hue_0c: Option<f32>,

    /// Override hue for base0D (Blue). Default: 250°
    #[arg(long, value_name = "DEGREES")]
    pub hue_0d: Option<f32>,

    /// Override hue for base0E (Purple). Default: 285°
    #[arg(long, value_name = "DEGREES")]
    pub hue_0e: Option<f32>,

    /// Override hue for base0F (Magenta). Default: 335°
    #[arg(long, value_name = "DEGREES")]
    pub hue_0f: Option<f32>,

    /// Scheme name
    #[arg(
        long,
        default_value_if("interactive", "true", "My Theme"),
        required_unless_present = "interactive"
    )]
    pub name: Option<String>,

    /// Author name
    #[arg(long)]
    pub author: Option<String>,

    /// Output file (stdout if not specified, required for --variant both)
    #[arg(short, long)]
    pub output: Option<std::path::PathBuf>,

    /// Output variant: auto (detect from background), dark, light, or both
    #[arg(long, value_enum, default_value_t = VariantArg::Auto)]
    pub variant: VariantArg,

    /// Fail on validation errors instead of auto-adjusting
    #[arg(long)]
    pub no_adjust: bool,

    /// Launch interactive TUI for previewing and editing the palette
    #[arg(short, long)]
    pub interactive: bool,

    // Curve configuration
    /// Lightness interpolation curve type
    #[arg(long, value_enum, default_value_t = CurveTypeArg::Smoothstep)]
    pub lightness_curve: CurveTypeArg,

    /// Lightness curve strength (for sigmoid, 1.0-10.0)
    #[arg(long, default_value_t = 2.0)]
    pub lightness_strength: f32,

    /// Chroma interpolation curve type
    #[arg(long, value_enum, default_value_t = CurveTypeArg::Linear)]
    pub chroma_curve: CurveTypeArg,

    /// Hue interpolation curve type
    #[arg(long, value_enum, default_value_t = CurveTypeArg::Linear)]
    pub hue_curve: CurveTypeArg,

    /// Load configuration from TOML file
    #[arg(long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Save current configuration to TOML file
    #[arg(long, value_name = "FILE")]
    pub save_config: Option<PathBuf>,
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

    /// Build InterpolationConfig from CLI arguments.
    pub fn interpolation_config(&self) -> InterpolationConfig {
        InterpolationConfig {
            lightness: CurveConfig {
                curve_type: self.lightness_curve.into(),
                strength: self.lightness_strength,
                control_points: None,
            },
            chroma: CurveConfig {
                curve_type: self.chroma_curve.into(),
                strength: 2.0,
                control_points: None,
            },
            hue: CurveConfig {
                curve_type: self.hue_curve.into(),
                strength: 2.0,
                control_points: None,
            },
        }
    }
}
