//! Help overlay widget.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::tui::state::TuiState;

/// Draw the help overlay.
pub fn draw_help_overlay(frame: &mut Frame, _state: &TuiState) {
    let area = frame.area();

    // Center the dialog
    let dialog_width = 55;
    let dialog_height = 18;
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    // Clear background
    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(" Keybindings ")
        .title_style(Style::default().add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let keybindings = [
        ("Tab", "Switch pane"),
        ("Down / j", "Next field / Scroll down"),
        ("Up / k", "Previous field / Scroll up"),
        ("Left / Right", "Adjust value / cycle"),
        ("Enter", "Edit text / Export"),
        ("Esc", "Cancel edit"),
        ("r / Ctrl+L", "Regenerate palette"),
        ("?", "Toggle help"),
        ("q / Ctrl+C", "Quit"),
        ("", ""),
        ("In text fields:", ""),
        ("  Any character", "Type to edit"),
        ("  Backspace/Delete", "Delete characters"),
        ("  Home/End", "Move cursor"),
    ];

    let mut lines = Vec::new();
    for (key, desc) in keybindings {
        if key.is_empty() {
            lines.push(Line::from(Span::raw("")));
        } else if desc.is_empty() {
            lines.push(Line::from(Span::styled(
                key,
                Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
        } else {
            lines.push(Line::from(vec![
                Span::styled(format!("{key:20}"), Style::default().fg(Color::Cyan)),
                Span::raw(" "),
                Span::raw(desc),
            ]));
        }
    }

    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(Span::styled(
        "Press any key to close",
        Style::default().add_modifier(Modifier::DIM),
    )));

    let content = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(0)])
        .split(inner);

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, content[0]);
}
