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
    ActionBinding, ActionConfig, AppEvent, EditingMode, TuiEvent, TuiRealmDispatcher, action,
    defaults, keys,
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

pub use highlighting::{Highlighter, SYNTAX_SET};
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
    /// Increment value by small step (1)
    ValueIncrementSmall,
    /// Decrement value by small step (1)
    ValueDecrementSmall,
    /// Increment value by large step (5)
    ValueIncrementLarge,
    /// Decrement value by large step (5)
    ValueDecrementLarge,
}

/// Global dispatcher instance - shared by all components.
/// Using LazyLock for zero-cost lazy initialization.
pub static DISPATCHER: LazyLock<TuiRealmDispatcher<AppAction>> = LazyLock::new(|| {
    let mut config = ActionConfig::new(EditingMode::Emacs);

    // Import all standard TuiEvent bindings wrapped in AppAction::Tui
    for binding in defaults::emacs_defaults().bindings() {
        config.bind(ActionBinding {
            action: AppAction::Tui(binding.action),
            keys: binding.keys.clone(),
            description: binding.description.clone(),
        });
    }

    // Add custom bindings
    config.bind(
        action(AppAction::CodePreview)
            .binding(keys::char('c'))
            .description("View code preview")
            .build(),
    );

    // Value adjustment bindings: [/] for small steps, {/} for large steps
    config.bind(
        action(AppAction::ValueDecrementSmall)
            .binding(keys::char('['))
            .description("Decrease value")
            .build(),
    );
    config.bind(
        action(AppAction::ValueIncrementSmall)
            .binding(keys::char(']'))
            .description("Increase value")
            .build(),
    );
    config.bind(
        action(AppAction::ValueDecrementLarge)
            .binding(keys::char('{'))
            .description("Decrease value (5x)")
            .build(),
    );
    config.bind(
        action(AppAction::ValueIncrementLarge)
            .binding(keys::char('}'))
            .description("Increase value (5x)")
            .build(),
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
