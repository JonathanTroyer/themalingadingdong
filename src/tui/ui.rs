//! UI layout and rendering.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use super::state::TuiState;
use super::widgets::{
    draw_help_overlay, draw_palette, draw_parameters, draw_preview, draw_validation,
};

/// Main draw function for the TUI.
pub fn draw(frame: &mut Frame, state: &TuiState) {
    let area = frame.area();

    // Main layout: title, content, status bar
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(10),   // Content
            Constraint::Length(3), // Status bar
        ])
        .split(area);

    // Title bar
    draw_title(frame, main_layout[0], state);

    // Content area: 2x2 grid
    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_layout[1]);

    // Left column: palette (remaining space) and preview (fixed height for content)
    // Preview: 11 lines of content + 2 borders = 13 total
    let left_column = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(13)])
        .split(content_layout[0]);

    // Right column: parameters (fixed height) and validation (remaining space)
    // Parameters: 15 rows of content + 2 borders = 17 total
    let right_column = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(17), Constraint::Min(0)])
        .split(content_layout[1]);

    // Draw widgets
    draw_palette(frame, left_column[0], state);
    draw_preview(frame, left_column[1], state);
    draw_parameters(frame, right_column[0], state);
    draw_validation(frame, right_column[1], state);

    // Status bar
    draw_status_bar(frame, main_layout[2], state);

    // Modals (on top)
    if state.show_export {
        draw_export_dialog(frame, state);
    }
    if state.show_help {
        draw_help_overlay(frame, state);
    }
}

fn draw_title(frame: &mut Frame, area: Rect, state: &TuiState) {
    let variant_str = match state.variant {
        crate::cli::VariantArg::Auto => "Auto",
        crate::cli::VariantArg::Dark => "Dark",
        crate::cli::VariantArg::Light => "Light",
        crate::cli::VariantArg::Both => "Both",
    };

    let title = format!(" {} [{}] ", state.name, variant_str);

    let block = Block::default()
        .title(title)
        .title_style(Style::default().add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    frame.render_widget(block, area);
}

fn draw_status_bar(frame: &mut Frame, area: Rect, state: &TuiState) {
    let status_text = if state.editing_text {
        "EDIT MODE | Enter: Confirm | Esc: Cancel"
    } else if state.show_export {
        "EXPORT | Enter: Save | Esc: Cancel"
    } else {
        "Tab: Switch pane | j/k: Navigate/Scroll | Left/Right: Adjust | ?: Help | q: Quit"
    };

    let message = state.message.as_deref().unwrap_or("");

    let spans = if message.is_empty() {
        vec![Span::raw(status_text)]
    } else {
        vec![
            Span::styled(message, Style::default().fg(Color::Yellow)),
            Span::raw(" | "),
            Span::raw(status_text),
        ]
    };

    let paragraph = Paragraph::new(Line::from(spans)).block(Block::default().borders(Borders::ALL));

    frame.render_widget(paragraph, area);
}

fn draw_export_dialog(frame: &mut Frame, state: &TuiState) {
    let area = frame.area();

    // Center the dialog
    let dialog_width = 50;
    let dialog_height = 7;
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    // Clear background
    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(" Export Scheme ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let content = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .margin(1)
        .split(inner);

    // Label
    let label = Paragraph::new("Path:");
    frame.render_widget(label, content[0]);

    // Input field
    let input_style = Style::default().fg(Color::White).bg(Color::DarkGray);
    let input = Paragraph::new(state.export_path.as_str()).style(input_style);
    frame.render_widget(input, content[1]);

    // Hint
    let hint =
        Paragraph::new("Enter: Save | Esc: Cancel").style(Style::default().fg(Color::DarkGray));
    frame.render_widget(hint, content[2]);
}
