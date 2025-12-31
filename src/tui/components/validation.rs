//! Validation results Component.

use crossterm_actions::{AppEvent, NavigationEvent, SelectionEvent, TuiEvent};
use ratatui::Frame;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use tuirealm::{
    Component, Event, MockComponent, State, StateValue,
    command::{Cmd, CmdResult, Direction as CmdDirection},
    props::{AttrValue, Attribute, Props},
};

use crate::tui::event::{UserEvent, dispatcher};
use crate::tui::msg::Msg;
use crate::validation::ValidationResult;

/// Validation results display with scrolling.
pub struct Validation {
    props: Props,
    results: Vec<ValidationResult>,
    warnings: Vec<String>,
    scroll: u16,
    has_scheme: bool,
}

impl Validation {
    pub fn new() -> Self {
        Self {
            props: Props::default(),
            results: Vec::new(),
            warnings: Vec::new(),
            scroll: 0,
            has_scheme: false,
        }
    }

    pub fn set_data(
        &mut self,
        results: Vec<ValidationResult>,
        warnings: Vec<String>,
        has_scheme: bool,
    ) {
        self.results = results;
        self.warnings = warnings;
        self.has_scheme = has_scheme;
        // Reset scroll when data changes
        self.scroll = 0;
    }

    fn content_lines(&self) -> Vec<Line<'static>> {
        use std::collections::HashMap;

        let mut lines = Vec::new();

        if !self.has_scheme {
            return lines;
        }

        // Group results by foreground color, tracking contrast per background
        struct ColorData<'a> {
            result: &'a ValidationResult,
            lc00: Option<f64>,
            lc01: Option<f64>,
            worst_lc: f64,
            worst_passes: bool,
        }

        let mut fg_data: HashMap<&str, ColorData> = HashMap::new();
        for result in &self.results {
            let fg = result.pair.foreground;
            let bg = result.pair.background;
            let abs_contrast = result.contrast.abs();

            let entry = fg_data.entry(fg).or_insert(ColorData {
                result,
                lc00: None,
                lc01: None,
                worst_lc: f64::MAX,
                worst_passes: true,
            });

            // Track contrast per background
            match bg {
                "base00" => entry.lc00 = Some(abs_contrast),
                "base01" => entry.lc01 = Some(abs_contrast),
                _ => {}
            }

            // Track worst case (lowest contrast)
            if abs_contrast < entry.worst_lc {
                entry.worst_lc = abs_contrast;
                entry.worst_passes = result.passes;
                entry.result = result;
            }
        }

        // UI Colors section (base06, base07) - simple format
        lines.push(Line::from(Span::styled(
            "UI Colors:".to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        )));

        let ui_colors = ["base06", "base07"];
        for fg in ui_colors {
            if let Some(data) = fg_data.get(fg) {
                let threshold = data.result.pair.threshold.min_lc;
                let (icon, style) = self.status_style(data.worst_passes, data.worst_lc, threshold);
                let lc00_str = data.lc00.map(|v| format!("{:.0}", v)).unwrap_or_default();
                let lc01_str = data.lc01.map(|v| format!("{:.0}", v)).unwrap_or_default();
                let text = format!(
                    "  {}: Lc00={:>3} Lc01={:>3}{}",
                    &fg[4..],
                    lc00_str,
                    lc01_str,
                    icon
                );
                lines.push(Line::from(Span::styled(text, style)));
            }
        }
        lines.push(Line::from(Span::raw("")));

        // Table header for accents
        lines.push(Line::from(Span::styled(
            "Accents:".to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            "       L     C     H   Lc00 Lc01".to_string(),
            Style::default().add_modifier(Modifier::DIM),
        )));

        // Primary Accents (base08-base0F)
        let primary_accents = [
            "base08", "base09", "base0A", "base0B", "base0C", "base0D", "base0E", "base0F",
        ];
        for fg in primary_accents {
            if let Some(data) = fg_data.get(fg) {
                lines.push(self.format_accent_row(
                    fg,
                    data.result,
                    data.lc00,
                    data.lc01,
                    data.worst_passes,
                    data.worst_lc,
                ));
            }
        }

        // Extended Accents (base10-base17)
        let extended_accents = [
            "base10", "base11", "base12", "base13", "base14", "base15", "base16", "base17",
        ];
        for fg in extended_accents {
            if let Some(data) = fg_data.get(fg) {
                lines.push(self.format_accent_row(
                    fg,
                    data.result,
                    data.lc00,
                    data.lc01,
                    data.worst_passes,
                    data.worst_lc,
                ));
            }
        }

        // Warnings
        if !self.warnings.is_empty() {
            lines.push(Line::from(Span::raw("")));
            lines.push(Line::from(Span::styled(
                "Warnings:".to_string(),
                Style::default().fg(Color::Yellow),
            )));
            for warning in &self.warnings {
                lines.push(Line::from(Span::styled(
                    format!("  {warning}"),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::DIM),
                )));
            }
        }

        lines
    }

    fn format_accent_row(
        &self,
        fg: &str,
        result: &ValidationResult,
        lc00: Option<f64>,
        lc01: Option<f64>,
        passes: bool,
        worst_lc: f64,
    ) -> Line<'static> {
        let threshold = result.pair.threshold.min_lc;
        let (icon, style) = self.status_style(passes, worst_lc, threshold);

        let (l, c, h) = result
            .fg_oklch
            .map(|oklch| {
                let hue = oklch.hue.into_positive_degrees();
                (oklch.l, oklch.chroma, hue)
            })
            .unwrap_or((0.0, 0.0, 0.0));

        let lc00_str = lc00
            .map(|v| format!("{:>3.0}", v))
            .unwrap_or_else(|| "  -".to_string());
        let lc01_str = lc01
            .map(|v| format!("{:>3.0}", v))
            .unwrap_or_else(|| "  -".to_string());

        let text = format!(
            "  {}  {:.2} {:.3} {:>5.1}  {} {}{}",
            &fg[4..],
            l,
            c,
            h,
            lc00_str,
            lc01_str,
            icon
        );
        Line::from(Span::styled(text, style))
    }

    /// Get status icon and style based on contrast value.
    fn status_style(&self, passes: bool, contrast: f64, threshold: f64) -> (&'static str, Style) {
        if passes {
            if contrast >= threshold + 5.0 {
                // Comfortably passing
                (" ✓", Style::default().fg(Color::Green))
            } else {
                // Close to threshold (within 5 Lc)
                (" ⚠", Style::default().fg(Color::Yellow))
            }
        } else {
            // Failing
            (" ✗", Style::default().fg(Color::Red))
        }
    }

    fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    fn scroll_down(&mut self, max: u16) {
        self.scroll = (self.scroll + 1).min(max);
    }
}

