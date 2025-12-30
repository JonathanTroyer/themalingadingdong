//! Hue overrides grid Component with 8 hue sliders.

use crossterm_actions::{AppEvent, InputEvent, NavigationEvent, SelectionEvent, TuiEvent};
use ratatui::Frame;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use tuirealm::{
    Component, Event, MockComponent, State, StateValue,
    command::{Cmd, CmdResult, Direction as CmdDirection},
    props::{AttrValue, Attribute, Props},
};

use crate::tui::event::{UserEvent, dispatcher};
use crate::tui::msg::Msg;

/// Default hues for accent colors (degrees).
const DEFAULT_HUES: [f32; 8] = [25.0, 55.0, 90.0, 145.0, 180.0, 250.0, 285.0, 335.0];

/// Hue color names.
const HUE_NAMES: [&str; 8] = [
    "Red", "Orange", "Yellow", "Green", "Cyan", "Blue", "Purple", "Magenta",
];

/// Hue overrides grid with 8 hue sliders in 2 rows of 4.
pub struct HueGrid {
    props: Props,
    hues: [Option<f32>; 8],
    selected: usize,
    /// Whether currently editing the selected hue value
    editing: bool,
    /// Buffer for typed input during editing
    edit_buffer: String,
}

impl HueGrid {
    pub fn new(hues: [Option<f32>; 8]) -> Self {
        Self {
            props: Props::default(),
            hues,
            selected: 0,
            editing: false,
            edit_buffer: String::new(),
        }
    }

    fn start_editing(&mut self) {
        self.editing = true;
        // Initialize buffer with current value
        let current = self.hues[self.selected].unwrap_or(DEFAULT_HUES[self.selected]);
        self.edit_buffer = format!("{:.0}", current);
    }

    fn cancel_editing(&mut self) {
        self.editing = false;
        self.edit_buffer.clear();
    }

    fn confirm_editing(&mut self) -> bool {
        self.editing = false;
        if let Ok(value) = self.edit_buffer.parse::<f32>() {
            let normalized = value.rem_euclid(360.0);
            self.hues[self.selected] = Some(normalized);
            self.edit_buffer.clear();
            true
        } else {
            self.edit_buffer.clear();
            false
        }
    }

    fn type_char(&mut self, c: char) {
        // Only allow digits and decimal point
        if c.is_ascii_digit() || (c == '.' && !self.edit_buffer.contains('.')) {
            self.edit_buffer.push(c);
        }
    }

    fn delete_char(&mut self) {
        self.edit_buffer.pop();
    }

    fn adjust_current(&mut self, delta: f64) {
        let current = self.hues[self.selected].unwrap_or(DEFAULT_HUES[self.selected]);
        let new_val = (current + delta as f32).rem_euclid(360.0);
        self.hues[self.selected] = Some(new_val);
    }

    fn draw_hue_input(&self, frame: &mut Frame, area: Rect, index: usize, focused: bool) {
        let name = HUE_NAMES[index];
        let is_override = self.hues[index].is_some();
        let is_editing_this = self.editing && self.selected == index;

        if is_editing_this {
            // Show edit buffer with cursor
            let prefix = format!("{}:", &name[..3]);
            let line = Line::from(vec![
                Span::styled(&prefix, Style::default().fg(Color::Cyan)),
                Span::styled(
                    &self.edit_buffer,
                    Style::default().fg(Color::White).bg(Color::DarkGray),
                ),
                Span::styled("°", Style::default().fg(Color::Cyan)),
            ]);
            let para = Paragraph::new(line);
            frame.render_widget(para, area);
        } else {
            let value = self.hues[index].unwrap_or(DEFAULT_HUES[index]);
            let style = if focused {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if is_override {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            // Compact format: "Red:25°"
            let text = format!("{}:{:.0}°", &name[..3], value);
            let para = Paragraph::new(text).style(style);
            frame.render_widget(para, area);
        }
    }
}

impl MockComponent for HueGrid {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let focused = self
            .props
            .get_or(Attribute::Focus, AttrValue::Flag(false))
            .unwrap_flag();

        // 3 rows: header + 2 rows of 4 hues
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Header
                Constraint::Length(1), // Row 1
                Constraint::Length(1), // Row 2
            ])
            .split(area);

        // Header
        let header =
            Paragraph::new("Hue Overrides:").style(Style::default().add_modifier(Modifier::DIM));
        frame.render_widget(header, rows[0]);

