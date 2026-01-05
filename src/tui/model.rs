//! Application model for the TUI.

use std::path::PathBuf;

use color_eyre::eyre::{Result, WrapErr};
use palette::Srgb;
use tinted_builder::{Base16Scheme, SchemeVariant};
use tuirealm::Update;

use crate::cli::{Cli, VariantArg};
use crate::curves::InterpolationConfig;
use crate::generate::{GenerateConfig, generate_for_variant, parse_color};
use crate::hellwig::HellwigJmh;
use crate::validation::{ValidationResults, validate};

use super::msg::Msg;

/// HellwigJmh color components for editing.
#[derive(Debug, Clone, Copy)]
pub struct HellwigComponents {
    /// Lightness with HK effect (J'), range 0-100
    pub lightness: f32,
    /// Colorfulness (M), range 0-105
    pub colorfulness: f32,
    /// Hue angle in degrees (0-360)
    pub hue: f32,
    /// Whether the color is out of sRGB gamut
    pub out_of_gamut: bool,
}

impl HellwigComponents {
    pub fn from_srgb(color: Srgb<u8>) -> Self {
        let hellwig = HellwigJmh::from_srgb_u8(color);
        Self {
            lightness: hellwig.lightness,
            colorfulness: hellwig.colorfulness,
            hue: hellwig.hue,
            out_of_gamut: false,
        }
    }

    pub fn to_srgb(self) -> Srgb<u8> {
        HellwigJmh::new(self.lightness, self.colorfulness, self.hue).into_srgb_u8()
    }

    pub fn check_gamut(&mut self) {
        let hellwig = HellwigJmh::new(self.lightness, self.colorfulness, self.hue);
        self.out_of_gamut = !hellwig.is_in_gamut();
    }
}

impl Default for HellwigComponents {
    fn default() -> Self {
        Self {
            lightness: 50.0,
            colorfulness: 0.0,
            hue: 0.0,
            out_of_gamut: false,
        }
    }
}

/// Application model containing all state.
pub struct Model {
    // Editable parameters
    pub background_hellwig: HellwigComponents,
    pub foreground_hellwig: HellwigComponents,
    pub min_contrast: f64,
    pub extended_min_contrast: f64,
    pub max_lightness_adjustment: f32,
    pub accent_colorfulness: f32,
    pub extended_colorfulness: f32,
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
    pub validation_results: Option<ValidationResults>,

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

        let background_hellwig = HellwigComponents::from_srgb(background);
        let foreground_hellwig = HellwigComponents::from_srgb(foreground);

        Ok(Self {
            background_hellwig,
            foreground_hellwig,
            min_contrast: cli.min_contrast,
            extended_min_contrast: cli.extended_min_contrast,
            max_lightness_adjustment: cli.max_lightness_adjustment,
            accent_colorfulness: cli.accent_colorfulness,
            extended_colorfulness: cli.extended_colorfulness,
            hue_overrides: cli.hue_overrides(),
            variant: cli.variant,
            name,
            author: cli.author.clone().unwrap_or_default(),
            interpolation: InterpolationConfig::default(),

            background,
            foreground,

            current_scheme: None,
            generation_warnings: Vec::new(),
            validation_results: None,

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
            accent_colorfulness: self.accent_colorfulness,
            extended_colorfulness: self.extended_colorfulness,
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
        // Recompute sRGB from HellwigJmh
        self.background = self.background_hellwig.to_srgb();
        self.foreground = self.foreground_hellwig.to_srgb();

        // Check gamut
        self.background_hellwig.check_gamut();
        self.foreground_hellwig.check_gamut();

        let config = self.to_generate_config();
        let forced = match self.variant {
            VariantArg::Auto => None,
            VariantArg::Dark => Some(SchemeVariant::Dark),
            VariantArg::Light => Some(SchemeVariant::Light),
            VariantArg::Both => None,
        };

        let result = generate_for_variant(&config, forced);
        self.validation_results = Some(validate(&result.scheme));
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

            // Background changes (HellwigJmh J', M, h)
            Msg::BackgroundJChanged(v) => {
                self.background_hellwig.lightness = v;
                Some(Msg::Regenerate)
            }
            Msg::BackgroundMChanged(v) => {
                self.background_hellwig.colorfulness = v;
                Some(Msg::Regenerate)
            }
            Msg::BackgroundHChanged(v) => {
                self.background_hellwig.hue = v;
                Some(Msg::Regenerate)
            }

            // Foreground changes (HellwigJmh J', M, h)
            Msg::ForegroundJChanged(v) => {
                self.foreground_hellwig.lightness = v;
                Some(Msg::Regenerate)
            }
            Msg::ForegroundMChanged(v) => {
                self.foreground_hellwig.colorfulness = v;
                Some(Msg::Regenerate)
            }
            Msg::ForegroundHChanged(v) => {
                self.foreground_hellwig.hue = v;
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
            Msg::AccentColorfulnessChanged(v) => {
                self.accent_colorfulness = v;
                Some(Msg::Regenerate)
            }
            Msg::ExtendedColorfulnessChanged(v) => {
                self.extended_colorfulness = v;
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
            Msg::ChromaCurveStrengthChanged(v) => {
                self.interpolation.chroma.strength = v;
                Some(Msg::Regenerate)
            }
            Msg::HueCurveTypeChanged(v) => {
                self.interpolation.hue.curve_type = v;
                Some(Msg::Regenerate)
            }
            Msg::HueCurveStrengthChanged(v) => {
                self.interpolation.hue.strength = v;
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
