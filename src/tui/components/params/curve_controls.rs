//! Grouped curve controls component for J/M/h interpolation.

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
use crate::tui::msg::Msg;
use crate::tui::{UserEvent, dispatcher, handle_global_app_events};

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
    /// Get all possible focus values in order.
    fn all() -> &'static [CurveFocus] {
        &[
            Self::JType,
            Self::JStrength,
            Self::MType,
            Self::MStrength,
            Self::HType,
            Self::HStrength,
        ]
    }

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

    /// Get visible focus items based on current curve types.
    fn visible_focuses(&self) -> Vec<CurveFocus> {
        CurveFocus::all()
            .iter()
            .copied()
            .filter(|f| self.is_strength_visible(*f))
            .collect()
    }

    /// Move to next visible focus.
    fn focus_next(&mut self) {
        let visible = self.visible_focuses();
        if visible.is_empty() {
            return;
        }
        let current_idx = visible
            .iter()
            .position(|&f| f == self.sub_focus)
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % visible.len();
        self.sub_focus = visible[next_idx];
    }

    /// Move to previous visible focus.
    fn focus_prev(&mut self) {
        let visible = self.visible_focuses();
        if visible.is_empty() {
            return;
        }
        let current_idx = visible
            .iter()
            .position(|&f| f == self.sub_focus)
            .unwrap_or(0);
        let prev_idx = (current_idx + visible.len() - 1) % visible.len();
        self.sub_focus = visible[prev_idx];
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

    fn draw_type_selector(
        &self,
        frame: &mut Frame,
        area: Rect,
        label: &str,
        curve_type: CurveType,
        focused: bool,
    ) {
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

        let line = Line::from(vec![
            Span::styled("◂ ", arrow_style),
            Span::styled(curve_type.display_name(), value_style),
            Span::styled(" ▸", arrow_style),
        ]);

        let value_para = Paragraph::new(line);
        frame.render_widget(value_para, cols[1]);
    }

    fn draw_strength_slider(&self, frame: &mut Frame, area: Rect, value: f32, focused: bool) {
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
        let label_text = Paragraph::new("  Strength:").style(label_style);
        frame.render_widget(label_text, cols[0]);

        let slider_width = cols[1].width.saturating_sub(8) as usize;
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

impl MockComponent for CurveControls {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let focused = self
            .props
            .get_or(Attribute::Focus, AttrValue::Flag(false))
            .unwrap_flag();

        // Calculate how many rows we need based on visibility
        let j_strength_visible = self.values.j_type.uses_strength();
        let m_strength_visible = self.values.m_type.uses_strength();
        let h_strength_visible = self.values.h_type.uses_strength();

        let row_count = 3 // J, M, h type selectors
            + if j_strength_visible { 1 } else { 0 }
            + if m_strength_visible { 1 } else { 0 }
            + if h_strength_visible { 1 } else { 0 };

        let constraints: Vec<Constraint> = (0..row_count).map(|_| Constraint::Length(1)).collect();

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        let mut row_idx = 0;

        // J curve type
        self.draw_type_selector(
            frame,
            rows[row_idx],
            "J Curve",
            self.values.j_type,
            focused && self.sub_focus == CurveFocus::JType,
        );
        row_idx += 1;

        // J strength (if visible)
        if j_strength_visible {
            self.draw_strength_slider(
                frame,
                rows[row_idx],
                self.values.j_strength,
                focused && self.sub_focus == CurveFocus::JStrength,
            );
            row_idx += 1;
        }

        // M curve type
        self.draw_type_selector(
            frame,
            rows[row_idx],
            "M Curve",
            self.values.m_type,
            focused && self.sub_focus == CurveFocus::MType,
        );
        row_idx += 1;

        // M strength (if visible)
        if m_strength_visible {
            self.draw_strength_slider(
                frame,
                rows[row_idx],
                self.values.m_strength,
                focused && self.sub_focus == CurveFocus::MStrength,
            );
            row_idx += 1;
        }

        // h curve type
        self.draw_type_selector(
            frame,
            rows[row_idx],
            "h Curve",
            self.values.h_type,
            focused && self.sub_focus == CurveFocus::HType,
        );
        row_idx += 1;

        // h strength (if visible)
        if h_strength_visible && row_idx < rows.len() {
            self.draw_strength_slider(
                frame,
                rows[row_idx],
                self.values.h_strength,
                focused && self.sub_focus == CurveFocus::HStrength,
            );
        }
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
