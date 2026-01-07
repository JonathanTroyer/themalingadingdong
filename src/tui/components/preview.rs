//! Sample text preview Component.

use ratatui::Frame;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use tinted_builder::Base16Scheme;
use tuirealm::{
    Component, Event, MockComponent, State,
    command::{Cmd, CmdResult},
    props::{AttrValue, Attribute, Props},
};

use crate::tui::UserEvent;
use crate::tui::msg::Msg;

/// Preview component showing sample text with palette colors.
pub struct Preview {
    props: Props,
    scheme: Option<Base16Scheme>,
}

impl Preview {
    pub fn new() -> Self {
        Self {
            props: Props::default(),
            scheme: None,
        }
    }

    pub fn set_scheme(&mut self, scheme: Option<Base16Scheme>) {
        self.scheme = scheme;
    }
}

impl Default for Preview {
    fn default() -> Self {
        Self::new()
    }
}

impl MockComponent for Preview {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::default().title(" Preview ").borders(Borders::ALL);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let Some(scheme) = &self.scheme else {
            let msg = Paragraph::new("No palette generated");
            frame.render_widget(msg, inner);
            return;
        };

        let bg = scheme
            .palette
            .get("base00")
            .map(|c| Color::Rgb(c.rgb.0, c.rgb.1, c.rgb.2))
            .unwrap_or(Color::Black);

        let mut lines = Vec::new();

        // Normal text
        if let Some(fg_color) = scheme.palette.get("base05") {
            let fg = Color::Rgb(fg_color.rgb.0, fg_color.rgb.1, fg_color.rgb.2);
            lines.push(Line::from(Span::styled(
                "Normal text (base05 on base00)",
                Style::default().fg(fg).bg(bg),
            )));
        }

        // Bright text
        if let Some(fg_color) = scheme.palette.get("base07") {
            let fg = Color::Rgb(fg_color.rgb.0, fg_color.rgb.1, fg_color.rgb.2);
            lines.push(Line::from(Span::styled(
                "Bright text (base07 on base00)",
                Style::default().fg(fg).bg(bg),
            )));
        }

        lines.push(Line::from(Span::raw("")));

        // Accent colors
        let accent_samples = [
            ("base08", "Error: Something went wrong"),
            ("base09", "Warning: Check this"),
            ("base0A", "Note: Important info"),
            ("base0B", "Success: Operation complete"),
            ("base0C", "Info: Additional details"),
            ("base0D", "Link: Click here"),
            ("base0E", "Keyword: const let var"),
            ("base0F", "Special: @#$%"),
        ];

        for (base, text) in accent_samples {
            if let Some(color) = scheme.palette.get(base) {
                let fg = Color::Rgb(color.rgb.0, color.rgb.1, color.rgb.2);
                let label = format!("{base}: {text}");
                lines.push(Line::from(Span::styled(
                    label,
                    Style::default().fg(fg).bg(bg),
                )));
            }
        }

        let paragraph = Paragraph::new(lines).style(Style::default().bg(bg));
        frame.render_widget(paragraph, inner);
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

impl Component<Msg, UserEvent> for Preview {
    fn on(&mut self, _ev: Event<UserEvent>) -> Option<Msg> {
        // Read-only component
        None
    }
}
