//! TUI state management.

use std::path::PathBuf;

use color_eyre::eyre::{Result, WrapErr};
use palette::Srgb;
use tinted_builder::{Base16Scheme, SchemeVariant};

use crate::cli::{Cli, VariantArg};
use crate::generate::{GenerateConfig, generate_for_variant, parse_hex};
use crate::validation::{ValidationResult, validate};

/// Which pane is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pane {
    #[default]
    Parameters,
    Validation,
}

/// Focus target for keyboard navigation within the parameters pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Focus {
    #[default]
    Background,
    Foreground,
    TargetContrast,
    ExtendedContrast,
    AccentChroma,
    ExtendedChroma,
    Variant,
    Hue08,
    Hue09,
    Hue0A,
    Hue0B,
    Hue0C,
    Hue0D,
    Hue0E,
    Hue0F,
    Name,
    Author,
}

impl Focus {
    /// Get the next focus target.
    pub fn next(self) -> Self {
        match self {
            Self::Background => Self::Foreground,
            Self::Foreground => Self::TargetContrast,
            Self::TargetContrast => Self::ExtendedContrast,
            Self::ExtendedContrast => Self::AccentChroma,
            Self::AccentChroma => Self::ExtendedChroma,
            Self::ExtendedChroma => Self::Variant,
            Self::Variant => Self::Hue08,
            Self::Hue08 => Self::Hue09,
            Self::Hue09 => Self::Hue0A,
            Self::Hue0A => Self::Hue0B,
            Self::Hue0B => Self::Hue0C,
            Self::Hue0C => Self::Hue0D,
            Self::Hue0D => Self::Hue0E,
            Self::Hue0E => Self::Hue0F,
            Self::Hue0F => Self::Name,
            Self::Name => Self::Author,
            Self::Author => Self::Background,
        }
    }

    /// Get the previous focus target.
    pub fn prev(self) -> Self {
        match self {
            Self::Background => Self::Author,
            Self::Foreground => Self::Background,
            Self::TargetContrast => Self::Foreground,
            Self::ExtendedContrast => Self::TargetContrast,
            Self::AccentChroma => Self::ExtendedContrast,
            Self::ExtendedChroma => Self::AccentChroma,
            Self::Variant => Self::ExtendedChroma,
            Self::Hue08 => Self::Variant,
            Self::Hue09 => Self::Hue08,
            Self::Hue0A => Self::Hue09,
            Self::Hue0B => Self::Hue0A,
            Self::Hue0C => Self::Hue0B,
            Self::Hue0D => Self::Hue0C,
            Self::Hue0E => Self::Hue0D,
            Self::Hue0F => Self::Hue0E,
            Self::Name => Self::Hue0F,
            Self::Author => Self::Name,
        }
    }
}

/// All editable state for the TUI.
#[derive(Debug)]
pub struct TuiState {
    // Editable parameters
    pub background_hex: String,
    pub foreground_hex: String,
    pub target_contrast: f64,
    pub extended_contrast: f64,
    pub accent_chroma: f32,
    pub extended_chroma: f32,
    pub hue_overrides: [Option<f32>; 8],
    pub variant: VariantArg,
    pub name: String,
    pub author: String,

    // Parsed colors (derived from hex strings)
    pub background: Option<Srgb<u8>>,
    pub foreground: Option<Srgb<u8>>,

    // Generated output
    pub current_scheme: Option<Base16Scheme>,
    pub generation_warnings: Vec<String>,
    pub validation_results: Vec<ValidationResult>,

    // UI state
    pub active_pane: Pane,
    pub focus: Focus,
    pub show_help: bool,
    pub show_export: bool,
    pub export_path: String,
    pub editing_text: bool,
    pub text_cursor: usize,
    pub message: Option<String>,
    pub validation_scroll: u16,
}

impl TuiState {
    /// Create state from CLI arguments.
    pub fn from_cli(cli: &Cli) -> Result<Self> {
        let background_hex = cli
            .background
            .clone()
            .unwrap_or_else(|| "#000000".to_string());
        let foreground_hex = cli
            .foreground
            .clone()
            .unwrap_or_else(|| "#FFFFFF".to_string());
        let name = cli.name.clone().unwrap_or_else(|| "My Theme".to_string());

        let background = parse_hex(&background_hex).ok();
        let foreground = parse_hex(&foreground_hex).ok();

        Ok(Self {
            background_hex,
            foreground_hex,
            target_contrast: cli.target_contrast,
            extended_contrast: cli.extended_contrast,
            accent_chroma: cli.accent_chroma,
            extended_chroma: cli.extended_chroma,
            hue_overrides: cli.hue_overrides(),
            variant: cli.variant,
            name,
            author: cli.author.clone().unwrap_or_default(),

            background,
            foreground,

            current_scheme: None,
            generation_warnings: Vec::new(),
            validation_results: Vec::new(),

            active_pane: Pane::Parameters,
            focus: Focus::Background,
            show_help: false,
            show_export: false,
            export_path: String::from("scheme.yaml"),
            editing_text: false,
            text_cursor: 0,
            message: None,
            validation_scroll: 0,
        })
    }

