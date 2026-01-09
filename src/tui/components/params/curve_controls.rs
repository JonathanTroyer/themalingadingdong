//! Grouped curve controls component for lightness/colorfulness/hue interpolation.

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

use crate::curves::CurveType;
use crate::tui::activities::{Msg, main::UserEvent};
use crate::tui::{dispatcher, handle_global_app_events};

/// Which curve control is focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CurveFocus {
    #[default]
    JType,
    JStrength,
    MType,
    MStrength,
    HType,
    HStrength,
}

impl CurveFocus {
    /// Whether this focus is a strength slider.
    fn is_strength(self) -> bool {
        matches!(self, Self::JStrength | Self::MStrength | Self::HStrength)
    }
}

/// Values for curve controls.
#[derive(Debug, Clone, Copy)]
pub struct CurveValues {
    pub j_type: CurveType,
    pub j_strength: f32,
    pub m_type: CurveType,
    pub m_strength: f32,
    pub h_type: CurveType,
    pub h_strength: f32,
}

/// Grouped curve controls with sub-focus navigation.
pub struct CurveControls {
    props: Props,
    values: CurveValues,
    sub_focus: CurveFocus,
}

impl CurveControls {
    pub fn new(values: CurveValues) -> Self {
        Self {
            props: Props::default(),
            values,
            sub_focus: CurveFocus::JType,
        }
    }

    /// Check if a strength focus should be visible.
    fn is_strength_visible(&self, focus: CurveFocus) -> bool {
        match focus {
            CurveFocus::JStrength => self.values.j_type.uses_strength(),
            CurveFocus::MStrength => self.values.m_type.uses_strength(),
            CurveFocus::HStrength => self.values.h_type.uses_strength(),
            _ => true,
        }
    }

    /// Move to next visible focus.
    fn focus_next(&mut self) {
        let order = [
            CurveFocus::JType,
            CurveFocus::JStrength,
            CurveFocus::MType,
            CurveFocus::MStrength,
            CurveFocus::HType,
            CurveFocus::HStrength,
        ];
        let current_idx = order.iter().position(|&f| f == self.sub_focus).unwrap_or(0);

        for i in 1..=order.len() {
            let next_idx = (current_idx + i) % order.len();
            let candidate = order[next_idx];
            if self.is_strength_visible(candidate) {
                self.sub_focus = candidate;
                return;
            }
        }
    }

    /// Move to previous visible focus.
    fn focus_prev(&mut self) {
        let order = [
            CurveFocus::JType,
            CurveFocus::JStrength,
            CurveFocus::MType,
            CurveFocus::MStrength,
            CurveFocus::HType,
            CurveFocus::HStrength,
        ];
        let current_idx = order.iter().position(|&f| f == self.sub_focus).unwrap_or(0);

        for i in 1..=order.len() {
            let prev_idx = (current_idx + order.len() - i) % order.len();
            let candidate = order[prev_idx];
            if self.is_strength_visible(candidate) {
                self.sub_focus = candidate;
                return;
            }
        }
    }

    /// Cycle the curve type at current focus.
    fn cycle_type(&mut self, forward: bool) {
        match self.sub_focus {
            CurveFocus::JType => {
                self.values.j_type = if forward {
                    self.values.j_type.next()
                } else {
                    self.values.j_type.prev()
                };
            }
            CurveFocus::MType => {
                self.values.m_type = if forward {
                    self.values.m_type.next()
                } else {
                    self.values.m_type.prev()
                };
            }
            CurveFocus::HType => {
                self.values.h_type = if forward {
                    self.values.h_type.next()
                } else {
                    self.values.h_type.prev()
                };
            }
            _ => {}
        }
    }

    /// Adjust the strength at current focus.
    fn adjust_strength(&mut self, delta: f32) {
        match self.sub_focus {
            CurveFocus::JStrength => {
                self.values.j_strength = (self.values.j_strength + delta).clamp(0.1, 5.0);
            }
            CurveFocus::MStrength => {
                self.values.m_strength = (self.values.m_strength + delta).clamp(0.1, 5.0);
            }
            CurveFocus::HStrength => {
                self.values.h_strength = (self.values.h_strength + delta).clamp(0.1, 5.0);
            }
            _ => {}
        }
    }

    /// Handle left/right adjustment based on current focus.
    fn adjust(&mut self, forward: bool) {
        if self.sub_focus.is_strength() {
            self.adjust_strength(if forward { 0.05 } else { -0.05 });
        } else {
            self.cycle_type(forward);
        }
    }

