//! Main activity - the primary palette editing screen.

use std::io::Stdout;
use std::time::Duration;

use color_eyre::eyre::Result;
use ratatui::{
    Terminal,
    crossterm::event::{self, Event, KeyCode},
    layout::{Constraint, Direction, Layout},
    prelude::CrosstermBackend,
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph},
};
use tuirealm::{Application, EventListenerCfg, PollStrategy, Update};

use crate::cli::VariantArg;
use crate::curves::CurveType;
use crate::tui::Model;
use crate::tui::activity::{Activity, Context, ExitReason};
use crate::tui::components::params::{
    AccentControls, AccentControlsType, AccentValues, CurveControls, CurveValues, HellwigPicker,
    HellwigPickerType, HellwigValues, HueGrid, WeightControls, WeightValues,
};
use crate::tui::components::{
    MAIN_FOOTER_ACTIONS, Palette, Preview, Validation, format_footer, render_help,
};

// ============================================================================
// Component identifiers (scoped to MainActivity)
// ============================================================================

/// Unique identifiers for all components in MainActivity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Id {
    // Display panels (read-only)
    Palette,
    Preview,

    // Parameter groups (editable)
    BackgroundPicker,
    ForegroundPicker,
    CurveControls,
    WeightControls,
    AccentControls,
    ExtendedAccentControls,
    HueOverrides,

    // Scrollable panel
    Validation,
}

// ============================================================================
// Messages (scoped to MainActivity)
// ============================================================================

/// All possible messages that can be sent in MainActivity.
#[derive(Debug, Clone, PartialEq)]
pub enum Msg {
    // Application control
    Quit,
    ShowHelp,
    HideHelp,

    // Focus/Navigation
    FocusNext,
    FocusPrev,

    // Background HellwigJmh changes (J', M, h)
    BackgroundJChanged(f32),
    BackgroundMChanged(f32),
    BackgroundHChanged(f32),

    // Foreground HellwigJmh changes (J', M, h)
    ForegroundJChanged(f32),
    ForegroundMChanged(f32),
    ForegroundHChanged(f32),

    // Numeric parameter changes
    MinContrastChanged(f64),
    ExtendedMinContrastChanged(f64),

    // Selection changes
    VariantChanged(VariantArg),
    LightnessCurveTypeChanged(CurveType),
    LightnessCurveStrengthChanged(f32),
    ChromaCurveTypeChanged(CurveType),
    ChromaCurveStrengthChanged(f32),
    HueCurveTypeChanged(CurveType),
    HueCurveStrengthChanged(f32),

    // Hue override changes (index 0-7)
    HueOverrideChanged(u8, Option<f32>),

    // Accent optimization changes
    AccentTargetJChanged(f32),
    AccentDeltaJChanged(f32),
    AccentTargetMChanged(f32),
    AccentDeltaMChanged(f32),
    ExtendedTargetJChanged(f32),
    ExtendedDeltaJChanged(f32),
    ExtendedTargetMChanged(f32),
    ExtendedDeltaMChanged(f32),

    // Optimization weight changes
    ContrastWeightChanged(f32),
    JWeightChanged(f32),

    // Metadata changes
    NameChanged(String),
    AuthorChanged(String),

    // Palette regeneration (chained after parameter changes)
    Regenerate,

    // Export flow
    ExportPathChanged(String),
    DoExport,
    ExportSuccess(String),
    ExportError(String),

    // Validation scroll
    ValidationScrollUp,
    ValidationScrollDown,

    // Activity transition
    SwitchToCodePreview,

    // Toggle dark/light variant
    ToggleDarkLight,
}

// ============================================================================
// User events (required by tui-realm, currently unused)
// ============================================================================

/// Custom user events (currently unused, but required by tui-realm).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserEvent {}

// ============================================================================
// Focus management (scoped to MainActivity)
// ============================================================================

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

/// Manages focus state for Tab navigation in MainActivity.
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

// ============================================================================
// MainActivity
// ============================================================================

/// The main palette editing activity.
#[derive(Default)]
pub struct MainActivity {
    app: Option<Application<Id, Msg, UserEvent>>,
    focus: FocusManager,
    context: Option<Context>,
    exit_reason: Option<ExitReason>,
}

impl MainActivity {
    /// Create and configure the tui-realm application.
    fn create_application() -> Application<Id, Msg, UserEvent> {
        Application::init(
            EventListenerCfg::default()
                .crossterm_input_listener(Duration::from_millis(20), 10)
                .poll_timeout(Duration::from_millis(50)),
        )
    }