        // Row 1 (hues 0-3)
        let cols1 = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
            ])
            .split(rows[1]);

        for (i, col) in cols1.iter().enumerate() {
            self.draw_hue_input(frame, *col, i, focused && self.selected == i);
        }

        // Row 2 (hues 4-7)
        let cols2 = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
            ])
            .split(rows[2]);

        for (i, col) in cols2.iter().enumerate() {
            let idx = i + 4;
            self.draw_hue_input(frame, *col, idx, focused && self.selected == idx);
        }
    }

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.props.get(attr)
    }

    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        self.props.set(attr, value);
    }

    fn state(&self) -> State {
        // Return selected index and current hue value
        State::Tup2((
            StateValue::U8(self.selected as u8),
            StateValue::F64(self.hues[self.selected].unwrap_or(DEFAULT_HUES[self.selected]) as f64),
        ))
    }

    fn perform(&mut self, cmd: Cmd) -> CmdResult {
        match cmd {
            Cmd::Move(CmdDirection::Up) => {
                // Move up a row (subtract 4)
                if self.selected >= 4 {
                    self.selected -= 4;
                }
                CmdResult::None
            }
            Cmd::Move(CmdDirection::Down) => {
                // Move down a row (add 4)
                if self.selected < 4 {
                    self.selected += 4;
                }
                CmdResult::None
            }
            Cmd::Move(CmdDirection::Left) => {
                self.adjust_current(-1.0);
                CmdResult::Changed(self.state())
            }
            Cmd::Move(CmdDirection::Right) => {
                self.adjust_current(1.0);
                CmdResult::Changed(self.state())
            }
            _ => CmdResult::None,
        }
    }
}

impl Component<Msg, UserEvent> for HueGrid {
    fn on(&mut self, ev: Event<UserEvent>) -> Option<Msg> {
        let focused = self
            .props
            .get_or(Attribute::Focus, AttrValue::Flag(false))
            .unwrap_flag();

        if !focused {
            return None;
        }

        // Extract keyboard event
        let Event::Keyboard(key_event) = ev else {
            return None;
        };

        // Handle editing mode separately (raw key input)
        if self.editing {
            match key_event.code {
                tuirealm::event::Key::Enter => {
                    if self.confirm_editing() {
                        return Some(Msg::HueOverrideChanged(
                            self.selected as u8,
                            self.hues[self.selected],
                        ));
                    }
                    return None;
                }
                tuirealm::event::Key::Esc => {
                    self.cancel_editing();
                    return None;
                }
                tuirealm::event::Key::Backspace => {
                    self.delete_char();
                    return None;
                }
                tuirealm::event::Key::Char(c) => {
                    self.type_char(c);
                    return None;
                }
                _ => return None,
            }
        }

        // Check for digit keys to start editing directly
        if let tuirealm::event::Key::Char(c) = key_event.code
            && c.is_ascii_digit()
        {
            // Start editing and add the digit
            self.editing = true;
            self.edit_buffer.clear();
            self.edit_buffer.push(c);
            return None;
        }

        // Use dispatcher to convert to semantic action
        let action = dispatcher().dispatch(&key_event)?;

        match action {
            // Global actions → bubble up as Msg
            TuiEvent::App(AppEvent::Quit) => Some(Msg::Quit),
            TuiEvent::App(AppEvent::Help) => Some(Msg::ShowHelp),
            TuiEvent::App(AppEvent::Refresh) => Some(Msg::Regenerate),

            // Enter starts editing with current value pre-filled
            TuiEvent::Input(InputEvent::Confirm) => {
                self.start_editing();
                None
            }

            // Tab bubbles up for component navigation
            TuiEvent::Selection(SelectionEvent::Next) => Some(Msg::FocusNext),
            TuiEvent::Selection(SelectionEvent::Prev) => Some(Msg::FocusPrev),

            // Grid navigation: arrows move between cells
            TuiEvent::Navigation(NavigationEvent::Up) => {
                if self.selected >= 4 {
                    // Move up a row
                    self.selected -= 4;
                }
                None
            }
            TuiEvent::Navigation(NavigationEvent::Down) => {
                if self.selected < 4 {
                    // Move down a row
                    self.selected += 4;
                }
                None
            }
            TuiEvent::Navigation(NavigationEvent::Left) => {
                // Move to previous cell in row, wrap to previous row
                if self.selected > 0 {
                    self.selected -= 1;
                }
                None
            }
            TuiEvent::Navigation(NavigationEvent::Right) => {
                // Move to next cell in row, wrap to next row
                if self.selected < 7 {
                    self.selected += 1;
                }
                None
            }

            _ => None,
        }
    }
}
