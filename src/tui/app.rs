//! Application setup for tui-realm.

use std::time::Duration;

use tuirealm::{Application, EventListenerCfg};

use super::components::params::{
    CycleSelector, HellwigPicker, HellwigPickerType, HellwigValues, HueGrid, SelectorType, Slider,
    SliderConfig, SliderType,
};
use super::components::{Palette, Preview, Validation};
use super::event::UserEvent;
use super::ids::Id;
use super::model::Model;
use super::msg::Msg;
use crate::curves::CurveType;

/// All focusable component IDs in order.
/// Some may be conditionally hidden (like strength sliders when sigmoid not selected).
const ALL_FOCUS_IDS: &[Id] = &[
    Id::BackgroundPicker,
    Id::ForegroundPicker,
    Id::LightnessCurve,
    Id::LightnessStrength,
    Id::ChromaCurve,
    Id::ChromaStrength,
    Id::HueCurve,
    Id::HueStrength,
    Id::TargetContrast,
    Id::ExtendedContrast,
    Id::AccentColorfulness,
    Id::ExtendedColorfulness,
    Id::HueOverrides,
    Id::Validation,
];

/// Create and configure the tui-realm application.
pub fn create_application() -> Application<Id, Msg, UserEvent> {
    Application::init(
        EventListenerCfg::default()
            .crossterm_input_listener(Duration::from_millis(20), 10)
            .poll_timeout(Duration::from_millis(50)),
    )
}

/// Mount all initial components.
pub fn mount_components(
    app: &mut Application<Id, Msg, UserEvent>,
    model: &Model,
) -> color_eyre::Result<()> {
    // Display components (read-only)
    let mut palette = Palette::new();
    palette.set_scheme(model.current_scheme.clone());
    palette.set_colors(
        model.background,
        model.foreground,
        model.interpolation.lightness.clone(),
    );
    app.mount(Id::Palette, Box::new(palette), vec![])?;

    let mut preview = Preview::new();
    preview.set_scheme(model.current_scheme.clone());
    app.mount(Id::Preview, Box::new(preview), vec![])?;

    // Parameter components
    let bg_picker = HellwigPicker::new(
        HellwigPickerType::Background,
        HellwigValues {
            lightness: model.background_hellwig.lightness,
            colorfulness: model.background_hellwig.colorfulness,
            hue: model.background_hellwig.hue,
            out_of_gamut: model.background_hellwig.out_of_gamut,
        },
        model.background,
    );
    app.mount(Id::BackgroundPicker, Box::new(bg_picker), vec![])?;

    let fg_picker = HellwigPicker::new(
        HellwigPickerType::Foreground,
        HellwigValues {
            lightness: model.foreground_hellwig.lightness,
            colorfulness: model.foreground_hellwig.colorfulness,
            hue: model.foreground_hellwig.hue,
            out_of_gamut: model.foreground_hellwig.out_of_gamut,
        },
        model.foreground,
    );
    app.mount(Id::ForegroundPicker, Box::new(fg_picker), vec![])?;

    let hue_grid = HueGrid::new(model.hue_overrides);
    app.mount(Id::HueOverrides, Box::new(hue_grid), vec![])?;

    // Curve selectors for J/M/h interpolation
    let lightness_curve = CycleSelector::new(
        "J Curve",
        curve_type_options(),
        curve_type_index(model.interpolation.lightness.curve_type),
        SelectorType::Lightness,
    );
    app.mount(Id::LightnessCurve, Box::new(lightness_curve), vec![])?;

    // Strength slider for sigmoid curve (only affects sigmoid)
    let lightness_strength = Slider::new(
        SliderConfig {
            label: "  Strength".to_string(),
            min: 0.1,
            max: 5.0,
            step: 0.05,
            precision: 2,
            suffix: String::new(),
            slider_type: SliderType::LightnessStrength,
        },
        f64::from(model.interpolation.lightness.strength),
    );
    app.mount(Id::LightnessStrength, Box::new(lightness_strength), vec![])?;

    let chroma_curve = CycleSelector::new(
        "M Curve",
        curve_type_options(),
        curve_type_index(model.interpolation.chroma.curve_type),
        SelectorType::Chroma,
    );
    app.mount(Id::ChromaCurve, Box::new(chroma_curve), vec![])?;

    // Strength slider for chroma sigmoid curve
    let chroma_strength = Slider::new(
        SliderConfig {
            label: "  Strength".to_string(),
            min: 0.1,
            max: 5.0,
            step: 0.05,
            precision: 2,
            suffix: String::new(),
            slider_type: SliderType::ChromaStrength,
        },
        f64::from(model.interpolation.chroma.strength),
    );
    app.mount(Id::ChromaStrength, Box::new(chroma_strength), vec![])?;

    let hue_curve = CycleSelector::new(
        "h Curve",
        curve_type_options(),
        curve_type_index(model.interpolation.hue.curve_type),
        SelectorType::Hue,
    );
    app.mount(Id::HueCurve, Box::new(hue_curve), vec![])?;

    // Strength slider for hue sigmoid curve
    let hue_strength = Slider::new(
        SliderConfig {
            label: "  Strength".to_string(),
            min: 0.1,
            max: 5.0,
            step: 0.05,
            precision: 2,
            suffix: String::new(),
            slider_type: SliderType::HueStrength,
        },
        f64::from(model.interpolation.hue.strength),
    );
    app.mount(Id::HueStrength, Box::new(hue_strength), vec![])?;

    // Contrast and colorfulness sliders
    let min_contrast = Slider::new(
        SliderConfig {
            label: "Min Lc".to_string(),
            min: 30.0,
            max: 90.0,
            step: 1.0,
            precision: 0,
            suffix: String::new(),
            slider_type: SliderType::MinContrast,
        },
        model.min_contrast,
    );
    app.mount(Id::TargetContrast, Box::new(min_contrast), vec![])?;

    let extended_min_contrast = Slider::new(
        SliderConfig {
            label: "Ext Min Lc".to_string(),
            min: 30.0,
            max: 90.0,
            step: 1.0,
            precision: 0,
            suffix: String::new(),
            slider_type: SliderType::ExtendedMinContrast,
        },
        model.extended_min_contrast,
    );
    app.mount(
        Id::ExtendedContrast,
        Box::new(extended_min_contrast),
        vec![],
    )?;

    // Colorfulness sliders (HellwigJmh M scale, 0-50 is typical for UI accents)
    let accent_colorfulness = Slider::new(
        SliderConfig {
            label: "Accent M".to_string(),
            min: 0.0,
            max: 50.0,
            step: 1.0,
            precision: 0,
            suffix: String::new(),
            slider_type: SliderType::AccentColorfulness,
        },
        f64::from(model.accent_colorfulness),
    );
    app.mount(
        Id::AccentColorfulness,
        Box::new(accent_colorfulness),
        vec![],
    )?;

    let extended_colorfulness = Slider::new(
        SliderConfig {
            label: "Extended M".to_string(),
            min: 0.0,
            max: 50.0,
            step: 1.0,
            precision: 0,
            suffix: String::new(),
            slider_type: SliderType::ExtendedColorfulness,
        },
        f64::from(model.extended_colorfulness),
    );
    app.mount(
        Id::ExtendedColorfulness,
        Box::new(extended_colorfulness),
        vec![],
    )?;

    // Validation panel
    let mut validation = Validation::new();
    validation.set_data(
        model.validation_results.clone(),
        model.generation_warnings.clone(),
        model.current_scheme.is_some(),
    );
    app.mount(Id::Validation, Box::new(validation), vec![])?;

    // Set initial focus
    app.active(&Id::BackgroundPicker)?;

    Ok(())
}

