//! Validation results widget.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::tui::state::TuiState;

/// Draw the validation results panel.
pub fn draw_validation(frame: &mut Frame, area: Rect, state: &TuiState) {
    let block = Block::default().title(" Validation ").borders(Borders::ALL);

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

    // Show failing pairs (or first few)
    let max_display = inner.height.saturating_sub(3) as usize;
    let failures: Vec<_> = state
        .validation_results
        .iter()
        .filter(|r| !r.passes)
        .take(max_display)
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

        if failing > max_display {
            lines.push(Line::from(Span::styled(
                format!("... and {} more failures", failing - max_display),
                Style::default().fg(Color::DarkGray),
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
        for warning in state.generation_warnings.iter().take(3) {
            lines.push(Line::from(Span::styled(
                format!("  {warning}"),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::DIM),
            )));
        }
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}
