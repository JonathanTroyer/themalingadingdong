//! OKLCH color picker Component with L/C/H sliders.

use crossterm_actions::{AppEvent, NavigationEvent, SelectionEvent, TuiEvent};
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

use crate::tui::event::{UserEvent, dispatcher};
use crate::tui::msg::Msg;

/// Which slider is focused within the picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OklchFocus {
    #[default]
    Lightness,
    Chroma,
    Hue,
}

impl OklchFocus {
    fn next(self) -> Self {
        match self {
            Self::Lightness => Self::Chroma,
            Self::Chroma => Self::Hue,
            Self::Hue => Self::Lightness,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Lightness => Self::Hue,
            Self::Chroma => Self::Lightness,
            Self::Hue => Self::Chroma,
        }
    }
}

/// OKLCH color values.
#[derive(Debug, Clone, Copy)]
pub struct OklchValues {
    pub lightness: f32,
    pub chroma: f32,
    pub hue: f32,
    pub out_of_gamut: bool,
}

impl Default for OklchValues {
    fn default() -> Self {
        Self {
            lightness: 0.5,
            chroma: 0.0,
            hue: 0.0,
            out_of_gamut: false,
        }
    }
}

/// Type of OKLCH picker (background or foreground).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OklchPickerType {
    Background,
    Foreground,
}

/// OKLCH color picker with L, C, H sliders grouped together.
pub struct OklchPicker {
    props: Props,
    picker_type: OklchPickerType,
    values: OklchValues,
    srgb_preview: Srgb<u8>,
    sub_focus: OklchFocus,
}

impl OklchPicker {
    pub fn new(picker_type: OklchPickerType, values: OklchValues, srgb: Srgb<u8>) -> Self {
        Self {
            props: Props::default(),
            picker_type,
            values,
            srgb_preview: srgb,
            sub_focus: OklchFocus::Lightness,
        }
    }

    fn label(&self) -> &'static str {
        match self.picker_type {
            OklchPickerType::Background => "Background",
            OklchPickerType::Foreground => "Foreground",
        }
    }

    fn adjust_current(&mut self, delta: f64) {
        match self.sub_focus {
            OklchFocus::Lightness => {
                self.values.lightness =
                    (self.values.lightness + delta as f32 * 0.01).clamp(0.0, 1.0);
            }
            OklchFocus::Chroma => {
                self.values.chroma = (self.values.chroma + delta as f32 * 0.01).clamp(0.0, 0.4);
            }
            OklchFocus::Hue => {
                self.values.hue = (self.values.hue + delta as f32).rem_euclid(360.0);
            }
        }
        // Update gamut status and sRGB preview after any change
        self.update_derived();
    }

    /// Update derived values (out_of_gamut flag and sRGB preview) from current OKLCH.
    fn update_derived(&mut self) {
        use palette::{IntoColor, Oklch};
        let oklch = Oklch::new(self.values.lightness, self.values.chroma, self.values.hue);
        let linear: palette::LinSrgb<f32> = oklch.into_color();
        let srgb: palette::Srgb<f32> = Srgb::from_linear(linear);

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

        let value_str = if show_degrees {
            format!(" {:.0}°", value)
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

impl MockComponent for OklchPicker {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let focused = self
            .props
            .get_or(Attribute::Focus, AttrValue::Flag(false))
            .unwrap_flag();

        // Split into 3 rows for L, C, H
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);

        let warning = if self.values.out_of_gamut { " !" } else { "" };

        // Lightness
        self.draw_slider(
            frame,
            rows[0],
            &format!("{} L{}", self.label(), warning),
            self.values.lightness,
            0.0,
            1.0,
            focused && self.sub_focus == OklchFocus::Lightness,
            false,
        );

        // Chroma
        self.draw_slider(
            frame,
            rows[1],
            "  C",
            self.values.chroma,
            0.0,
            0.4,
            focused && self.sub_focus == OklchFocus::Chroma,
            false,
        );

        // Hue with color swatch
        let hue_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(10), Constraint::Length(3)])
            .split(rows[2]);

        self.draw_slider(
            frame,
            hue_cols[0],
            "  H",
            self.values.hue,
            0.0,
            360.0,
            focused && self.sub_focus == OklchFocus::Hue,
            true,
        );

        // Color swatch
        let swatch_style = Style::default().bg(Color::Rgb(
            self.srgb_preview.red,
            self.srgb_preview.green,
            self.srgb_preview.blue,
        ));
        let swatch = Paragraph::new("   ").style(swatch_style);
        frame.render_widget(swatch, hue_cols[1]);
    }

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.props.get(attr)
    }

    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        self.props.set(attr, value);
    }

    fn state(&self) -> State {
        // Return L, C, H as a tuple-like state
        State::Tup3((
            StateValue::F64(self.values.lightness as f64),
            StateValue::F64(self.values.chroma as f64),
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

impl Component<Msg, UserEvent> for OklchPicker {
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

            // Navigation within picker
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

impl OklchPicker {
    fn msg_for_change(&self) -> Option<Msg> {
        match self.picker_type {
            OklchPickerType::Background => match self.sub_focus {
                OklchFocus::Lightness => Some(Msg::BackgroundLChanged(self.values.lightness)),
                OklchFocus::Chroma => Some(Msg::BackgroundCChanged(self.values.chroma)),
                OklchFocus::Hue => Some(Msg::BackgroundHChanged(self.values.hue)),
            },
            OklchPickerType::Foreground => match self.sub_focus {
                OklchFocus::Lightness => Some(Msg::ForegroundLChanged(self.values.lightness)),
                OklchFocus::Chroma => Some(Msg::ForegroundCChanged(self.values.chroma)),
                OklchFocus::Hue => Some(Msg::ForegroundHChanged(self.values.hue)),
            },
        }
    }
}
