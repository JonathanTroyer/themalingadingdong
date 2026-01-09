//! Grouped accent color controls component.

use crate::tui::AppAction;
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

use crate::tui::activities::{Msg, main::UserEvent};
use crate::tui::{dispatcher, handle_global_app_events};

/// Which control is focused within the accent group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccentFocus {
    #[default]
    MinContrast,
    TargetJ,
    DeltaJ,
    TargetM,
    DeltaM,
}

impl AccentFocus {
    fn next(self) -> Self {
        match self {
            Self::MinContrast => Self::TargetJ,
            Self::TargetJ => Self::DeltaJ,
            Self::DeltaJ => Self::TargetM,
            Self::TargetM => Self::DeltaM,
            Self::DeltaM => Self::MinContrast,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::MinContrast => Self::DeltaM,
            Self::TargetJ => Self::MinContrast,
            Self::DeltaJ => Self::TargetJ,
            Self::TargetM => Self::DeltaJ,
            Self::DeltaM => Self::TargetM,
        }
    }
}

/// Values for accent color controls.
#[derive(Debug, Clone, Copy)]
pub struct AccentValues {
    /// Minimum APCA contrast (Lc), range 30-90
    pub min_contrast: f64,
    /// Target lightness (J'), range 20-95
    pub target_j: f32,
    /// Lightness tolerance (delta J'), range 1-30
    pub delta_j: f32,
    /// Target colorfulness (M), range 5-50
    pub target_m: f32,
    /// Colorfulness tolerance (delta M), range 1-25
    pub delta_m: f32,
}

/// Parameters for drawing a slider.
struct SliderParams<'a> {
    area: Rect,
    label: &'a str,
    value: f64,
    min: f64,
    max: f64,
    focused: bool,
    precision: usize,
}

/// Type of accent controls (base accents or extended accents).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccentControlsType {
    /// Base accent colors (base08-base0F)
    Base,
    /// Extended accent colors (base10-base17)
    Extended,
}

/// Grouped accent color controls with sub-focus navigation.
pub struct AccentControls {
    props: Props,
    controls_type: AccentControlsType,
    values: AccentValues,
    sub_focus: AccentFocus,
}

impl AccentControls {
    pub fn new(controls_type: AccentControlsType, values: AccentValues) -> Self {
        Self {
            props: Props::default(),
            controls_type,
            values,
            sub_focus: AccentFocus::MinContrast,
        }
    }

    fn label_prefix(&self) -> &'static str {
        match self.controls_type {
            AccentControlsType::Base => "Accent",
            AccentControlsType::Extended => "Extended",
        }
    }

    fn adjust_current(&mut self, delta: f64) {
        match self.sub_focus {
            AccentFocus::MinContrast => {
                self.values.min_contrast = (self.values.min_contrast + delta).clamp(30.0, 90.0);
            }
            AccentFocus::TargetJ => {
                self.values.target_j = (self.values.target_j + delta as f32).clamp(20.0, 95.0);
            }
            AccentFocus::DeltaJ => {
                self.values.delta_j = (self.values.delta_j + delta as f32).clamp(1.0, 30.0);
            }
            AccentFocus::TargetM => {
                self.values.target_m = (self.values.target_m + delta as f32).clamp(5.0, 50.0);
            }
            AccentFocus::DeltaM => {
                self.values.delta_m = (self.values.delta_m + delta as f32).clamp(1.0, 25.0);
            }
        }
    }

    fn draw_slider(&self, frame: &mut Frame, params: SliderParams) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(17), Constraint::Min(10)])
            .split(params.area);

        let label_style = if params.focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let label_text = Paragraph::new(format!("{}:", params.label)).style(label_style);
        frame.render_widget(label_text, cols[0]);

        let slider_width = cols[1].width.saturating_sub(8) as usize;
        let ratio = (params.value - params.min) / (params.max - params.min);
        let pos = (ratio * slider_width as f64).round() as usize;
        let pos = pos.min(slider_width.saturating_sub(1));

        let (filled_style, empty_style, handle_style) = if params.focused {
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

        let value_str = if params.precision == 0 {
            format!(" {:.0}", params.value)
        } else {
            format!(" {:.prec$}", params.value, prec = params.precision)
        };
        spans.push(Span::styled(
            value_str,
            if params.focused {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            },
        ));

        let slider_line = Paragraph::new(Line::from(spans));
        frame.render_widget(slider_line, cols[1]);
    }
}

impl MockComponent for AccentControls {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let focused = self
            .props
            .get_or(Attribute::Focus, AttrValue::Flag(false))
            .unwrap_flag();

        let prefix = self.label_prefix();

