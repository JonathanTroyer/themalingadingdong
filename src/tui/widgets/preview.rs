//! Sample text preview widget.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::tui::state::TuiState;

/// Draw the sample text preview.
pub fn draw_preview(frame: &mut Frame, area: Rect, state: &TuiState) {
    let block = Block::default().title(" Preview ").borders(Borders::ALL);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(scheme) = &state.current_scheme else {
        let msg = Paragraph::new("No palette generated");
        frame.render_widget(msg, inner);
        return;
    };

    // Get background color for text background
    let bg = scheme
        .palette
        .get("base00")
        .map(|c| Color::Rgb(c.rgb.0, c.rgb.1, c.rgb.2))
        .unwrap_or(Color::Black);

    let mut lines = Vec::new();

    // Foreground text samples
    if let Some(fg_color) = scheme.palette.get("base05") {
        let fg = Color::Rgb(fg_color.rgb.0, fg_color.rgb.1, fg_color.rgb.2);
        lines.push(Line::from(Span::styled(
            "Normal text (base05 on base00)",
            Style::default().fg(fg).bg(bg),
        )));
    }

    if let Some(fg_color) = scheme.palette.get("base07") {
        let fg = Color::Rgb(fg_color.rgb.0, fg_color.rgb.1, fg_color.rgb.2);
        lines.push(Line::from(Span::styled(
            "Bright text (base07 on base00)",
            Style::default().fg(fg).bg(bg),
        )));
    }

    lines.push(Line::from(Span::raw("")));

    // Accent color samples
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
