//! Interactive TUI for previewing and editing Base24 color palettes.

mod input;
mod state;
mod ui;
mod widgets;

use std::io::{self, stdout};

use color_eyre::eyre::Result;
use ratatui::{
    Terminal,
    crossterm::{
        ExecutableCommand,
        event::{self, Event, KeyEventKind},
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    prelude::CrosstermBackend,
};

use crate::cli::Cli;

pub use state::TuiState;

use input::EventHandler;
use ui::draw;

/// Run the interactive TUI.
pub fn run(cli: &Cli) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    // Initialize state from CLI args
    let mut state = TuiState::from_cli(cli)?;
    state.regenerate();

    // Event handler with crossterm-actions
    let event_handler = EventHandler::new();

    // Main loop
    let result = run_loop(&mut terminal, &mut state, &event_handler);

    // Cleanup terminal
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut TuiState,
    event_handler: &EventHandler,
) -> Result<()> {
    loop {
        // Draw UI
        terminal.draw(|frame| draw(frame, state))?;

        // Handle events
        if event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            // Only handle key press events, not releases
            if key.kind == KeyEventKind::Press
                && let Some(action) = event_handler.handle(key, state)
            {
                match action {
                    input::Action::Quit => break,
                    input::Action::Regenerate => state.regenerate(),
                    input::Action::Export => state.export()?,
                    input::Action::None => {}
                }
            }
        }
    }

    Ok(())
}
