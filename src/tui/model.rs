//! Application model for the TUI.

use std::path::PathBuf;

use color_eyre::eyre::{Result, WrapErr};
use palette::Srgb;
use tinted_builder::{Base16Scheme, SchemeVariant};
use tuirealm::Update;

use crate::cli::{Cli, VariantArg};
use crate::curves::InterpolationConfig;
use crate::generate::{GenerateConfig, generate_for_variant, parse_color};
use crate::interpolation::srgb_to_oklch;
use crate::validation::{ValidationResult, validate};

use super::msg::Msg;

/// OKLCH color components for editing.
#[derive(Debug, Clone, Copy)]
pub struct OklchComponents {
    pub lightness: f32,
    pub chroma: f32,
    pub hue: f32,
    pub out_of_gamut: bool,
}

impl OklchComponents {
    pub fn from_srgb(color: Srgb<u8>) -> Self {
        let (l, c, h) = srgb_to_oklch(color);
        Self {
            lightness: l,
            chroma: c,
            hue: h,
            out_of_gamut: false,
        }
    }

    pub fn to_srgb(self) -> Srgb<u8> {
        use palette::{IntoColor, Oklch};
        let oklch = Oklch::new(self.lightness, self.chroma, self.hue);
        let linear: palette::LinSrgb<f32> = oklch.into_color();
        let srgb: palette::Srgb<f32> = palette::Srgb::from_linear(linear);
        Srgb::new(
            (srgb.red.clamp(0.0, 1.0) * 255.0).round() as u8,
            (srgb.green.clamp(0.0, 1.0) * 255.0).round() as u8,
            (srgb.blue.clamp(0.0, 1.0) * 255.0).round() as u8,
        )
    }

    pub fn check_gamut(&mut self) {
        use palette::{IntoColor, Oklch};
        let oklch = Oklch::new(self.lightness, self.chroma, self.hue);
        let linear: palette::LinSrgb<f32> = oklch.into_color();
        let srgb: palette::Srgb<f32> = palette::Srgb::from_linear(linear);
        self.out_of_gamut = srgb.red < 0.0
            || srgb.red > 1.0
            || srgb.green < 0.0
            || srgb.green > 1.0
            || srgb.blue < 0.0
            || srgb.blue > 1.0;
    }
}

impl Default for OklchComponents {
    fn default() -> Self {
        Self {
            lightness: 0.5,
            chroma: 0.0,
            hue: 0.0,
            out_of_gamut: false,
        }
    }
}

/// Application model containing all state.
pub struct Model {
    // Editable parameters
    pub background_oklch: OklchComponents,
    pub foreground_oklch: OklchComponents,
    pub min_contrast: f64,
    pub extended_min_contrast: f64,
    pub max_lightness_adjustment: f32,
    pub accent_chroma: f32,
    pub extended_chroma: f32,
    pub hue_overrides: [Option<f32>; 8],
    pub variant: VariantArg,
    pub name: String,
    pub author: String,
    pub interpolation: InterpolationConfig,

    // Derived sRGB colors
    pub background: Srgb<u8>,
    pub foreground: Srgb<u8>,

    // Generated output
    pub current_scheme: Option<Base16Scheme>,
    pub generation_warnings: Vec<String>,
    pub validation_results: Vec<ValidationResult>,

    // UI state
    pub quit: bool,
    pub message: Option<String>,
    pub export_path: String,
}

impl Model {
    /// Create model from CLI arguments.
    pub fn from_cli(cli: &Cli) -> Result<Self> {
        let bg_str = cli
            .background
            .clone()
            .unwrap_or_else(|| "#000000".to_string());
        let fg_str = cli
            .foreground
            .clone()
            .unwrap_or_else(|| "#FFFFFF".to_string());
        let name = cli.name.clone().unwrap_or_else(|| "My Theme".to_string());

        let background = parse_color(&bg_str)
            .map_err(|e| color_eyre::eyre::eyre!("Invalid background color: {}", e))?;
        let foreground = parse_color(&fg_str)
            .map_err(|e| color_eyre::eyre::eyre!("Invalid foreground color: {}", e))?;

        let background_oklch = OklchComponents::from_srgb(background);
        let foreground_oklch = OklchComponents::from_srgb(foreground);

        Ok(Self {
            background_oklch,
            foreground_oklch,
            min_contrast: cli.min_contrast,
            extended_min_contrast: cli.extended_min_contrast,
            max_lightness_adjustment: cli.max_lightness_adjustment,
            accent_chroma: cli.accent_chroma,
            extended_chroma: cli.extended_chroma,
            hue_overrides: cli.hue_overrides(),
            variant: cli.variant,
            name,
            author: cli.author.clone().unwrap_or_default(),
            interpolation: InterpolationConfig::default(),

            background,
            foreground,

            current_scheme: None,
            generation_warnings: Vec::new(),
            validation_results: Vec::new(),

            quit: false,
            message: None,
            export_path: String::from("scheme.yaml"),
        })
    }

    /// Convert current state to GenerateConfig.
    fn to_generate_config(&self) -> GenerateConfig {
        GenerateConfig {
            background: self.background,
            foreground: self.foreground,
            hue_overrides: self.hue_overrides,
            min_contrast: self.min_contrast,
            extended_min_contrast: self.extended_min_contrast,
            max_lightness_adjustment: self.max_lightness_adjustment,
            accent_chroma: self.accent_chroma,
            extended_chroma: self.extended_chroma,
            name: self.name.clone(),
            author: if self.author.is_empty() {
                None
            } else {
                Some(self.author.clone())
            },
            interpolation: self.interpolation.clone(),
        }
    }

