//! Palette display Component showing 24 color swatches.

use palette::Srgb;
use ratatui::Frame;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use tinted_builder::Base16Scheme;
use tuirealm::{
    Component, Event, MockComponent, State,
    command::{Cmd, CmdResult},
    props::{AttrValue, Attribute, Props},
};

use crate::curves::{CurveConfig, compute_sample_positions};
use crate::tui::UserEvent;
use crate::tui::msg::Msg;

/// Color names in Base24 order.
const COLOR_NAMES: [&str; 24] = [
    "base00", "base01", "base02", "base03", "base04", "base05", "base06", "base07", "base08",
    "base09", "base0A", "base0B", "base0C", "base0D", "base0E", "base0F", "base10", "base11",
    "base12", "base13", "base14", "base15", "base16", "base17",
];

/// Palette display component showing gradient and 24 color swatches.
pub struct Palette {
    props: Props,
    scheme: Option<Base16Scheme>,
    background: Srgb<u8>,
    foreground: Srgb<u8>,
    curve: CurveConfig,
}

impl Palette {
    pub fn new() -> Self {
        Self {
            props: Props::default(),
            scheme: None,
            background: Srgb::new(0, 0, 0),
            foreground: Srgb::new(255, 255, 255),
            curve: CurveConfig::default(),
        }
    }

    pub fn set_scheme(&mut self, scheme: Option<Base16Scheme>) {
        self.scheme = scheme;
    }

    pub fn set_colors(&mut self, bg: Srgb<u8>, fg: Srgb<u8>, curve: CurveConfig) {
        self.background = bg;
        self.foreground = fg;
        self.curve = curve;
    }

    fn draw_swatch(&self, frame: &mut Frame, area: Rect, name: &str, rgb: (u8, u8, u8)) {
        let bg_color = Color::Rgb(rgb.0, rgb.1, rgb.2);

        // Choose contrasting text color
        let luminance =
            0.299 * f32::from(rgb.0) + 0.587 * f32::from(rgb.1) + 0.114 * f32::from(rgb.2);
        let fg_color = if luminance > 128.0 {
            Color::Black
        } else {
            Color::White
        };

        let style = Style::default().bg(bg_color).fg(fg_color);

        // Short name (last 2 chars)
        let short_name = &name[4..];
        let hex = format!("{:02X}{:02X}{:02X}", rgb.0, rgb.1, rgb.2);

        let lines = if area.height >= 3 {
            vec![
                Line::from(Span::styled(short_name, style)),
                Line::from(Span::styled(hex, style)),
            ]
        } else {
            vec![Line::from(Span::styled(short_name, style))]
        };

        let paragraph = Paragraph::new(lines).style(style);
        frame.render_widget(paragraph, area);
    }

    fn draw_gradient_with_markers(&self, frame: &mut Frame, area: Rect) {
        if area.height < 2 || area.width < 8 {
            return;
        }

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area);

        let gradient_row = rows[0];
        let marker_row = rows[1];
        let width = gradient_row.width as usize;

        // Draw gradient bar
        let mut gradient_spans = Vec::with_capacity(width);
        for x in 0..width {
            let t = x as f32 / (width.saturating_sub(1)) as f32;
            let r = lerp_u8(self.background.red, self.foreground.red, t);
            let g = lerp_u8(self.background.green, self.foreground.green, t);
            let b = lerp_u8(self.background.blue, self.foreground.blue, t);
            let style = Style::default().bg(Color::Rgb(r, g, b));
            gradient_spans.push(Span::styled(" ", style));
        }
        let gradient_line = Line::from(gradient_spans);
        let gradient_para = Paragraph::new(gradient_line);
        frame.render_widget(gradient_para, gradient_row);

        // Draw markers
        let samples = compute_sample_positions(8, &self.curve);
        let mut marker_chars = vec![' '; width];

        for (i, sample_t) in samples.iter().enumerate() {
            let x = (sample_t * (width.saturating_sub(1)) as f32).round() as usize;
            let x = x.min(width.saturating_sub(1));
            let digit = char::from_digit(i as u32, 10).unwrap_or('?');
            marker_chars[x] = digit;
        }

        let mut marker_spans = Vec::new();
        for c in marker_chars {
            if c == ' ' {
                marker_spans.push(Span::styled(" ", Style::default()));
            } else {
                marker_spans.push(Span::styled(
                    c.to_string(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ));
            }
        }
        let marker_line = Line::from(marker_spans);
        let marker_para = Paragraph::new(marker_line);
        frame.render_widget(marker_para, marker_row);
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::new()
    }
}

impl MockComponent for Palette {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::default().title(" Palette ").borders(Borders::ALL);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let Some(scheme) = &self.scheme else {
            let msg = Paragraph::new("No palette generated");
            frame.render_widget(msg, inner);
            return;
        };

        // Layout: gradient + 3 rows of swatches
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Gradient + markers
                Constraint::Min(6),    // Swatches
            ])
            .split(inner);

        self.draw_gradient_with_markers(frame, sections[0]);

        // Swatches: 3 rows of 8
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
            ])
            .split(sections[1]);

        for (row_idx, row_area) in rows.iter().enumerate() {
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Ratio(1, 8),
                    Constraint::Ratio(1, 8),
                    Constraint::Ratio(1, 8),
                    Constraint::Ratio(1, 8),
                    Constraint::Ratio(1, 8),
                    Constraint::Ratio(1, 8),
                    Constraint::Ratio(1, 8),
                    Constraint::Ratio(1, 8),
                ])
                .split(*row_area);

            for (col_idx, col_area) in cols.iter().enumerate() {
                let color_idx = row_idx * 8 + col_idx;
                let color_name = COLOR_NAMES[color_idx];

                if let Some(color) = scheme.palette.get(color_name) {
                    self.draw_swatch(frame, *col_area, color_name, color.rgb);
                }
            }
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

    fn perform(&mut self, _cmd: Cmd) -> CmdResult {
        CmdResult::None
    }
}

impl Component<Msg, UserEvent> for Palette {
    fn on(&mut self, _ev: Event<UserEvent>) -> Option<Msg> {
        // Read-only component, no events handled
        None
    }
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    let a = f32::from(a);
    let b = f32::from(b);
    (a + (b - a) * t).round().clamp(0.0, 255.0) as u8
}
