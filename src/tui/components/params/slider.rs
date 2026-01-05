//! Reusable slider MockComponent.

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

/// Type of slider (determines which Msg to send on change).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliderType {
    MinContrast,
    ExtendedMinContrast,
    AccentColorfulness,
    ExtendedColorfulness,
    LightnessStrength,
    ChromaStrength,
    HueStrength,
}

/// Configuration for a slider.
#[derive(Debug, Clone)]
pub struct SliderConfig {
    pub label: String,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub precision: usize,
    pub suffix: String,
    pub slider_type: SliderType,
}

impl Default for SliderConfig {
    fn default() -> Self {
        Self {
            label: String::new(),
            min: 0.0,
            max: 100.0,
            step: 1.0,
            precision: 2,
            suffix: String::new(),
            slider_type: SliderType::MinContrast,
        }
    }
}

/// A horizontal slider with label and value display.
pub struct Slider {
    props: Props,
    value: f64,
    config: SliderConfig,
}

impl Slider {
    pub fn new(config: SliderConfig, initial_value: f64) -> Self {
        Self {
            props: Props::default(),
            value: initial_value.clamp(config.min, config.max),
            config,
        }
    }

    fn adjust(&mut self, delta: f64) {
        self.value = (self.value + delta).clamp(self.config.min, self.config.max);
    }
}

impl MockComponent for Slider {
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
        let label_text = Paragraph::new(format!("{}:", self.config.label)).style(label_style);
        frame.render_widget(label_text, cols[0]);

        // Slider
        let slider_width = cols[1].width.saturating_sub(8) as usize;
        let ratio = (self.value - self.config.min) / (self.config.max - self.config.min);
        let pos = (ratio * slider_width as f64).round() as usize;
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

        let value_str = format!(
            " {:.*}{}",
            self.config.precision, self.value, self.config.suffix
        );
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

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.props.get(attr)
    }

    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        self.props.set(attr, value);
    }

    fn state(&self) -> State {
        State::One(StateValue::F64(self.value))
    }

    fn perform(&mut self, cmd: Cmd) -> CmdResult {
        match cmd {
            Cmd::Move(CmdDirection::Left) => {
                self.adjust(-self.config.step);
                CmdResult::Changed(self.state())
            }
            Cmd::Move(CmdDirection::Right) => {
                self.adjust(self.config.step);
                CmdResult::Changed(self.state())
            }
            _ => CmdResult::None,
        }
    }
}

impl Component<Msg, UserEvent> for Slider {
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

impl Slider {
    fn msg_for_change(&self) -> Option<Msg> {
        match self.config.slider_type {
            SliderType::MinContrast => Some(Msg::MinContrastChanged(self.value)),
            SliderType::ExtendedMinContrast => Some(Msg::ExtendedMinContrastChanged(self.value)),
            SliderType::AccentColorfulness => {
                Some(Msg::AccentColorfulnessChanged(self.value as f32))
            }
            SliderType::ExtendedColorfulness => {
                Some(Msg::ExtendedColorfulnessChanged(self.value as f32))
            }
            SliderType::LightnessStrength => {
                Some(Msg::LightnessCurveStrengthChanged(self.value as f32))
            }
            SliderType::ChromaStrength => Some(Msg::ChromaCurveStrengthChanged(self.value as f32)),
            SliderType::HueStrength => Some(Msg::HueCurveStrengthChanged(self.value as f32)),
        }
    }
}