    /// Draw a single curve row with type selector and optional inline strength slider.
    #[allow(clippy::too_many_arguments)]
    fn draw_curve_row(
        &self,
        frame: &mut Frame,
        area: Rect,
        label: &str,
        curve_type: CurveType,
        strength: f32,
        type_focus: CurveFocus,
        strength_focus: CurveFocus,
        focused: bool,
    ) {
        let type_focused = focused && self.sub_focus == type_focus;
        let strength_focused = focused && self.sub_focus == strength_focus;
        let shows_strength = curve_type.uses_strength();

        // Layout: Label (17) | Type selector (14) | Strength slider (rest)
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(17),
                Constraint::Length(14),
                Constraint::Min(8),
            ])
            .split(area);

        // Label
        let label_style = if type_focused || strength_focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let label_text = Paragraph::new(format!("{}:", label)).style(label_style);
        frame.render_widget(label_text, cols[0]);

        // Type selector
        let value_style = if type_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };
        let arrow_style = if type_focused {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM)
        };

        let type_line = Line::from(vec![
            Span::styled("◂ ", arrow_style),
            Span::styled(curve_type.display_name(), value_style),
            Span::styled(" ▸", arrow_style),
        ]);
        let type_para = Paragraph::new(type_line);
        frame.render_widget(type_para, cols[1]);

        // Strength slider (only if curve type uses strength)
        if shows_strength {
            self.draw_inline_strength(frame, cols[2], strength, strength_focused);
        }
    }

    /// Draw an inline strength slider.
    fn draw_inline_strength(&self, frame: &mut Frame, area: Rect, value: f32, focused: bool) {
        let slider_width = area.width.saturating_sub(6) as usize;
        if slider_width == 0 {
            return;
        }

        // Map 0.1-5.0 to 0-1 for display
        let ratio = (f64::from(value) - 0.1) / (5.0 - 0.1);
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

        let value_str = format!(" {:.1}", value);
        spans.push(Span::styled(
            value_str,
            if focused {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            },
        ));

        let slider_line = Paragraph::new(Line::from(spans));
        frame.render_widget(slider_line, area);
    }
}

impl MockComponent for CurveControls {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let focused = self
            .props
            .get_or(Attribute::Focus, AttrValue::Flag(false))
            .unwrap_flag();

        // Fixed 3 rows - one per curve type
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);

        // Lightness curve
        self.draw_curve_row(
            frame,
            rows[0],
            "Lightness Crv",
            self.values.j_type,
            self.values.j_strength,
            CurveFocus::JType,
            CurveFocus::JStrength,
            focused,
        );

        // Colorfulness curve
        self.draw_curve_row(
            frame,
            rows[1],
            "Colorful Crv",
            self.values.m_type,
            self.values.m_strength,
            CurveFocus::MType,
            CurveFocus::MStrength,
            focused,
        );

        // Hue curve
        self.draw_curve_row(
            frame,
            rows[2],
            "Hue Curve",
            self.values.h_type,
            self.values.h_strength,
            CurveFocus::HType,
            CurveFocus::HStrength,
            focused,
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
                self.focus_prev();
                CmdResult::None
            }
            Cmd::Move(CmdDirection::Down) => {
                self.focus_next();
                CmdResult::None
            }
            Cmd::Move(CmdDirection::Left) => {
                self.adjust(false);
                CmdResult::Changed(self.state())
            }
            Cmd::Move(CmdDirection::Right) => {
                self.adjust(true);
                CmdResult::Changed(self.state())
            }
            _ => CmdResult::None,
        }
    }
}

impl Component<Msg, UserEvent> for CurveControls {
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

            // Value adjustment: [/] for ±0.05, {/} for ±0.25 (strength sliders only)
            AppAction::ValueDecrementSmall => {
                if self.sub_focus.is_strength() {
                    self.adjust_strength(-0.05);
                    self.msg_for_change()
                } else {
                    self.cycle_type(false);
                    self.msg_for_change()
                }
            }
            AppAction::ValueIncrementSmall => {
                if self.sub_focus.is_strength() {
                    self.adjust_strength(0.05);
                    self.msg_for_change()
                } else {
                    self.cycle_type(true);
                    self.msg_for_change()
                }
            }
            AppAction::ValueDecrementLarge => {
                if self.sub_focus.is_strength() {
                    self.adjust_strength(-0.25);
                    self.msg_for_change()
                } else {
                    self.cycle_type(false);
                    self.msg_for_change()
                }
            }
            AppAction::ValueIncrementLarge => {
                if self.sub_focus.is_strength() {
                    self.adjust_strength(0.25);
                    self.msg_for_change()
                } else {
                    self.cycle_type(true);
                    self.msg_for_change()
                }
            }

            _ => None,
        }
    }
}

impl CurveControls {
    fn msg_for_change(&self) -> Option<Msg> {
        match self.sub_focus {
            CurveFocus::JType => Some(Msg::LightnessCurveTypeChanged(self.values.j_type)),
            CurveFocus::JStrength => {
                Some(Msg::LightnessCurveStrengthChanged(self.values.j_strength))
            }
            CurveFocus::MType => Some(Msg::ChromaCurveTypeChanged(self.values.m_type)),
            CurveFocus::MStrength => Some(Msg::ChromaCurveStrengthChanged(self.values.m_strength)),
            CurveFocus::HType => Some(Msg::HueCurveTypeChanged(self.values.h_type)),
            CurveFocus::HStrength => Some(Msg::HueCurveStrengthChanged(self.values.h_strength)),
        }
    }
}
