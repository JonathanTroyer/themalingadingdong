//! TOML configuration file support for theme generation.
//!
//! Uses Figment for hierarchical configuration with layered overrides:
//! `defaults < TOML file < CLI args`

use std::path::Path;
use std::str::FromStr;

use csscolorparser::Color as CssColor;
use figment::Figment;
use figment::providers::{Format, Serialized, Toml};
use palette::Srgb;
use serde::{Deserialize, Serialize};

use crate::curves::InterpolationConfig;
use crate::generate::GenerateConfig;

/// Error type for configuration operations.
#[derive(Debug)]
pub enum ConfigError {
    /// IO error reading/writing file
    Io(std::io::Error),
    /// Figment configuration error (boxed to reduce size)
    Figment(Box<figment::Error>),
    /// Invalid color format
    InvalidColor(String),
    /// Missing required field
    MissingField(&'static str),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {}", e),
            Self::Figment(e) => write!(f, "Configuration error: {}", e),
            Self::InvalidColor(s) => write!(f, "Invalid color: {}", s),
            Self::MissingField(field) => write!(f, "Missing required field: {}", field),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<figment::Error> for ConfigError {
    fn from(e: figment::Error) -> Self {
        Self::Figment(Box::new(e))
    }
}

/// Load configuration with Figment layering.
///
/// Priority: defaults < TOML file < CLI overrides
pub fn load_config(
    config_path: Option<&Path>,
    cli_overrides: &ThemeConfig,
) -> Result<ThemeConfig, ConfigError> {
    let mut figment = Figment::new().merge(Serialized::defaults(ThemeConfig::default()));

    if let Some(path) = config_path {
        figment = figment.merge(Toml::file(path));
    }

    figment = figment.merge(Serialized::defaults(cli_overrides));
    Ok(figment.extract()?)
}

/// Validate that required fields are present in the configuration.
pub fn validate_config(config: &ThemeConfig) -> Result<(), ConfigError> {
    if config.colors.background.is_none() {
        return Err(ConfigError::MissingField("colors.background"));
    }
    if config.colors.foreground.is_none() {
        return Err(ConfigError::MissingField("colors.foreground"));
    }
    Ok(())
}

/// Root configuration structure for TOML files.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    /// Theme metadata
    pub theme: ThemeMetadata,
    /// Color settings
    pub colors: ColorConfig,
    /// Interpolation curve settings
    pub curves: InterpolationConfig,
    /// Contrast settings
    pub contrast: ContrastConfig,
    /// Accent optimization settings for main accents (base08-0F)
    pub optimization: AccentOptSettings,
    /// Accent optimization settings for extended accents (base10-17)
    pub extended_optimization: AccentOptSettings,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            theme: ThemeMetadata::default(),
            colors: ColorConfig::default(),
            curves: InterpolationConfig::default(),
            contrast: ContrastConfig::default(),
            optimization: AccentOptSettings::default(),
            extended_optimization: AccentOptSettings {
                target_j: 70.0,
                target_m: 35.0,
                ..AccentOptSettings::default()
            },
        }
    }
}

/// Theme metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeMetadata {
    /// Name of the theme
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// Author of the theme
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Variant hint (dark, light, auto)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
}

/// Color configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ColorConfig {
    /// Background color (any CSS color format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    /// Foreground color (any CSS color format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreground: Option<String>,
    /// Hue overrides for accent colors
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hue_overrides: Option<HueOverrides>,
}

/// Hue overrides for individual accent colors.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HueOverrides {
    /// base08 (Red) hue in degrees
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base08: Option<f32>,
    /// base09 (Orange) hue in degrees
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base09: Option<f32>,
    /// base0A (Yellow) hue in degrees
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base0a: Option<f32>,
    /// base0B (Green) hue in degrees
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base0b: Option<f32>,
    /// base0C (Cyan) hue in degrees
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base0c: Option<f32>,
    /// base0D (Blue) hue in degrees
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base0d: Option<f32>,
    /// base0E (Purple) hue in degrees
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base0e: Option<f32>,
    /// base0F (Magenta) hue in degrees
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base0f: Option<f32>,
}

impl HueOverrides {
    /// Convert to array of optional hue values.
    pub fn to_array(&self) -> [Option<f32>; 8] {
        [
            self.base08,
            self.base09,
            self.base0a,
            self.base0b,
            self.base0c,
            self.base0d,
            self.base0e,
            self.base0f,
        ]
    }

    /// Create from array of optional hue values.
    pub fn from_array(arr: [Option<f32>; 8]) -> Self {
        Self {
            base08: arr[0],
            base09: arr[1],
            base0a: arr[2],
            base0b: arr[3],
            base0c: arr[4],
            base0d: arr[5],
            base0e: arr[6],
            base0f: arr[7],
        }
    }
}

