//! TOML configuration file support for theme generation.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::curves::InterpolationConfig;
use crate::generate::{GenerateConfig, parse_color};

/// Error type for configuration operations.
#[derive(Debug)]
pub enum ConfigError {
    /// IO error reading/writing file
    Io(std::io::Error),
    /// TOML parsing error
    Parse(toml::de::Error),
    /// TOML serialization error
    Serialize(toml::ser::Error),
    /// Invalid color format
    InvalidColor(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {}", e),
            Self::Parse(e) => write!(f, "TOML parse error: {}", e),
            Self::Serialize(e) => write!(f, "TOML serialize error: {}", e),
            Self::InvalidColor(s) => write!(f, "Invalid color: {}", s),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(e: toml::de::Error) -> Self {
        Self::Parse(e)
    }
}

impl From<toml::ser::Error> for ConfigError {
    fn from(e: toml::ser::Error) -> Self {
        Self::Serialize(e)
    }
}

/// Root configuration structure for TOML files.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
}

/// Theme metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeMetadata {
    /// Name of the theme
    pub name: String,
    /// Author of the theme
    pub author: Option<String>,
    /// Variant hint (dark, light, auto)
    pub variant: Option<String>,
}

/// Color configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ColorConfig {
    /// Background color (any CSS color format)
    pub background: Option<String>,
    /// Foreground color (any CSS color format)
    pub foreground: Option<String>,
    /// Chroma for accent colors (0.0-0.4)
    pub accent_chroma: Option<f32>,
    /// Chroma for extended accent colors (0.0-0.4)
    pub extended_chroma: Option<f32>,
    /// Hue overrides for accent colors
    pub hue_overrides: Option<HueOverrides>,
}

/// Hue overrides for individual accent colors.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HueOverrides {
    /// base08 (Red) hue in degrees
    pub base08: Option<f32>,
    /// base09 (Orange) hue in degrees
    pub base09: Option<f32>,
    /// base0A (Yellow) hue in degrees
    pub base0a: Option<f32>,
    /// base0B (Green) hue in degrees
    pub base0b: Option<f32>,
    /// base0C (Cyan) hue in degrees
    pub base0c: Option<f32>,
    /// base0D (Blue) hue in degrees
    pub base0d: Option<f32>,
    /// base0E (Purple) hue in degrees
    pub base0e: Option<f32>,
    /// base0F (Magenta) hue in degrees
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
    /// Target APCA contrast for accent colors (Lc value)
    pub target: f64,
    /// Target APCA contrast for extended accent colors (Lc value)
    pub extended: f64,
}

impl Default for ContrastConfig {
    fn default() -> Self {
        Self {
            target: 75.0,
            extended: 60.0,
        }
    }
}

impl ThemeConfig {
    /// Load configuration from a TOML file.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to a TOML file.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let content = toml::to_string_pretty(self)?;
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
            target_contrast: self.contrast.target,
            extended_contrast: self.contrast.extended,
            accent_chroma: self.colors.accent_chroma.unwrap_or(defaults.accent_chroma),
            extended_chroma: self
                .colors
                .extended_chroma
                .unwrap_or(defaults.extended_chroma),
            name: if self.theme.name.is_empty() {
                defaults.name
            } else {
                self.theme.name.clone()
            },
            author: self.theme.author.clone(),
            interpolation: self.curves.clone(),
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
                accent_chroma: Some(config.accent_chroma),
                extended_chroma: Some(config.extended_chroma),
                hue_overrides: Some(HueOverrides::from_array(config.hue_overrides)),
            },
            curves: config.interpolation.clone(),
            contrast: ContrastConfig {
                target: config.target_contrast,
                extended: config.extended_contrast,
            },
        }
    }
}
