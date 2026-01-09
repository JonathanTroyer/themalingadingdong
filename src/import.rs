//! Import Base16/Base24 scheme files.
//!
//! Supports both the modern tinted-theming format and legacy Base16 format.

use std::collections::HashMap;
use std::path::Path;

use color_eyre::eyre::{Result, WrapErr, bail};
use palette::Srgb;
use serde::Deserialize;
use tinted_builder::{Base16Scheme, SchemeSystem, SchemeVariant};

use crate::config::{
    AccentOptSettings, ColorConfig, ContrastConfig, HueOverrides, ThemeConfig, ThemeMetadata,
};
use crate::curves::InterpolationConfig;
use crate::hellwig::HellwigJmh;

/// Legacy Base16 scheme format (colors at top level).
#[derive(Debug, Deserialize)]
struct LegacyScheme {
    scheme: String,
    author: Option<String>,
    base00: String,
    base01: String,
    base02: String,
    base03: String,
    base04: String,
    base05: String,
    base06: String,
    base07: String,
    base08: String,
    base09: String,
    #[serde(alias = "base0a")]
    #[serde(rename = "base0A")]
    base0_a: String,
    #[serde(alias = "base0b")]
    #[serde(rename = "base0B")]
    base0_b: String,
    #[serde(alias = "base0c")]
    #[serde(rename = "base0C")]
    base0_c: String,
    #[serde(alias = "base0d")]
    #[serde(rename = "base0D")]
    base0_d: String,
    #[serde(alias = "base0e")]
    #[serde(rename = "base0E")]
    base0_e: String,
    #[serde(alias = "base0f")]
    #[serde(rename = "base0F")]
    base0_f: String,
}

impl LegacyScheme {
    fn into_base16_scheme(self) -> Result<Base16Scheme> {
        use tinted_builder::Color;

        let mut palette = HashMap::new();
        palette.insert("base00".to_string(), Color::new(self.base00)?);
        palette.insert("base01".to_string(), Color::new(self.base01)?);
        palette.insert("base02".to_string(), Color::new(self.base02)?);
        palette.insert("base03".to_string(), Color::new(self.base03)?);
        palette.insert("base04".to_string(), Color::new(self.base04)?);
        palette.insert("base05".to_string(), Color::new(self.base05)?);
        palette.insert("base06".to_string(), Color::new(self.base06)?);
        palette.insert("base07".to_string(), Color::new(self.base07)?);
        palette.insert("base08".to_string(), Color::new(self.base08)?);
        palette.insert("base09".to_string(), Color::new(self.base09)?);
        palette.insert("base0A".to_string(), Color::new(self.base0_a)?);
        palette.insert("base0B".to_string(), Color::new(self.base0_b)?);
        palette.insert("base0C".to_string(), Color::new(self.base0_c)?);
        palette.insert("base0D".to_string(), Color::new(self.base0_d)?);
        palette.insert("base0E".to_string(), Color::new(self.base0_e)?);
        palette.insert("base0F".to_string(), Color::new(self.base0_f)?);

        let slug: String = self
            .scheme
            .to_lowercase()
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect();

        Ok(Base16Scheme {
            system: SchemeSystem::Base16,
            name: self.scheme,
            slug,
            author: self.author.unwrap_or_default(),
            description: None,
            variant: SchemeVariant::Dark,
            palette,
        })
    }
}

/// Result of importing a scheme file.
pub struct ImportResult {
    /// ThemeConfig for TUI editing
    pub config: ThemeConfig,
    /// Original parsed scheme for validation
    pub scheme: Base16Scheme,
}

/// Import a scheme file and convert to ThemeConfig.
///
/// Supports both modern tinted-theming format and legacy Base16 format.
/// Extracts:
/// - base00 as background
/// - base07 as foreground
/// - Hues from base08-base0F accent colors
///
/// Returns both the ThemeConfig (for editing) and the original scheme (for validation).
pub fn import_scheme(path: &Path) -> Result<ImportResult> {
    let content = std::fs::read_to_string(path)
        .wrap_err_with(|| format!("Failed to read {}", path.display()))?;

    let scheme = parse_scheme(&content, path)?;
    let config = scheme_to_config(&scheme)?;

    Ok(ImportResult { config, scheme })
}

