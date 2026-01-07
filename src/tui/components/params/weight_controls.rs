//! Grouped optimization weight controls component.

use crossterm_actions::{NavigationEvent, SelectionEvent, TuiEvent};
use ratatui::Frame;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use tuirealm::{
    Component, Event, MockComponent, State,
    command::{Cmd, CmdResult, Direction as CmdDirection},
    props::{AttrValue, Attribute, Props},
};

use crate::tui::msg::Msg;
use crate::tui::{UserEvent, dispatcher, handle_global_app_events};

/// Which weight is focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WeightFocus {
    #[default]
    ContrastWeight,
    JWeight,
}

impl WeightFocus {
    fn next(self) -> Self {
        match self {
            Self::ContrastWeight => Self::JWeight,
            Self::JWeight => Self::ContrastWeight,
        }
    }

    fn prev(self) -> Self {
        self.next() // Only 2 options, prev == next
    }
}

/// Values for weight controls.
#[derive(Debug, Clone, Copy)]
pub struct WeightValues {
    /// Contrast weight (Lc), range 0-1
    pub contrast_weight: f32,
    /// Uniformity weight (J'), range 0-1
    pub j_weight: f32,
}

/// Grouped optimization weight controls with sub-focus navigation.
pub struct WeightControls {
    props: Props,
    values: WeightValues,
    sub_focus: WeightFocus,
}

impl WeightControls {
    pub fn new(values: WeightValues) -> Self {
        Self {
            props: Props::default(),
            values,
            sub_focus: WeightFocus::ContrastWeight,
        }
    }

    fn adjust_current(&mut self, delta: f32) {
        match self.sub_focus {
            WeightFocus::ContrastWeight => {
                self.values.contrast_weight = (self.values.contrast_weight + delta).clamp(0.0, 1.0);
            }
            WeightFocus::JWeight => {
                self.values.j_weight = (self.values.j_weight + delta).clamp(0.0, 1.0);
            }
        }
    }

    fn draw_slider(&self, frame: &mut Frame, area: Rect, label: &str, value: f32, focused: bool) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(15), Constraint::Min(10)])
            .split(area);

        let label_style = if focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let label_text = Paragraph::new(format!("{}:", label)).style(label_style);
        frame.render_widget(label_text, cols[0]);

        let slider_width = cols[1].width.saturating_sub(8) as usize;
        let pos = (f64::from(value) * slider_width as f64).round() as usize;
        let pos = pos.min(slider_width.saturating_sub(1));

        let (filled_style, empty_style, handle_style) = if focused {
            (
                Style::default().fg(Color::Cyan),
                Style::default().fg(Color::DarkGray),
                Style::default().fg(Color::White),
            )
        } else {
            (
                Style::default().fg(Color::DarkGray),
                Style::default().fg(Color::DarkGray),
                Style::default().fg(Color::Gray),
            )
        };

        let mut spans = Vec::new();
        for i in 0..slider_width {
            if i == pos {
                spans.push(Span::styled("●", handle_style));
            } else if i < pos {
                spans.push(Span::styled("━", filled_style));
            } else {
                spans.push(Span::styled("─", empty_style));
            }
        }

        let value_str = format!(" {:.2}", value);
        spans.push(Span::styled(
            value_str,
            if focused {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            },
        ));

        let slider_line = Paragraph::new(Line::from(spans));
        frame.render_widget(slider_line, cols[1]);
    }
}

impl MockComponent for WeightControls {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let focused = self
            .props
            .get_or(Attribute::Focus, AttrValue::Flag(false))
            .unwrap_flag();

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Lc Weight
                Constraint::Length(1), // J Weight
            ])
            .split(area);

        self.draw_slider(
            frame,
            rows[0],
            "Lc Weight",
            self.values.contrast_weight,
            focused && self.sub_focus == WeightFocus::ContrastWeight,
        );

        self.draw_slider(
            frame,
            rows[1],
            "J Weight",
            self.values.j_weight,
            focused && self.sub_focus == WeightFocus::JWeight,
        );
    }

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.props.get(attr)
    }

    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        self.props.set(attr, value);
    }

    fn state(&self) -> State {
        State::None
    }

    fn perform(&mut self, cmd: Cmd) -> CmdResult {
        match cmd {
            Cmd::Move(CmdDirection::Up) => {
                self.sub_focus = self.sub_focus.prev();
                CmdResult::None
            }
            Cmd::Move(CmdDirection::Down) => {
                self.sub_focus = self.sub_focus.next();
                CmdResult::None
            }
            Cmd::Move(CmdDirection::Left) => {
                self.adjust_current(-0.05);
                CmdResult::Changed(self.state())
            }
            Cmd::Move(CmdDirection::Right) => {
                self.adjust_current(0.05);
                CmdResult::Changed(self.state())
            }
            _ => CmdResult::None,
        }
    }
}

impl Component<Msg, UserEvent> for WeightControls {
    fn on(&mut self, ev: Event<UserEvent>) -> Option<Msg> {
        let focused = self
            .props
            .get_or(Attribute::Focus, AttrValue::Flag(false))
            .unwrap_flag();

        if !focused {
            return None;
        }

        let Event::Keyboard(key_event) = ev else {
            return None;
        };

        let action = dispatcher().dispatch(&key_event)?;

        if let Some(msg) = handle_global_app_events(&action) {
            return Some(msg);
        }

        match action {
            TuiEvent::Selection(SelectionEvent::Next) => Some(Msg::FocusNext),
            TuiEvent::Selection(SelectionEvent::Prev) => Some(Msg::FocusPrev),
            TuiEvent::Navigation(NavigationEvent::Up) => {
                self.perform(Cmd::Move(CmdDirection::Up));
                None
            }
            TuiEvent::Navigation(NavigationEvent::Down) => {
                self.perform(Cmd::Move(CmdDirection::Down));
                None
            }
            TuiEvent::Navigation(NavigationEvent::Left) => {
                if let CmdResult::Changed(_) = self.perform(Cmd::Move(CmdDirection::Left)) {
                    self.msg_for_change()
                } else {
                    None
                }
            }
            TuiEvent::Navigation(NavigationEvent::Right) => {
                if let CmdResult::Changed(_) = self.perform(Cmd::Move(CmdDirection::Right)) {
                    self.msg_for_change()
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl WeightControls {
    fn msg_for_change(&self) -> Option<Msg> {
        match self.sub_focus {
            WeightFocus::ContrastWeight => {
                Some(Msg::ContrastWeightChanged(self.values.contrast_weight))
            }
            WeightFocus::JWeight => Some(Msg::JWeightChanged(self.values.j_weight)),
        }
    }
}
