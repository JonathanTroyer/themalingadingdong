//! Interactive TUI for previewing and editing Base24 color palettes.
//!
//! Architecture: tui-realm for components, crossterm-actions for semantic key dispatch.
//! Components handle events via `on()` using the shared dispatcher.

mod app;
mod components;
mod model;
mod msg;

use std::io::stdout;
use std::sync::LazyLock;

use color_eyre::eyre::Result;
use crossterm_actions::{AppEvent, EditingMode, TuiEvent, TuiRealmDispatcher};
use ratatui::{
    Terminal,
    crossterm::ExecutableCommand,
    crossterm::terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    },
    layout::{Constraint, Direction, Layout},
    prelude::CrosstermBackend,
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph},
};
use tuirealm::{PollStrategy, Update};

use crate::cli::Cli;

pub use model::Model;

use app::{FocusManager, create_application, mount_components};
use components::{Palette, Preview, Validation};
use msg::Msg;

// ============================================================================
// Component identifiers (merged from ids.rs)
// ============================================================================

/// Unique identifiers for all components in the TUI.
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
// Event handling (merged from event.rs)
// ============================================================================

/// Custom user events (currently unused, but required by tui-realm).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserEvent {}

/// Global dispatcher instance - shared by all components.
/// Using LazyLock for zero-cost lazy initialization.
pub static DISPATCHER: LazyLock<TuiRealmDispatcher> =
    LazyLock::new(|| TuiRealmDispatcher::with_defaults(EditingMode::Emacs));

/// Convenience function for components to access the dispatcher.
pub fn dispatcher() -> &'static TuiRealmDispatcher {
    &DISPATCHER
}

/// Handle global application events that are common across all components.
/// Returns Some(Msg) if the action was handled, None otherwise.
pub fn handle_global_app_events(action: &TuiEvent) -> Option<Msg> {
    match action {
        TuiEvent::App(AppEvent::Quit) => Some(Msg::Quit),
        TuiEvent::App(AppEvent::Help) => Some(Msg::ShowHelp),
        TuiEvent::App(AppEvent::Refresh) => Some(Msg::Regenerate),
        _ => None,
    }
}

// ============================================================================
// TUI entry point
// ============================================================================

/// Run the interactive TUI using tui-realm.
pub fn run(cli: &Cli) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    // Initialize model from CLI args
    let mut model = Model::from_cli(cli)?;
    model.regenerate();

    // Create tui-realm application
    let mut app = create_application();

    // Mount components
    mount_components(&mut app, &model)?;

    // Main loop
    let result = run_loop(&mut terminal, &mut app, &mut model);

    // Cleanup terminal
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut tuirealm::Application<Id, Msg, UserEvent>,
    model: &mut Model,
) -> Result<()> {
    // Focus manager for Tab/Shift-Tab navigation
    let mut focus = FocusManager::new();

    while !model.quit {
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
                    crate::cli::VariantArg::Auto => "auto",
                    crate::cli::VariantArg::Dark => "dark",
                    crate::cli::VariantArg::Light => "light",
                    crate::cli::VariantArg::Both => "both",
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

            // Right column: Parameters (fixed) + Validation (grows)
            let right_rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(30), Constraint::Min(8)])
                .split(cols[1]);

            // Render components
            app.view(&Id::Palette, frame, left_rows[0]);
            app.view(&Id::Preview, frame, left_rows[1]);

            // Parameters section
            let params_area = right_rows[0];
            let params_block = Block::default().title(" Parameters ").borders(Borders::ALL);
            let params_inner = params_block.inner(params_area);
            frame.render_widget(params_block, params_area);

            // Calculate curve controls height (3 type rows + conditional strength rows)
            let curve_height =
                3 + if model.interpolation.lightness.curve_type.uses_strength() {
                    1
                } else {
                    0
                } + if model.interpolation.chroma.curve_type.uses_strength() {
                    1
                } else {
                    0
                } + if model.interpolation.hue.curve_type.uses_strength() {
                    1
                } else {
                    0
                };

            let param_rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),            // 0: Background picker (J/M/h)
                    Constraint::Length(3),            // 1: Foreground picker (J/M/h)
                    Constraint::Length(1),            // 2: Spacer
                    Constraint::Length(curve_height), // 3: Curve controls (grouped)
                    Constraint::Length(2),            // 4: Weight controls (grouped)
                    Constraint::Length(1),            // 5: Spacer
                    Constraint::Length(5),            // 6: Accent controls (grouped)
                    Constraint::Length(5),            // 7: Extended accent controls (grouped)
                    Constraint::Length(1),            // 8: Spacer
                    Constraint::Length(3),            // 9: Hue overrides
                    Constraint::Min(0),
                ])
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
                .unwrap_or_else(|| "Tab: switch | arrows: adjust | ?: help | q: quit".to_string());
            let status_widget =
                Paragraph::new(status).style(Style::default().add_modifier(Modifier::DIM));
            frame.render_widget(status_widget, main_rows[2]);
        })?;

        // Use tick() - the canonical tui-realm heartbeat
        // This polls events AND forwards to active component automatically
        match app.tick(PollStrategy::Once) {
            Ok(messages) => {
                let mut needs_sync = false;

                for msg in messages {
                    // Handle focus changes at event loop level
                    match &msg {
                        Msg::FocusNext => {
                            let next = focus.focus_next();
                            let _ = app.active(&next);
                        }
                        Msg::FocusPrev => {
                            let prev = focus.focus_prev();
                            let _ = app.active(&prev);
                        }
                        _ => {}
                    }

                    // Process through model, handle chained messages
                    let mut current = Some(msg);
                    while let Some(m) = current {
                        // Track if regeneration happened (chain ends with Regenerate)
                        if matches!(m, Msg::Regenerate) {
                            needs_sync = true;
                        }
                        current = model.update(Some(m));
                    }
                }

                // Sync display components after regeneration
                if needs_sync {
                    sync_display_components(app, model, &focus);
                }
            }
            Err(_) => {
                // Timeout is fine, just continue
            }
        }
    }

    Ok(())
}

/// Sync display-only components (Palette, Preview, Validation) with current model data.
/// Does NOT remount interactive components to preserve their internal state.
fn sync_display_components(
    app: &mut tuirealm::Application<Id, Msg, UserEvent>,
    model: &Model,
    _focus: &FocusManager,
) {
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

    // NOTE: We intentionally do NOT remount interactive components (HellwigPicker, sliders, etc.)
    // to preserve their internal state (sub_focus, etc.). The display updates via regenerate()
    // and pickers read their own values which stay in sync.
}