impl Default for Validation {
    fn default() -> Self {
        Self::new()
    }
}

impl MockComponent for Validation {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let focused = self
            .props
            .get_or(Attribute::Focus, AttrValue::Flag(false))
            .unwrap_flag();

        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };

        let block = Block::default()
            .title(" Validation ")
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if !self.has_scheme {
            let msg = Paragraph::new("No scheme to validate");
            frame.render_widget(msg, inner);
            return;
        }

        let lines = self.content_lines();
        let content_height = lines.len() as u16;
        let visible_height = inner.height;
        let needs_scroll = content_height > visible_height;

        let paragraph = Paragraph::new(lines).scroll((self.scroll, 0));
        frame.render_widget(paragraph, inner);

        if needs_scroll {
            let max_scroll = content_height.saturating_sub(visible_height);
            let mut scrollbar_state =
                ScrollbarState::new(max_scroll as usize).position(self.scroll as usize);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
            frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }
    }

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.props.get(attr)
    }

    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        self.props.set(attr, value);
    }

    fn state(&self) -> State {
        State::One(StateValue::U16(self.scroll))
    }

    fn perform(&mut self, cmd: Cmd) -> CmdResult {
        match cmd {
            Cmd::Scroll(CmdDirection::Up) => {
                self.scroll_up();
                CmdResult::Changed(self.state())
            }
            Cmd::Scroll(CmdDirection::Down) => {
                let lines = self.content_lines();
                let max = lines.len().saturating_sub(1) as u16;
                self.scroll_down(max);
                CmdResult::Changed(self.state())
            }
            _ => CmdResult::None,
        }
    }
}

impl Component<Msg, UserEvent> for Validation {
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

            // Scrolling
            TuiEvent::Navigation(NavigationEvent::Up) => {
                self.perform(Cmd::Scroll(CmdDirection::Up));
                Some(Msg::ValidationScrollUp)
            }
            TuiEvent::Navigation(NavigationEvent::Down) => {
                self.perform(Cmd::Scroll(CmdDirection::Down));
                Some(Msg::ValidationScrollDown)
            }

            _ => None,
        }
    }
}
