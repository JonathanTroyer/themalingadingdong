//! HellwigJmh color picker Component with J/M/h sliders.

use crate::tui::AppAction;
use crossterm_actions::{NavigationEvent, SelectionEvent, TuiEvent};
use palette::Srgb;
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

use crate::hellwig::HellwigJmh;
use crate::tui::activities::{Msg, main::UserEvent};
use crate::tui::{dispatcher, handle_global_app_events};

/// Which slider is focused within the picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HellwigFocus {
    #[default]
    Lightness,
    Colorfulness,
    Hue,
}

impl HellwigFocus {
    fn next(self) -> Self {
        match self {
            Self::Lightness => Self::Colorfulness,
            Self::Colorfulness => Self::Hue,
            Self::Hue => Self::Lightness,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Lightness => Self::Hue,
            Self::Colorfulness => Self::Lightness,
            Self::Hue => Self::Colorfulness,
        }
    }
}

/// HellwigJmh color values.
#[derive(Debug, Clone, Copy)]
pub struct HellwigValues {
    /// Lightness (J'), range 0-100
    pub lightness: f32,
    /// Colorfulness (M), range 0-105
    pub colorfulness: f32,
    /// Hue (h), range 0-360 degrees
    pub hue: f32,
    /// Whether the color is out of sRGB gamut
    pub out_of_gamut: bool,
}

impl Default for HellwigValues {
    fn default() -> Self {
        Self {
            lightness: 50.0,
            colorfulness: 0.0,
            hue: 0.0,
            out_of_gamut: false,
        }
    }
}

/// Type of Hellwig picker (background or foreground).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HellwigPickerType {
    Background,
    Foreground,
}

/// HellwigJmh color picker with J, M, h sliders grouped together.
pub struct HellwigPicker {
    props: Props,
    picker_type: HellwigPickerType,
    values: HellwigValues,
    srgb_preview: Srgb<u8>,
    sub_focus: HellwigFocus,
}

impl HellwigPicker {
    pub fn new(picker_type: HellwigPickerType, values: HellwigValues, srgb: Srgb<u8>) -> Self {
        Self {
            props: Props::default(),
            picker_type,
            values,
            srgb_preview: srgb,
            sub_focus: HellwigFocus::Lightness,
        }
    }

    fn label(&self) -> &'static str {
        match self.picker_type {
            HellwigPickerType::Background => "Background",
            HellwigPickerType::Foreground => "Foreground",
        }
    }

    fn adjust_current(&mut self, delta: f64) {
        match self.sub_focus {
            HellwigFocus::Lightness => {
                // Increment of 1.0 per keystroke (0-100 range)
                self.values.lightness =
                    (self.values.lightness + delta as f32 * 1.0).clamp(0.0, 100.0);
            }
            HellwigFocus::Colorfulness => {
                // Increment of 1.0 per keystroke (0-105 range)
                self.values.colorfulness =
                    (self.values.colorfulness + delta as f32 * 1.0).clamp(0.0, 105.0);
            }
            HellwigFocus::Hue => {
                // Increment of 1.0 degree per keystroke
                self.values.hue = (self.values.hue + delta as f32).rem_euclid(360.0);
            }
        }
        // Update gamut status and sRGB preview after any change
        self.update_derived();
    }

    /// Update derived values (out_of_gamut flag and sRGB preview) from current HellwigJmh.
    fn update_derived(&mut self) {
        let hellwig = HellwigJmh::new(
            self.values.lightness,
            self.values.colorfulness,
            self.values.hue,
        );
        let srgb = hellwig.into_srgb();

        // Check gamut
        self.values.out_of_gamut = srgb.red < 0.0
            || srgb.red > 1.0
            || srgb.green < 0.0
            || srgb.green > 1.0
            || srgb.blue < 0.0
            || srgb.blue > 1.0;

        // Update sRGB preview (clamped to valid range)
        self.srgb_preview = Srgb::new(
            (srgb.red.clamp(0.0, 1.0) * 255.0).round() as u8,
            (srgb.green.clamp(0.0, 1.0) * 255.0).round() as u8,
            (srgb.blue.clamp(0.0, 1.0) * 255.0).round() as u8,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_slider(
        &self,
        frame: &mut Frame,
        area: Rect,
        label: &str,
        value: f32,
        min: f32,
        max: f32,
        focused: bool,
        show_degrees: bool,
    ) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(17), Constraint::Min(10)])
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
        let ratio = ((value - min) / (max - min)) as f64;
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

        // Format value - show integer for J and M, degrees for H
        let value_str = if show_degrees {
            format!(" {:.0}°", value)
        } else if max > 10.0 {
            // For J (0-100) and M (0-105), show as integer
            format!(" {:.0}", value)
        } else {
            format!(" {:.2}", value)
        };
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