/// Contrast settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ContrastConfig {
    /// Minimum APCA contrast for accent colors (Lc value).
    /// Colors will achieve at least this contrast while maintaining uniform lightness.
    #[serde(alias = "target")]
    pub minimum: f64,
    /// Minimum APCA contrast for extended accent colors (Lc value)
    #[serde(alias = "extended")]
    pub extended_minimum: f64,
    /// Maximum per-hue lightness adjustment allowed (0-10 J' units, default 2.0).
    /// Small adjustments help difficult hues reach minimum contrast while
    /// keeping colors near-uniform.
    pub max_adjustment: f32,
}

impl Default for ContrastConfig {
    fn default() -> Self {
        Self {
            minimum: 75.0,
            extended_minimum: 60.0,
            max_adjustment: 2.0,
        }
    }
}

/// Accent color optimization settings for COBYLA solver.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AccentOptSettings {
    /// Target lightness (J') for accent colors
    pub target_j: f32,
    /// Target colorfulness (M) for accent colors
    pub target_m: f32,
    /// Maximum deviation from target_j (box constraint)
    pub delta_j: f32,
    /// Maximum deviation from target_m (box constraint)
    pub delta_m: f32,
    /// Weight for J vs M uniformity (0.0=M priority, 1.0=J priority)
    pub j_weight: f32,
    /// Weight for contrast vs uniformity (0.0=uniformity, 1.0=contrast)
    pub contrast_weight: f32,
}

impl Default for AccentOptSettings {
    fn default() -> Self {
        Self {
            target_j: 80.0,
            target_m: 25.0,
            delta_j: 3.0,
            delta_m: 6.0,
            j_weight: 0.75,
            contrast_weight: 0.8,
        }
    }
}

impl ThemeConfig {
    /// Save configuration to a TOML file.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::Figment(Box::new(figment::Error::from(e.to_string()))))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Convert to GenerateConfig.
    ///
    /// Uses defaults for any unspecified values.
    pub fn to_generate_config(&self) -> Result<GenerateConfig, ConfigError> {
        let defaults = GenerateConfig::default();

        let background = if let Some(ref bg) = self.colors.background {
            parse_color(bg).map_err(ConfigError::InvalidColor)?
        } else {
            defaults.background
        };

        let foreground = if let Some(ref fg) = self.colors.foreground {
            parse_color(fg).map_err(ConfigError::InvalidColor)?
        } else {
            defaults.foreground
        };

        let hue_overrides = self
            .colors
            .hue_overrides
            .as_ref()
            .map(|h| h.to_array())
            .unwrap_or([None; 8]);

        Ok(GenerateConfig {
            background,
            foreground,
            hue_overrides,
            min_contrast: self.contrast.minimum,
            extended_min_contrast: self.contrast.extended_minimum,
            max_lightness_adjustment: self.contrast.max_adjustment,
            name: if self.theme.name.is_empty() {
                defaults.name
            } else {
                self.theme.name.clone()
            },
            author: self.theme.author.clone(),
            interpolation: self.curves.clone(),
            accent_opt: self.optimization.clone(),
            extended_accent_opt: self.extended_optimization.clone(),
        })
    }

    /// Create from a GenerateConfig.
    pub fn from_generate_config(config: &GenerateConfig) -> Self {
        Self {
            theme: ThemeMetadata {
                name: config.name.clone(),
                author: config.author.clone(),
                variant: None,
            },
            colors: ColorConfig {
                background: Some(format!(
                    "#{:02x}{:02x}{:02x}",
                    config.background.red, config.background.green, config.background.blue
                )),
                foreground: Some(format!(
                    "#{:02x}{:02x}{:02x}",
                    config.foreground.red, config.foreground.green, config.foreground.blue
                )),
                hue_overrides: Some(HueOverrides::from_array(config.hue_overrides)),
            },
            curves: config.interpolation.clone(),
            contrast: ContrastConfig {
                minimum: config.min_contrast,
                extended_minimum: config.extended_min_contrast,
                max_adjustment: config.max_lightness_adjustment,
            },
            optimization: config.accent_opt.clone(),
            extended_optimization: config.extended_accent_opt.clone(),
        }
    }
}

/// Parse any CSS color string into Srgb<u8>.
///
/// Supports: hex (#RRGGBB), rgb(), oklch(), named colors, etc.
pub fn parse_color(input: &str) -> Result<Srgb<u8>, String> {
    let css_color: CssColor = input
        .parse()
        .map_err(|e| format!("Invalid color '{}': {}", input, e))?;
    let [r, g, b, _a] = css_color.to_rgba8();
    Ok(Srgb::new(r, g, b))
}

/// Parse a hex color string into an Srgb<u8>.
#[deprecated(note = "Use parse_color() instead which supports more formats")]
pub fn parse_hex(hex: &str) -> Result<Srgb<u8>, String> {
    // palette's FromStr expects 6 hex chars without #
    let hex = hex.trim_start_matches('#');
    Srgb::from_str(hex).map_err(|e| format!("Invalid hex color: {e}"))
}
