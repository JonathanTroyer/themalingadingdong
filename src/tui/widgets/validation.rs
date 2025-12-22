//! Validation results widget.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::tui::state::{Pane, TuiState};

/// Draw the validation results panel.
pub fn draw_validation(frame: &mut Frame, area: Rect, state: &TuiState) {
    let is_active = state.active_pane == Pane::Validation;
    let border_style = if is_active {
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

    if state.current_scheme.is_none() {
        let msg = Paragraph::new("No scheme to validate");
        frame.render_widget(msg, inner);
        return;
    }

    // Count passes and failures
    let total = state.validation_results.len();
    let passing = state.validation_results.iter().filter(|r| r.passes).count();
    let failing = total - passing;

    let mut lines = Vec::new();

    // Summary line
    let summary_style = if failing == 0 {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    };

    lines.push(Line::from(Span::styled(
        format!("{passing}/{total} pairs pass APCA contrast"),
        summary_style,
    )));

    lines.push(Line::from(Span::raw("")));

    // Show all failing pairs (scrollable)
    let failures: Vec<_> = state
        .validation_results
        .iter()
        .filter(|r| !r.passes)
        .collect();

    if failures.is_empty() {
        lines.push(Line::from(Span::styled(
            "All color pairs meet contrast requirements",
            Style::default().fg(Color::Green),
        )));
    } else {
        for result in &failures {
            let line = format!(
                "{}/{}: Lc={:.1} (need {:.0})",
                result.pair.foreground,
                result.pair.background,
                result.contrast.abs(),
                result.pair.threshold.min_lc,
            );
            lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(Color::Red),
            )));
        }
    }

    // Show generation warnings if any
    if !state.generation_warnings.is_empty() {
        lines.push(Line::from(Span::raw("")));
        lines.push(Line::from(Span::styled(
            "Generation warnings:",
            Style::default().fg(Color::Yellow),
        )));
        for warning in &state.generation_warnings {
            lines.push(Line::from(Span::styled(
                format!("  {warning}"),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::DIM),
            )));
        }
    }

    let content_height = lines.len() as u16;
    let visible_height = inner.height;
    let needs_scroll = content_height > visible_height;

    let paragraph = Paragraph::new(lines).scroll((state.validation_scroll, 0));
    frame.render_widget(paragraph, inner);

    // Render scrollbar if content overflows
    if needs_scroll {
        let max_scroll = content_height.saturating_sub(visible_height);
        let mut scrollbar_state =
            ScrollbarState::new(max_scroll as usize).position(state.validation_scroll as usize);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}
