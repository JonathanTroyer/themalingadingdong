//! Interactive TUI for previewing and editing Base24 color palettes.
//!
//! Architecture: Activity-based with tui-realm for components.
//! Each screen (activity) has its own Application instance and message types.

mod activities;
mod activity;
mod components;
mod highlighting;
mod model;
mod snippets;

use std::io::stdout;
use std::sync::LazyLock;

use color_eyre::eyre::Result;
use crossterm_actions::{
    AppEvent, TuiEvent, TuiRealmDispatcher, bind_action, emacs_defaults, keys,
};
use ratatui::{
    Terminal,
    crossterm::ExecutableCommand,
    crossterm::terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    },
    prelude::CrosstermBackend,
};

use crate::cli::Cli;

pub use highlighting::Highlighter;
pub use model::Model;

use activities::Msg;
use activity::{ActivityManager, Context};

// ============================================================================
// Event handling (shared across activities)
// ============================================================================

/// Unified application events - wraps TuiEvent + custom actions.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum AppAction {
    /// Standard TUI events (navigation, input, selection, app)
    Tui(TuiEvent),
    /// Switch to code preview screen
    CodePreview,
    /// Export the current palette to a file
    Export,
    /// Increment value by small step (1)
    ValueIncrementSmall,
    /// Decrement value by small step (1)
    ValueDecrementSmall,
    /// Increment value by large step (5)
    ValueIncrementLarge,
    /// Decrement value by large step (5)
    ValueDecrementLarge,
    /// Toggle between dark and light variants
    ToggleDarkLight,
}

/// Global dispatcher instance - shared by all components.
/// Using LazyLock for zero-cost lazy initialization.
pub static DISPATCHER: LazyLock<TuiRealmDispatcher<AppAction>> = LazyLock::new(|| {
    // Transform emacs defaults to wrap in AppAction::Tui
    let mut config = emacs_defaults().map_actions(AppAction::Tui);

    // Custom app bindings
    bind_action!(
        config,
        AppAction::CodePreview,
        keys::char('c'),
        "View code preview"
    );
    bind_action!(config, AppAction::Export, keys::char('e'), "Export palette");
    bind_action!(
        config,
        AppAction::ValueDecrementSmall,
        keys::char('['),
        "Decrease value"
    );
    bind_action!(
        config,
        AppAction::ValueIncrementSmall,
        keys::char(']'),
        "Increase value"
    );
    bind_action!(
        config,
        AppAction::ValueDecrementLarge,
        keys::char('{'),
        "Decrease value (5x)"
    );
    bind_action!(
        config,
        AppAction::ValueIncrementLarge,
        keys::char('}'),
        "Increase value (5x)"
    );
    bind_action!(
        config,
        AppAction::ToggleDarkLight,
        keys::char('t'),
        "Toggle dark/light variant"
    );

    config.compile();
    TuiRealmDispatcher::new(config)
});

/// Convenience function for components to access the dispatcher.
pub fn dispatcher() -> &'static TuiRealmDispatcher<AppAction> {
    &DISPATCHER
}

/// Handle global application events that are common across all components.
/// Returns Some(Msg) if the action was handled, None otherwise.
pub fn handle_global_app_events(action: &AppAction) -> Option<Msg> {
    match action {
        AppAction::Tui(TuiEvent::App(AppEvent::Quit)) => Some(Msg::Quit),
        AppAction::Tui(TuiEvent::App(AppEvent::Help)) => Some(Msg::ShowHelp),
        AppAction::Tui(TuiEvent::App(AppEvent::Refresh)) => Some(Msg::Regenerate),
        AppAction::CodePreview => Some(Msg::SwitchToCodePreview),
        AppAction::Export => Some(Msg::DoExport),
        AppAction::ToggleDarkLight => Some(Msg::ToggleDarkLight),
        _ => None,
    }
}

// ============================================================================
// TUI entry point
// ============================================================================

/// Run the interactive TUI using activity-based architecture.
pub fn run(cli: &Cli) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    // Initialize model from CLI args
    let mut model = Model::from_cli(cli)?;
    model.regenerate();

    // Create context and activity manager
    let context = Context { model };
    let mut manager = ActivityManager::new(context);

    // Run the activity loop
    let result = manager.run(&mut terminal);

    // Cleanup terminal
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    result
}