        // Split into 5 rows for all controls
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Min contrast
                Constraint::Length(1), // Target lightness
                Constraint::Length(1), // Delta lightness
                Constraint::Length(1), // Target colorfulness
                Constraint::Length(1), // Delta colorfulness
            ])
            .split(area);

        self.draw_slider(
            frame,
            SliderParams {
                area: rows[0],
                label: &format!("{} Lc", prefix),
                value: self.values.min_contrast,
                min: 30.0,
                max: 90.0,
                focused: focused && self.sub_focus == AccentFocus::MinContrast,
                precision: 0,
            },
        );

        self.draw_slider(
            frame,
            SliderParams {
                area: rows[1],
                label: "  Tgt Lightness",
                value: f64::from(self.values.target_j),
                min: 20.0,
                max: 95.0,
                focused: focused && self.sub_focus == AccentFocus::TargetJ,
                precision: 0,
            },
        );

        self.draw_slider(
            frame,
            SliderParams {
                area: rows[2],
                label: "  Δ Lightness",
                value: f64::from(self.values.delta_j),
                min: 1.0,
                max: 30.0,
                focused: focused && self.sub_focus == AccentFocus::DeltaJ,
                precision: 0,
            },
        );

        self.draw_slider(
            frame,
            SliderParams {
                area: rows[3],
                label: "  Tgt Colorful",
                value: f64::from(self.values.target_m),
                min: 5.0,
                max: 50.0,
                focused: focused && self.sub_focus == AccentFocus::TargetM,
                precision: 0,
            },
        );

        self.draw_slider(
            frame,
            SliderParams {
                area: rows[4],
                label: "  Δ Colorful",
                value: f64::from(self.values.delta_m),
                min: 1.0,
                max: 25.0,
                focused: focused && self.sub_focus == AccentFocus::DeltaM,
                precision: 0,
            },
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

impl Component<Msg, UserEvent> for AccentControls {
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
            AppAction::Tui(TuiEvent::Selection(SelectionEvent::Next)) => Some(Msg::FocusNext),
            AppAction::Tui(TuiEvent::Selection(SelectionEvent::Prev)) => Some(Msg::FocusPrev),
            AppAction::Tui(TuiEvent::Navigation(NavigationEvent::Up)) => {
                self.perform(Cmd::Move(CmdDirection::Up));
                None
            }
            AppAction::Tui(TuiEvent::Navigation(NavigationEvent::Down)) => {
                self.perform(Cmd::Move(CmdDirection::Down));
                None
            }
            AppAction::Tui(TuiEvent::Navigation(NavigationEvent::Left)) => {
                if let CmdResult::Changed(_) = self.perform(Cmd::Move(CmdDirection::Left)) {
                    self.msg_for_change()
                } else {
                    None
                }
            }
            AppAction::Tui(TuiEvent::Navigation(NavigationEvent::Right)) => {
                if let CmdResult::Changed(_) = self.perform(Cmd::Move(CmdDirection::Right)) {
                    self.msg_for_change()
                } else {
                    None
                }
            }

            // Value adjustment: [/] for ±1, {/} for ±5
            AppAction::ValueDecrementSmall => {
                self.adjust_current(-1.0);
                self.msg_for_change()
            }
            AppAction::ValueIncrementSmall => {
                self.adjust_current(1.0);
                self.msg_for_change()
            }
            AppAction::ValueDecrementLarge => {
                self.adjust_current(-5.0);
                self.msg_for_change()
            }
            AppAction::ValueIncrementLarge => {
                self.adjust_current(5.0);
                self.msg_for_change()
            }

            _ => None,
        }
    }
}

impl AccentControls {
    fn msg_for_change(&self) -> Option<Msg> {
        match self.controls_type {
            AccentControlsType::Base => match self.sub_focus {
                AccentFocus::MinContrast => Some(Msg::MinContrastChanged(self.values.min_contrast)),
                AccentFocus::TargetJ => Some(Msg::AccentTargetJChanged(self.values.target_j)),
                AccentFocus::DeltaJ => Some(Msg::AccentDeltaJChanged(self.values.delta_j)),
                AccentFocus::TargetM => Some(Msg::AccentTargetMChanged(self.values.target_m)),
                AccentFocus::DeltaM => Some(Msg::AccentDeltaMChanged(self.values.delta_m)),
            },
            AccentControlsType::Extended => match self.sub_focus {
                AccentFocus::MinContrast => {
                    Some(Msg::ExtendedMinContrastChanged(self.values.min_contrast))
                }
                AccentFocus::TargetJ => Some(Msg::ExtendedTargetJChanged(self.values.target_j)),
                AccentFocus::DeltaJ => Some(Msg::ExtendedDeltaJChanged(self.values.delta_j)),
                AccentFocus::TargetM => Some(Msg::ExtendedTargetMChanged(self.values.target_m)),
                AccentFocus::DeltaM => Some(Msg::ExtendedDeltaMChanged(self.values.delta_m)),
            },
        }
    }
}
