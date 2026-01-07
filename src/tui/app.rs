//! Application setup for tui-realm.

use std::time::Duration;

use tuirealm::{Application, EventListenerCfg};

use super::components::params::{
    AccentControls, AccentControlsType, AccentValues, CurveControls, CurveValues, HellwigPicker,
    HellwigPickerType, HellwigValues, HueGrid, WeightControls, WeightValues,
};
use super::components::{Palette, Preview, Validation};
use super::model::Model;
use super::msg::Msg;
use super::{Id, UserEvent};

/// All focusable component IDs in order.
const ALL_FOCUS_IDS: &[Id] = &[
    Id::BackgroundPicker,
    Id::ForegroundPicker,
    Id::CurveControls,
    Id::WeightControls,
    Id::AccentControls,
    Id::ExtendedAccentControls,
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

    // Grouped curve controls (J/M/h interpolation)
    let curve_controls = CurveControls::new(CurveValues {
        j_type: model.interpolation.lightness.curve_type,
        j_strength: model.interpolation.lightness.strength,
        m_type: model.interpolation.chroma.curve_type,
        m_strength: model.interpolation.chroma.strength,
        h_type: model.interpolation.hue.curve_type,
        h_strength: model.interpolation.hue.strength,
    });
    app.mount(Id::CurveControls, Box::new(curve_controls), vec![])?;

    // Grouped weight controls (Lc/J optimization weights)
    let weight_controls = WeightControls::new(WeightValues {
        contrast_weight: model.accent_opt.contrast_weight,
        j_weight: model.accent_opt.j_weight,
    });
    app.mount(Id::WeightControls, Box::new(weight_controls), vec![])?;

    // Grouped accent controls (base08-base0F)
    let accent_controls = AccentControls::new(
        AccentControlsType::Base,
        AccentValues {
            min_contrast: model.min_contrast,
            target_j: model.accent_opt.target_j,
            delta_j: model.accent_opt.delta_j,
            target_m: model.accent_opt.target_m,
            delta_m: model.accent_opt.delta_m,
        },
    );
    app.mount(Id::AccentControls, Box::new(accent_controls), vec![])?;

    // Grouped extended accent controls (base10-base17)
    let extended_controls = AccentControls::new(
        AccentControlsType::Extended,
        AccentValues {
            min_contrast: model.extended_min_contrast,
            target_j: model.extended_accent_opt.target_j,
            delta_j: model.extended_accent_opt.delta_j,
            target_m: model.extended_accent_opt.target_m,
            delta_m: model.extended_accent_opt.delta_m,
        },
    );
    app.mount(
        Id::ExtendedAccentControls,
        Box::new(extended_controls),
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

/// Manages focus state for Tab navigation.
pub struct FocusManager {
    current_idx: usize,
}

impl FocusManager {
    pub fn new() -> Self {
        Self { current_idx: 0 }
    }

    /// Get the current focus component ID.
    pub fn current_focus(&self) -> Id {
        ALL_FOCUS_IDS
            .get(self.current_idx)
            .copied()
            .unwrap_or(Id::BackgroundPicker)
    }

    /// Move focus to next component and return its ID.
    pub fn focus_next(&mut self) -> Id {
        self.current_idx = (self.current_idx + 1) % ALL_FOCUS_IDS.len();
        self.current_focus()
    }

    /// Move focus to previous component and return its ID.
    pub fn focus_prev(&mut self) -> Id {
        self.current_idx = (self.current_idx + ALL_FOCUS_IDS.len() - 1) % ALL_FOCUS_IDS.len();
        self.current_focus()
    }
}

impl Default for FocusManager {
    fn default() -> Self {
        Self::new()
    }
}