    /// Convert current state to GenerateConfig.
    fn to_generate_config(&self) -> Result<GenerateConfig, String> {
        let background = self.background.ok_or("Invalid background color")?;
        let foreground = self.foreground.ok_or("Invalid foreground color")?;

        Ok(GenerateConfig {
            background,
            foreground,
            hue_overrides: self.hue_overrides,
            target_contrast: self.target_contrast,
            extended_contrast: self.extended_contrast,
            accent_chroma: self.accent_chroma,
            extended_chroma: self.extended_chroma,
            name: self.name.clone(),
            author: if self.author.is_empty() {
                None
            } else {
                Some(self.author.clone())
            },
        })
    }

    /// Regenerate the palette from current state.
    pub fn regenerate(&mut self) {
        // Reparse colors from hex strings
        self.background = parse_hex(&self.background_hex).ok();
        self.foreground = parse_hex(&self.foreground_hex).ok();

        match self.to_generate_config() {
            Ok(config) => {
                let forced = match self.variant {
                    VariantArg::Auto => None,
                    VariantArg::Dark => Some(SchemeVariant::Dark),
                    VariantArg::Light => Some(SchemeVariant::Light),
                    VariantArg::Both => None, // TUI shows one at a time
                };

                let result = generate_for_variant(&config, forced);
                self.validation_results = validate(&result.scheme);
                self.generation_warnings = result.warnings;
                self.current_scheme = Some(result.scheme);
                self.message = None;
            }
            Err(e) => {
                self.message = Some(format!("Error: {e}"));
                self.current_scheme = None;
                self.generation_warnings.clear();
                self.validation_results.clear();
            }
        }
    }

    /// Export the current scheme to a file.
    pub fn export(&mut self) -> Result<()> {
        if let Some(ref scheme) = self.current_scheme {
            let yaml = serde_yaml::to_string(scheme).wrap_err("Failed to serialize scheme")?;

            let path = PathBuf::from(&self.export_path);
            std::fs::write(&path, &yaml)
                .wrap_err_with(|| format!("Failed to write to {}", path.display()))?;

            self.message = Some(format!("Exported to {}", path.display()));
            self.show_export = false;
        } else {
            self.message = Some("No scheme to export".to_string());
        }
        Ok(())
    }

    /// Adjust a numeric value based on focus.
    pub fn adjust_value(&mut self, delta: f64) {
        match self.focus {
            Focus::TargetContrast => {
                self.target_contrast = (self.target_contrast + delta).clamp(30.0, 100.0);
            }
            Focus::ExtendedContrast => {
                self.extended_contrast = (self.extended_contrast + delta).clamp(30.0, 100.0);
            }
            Focus::AccentChroma => {
                self.accent_chroma = (self.accent_chroma + delta as f32 * 0.01).clamp(0.0, 0.4);
            }
            Focus::ExtendedChroma => {
                self.extended_chroma = (self.extended_chroma + delta as f32 * 0.01).clamp(0.0, 0.4);
            }
            Focus::Hue08 => self.adjust_hue(0, delta),
            Focus::Hue09 => self.adjust_hue(1, delta),
            Focus::Hue0A => self.adjust_hue(2, delta),
            Focus::Hue0B => self.adjust_hue(3, delta),
            Focus::Hue0C => self.adjust_hue(4, delta),
            Focus::Hue0D => self.adjust_hue(5, delta),
            Focus::Hue0E => self.adjust_hue(6, delta),
            Focus::Hue0F => self.adjust_hue(7, delta),
            _ => {}
        }
    }

    fn adjust_hue(&mut self, index: usize, delta: f64) {
        let default_hues = [25.0, 55.0, 90.0, 145.0, 180.0, 250.0, 285.0, 335.0];
        let current = self.hue_overrides[index].unwrap_or(default_hues[index]);
        let new_val = (current + delta as f32).rem_euclid(360.0);
        self.hue_overrides[index] = Some(new_val);
    }

    /// Cycle through variant options.
    pub fn cycle_variant(&mut self, forward: bool) {
        self.variant = if forward {
            match self.variant {
                VariantArg::Auto => VariantArg::Dark,
                VariantArg::Dark => VariantArg::Light,
                VariantArg::Light => VariantArg::Auto,
                VariantArg::Both => VariantArg::Auto,
            }
        } else {
            match self.variant {
                VariantArg::Auto => VariantArg::Light,
                VariantArg::Dark => VariantArg::Auto,
                VariantArg::Light => VariantArg::Dark,
                VariantArg::Both => VariantArg::Auto,
            }
        };
    }

