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

    /// Target APCA contrast for accent colors (Lc 75 = body text, 90 = small text)
    #[arg(long, default_value_t = 75.0)]
    pub target_contrast: f64,

    /// Chroma for accent colors (0.0-0.4 typical, higher = more saturated)
    #[arg(long, default_value_t = 0.15)]
    pub accent_chroma: f32,

    /// Target APCA contrast for extended accent colors base10-base17 (Lc 60 = large text)
    #[arg(long, default_value_t = 60.0)]
    pub extended_contrast: f64,

    /// Chroma for extended accent colors base10-base17 (0.0-0.4 typical)
    #[arg(long, default_value_t = 0.20)]
    pub extended_chroma: f32,

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
}
