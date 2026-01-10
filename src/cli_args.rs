//! CLI argument definitions (build.rs compatible).
//!
//! This module contains only struct/enum definitions with no dependencies on
//! other crate modules, allowing it to be included from build.rs for man page
//! generation.

use std::path::PathBuf;

use clap::builder::ArgPredicate;
use clap::{Parser, ValueEnum};
use serde::Serialize;

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

/// Output format selection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// YAML format (default, tinted-theming compatible)
    #[default]
    Yaml,
    /// JSON format (tinted-theming compatible)
    Json,
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

/// Base24 palette generator using HellwigJmh color space with APCA validation.
#[derive(Parser, Debug, Serialize)]
#[command(name = "themalingadingdong")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Background color (base00) in any CSS format (hex, rgb(), oklch(), hsl(), named)
    #[arg(
        short,
        long,
        default_value_if("interactive", "true", "#000000"),
        default_value_if("input", ArgPredicate::IsPresent, "#000000"),
        required_unless_present_any = ["interactive", "config", "completions", "input"],
        value_parser = |s: &str| s.parse::<csscolorparser::Color>().map(|_| s.to_string()).map_err(|e| e.to_string())
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,

    /// Foreground color (base07) in any CSS format (hex, rgb(), oklch(), hsl(), named)
    #[arg(
        short,
        long,
        default_value_if("interactive", "true", "#FFFFFF"),
        default_value_if("input", ArgPredicate::IsPresent, "#FFFFFF"),
        required_unless_present_any = ["interactive", "config", "completions", "input"],
        value_parser = |s: &str| s.parse::<csscolorparser::Color>().map(|_| s.to_string()).map_err(|e| e.to_string())
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
        default_value_if("input", ArgPredicate::IsPresent, "Imported Theme"),
        required_unless_present_any = ["interactive", "config", "completions", "input"]
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

    /// Output format: yaml or json (tinted-theming compatible)
    #[arg(long, value_enum, default_value_t = OutputFormat::Yaml)]
    #[serde(skip)]
    pub format: OutputFormat,

    /// Import Base16/Base24 scheme file for editing (implies --interactive)
    #[arg(long, value_name = "FILE")]
    #[serde(skip)]
    pub input: Option<PathBuf>,

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

    /// Generate shell completions for the specified shell
    #[arg(long, value_enum, value_name = "SHELL")]
    #[serde(skip)]
    pub completions: Option<clap_complete::Shell>,

    /// Preview validation results without generating output
    #[arg(long)]
    #[serde(skip)]
    pub dry_run: bool,
}