/// Get the list of curve type display names for selectors.
fn curve_type_options() -> Vec<String> {
    vec![
        CurveType::Linear.display_name().to_string(),
        CurveType::Smoothstep.display_name().to_string(),
        CurveType::Smootherstep.display_name().to_string(),
        CurveType::SmoothStart.display_name().to_string(),
        CurveType::SmoothEnd.display_name().to_string(),
        CurveType::Sigmoid.display_name().to_string(),
        CurveType::BSpline.display_name().to_string(),
    ]
}

/// Get the index of a curve type in the options list.
fn curve_type_index(curve_type: CurveType) -> usize {
    match curve_type {
        CurveType::Linear => 0,
        CurveType::Smoothstep => 1,
        CurveType::Smootherstep => 2,
        CurveType::SmoothStart => 3,
        CurveType::SmoothEnd => 4,
        CurveType::Sigmoid => 5,
        CurveType::BSpline => 6,
    }
}

/// Manages focus state for Tab navigation.
pub struct FocusManager {
    current_idx: usize,
    /// Current list of visible/focusable IDs
    visible_ids: Vec<Id>,
}

impl FocusManager {
    pub fn new() -> Self {
        Self {
            current_idx: 0,
            visible_ids: ALL_FOCUS_IDS.to_vec(),
        }
    }

    /// Update which components are visible based on model state.
    pub fn update_visible(&mut self, model: &Model) {
        let show_lightness_strength = model.interpolation.lightness.curve_type.uses_strength();
        let show_chroma_strength = model.interpolation.chroma.curve_type.uses_strength();
        let show_hue_strength = model.interpolation.hue.curve_type.uses_strength();

        self.visible_ids = ALL_FOCUS_IDS
            .iter()
            .copied()
            .filter(|id| {
                // Strength sliders only visible when sigmoid selected for that curve
                match *id {
                    Id::LightnessStrength => show_lightness_strength,
                    Id::ChromaStrength => show_chroma_strength,
                    Id::HueStrength => show_hue_strength,
                    _ => true,
                }
            })
            .collect();

        // Clamp current index if it's now out of bounds
        if self.current_idx >= self.visible_ids.len() {
            self.current_idx = self.visible_ids.len().saturating_sub(1);
        }
    }

    /// Get the current focus component ID.
    pub fn current_focus(&self) -> Id {
        self.visible_ids
            .get(self.current_idx)
            .copied()
            .unwrap_or(Id::BackgroundPicker)
    }

    /// Move focus to next component and return its ID.
    pub fn focus_next(&mut self) -> Id {
        if !self.visible_ids.is_empty() {
            self.current_idx = (self.current_idx + 1) % self.visible_ids.len();
        }
        self.current_focus()
    }

    /// Move focus to previous component and return its ID.
    pub fn focus_prev(&mut self) -> Id {
        if !self.visible_ids.is_empty() {
            self.current_idx =
                (self.current_idx + self.visible_ids.len() - 1) % self.visible_ids.len();
        }
        self.current_focus()
    }
}

impl Default for FocusManager {
    fn default() -> Self {
        Self::new()
    }
}