    /// Mount all initial components.
    fn mount_components(app: &mut Application<Id, Msg, UserEvent>, model: &Model) -> Result<()> {
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

    /// Sync display-only components (Palette, Preview, Validation) with current model data.
    fn sync_display_components(app: &mut Application<Id, Msg, UserEvent>, model: &Model) {
        // Remount Palette with updated scheme and colors
        let _ = app.umount(&Id::Palette);
        let mut palette = Palette::new();
        palette.set_scheme(model.current_scheme.clone());
        palette.set_colors(
            model.background,
            model.foreground,
            model.interpolation.lightness.clone(),
        );
        let _ = app.mount(Id::Palette, Box::new(palette), vec![]);

        // Remount Preview with updated scheme
        let _ = app.umount(&Id::Preview);
        let mut preview = Preview::new();
        preview.set_scheme(model.current_scheme.clone());
        let _ = app.mount(Id::Preview, Box::new(preview), vec![]);

        // Remount Validation with updated results
        let _ = app.umount(&Id::Validation);
        let mut validation = Validation::new();
        validation.set_data(
            model.validation_results.clone(),
            model.generation_warnings.clone(),
            model.current_scheme.is_some(),
        );
        let _ = app.mount(Id::Validation, Box::new(validation), vec![]);
    }

    /// Sync all components including parameter editors (used after dark/light toggle).
    fn sync_all_components(app: &mut Application<Id, Msg, UserEvent>, model: &Model) {
        // Remount background picker
        let _ = app.umount(&Id::BackgroundPicker);
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
        let _ = app.mount(Id::BackgroundPicker, Box::new(bg_picker), vec![]);

        // Remount foreground picker
        let _ = app.umount(&Id::ForegroundPicker);
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
        let _ = app.mount(Id::ForegroundPicker, Box::new(fg_picker), vec![]);

        // Remount accent controls (target_j changed)
        let _ = app.umount(&Id::AccentControls);
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
        let _ = app.mount(Id::AccentControls, Box::new(accent_controls), vec![]);

        // Remount extended accent controls
        let _ = app.umount(&Id::ExtendedAccentControls);
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
        let _ = app.mount(
            Id::ExtendedAccentControls,
            Box::new(extended_controls),
            vec![],
        );

        // Also sync display components
        Self::sync_display_components(app, model);
    }
}

impl Activity for MainActivity {
    fn on_create(&mut self, context: Context) {
        self.context = Some(context);
        let mut app = Self::create_application();

        let model = &self.context.as_ref().unwrap().model;
        if let Err(e) = Self::mount_components(&mut app, model) {
            tracing::error!("Failed to mount components: {}", e);
        }

        self.app = Some(app);
    }

    fn on_draw(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        let app = self.app.as_mut().expect("app should be initialized");
        let model = &mut self.context.as_mut().expect("context should be set").model;

        // Draw UI
        terminal.draw(|frame| {
            let area = frame.area();

            let main_rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // Title
                    Constraint::Min(10),   // Content
                    Constraint::Length(1), // Status
                ])
                .split(area);

            // Title bar
            let title = format!(
                " {} - {} ",
                model.name,
                match model.variant {
                    VariantArg::Auto => "auto",
                    VariantArg::Dark => "dark",
                    VariantArg::Light => "light",
                    VariantArg::Both => "both",
                }
            );
            let title_widget =
                Paragraph::new(title).style(Style::default().add_modifier(Modifier::BOLD));
            frame.render_widget(title_widget, main_rows[0]);

            // Content: 2 columns
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_rows[1]);

