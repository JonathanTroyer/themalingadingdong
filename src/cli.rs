//! CLI argument parsing and command handling.

use clap::{Parser, ValueEnum};

/// Output variant selection.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
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

/// Base24 palette generator using OKLCH color space with APCA validation.
#[derive(Parser, Debug)]
#[command(name = "themalingadingdong")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Background color (base00) in hex format (#RRGGBB or RRGGBB)
    #[arg(short, long)]
    pub background: String,

    /// Foreground color (base07) in hex format (#RRGGBB or RRGGBB)
    #[arg(short, long)]
    pub foreground: String,

    /// Starting hue for accent colors (degrees, 0-360)
    #[arg(long, default_value_t = 25.0)]
    pub start_hue: f32,

    /// Target APCA contrast for accent colors (Lc 75 = body text, 90 = small text)
    #[arg(long, default_value_t = 75.0)]
    pub target_contrast: f64,

    /// Chroma for accent colors (0.0-0.4 typical, higher = more saturated)
    #[arg(long, default_value_t = 0.25)]
    pub accent_chroma: f32,

    /// Target APCA contrast for extended accent colors base10-base17 (Lc 60 = large text)
    #[arg(long, default_value_t = 60.0)]
    pub extended_contrast: f64,

    /// Chroma for extended accent colors base10-base17 (0.0-0.4 typical)
    #[arg(long, default_value_t = 0.20)]
    pub extended_chroma: f32,

    /// Scheme name
    #[arg(long)]
    pub name: String,

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
}