impl MockComponent for HellwigPicker {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let focused = self
            .props
            .get_or(Attribute::Focus, AttrValue::Flag(false))
            .unwrap_flag();

        // Split into 4 rows: header + lightness, colorfulness, hue
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Header with label and swatch
                Constraint::Length(1), // Lightness
                Constraint::Length(1), // Colorfulness
                Constraint::Length(1), // Hue
            ])
            .split(area);

        // Header row with label, warning, and color swatch
        let header_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(10), Constraint::Length(3)])
            .split(rows[0]);

        let warning = if self.values.out_of_gamut { " !" } else { "" };
        let header_style = if focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().add_modifier(Modifier::BOLD)
        };
        let header_text =
            Paragraph::new(format!("{}:{}", self.label(), warning)).style(header_style);
        frame.render_widget(header_text, header_cols[0]);

        // Color swatch in header
        let swatch_style = Style::default().bg(Color::Rgb(
            self.srgb_preview.red,
            self.srgb_preview.green,
            self.srgb_preview.blue,
        ));
        let swatch = Paragraph::new("   ").style(swatch_style);
        frame.render_widget(swatch, header_cols[1]);

        // Lightness slider
        self.draw_slider(
            frame,
            rows[1],
            "  Lightness",
            self.values.lightness,
            0.0,
            100.0,
            focused && self.sub_focus == HellwigFocus::Lightness,
            false,
        );

        // Colorfulness slider
        self.draw_slider(
            frame,
            rows[2],
            "  Colorfulness",
            self.values.colorfulness,
            0.0,
            105.0,
            focused && self.sub_focus == HellwigFocus::Colorfulness,
            false,
        );

        // Hue slider
        self.draw_slider(
            frame,
            rows[3],
            "  Hue",
            self.values.hue,
            0.0,
            360.0,
            focused && self.sub_focus == HellwigFocus::Hue,
            true,
        );
    }

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.props.get(attr)
    }

    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        self.props.set(attr, value);
    }

    fn state(&self) -> State {
        // Return J, M, h as a tuple-like state
        State::Tup3((
            StateValue::F64(self.values.lightness as f64),
            StateValue::F64(self.values.colorfulness as f64),
            StateValue::F64(self.values.hue as f64),
        ))
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

impl Component<Msg, UserEvent> for HellwigPicker {
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

        if let Some(msg) = handle_global_app_events(&action) {
            return Some(msg);
        }

        match action {
            // Focus navigation
            AppAction::Tui(TuiEvent::Selection(SelectionEvent::Next)) => Some(Msg::FocusNext),
            AppAction::Tui(TuiEvent::Selection(SelectionEvent::Prev)) => Some(Msg::FocusPrev),

            // Navigation within picker
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

            _ => None,
        }
    }
}

impl HellwigPicker {
    fn msg_for_change(&self) -> Option<Msg> {
        match self.picker_type {
            HellwigPickerType::Background => match self.sub_focus {
                HellwigFocus::Lightness => Some(Msg::BackgroundJChanged(self.values.lightness)),
                HellwigFocus::Colorfulness => {
                    Some(Msg::BackgroundMChanged(self.values.colorfulness))
                }
                HellwigFocus::Hue => Some(Msg::BackgroundHChanged(self.values.hue)),
            },
            HellwigPickerType::Foreground => match self.sub_focus {
                HellwigFocus::Lightness => Some(Msg::ForegroundJChanged(self.values.lightness)),
                HellwigFocus::Colorfulness => {
                    Some(Msg::ForegroundMChanged(self.values.colorfulness))
                }
                HellwigFocus::Hue => Some(Msg::ForegroundHChanged(self.values.hue)),
            },
        }
    }
}
