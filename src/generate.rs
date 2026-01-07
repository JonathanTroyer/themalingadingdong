//! Palette generation logic.

use std::collections::HashMap;

use csscolorparser::Color as CssColor;
use palette::Srgb;
use tinted_builder::{Base16Scheme, Color, SchemeSystem, SchemeVariant};
use tracing::info;

#[cfg(debug_assertions)]
use tracing::instrument;

use crate::config::AccentOptSettings;
use crate::curves::InterpolationConfig;
use crate::hellwig::hellwig_lightness;
use crate::interpolation::{
    AccentResult, build_hues_with_overrides, generate_accents_uniform, interpolate_with_curves,
    srgb_to_f32, srgb_to_hex, srgb_to_u8,
};

/// Result of palette generation including any warnings.
#[derive(Debug)]
pub struct GenerationResult {
    /// The generated color scheme
    pub scheme: Base16Scheme,
    /// Warnings for hues that couldn't achieve target contrast
    pub warnings: Vec<String>,
    /// Accent results for base08-base0F
    pub base_accent_results: Vec<AccentResult>,
    /// Accent results for base10-base17
    pub extended_accent_results: Vec<AccentResult>,
}

/// Configuration for palette generation.
#[derive(Debug, Clone)]
pub struct GenerateConfig {
    /// Background color (base00)
    pub background: Srgb<u8>,
    /// Foreground color (base07)
    pub foreground: Srgb<u8>,
    /// Hue overrides for accent colors (base08-base0F).
    /// `None` values use defaults from `DEFAULT_BASE16_HUES`.
    pub hue_overrides: [Option<f32>; 8],
    /// Minimum APCA contrast for accent colors (Lc value, 30-90 typical).
    /// Colors achieve at least this contrast while maintaining uniform lightness.
    pub min_contrast: f64,
    /// Minimum APCA contrast for extended accent colors base10-base17 (Lc value)
    pub extended_min_contrast: f64,
    /// Maximum per-hue lightness adjustment allowed (0-10 J' units, default 2.0).
    /// Small adjustments help difficult hues reach minimum contrast.
    pub max_lightness_adjustment: f32,
    /// Scheme name
    pub name: String,
    /// Author name (optional)
    pub author: Option<String>,
    /// Interpolation curve configuration for L/C/H
    pub interpolation: InterpolationConfig,
    /// Accent optimization settings for main accents (base08-0F)
    pub accent_opt: AccentOptSettings,
    /// Accent optimization settings for extended accents (base10-17)
    pub extended_accent_opt: AccentOptSettings,
}

impl Default for GenerateConfig {
    fn default() -> Self {
        Self {
            background: Srgb::new(26u8, 26, 46),    // #1a1a2e
            foreground: Srgb::new(234u8, 234, 234), // #eaeaea
            hue_overrides: [None; 8],               // Use DEFAULT_BASE16_HUES
            min_contrast: 75.0,
            extended_min_contrast: 60.0,
            max_lightness_adjustment: 2.0,
            name: "Generated Scheme".to_string(),
            author: None,
            interpolation: InterpolationConfig::default(),
            accent_opt: AccentOptSettings::default(),
            extended_accent_opt: AccentOptSettings {
                target_j: 70.0,
                target_m: 35.0,
                ..AccentOptSettings::default()
            },
        }
    }
}

/// Generate a Base24 color scheme from the given configuration.
///
/// Uses uniform lightness optimization to produce visually cohesive accent colors.
/// Returns a `GenerationResult` containing the scheme and any warnings.
#[cfg_attr(debug_assertions, instrument(skip(config), fields(name = %config.name)))]
pub fn generate(config: &GenerateConfig) -> GenerationResult {
    generate_for_variant(config, None)
}

