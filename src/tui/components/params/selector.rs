//! Cycle selector MockComponent for enum values.

use crossterm_actions::{AppEvent, NavigationEvent, SelectionEvent, TuiEvent};
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

/// Type of selector (determines which Msg to send on change).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectorType {
    Lightness,
    Chroma,
    Hue,
}

/// A selector that cycles through a list of options.
pub struct CycleSelector {
    props: Props,
    options: Vec<String>,
    selected: usize,
    label: String,
    selector_type: SelectorType,
}

impl CycleSelector {
    pub fn new(
        label: impl Into<String>,
        options: Vec<String>,
        initial: usize,
        selector_type: SelectorType,
    ) -> Self {
        let selected = initial.min(options.len().saturating_sub(1));
        Self {
            props: Props::default(),
            options,
            selected,
            label: label.into(),
            selector_type,
        }
    }

    fn cycle_next(&mut self) {
        if !self.options.is_empty() {
            self.selected = (self.selected + 1) % self.options.len();
        }
    }

    fn cycle_prev(&mut self) {
        if !self.options.is_empty() {
            self.selected = (self.selected + self.options.len() - 1) % self.options.len();
        }
    }
}

impl MockComponent for CycleSelector {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let focused = self
            .props
            .get_or(Attribute::Focus, AttrValue::Flag(false))
            .unwrap_flag();

        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(15), Constraint::Min(10)])
            .split(area);

        // Label
        let label_style = if focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let label_text = Paragraph::new(format!("{}:", self.label)).style(label_style);
        frame.render_widget(label_text, cols[0]);

        // Value with arrows
        let value_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };
        let arrow_style = if focused {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM)
        };

        let current = self.options.get(self.selected).map_or("", String::as_str);
        let line = Line::from(vec![
            Span::styled("◂ ", arrow_style),
            Span::styled(current, value_style),
            Span::styled(" ▸", arrow_style),
        ]);

        let value_para = Paragraph::new(line);
        frame.render_widget(value_para, cols[1]);
    }

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.props.get(attr)
    }

    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        self.props.set(attr, value);
    }

    fn state(&self) -> State {
        State::One(StateValue::Usize(self.selected))
    }

    fn perform(&mut self, cmd: Cmd) -> CmdResult {
        match cmd {
            Cmd::Move(CmdDirection::Left) => {
                self.cycle_prev();
                CmdResult::Changed(self.state())
            }
            Cmd::Move(CmdDirection::Right) => {
                self.cycle_next();
                CmdResult::Changed(self.state())
            }
            _ => CmdResult::None,
        }
    }
}

impl Component<Msg, UserEvent> for CycleSelector {
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

        // Use dispatcher to convert to semantic action
        let action = dispatcher().dispatch(&key_event)?;

        match action {
            // Global actions → bubble up as Msg
            TuiEvent::App(AppEvent::Quit) => Some(Msg::Quit),
            TuiEvent::App(AppEvent::Help) => Some(Msg::ShowHelp),
            TuiEvent::App(AppEvent::Refresh) => Some(Msg::Regenerate),

            // Focus navigation → bubble up as Msg
            TuiEvent::Selection(SelectionEvent::Next) => Some(Msg::FocusNext),
            TuiEvent::Selection(SelectionEvent::Prev) => Some(Msg::FocusPrev),

            // Value adjustment
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

impl CycleSelector {
    fn msg_for_change(&self) -> Option<Msg> {
        let curve = curve_type_from_index(self.selected);
        match self.selector_type {
            SelectorType::Lightness => Some(Msg::LightnessCurveTypeChanged(curve)),
            SelectorType::Chroma => Some(Msg::ChromaCurveTypeChanged(curve)),
            SelectorType::Hue => Some(Msg::HueCurveTypeChanged(curve)),
        }
    }
}

fn curve_type_from_index(idx: usize) -> crate::curves::CurveType {
    use crate::curves::CurveType;
    match idx {
        0 => CurveType::Linear,
        1 => CurveType::Smoothstep,
        2 => CurveType::Smootherstep,
        3 => CurveType::SmoothStart,
        4 => CurveType::SmoothEnd,
        5 => CurveType::Sigmoid,
        _ => CurveType::BSpline,
    }
}
