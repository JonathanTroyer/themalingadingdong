//! Event handling with crossterm-actions integration.

use crossterm_actions::{
    AppEvent, EditingMode, EventDispatcher, InputEvent, NavigationEvent, SelectionEvent, TuiEvent,
};
use ratatui::crossterm::event::{KeyCode, KeyEvent};

use super::state::TuiState;

/// Actions that can be returned from event handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Quit the application.
    Quit,
    /// Regenerate the palette.
    Regenerate,
    /// Export the scheme to file.
    Export,
    /// No action needed.
    None,
}

/// Event handler using crossterm-actions defaults.
pub struct EventHandler {
    dispatcher: EventDispatcher,
}

impl EventHandler {
    /// Create a new event handler with default Emacs keybindings.
    pub fn new() -> Self {
        let dispatcher = EventDispatcher::with_defaults(EditingMode::Emacs);
        Self { dispatcher }
    }

    /// Handle a key event and return the resulting action.
    pub fn handle(&self, key: KeyEvent, state: &mut TuiState) -> Option<Action> {
        // If showing help overlay, any key closes it
        if state.show_help {
            state.show_help = false;
            return Some(Action::None);
        }

        // If showing export dialog, handle it specially
        if state.show_export {
            return self.handle_export_dialog(key, state);
        }

        // If editing text, handle text input
        if state.editing_text {
            return self.handle_text_input(key, state);
        }

        // Use crossterm-actions dispatcher for navigation
        if let Some(tui_event) = self.dispatcher.dispatch(&key) {
            return self.handle_tui_event(tui_event, state);
        }

        // Handle character input for text fields when not in edit mode
        if state.is_text_field()
            && let KeyCode::Char(_) = key.code
        {
            // Enter text editing mode
            state.editing_text = true;
            state.text_cursor = state.focused_text().map(|s| s.len()).unwrap_or(0);
            return self.handle_text_input(key, state);
        }

        Some(Action::None)
    }

    fn handle_tui_event(&self, event: TuiEvent, state: &mut TuiState) -> Option<Action> {
        match event {
            TuiEvent::App(AppEvent::Quit) => Some(Action::Quit),
            TuiEvent::App(AppEvent::Help) => {
                state.show_help = !state.show_help;
                Some(Action::None)
            }
            TuiEvent::App(AppEvent::Refresh) => Some(Action::Regenerate),

            TuiEvent::Navigation(NavigationEvent::Down) => {
                state.focus = state.focus.next();
                state.editing_text = false;
                Some(Action::None)
            }
            TuiEvent::Navigation(NavigationEvent::Up) => {
                state.focus = state.focus.prev();
                state.editing_text = false;
                Some(Action::None)
            }
            TuiEvent::Navigation(NavigationEvent::Left) => {
                if state.is_variant_field() {
                    state.cycle_variant(false);
                    Some(Action::Regenerate)
                } else if !state.is_text_field() {
                    state.adjust_value(-1.0);
                    Some(Action::Regenerate)
                } else {
                    Some(Action::None)
                }
            }
            TuiEvent::Navigation(NavigationEvent::Right) => {
                if state.is_variant_field() {
                    state.cycle_variant(true);
                    Some(Action::Regenerate)
                } else if !state.is_text_field() {
                    state.adjust_value(1.0);
                    Some(Action::Regenerate)
                } else {
                    Some(Action::None)
                }
            }

            TuiEvent::Selection(SelectionEvent::Next) => {
                state.focus = state.focus.next();
                state.editing_text = false;
                Some(Action::None)
            }
            TuiEvent::Selection(SelectionEvent::Prev) => {
                state.focus = state.focus.prev();
                state.editing_text = false;
                Some(Action::None)
            }

            TuiEvent::Input(InputEvent::Confirm) => {
                if state.is_text_field() {
                    if state.editing_text {
                        // Confirm text entry and regenerate
                        state.editing_text = false;
                        Some(Action::Regenerate)
                    } else {
                        // Enter text editing mode
                        state.editing_text = true;
                        state.text_cursor = state.focused_text().map(|s| s.len()).unwrap_or(0);
                        Some(Action::None)
                    }
                } else {
                    state.show_export = true;
                    Some(Action::None)
                }
            }
            TuiEvent::Input(InputEvent::Cancel) => {
                if state.editing_text {
                    state.editing_text = false;
                }
                Some(Action::None)
            }

            _ => Some(Action::None),
        }
    }

    fn handle_text_input(&self, key: KeyEvent, state: &mut TuiState) -> Option<Action> {
        match key.code {
            KeyCode::Char(c) => {
                state.insert_char(c);
                Some(Action::None)
            }
            KeyCode::Backspace => {
                state.delete_char_before();
                Some(Action::None)
            }
            KeyCode::Delete => {
                state.delete_char_at();
                Some(Action::None)
            }
            KeyCode::Left => {
                if state.text_cursor > 0 {
                    state.text_cursor -= 1;
                }
                Some(Action::None)
            }
            KeyCode::Right => {
                let len = state.focused_text().map(|s| s.len()).unwrap_or(0);
                if state.text_cursor < len {
                    state.text_cursor += 1;
                }
                Some(Action::None)
            }
            KeyCode::Home => {
                state.text_cursor = 0;
                Some(Action::None)
            }
            KeyCode::End => {
                state.text_cursor = state.focused_text().map(|s| s.len()).unwrap_or(0);
                Some(Action::None)
            }
            KeyCode::Enter => {
                state.editing_text = false;
                Some(Action::Regenerate)
            }
            KeyCode::Esc => {
                state.editing_text = false;
                Some(Action::None)
            }
            KeyCode::Tab => {
                state.editing_text = false;
                state.focus = state.focus.next();
                Some(Action::Regenerate)
            }
            _ => Some(Action::None),
        }
    }

    fn handle_export_dialog(&self, key: KeyEvent, state: &mut TuiState) -> Option<Action> {
        match key.code {
            KeyCode::Char(c) => {
                state.export_path.push(c);
                Some(Action::None)
            }
            KeyCode::Backspace => {
                state.export_path.pop();
                Some(Action::None)
            }
            KeyCode::Enter => Some(Action::Export),
            KeyCode::Esc => {
                state.show_export = false;
                Some(Action::None)
            }
            _ => Some(Action::None),
        }
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}