/// Generate a Base24 color scheme for a specific variant.
///
/// If `forced_variant` is Some, swaps bg/fg colors for light variant.
/// If None, auto-detects based on background luminance.
#[cfg_attr(debug_assertions, instrument(skip(config), fields(name = %config.name, min_contrast = config.min_contrast)))]
pub fn generate_for_variant(
    config: &GenerateConfig,
    forced_variant: Option<SchemeVariant>,
) -> GenerationResult {
    let bg_l = hellwig_lightness(config.background);
    let fg_l = hellwig_lightness(config.foreground);

    let (darker, lighter) = if bg_l < fg_l {
        (config.background, config.foreground)
    } else {
        (config.foreground, config.background)
    };

    // Assign colors based on variant
    let (background, foreground, variant) = match forced_variant {
        Some(SchemeVariant::Dark) => (darker, lighter, SchemeVariant::Dark),
        Some(SchemeVariant::Light) => (lighter, darker, SchemeVariant::Light),
        Some(v) => unimplemented!("unsupported variant: {v:?}"),
        None => {
            // Auto: dark if input bg is darker than fg
            if bg_l < fg_l {
                (darker, lighter, SchemeVariant::Dark)
            } else {
                (lighter, darker, SchemeVariant::Light)
            }
        }
    };

    let bg_f32 = srgb_to_f32(background);
    let fg_f32 = srgb_to_f32(foreground);
    let ui_colors = interpolate_with_curves(bg_f32, fg_f32, 8, &config.interpolation);

    let mut warnings = Vec::new();
    let accent_hues = build_hues_with_overrides(&config.hue_overrides);

    // Generate base accents (base08-base0F) with COBYLA optimization
    // User's J/M bounds are HARD constraints; contrast is SOFT
    let base_accent_results = generate_accents_uniform(
        &accent_hues,
        &config.accent_opt,
        config.min_contrast,
        background,
    );

    warnings.extend(base_accent_results.iter().filter_map(|r| r.warning.clone()));

    // Generate extended accents (base10-base17) with COBYLA optimization
    // User's J/M bounds are HARD constraints; contrast is SOFT
    let extended_accent_results = generate_accents_uniform(
        &accent_hues,
        &config.extended_accent_opt,
        config.extended_min_contrast,
        background,
    );

    warnings.extend(
        extended_accent_results
            .iter()
            .filter_map(|r| r.warning.clone()),
    );

    // Build the palette HashMap
    let mut palette = HashMap::new();

    for (i, color) in ui_colors.iter().enumerate() {
        let name = format!("base0{:X}", i);
        let hex = srgb_to_hex(srgb_to_u8(*color));
        palette.insert(name, Color::new(hex).expect("valid hex"));
    }

    for (i, result) in base_accent_results.iter().enumerate() {
        let name = format!("base0{:X}", 8 + i);
        let hex = srgb_to_hex(srgb_to_u8(result.color));
        palette.insert(name, Color::new(hex).expect("valid hex"));
    }

    for (i, result) in extended_accent_results.iter().enumerate() {
        let name = format!("base1{:X}", i);
        let hex = srgb_to_hex(srgb_to_u8(result.color));
        palette.insert(name, Color::new(hex).expect("valid hex"));
    }

    // Create slug from name with variant suffix
    let variant_suffix = match variant {
        SchemeVariant::Dark => "-dark",
        SchemeVariant::Light => "-light",
        v => unimplemented!("unsupported variant: {v:?}"),
    };
    let base_slug: String = config
        .name
        .to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect();
    let slug = format!("{}{}", base_slug, variant_suffix);

    let scheme = Base16Scheme {
        system: SchemeSystem::Base24,
        name: config.name.clone(),
        slug,
        author: config.author.clone().unwrap_or_default(),
        description: None,
        variant,
        palette,
    };

    if !warnings.is_empty() {
        info!(
            warning_count = warnings.len(),
            "scheme generated with warnings"
        );
    }

    GenerationResult {
        scheme,
        warnings,
        base_accent_results,
        extended_accent_results,
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