/// Parse scheme content, trying modern format first, then legacy.
fn parse_scheme(content: &str, path: &Path) -> Result<Base16Scheme> {
    let is_json = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"));

    // Try modern tinted-theming format first
    let modern_result: Result<Base16Scheme, String> = if is_json {
        serde_json::from_str(content).map_err(|e| e.to_string())
    } else {
        serde_yaml::from_str(content).map_err(|e| e.to_string())
    };

    if let Ok(scheme) = modern_result {
        return Ok(scheme);
    }

    // Fall back to legacy Base16 format
    let legacy_result: Result<LegacyScheme, String> = if is_json {
        serde_json::from_str(content).map_err(|e| e.to_string())
    } else {
        serde_yaml::from_str(content).map_err(|e| e.to_string())
    };

    match legacy_result {
        Ok(legacy) => legacy
            .into_base16_scheme()
            .wrap_err("Failed to convert legacy scheme"),
        Err(e) => bail!(
            "Failed to parse scheme (tried modern and legacy formats): {}",
            e
        ),
    }
}

/// Convert Base16Scheme to ThemeConfig with extracted hues.
fn scheme_to_config(scheme: &Base16Scheme) -> Result<ThemeConfig> {
    let background = get_color(scheme, "base00")?;
    let foreground = get_color(scheme, "base07")?;

    // Extract hues from accent colors
    let accent_names = [
        "base08", "base09", "base0A", "base0B", "base0C", "base0D", "base0E", "base0F",
    ];
    let mut hues = [None; 8];

    for (i, name) in accent_names.iter().enumerate() {
        if let Ok(color) = get_color(scheme, name) {
            let hellwig = HellwigJmh::from_srgb_u8(color);
            // Only extract hue if color has meaningful colorfulness
            if hellwig.colorfulness > 5.0 {
                hues[i] = Some(hellwig.hue);
            }
        }
    }

    let variant_str = format!("{:?}", scheme.variant).to_lowercase();

    Ok(ThemeConfig {
        theme: ThemeMetadata {
            name: scheme.name.clone(),
            author: if scheme.author.is_empty() {
                None
            } else {
                Some(scheme.author.clone())
            },
            variant: Some(variant_str),
        },
        colors: ColorConfig {
            background: Some(format!(
                "#{:02x}{:02x}{:02x}",
                background.red, background.green, background.blue
            )),
            foreground: Some(format!(
                "#{:02x}{:02x}{:02x}",
                foreground.red, foreground.green, foreground.blue
            )),
            hue_overrides: Some(HueOverrides::from_array(hues)),
        },
        curves: InterpolationConfig::default(),
        contrast: ContrastConfig::default(),
        optimization: AccentOptSettings::default(),
        extended_optimization: AccentOptSettings {
            target_j: 70.0,
            target_m: 35.0,
            ..AccentOptSettings::default()
        },
    })
}

/// Extract an sRGB color from the scheme palette.
fn get_color(scheme: &Base16Scheme, name: &str) -> Result<Srgb<u8>> {
    // Try both uppercase and lowercase variants for base0A-base0F
    let color = scheme
        .palette
        .get(name)
        .or_else(|| scheme.palette.get(&name.to_lowercase()))
        .or_else(|| scheme.palette.get(&name.to_uppercase()))
        .ok_or_else(|| color_eyre::eyre::eyre!("Missing palette color: {}", name))?;

    let hex = color.to_hex();
    if hex.len() < 6 {
        bail!("Invalid hex color for {}: {}", name, hex);
    }

    let r = u8::from_str_radix(&hex[0..2], 16)
        .wrap_err_with(|| format!("Invalid red component in {}", name))?;
    let g = u8::from_str_radix(&hex[2..4], 16)
        .wrap_err_with(|| format!("Invalid green component in {}", name))?;
    let b = u8::from_str_radix(&hex[4..6], 16)
        .wrap_err_with(|| format!("Invalid blue component in {}", name))?;

    Ok(Srgb::new(r, g, b))
}