    /// Regenerate the palette from current state.
    pub fn regenerate(&mut self) {
        // Recompute sRGB from OKLCH
        self.background = self.background_oklch.to_srgb();
        self.foreground = self.foreground_oklch.to_srgb();

        // Check gamut
        self.background_oklch.check_gamut();
        self.foreground_oklch.check_gamut();

        let config = self.to_generate_config();
        let forced = match self.variant {
            VariantArg::Auto => None,
            VariantArg::Dark => Some(SchemeVariant::Dark),
            VariantArg::Light => Some(SchemeVariant::Light),
            VariantArg::Both => None,
        };

        let result = generate_for_variant(&config, forced);
        self.validation_results = validate(&result.scheme);
        self.generation_warnings = result.warnings;
        self.current_scheme = Some(result.scheme);
        self.message = None;
    }

    /// Export the current scheme to a file.
    pub fn export(&mut self) -> Result<()> {
        if let Some(ref scheme) = self.current_scheme {
            let yaml = serde_yaml::to_string(scheme).wrap_err("Failed to serialize scheme")?;

            let path = PathBuf::from(&self.export_path);
            std::fs::write(&path, &yaml)
                .wrap_err_with(|| format!("Failed to write to {}", path.display()))?;

            self.message = Some(format!("Exported to {}", path.display()));
        } else {
            self.message = Some("No scheme to export".to_string());
        }
        Ok(())
    }
}

impl Update<Msg> for Model {
    fn update(&mut self, msg: Option<Msg>) -> Option<Msg> {
        let msg = msg?;

        match msg {
            Msg::Quit => {
                self.quit = true;
                None
            }

            // Background changes
            Msg::BackgroundLChanged(v) => {
                self.background_oklch.lightness = v;
                Some(Msg::Regenerate)
            }
            Msg::BackgroundCChanged(v) => {
                self.background_oklch.chroma = v;
                Some(Msg::Regenerate)
            }
            Msg::BackgroundHChanged(v) => {
                self.background_oklch.hue = v;
                Some(Msg::Regenerate)
            }

            // Foreground changes
            Msg::ForegroundLChanged(v) => {
                self.foreground_oklch.lightness = v;
                Some(Msg::Regenerate)
            }
            Msg::ForegroundCChanged(v) => {
                self.foreground_oklch.chroma = v;
                Some(Msg::Regenerate)
            }
            Msg::ForegroundHChanged(v) => {
                self.foreground_oklch.hue = v;
                Some(Msg::Regenerate)
            }

            // Numeric parameters
            Msg::MinContrastChanged(v) => {
                self.min_contrast = v;
                Some(Msg::Regenerate)
            }
            Msg::ExtendedMinContrastChanged(v) => {
                self.extended_min_contrast = v;
                Some(Msg::Regenerate)
            }
            Msg::MaxLightnessAdjustmentChanged(v) => {
                self.max_lightness_adjustment = v;
                Some(Msg::Regenerate)
            }
            Msg::AccentChromaChanged(v) => {
                self.accent_chroma = v;
                Some(Msg::Regenerate)
            }
            Msg::ExtendedChromaChanged(v) => {
                self.extended_chroma = v;
                Some(Msg::Regenerate)
            }

            // Selectors
            Msg::VariantChanged(v) => {
                self.variant = v;
                Some(Msg::Regenerate)
            }
            Msg::LightnessCurveTypeChanged(v) => {
                self.interpolation.lightness.curve_type = v;
                Some(Msg::Regenerate)
            }
            Msg::LightnessCurveStrengthChanged(v) => {
                self.interpolation.lightness.strength = v;
                Some(Msg::Regenerate)
            }
            Msg::ChromaCurveTypeChanged(v) => {
                self.interpolation.chroma.curve_type = v;
                Some(Msg::Regenerate)
            }
            Msg::HueCurveTypeChanged(v) => {
                self.interpolation.hue.curve_type = v;
                Some(Msg::Regenerate)
            }

            // Hue overrides
            Msg::HueOverrideChanged(idx, val) => {
                if (idx as usize) < 8 {
                    self.hue_overrides[idx as usize] = val;
                }
                Some(Msg::Regenerate)
            }

            // Metadata
            Msg::NameChanged(v) => {
                self.name = v;
                None
            }
            Msg::AuthorChanged(v) => {
                self.author = v;
                None
            }

            // Regenerate
            Msg::Regenerate => {
                self.regenerate();
                None
            }

            // Export
            Msg::ExportPathChanged(v) => {
                self.export_path = v;
                None
            }
            Msg::DoExport => {
                if let Err(e) = self.export() {
                    self.message = Some(format!("Export failed: {}", e));
                }
                None
            }
            Msg::ExportSuccess(path) => {
                self.message = Some(format!("Exported to {}", path));
                None
            }
            Msg::ExportError(err) => {
                self.message = Some(format!("Export failed: {}", err));
                None
            }

            // These messages don't need model updates
            Msg::ShowHelp
            | Msg::HideHelp
            | Msg::ShowExport
            | Msg::HideExport
            | Msg::FocusNext
            | Msg::FocusPrev
            | Msg::FocusUp
            | Msg::FocusDown
            | Msg::ValidationScrollUp
            | Msg::ValidationScrollDown
            | Msg::None => None,
        }
    }
}