    /// Get the currently focused text field for editing.
    pub fn focused_text(&self) -> Option<&str> {
        match self.focus {
            Focus::Background => Some(&self.background_hex),
            Focus::Foreground => Some(&self.foreground_hex),
            Focus::Name => Some(&self.name),
            Focus::Author => Some(&self.author),
            _ => None,
        }
    }

    /// Get mutable reference to currently focused text field.
    pub fn focused_text_mut(&mut self) -> Option<&mut String> {
        match self.focus {
            Focus::Background => Some(&mut self.background_hex),
            Focus::Foreground => Some(&mut self.foreground_hex),
            Focus::Name => Some(&mut self.name),
            Focus::Author => Some(&mut self.author),
            _ => None,
        }
    }

    /// Check if the current focus is a text field.
    pub fn is_text_field(&self) -> bool {
        matches!(
            self.focus,
            Focus::Background | Focus::Foreground | Focus::Name | Focus::Author
        )
    }

    /// Check if the current focus is the variant selector.
    pub fn is_variant_field(&self) -> bool {
        matches!(self.focus, Focus::Variant)
    }

    /// Insert a character at the cursor position in the focused text field.
    pub fn insert_char(&mut self, c: char) {
        let cursor = self.text_cursor;
        match self.focus {
            Focus::Background => {
                let pos = cursor.min(self.background_hex.len());
                self.background_hex.insert(pos, c);
                self.text_cursor = pos + 1;
            }
            Focus::Foreground => {
                let pos = cursor.min(self.foreground_hex.len());
                self.foreground_hex.insert(pos, c);
                self.text_cursor = pos + 1;
            }
            Focus::Name => {
                let pos = cursor.min(self.name.len());
                self.name.insert(pos, c);
                self.text_cursor = pos + 1;
            }
            Focus::Author => {
                let pos = cursor.min(self.author.len());
                self.author.insert(pos, c);
                self.text_cursor = pos + 1;
            }
            _ => {}
        }
    }

    /// Delete the character before the cursor in the focused text field.
    pub fn delete_char_before(&mut self) {
        if self.text_cursor == 0 {
            return;
        }
        let cursor = self.text_cursor;
        match self.focus {
            Focus::Background => {
                let pos = cursor.min(self.background_hex.len());
                if pos > 0 {
                    self.background_hex.remove(pos - 1);
                    self.text_cursor = pos - 1;
                }
            }
            Focus::Foreground => {
                let pos = cursor.min(self.foreground_hex.len());
                if pos > 0 {
                    self.foreground_hex.remove(pos - 1);
                    self.text_cursor = pos - 1;
                }
            }
            Focus::Name => {
                let pos = cursor.min(self.name.len());
                if pos > 0 {
                    self.name.remove(pos - 1);
                    self.text_cursor = pos - 1;
                }
            }
            Focus::Author => {
                let pos = cursor.min(self.author.len());
                if pos > 0 {
                    self.author.remove(pos - 1);
                    self.text_cursor = pos - 1;
                }
            }
            _ => {}
        }
    }

    /// Delete the character at the cursor position in the focused text field.
    pub fn delete_char_at(&mut self) {
        let cursor = self.text_cursor;
        match self.focus {
            Focus::Background => {
                let pos = cursor.min(self.background_hex.len());
                if pos < self.background_hex.len() {
                    self.background_hex.remove(pos);
                }
            }
            Focus::Foreground => {
                let pos = cursor.min(self.foreground_hex.len());
                if pos < self.foreground_hex.len() {
                    self.foreground_hex.remove(pos);
                }
            }
            Focus::Name => {
                let pos = cursor.min(self.name.len());
                if pos < self.name.len() {
                    self.name.remove(pos);
                }
            }
            Focus::Author => {
                let pos = cursor.min(self.author.len());
                if pos < self.author.len() {
                    self.author.remove(pos);
                }
            }
            _ => {}
        }
    }

    /// Scroll the validation pane up.
    pub fn scroll_validation_up(&mut self, lines: u16) {
        self.validation_scroll = self.validation_scroll.saturating_sub(lines);
    }

    /// Scroll the validation pane down.
    pub fn scroll_validation_down(&mut self, lines: u16, max_scroll: u16) {
        self.validation_scroll = self.validation_scroll.saturating_add(lines).min(max_scroll);
    }

    /// Calculate the total number of lines in validation content.
    pub fn validation_content_lines(&self) -> u16 {
        if self.current_scheme.is_none() {
            return 1;
        }

        let failures: usize = self.validation_results.iter().filter(|r| !r.passes).count();

        // Summary line + empty line + failure lines (or success message) + warnings section
        let mut lines = 2; // summary + empty line

        if failures == 0 {
            lines += 1; // success message
        } else {
            lines += failures;
        }

        // Warnings section
        if !self.generation_warnings.is_empty() {
            lines += 2; // empty line + header
            lines += self.generation_warnings.len().min(3);
        }

        lines as u16
    }
}