            // Left column: Palette + Preview
            let left_rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(10), Constraint::Length(13)])
                .split(cols[0]);

            // Render components
            app.view(&Id::Palette, frame, left_rows[0]);
            app.view(&Id::Preview, frame, left_rows[1]);

            // Parameters section layout - heights defined once, total computed automatically
            const PARAM_HEIGHTS: &[u16] = &[
                4, // 0: Background picker (header + J/M/h)
                4, // 1: Foreground picker (header + J/M/h)
                1, // 2: Spacer
                3, // 3: Curve controls (3 rows, inline strength)
                2, // 4: Weight controls (grouped)
                1, // 5: Spacer
                5, // 6: Accent controls (grouped)
                5, // 7: Extended accent controls (grouped)
                1, // 8: Spacer
                3, // 9: Hue overrides
            ];
            const PARAMS_CONTENT_HEIGHT: u16 = const {
                let mut sum = 0u16;
                let mut i = 0;
                while i < PARAM_HEIGHTS.len() {
                    sum += PARAM_HEIGHTS[i];
                    i += 1;
                }
                sum
            };

            // Right column: Parameters (content + borders) + Validation (fills remaining)
            let right_rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(PARAMS_CONTENT_HEIGHT + 2),
                    Constraint::Fill(1),
                ])
                .split(cols[1]);

            let params_area = right_rows[0];
            let params_block = Block::default().title(" Parameters ").borders(Borders::ALL);
            let params_inner = params_block.inner(params_area);
            frame.render_widget(params_block, params_area);

            let param_constraints: [Constraint; 11] = [
                Constraint::Length(PARAM_HEIGHTS[0]),
                Constraint::Length(PARAM_HEIGHTS[1]),
                Constraint::Length(PARAM_HEIGHTS[2]),
                Constraint::Length(PARAM_HEIGHTS[3]),
                Constraint::Length(PARAM_HEIGHTS[4]),
                Constraint::Length(PARAM_HEIGHTS[5]),
                Constraint::Length(PARAM_HEIGHTS[6]),
                Constraint::Length(PARAM_HEIGHTS[7]),
                Constraint::Length(PARAM_HEIGHTS[8]),
                Constraint::Length(PARAM_HEIGHTS[9]),
                Constraint::Min(0),
            ];
            let param_rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints(param_constraints)
                .split(params_inner);

            app.view(&Id::BackgroundPicker, frame, param_rows[0]);
            app.view(&Id::ForegroundPicker, frame, param_rows[1]);
            app.view(&Id::CurveControls, frame, param_rows[3]);
            app.view(&Id::WeightControls, frame, param_rows[4]);
            app.view(&Id::AccentControls, frame, param_rows[6]);
            app.view(&Id::ExtendedAccentControls, frame, param_rows[7]);
            app.view(&Id::HueOverrides, frame, param_rows[9]);

            // Validation panel
            app.view(&Id::Validation, frame, right_rows[1]);

            // Status bar
            let status = model
                .message
                .clone()
                .unwrap_or_else(|| format_footer(MAIN_FOOTER_ACTIONS, &[("adjust", "[]/{}")]));

            let status_widget =
                Paragraph::new(status).style(Style::default().add_modifier(Modifier::DIM));
            frame.render_widget(status_widget, main_rows[2]);

            // Help modal overlay
            if model.show_help {
                render_help(frame);
            }
        })?;

        // Handle help modal events separately (intercepts all input when visible)
        if model.show_help {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('?') => {
                        model.show_help = false;
                    }
                    _ => {}
                }
            }
            return Ok(());
        }

        // Use tick() - the canonical tui-realm heartbeat
        match app.tick(PollStrategy::Once) {
            Ok(messages) => {
                let mut needs_sync = false;
                let mut needs_full_sync = false;

                for msg in messages {
                    // Handle focus changes at activity level
                    match &msg {
                        Msg::FocusNext => {
                            let next = self.focus.focus_next();
                            let _ = app.active(&next);
                        }
                        Msg::FocusPrev => {
                            let prev = self.focus.focus_prev();
                            let _ = app.active(&prev);
                        }
                        Msg::SwitchToCodePreview => {
                            self.exit_reason = Some(ExitReason::SwitchToCodePreview);
                            return Ok(());
                        }
                        Msg::ToggleDarkLight => {
                            needs_full_sync = true;
                        }
                        _ => {}
                    }

                    // Check for quit
                    if matches!(msg, Msg::Quit) {
                        self.exit_reason = Some(ExitReason::Quit);
                        return Ok(());
                    }

                    // Process through model, handle chained messages
                    let mut current = Some(msg);
                    while let Some(m) = current {
                        // Track if regeneration happened
                        if matches!(m, Msg::Regenerate) {
                            needs_sync = true;
                        }
                        current = model.update(Some(m));
                    }
                }

                // Sync components after changes
                if needs_full_sync {
                    Self::sync_all_components(app, model);
                    // Restore focus after remounting
                    let _ = app.active(&self.focus.current_focus());
                } else if needs_sync {
                    Self::sync_display_components(app, model);
                }
            }
            Err(_) => {
                // Timeout is fine, just continue
            }
        }

        Ok(())
    }

    fn will_umount(&self) -> Option<&ExitReason> {
        self.exit_reason.as_ref()
    }

    fn on_destroy(&mut self) -> Option<Context> {
        self.app = None;
        self.context.take()
    }
}
