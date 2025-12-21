//! Palette preview widget showing 24 color swatches.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::tui::state::TuiState;

/// Color names in Base24 order.
const COLOR_NAMES: [&str; 24] = [
    "base00", "base01", "base02", "base03", "base04", "base05", "base06", "base07", "base08",
    "base09", "base0A", "base0B", "base0C", "base0D", "base0E", "base0F", "base10", "base11",
    "base12", "base13", "base14", "base15", "base16", "base17",
];

/// Draw the palette preview widget.
pub fn draw_palette(frame: &mut Frame, area: Rect, state: &TuiState) {
    let block = Block::default().title(" Palette ").borders(Borders::ALL);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(scheme) = &state.current_scheme else {
        let msg = Paragraph::new("No palette generated");
        frame.render_widget(msg, inner);
        return;
    };

    // Layout: 3 rows of 8 colors each
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(inner);

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
                draw_swatch(frame, *col_area, color_name, color.rgb);
            }
        }
    }
}

fn draw_swatch(frame: &mut Frame, area: Rect, name: &str, rgb: (u8, u8, u8)) {
    let bg_color = Color::Rgb(rgb.0, rgb.1, rgb.2);

    // Choose contrasting text color
    let luminance = 0.299 * f32::from(rgb.0) + 0.587 * f32::from(rgb.1) + 0.114 * f32::from(rgb.2);
    let fg_color = if luminance > 128.0 {
        Color::Black
    } else {
        Color::White
    };

    let style = Style::default().bg(bg_color).fg(fg_color);

    // Short name (last 2 chars)
    let short_name = &name[4..];
    let hex = format!("{:02X}{:02X}{:02X}", rgb.0, rgb.1, rgb.2);

    // Create lines for name and hex
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
