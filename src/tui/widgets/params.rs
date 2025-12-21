//! Parameter editing widget.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::cli::VariantArg;
use crate::tui::state::{Focus, TuiState};

/// Default hues for accent colors (degrees).
const DEFAULT_HUES: [f32; 8] = [25.0, 55.0, 90.0, 145.0, 180.0, 250.0, 285.0, 335.0];

/// Hue color names.
const HUE_NAMES: [&str; 8] = [
    "Red", "Orange", "Yellow", "Green", "Cyan", "Blue", "Purple", "Magenta",
];

/// Draw the parameters panel.
pub fn draw_parameters(frame: &mut Frame, area: Rect, state: &TuiState) {
    let block = Block::default().title(" Parameters ").borders(Borders::ALL);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Background
            Constraint::Length(1), // Foreground
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Target Contrast
            Constraint::Length(1), // Extended Contrast
            Constraint::Length(1), // Accent Chroma
            Constraint::Length(1), // Extended Chroma
            Constraint::Length(1), // Variant
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Hue header
            Constraint::Length(1), // Hue row 1
            Constraint::Length(1), // Hue row 2
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Name
            Constraint::Length(1), // Author
            Constraint::Min(0),    // Remaining space
        ])
        .split(inner);

    // Text fields
    draw_text_field(
        frame,
        rows[0],
        "Background",
        &state.background_hex,
        state.focus == Focus::Background,
        state.editing_text && state.focus == Focus::Background,
        state.text_cursor,
    );
    draw_text_field(
        frame,
        rows[1],
        "Foreground",
        &state.foreground_hex,
        state.focus == Focus::Foreground,
        state.editing_text && state.focus == Focus::Foreground,
        state.text_cursor,
    );

    // Sliders
    draw_slider(
        frame,
        rows[3],
        "Target Contrast",
        state.target_contrast,
        30.0,
        100.0,
        state.focus == Focus::TargetContrast,
    );
    draw_slider(
        frame,
        rows[4],
        "Ext. Contrast",
        state.extended_contrast,
        30.0,
        100.0,
        state.focus == Focus::ExtendedContrast,
    );
    draw_slider(
        frame,
        rows[5],
        "Accent Chroma",
        f64::from(state.accent_chroma) * 100.0,
        0.0,
        40.0,
        state.focus == Focus::AccentChroma,
    );
    draw_slider(
        frame,
        rows[6],
        "Ext. Chroma",
        f64::from(state.extended_chroma) * 100.0,
        0.0,
        40.0,
        state.focus == Focus::ExtendedChroma,
    );

    // Variant selector
    draw_variant_select(frame, rows[7], state.variant, state.focus == Focus::Variant);

    // Hue header
    let header =
        Paragraph::new("Hue Overrides:").style(Style::default().add_modifier(Modifier::DIM));
    frame.render_widget(header, rows[9]);

    // Hue overrides - first row (4 hues)
    let hue_row1 = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
        ])
        .split(rows[10]);

    for (i, col_area) in hue_row1.iter().enumerate() {
        let focus = match i {
            0 => Focus::Hue08,
            1 => Focus::Hue09,
            2 => Focus::Hue0A,
            3 => Focus::Hue0B,
            _ => Focus::Hue08,
        };
        draw_hue_input(
            frame,
            *col_area,
            HUE_NAMES[i],
            state.hue_overrides[i],
            DEFAULT_HUES[i],
            state.focus == focus,
        );
    }

    // Hue overrides - second row (4 hues)
    let hue_row2 = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
        ])
        .split(rows[11]);

    for (i, col_area) in hue_row2.iter().enumerate() {
        let idx = i + 4;
        let focus = match idx {
            4 => Focus::Hue0C,
            5 => Focus::Hue0D,
            6 => Focus::Hue0E,
            7 => Focus::Hue0F,
            _ => Focus::Hue0C,
        };
        draw_hue_input(
            frame,
            *col_area,
            HUE_NAMES[idx],
            state.hue_overrides[idx],
            DEFAULT_HUES[idx],
            state.focus == focus,
        );
    }

    // Name and Author
    draw_text_field(
        frame,
        rows[13],
        "Name",
        &state.name,
        state.focus == Focus::Name,
        state.editing_text && state.focus == Focus::Name,
        state.text_cursor,
    );
    draw_text_field(
        frame,
        rows[14],
        "Author",
        &state.author,
        state.focus == Focus::Author,
        state.editing_text && state.focus == Focus::Author,
        state.text_cursor,
    );
}

fn draw_text_field(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    focused: bool,
    editing: bool,
    cursor: usize,
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

    let label_text = Paragraph::new(format!("{label}:")).style(label_style);
    frame.render_widget(label_text, cols[0]);

    let value_style = if editing {
        Style::default().bg(Color::DarkGray).fg(Color::White)
    } else if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    // Show cursor when editing
    let display_value = if editing {
        let mut v = value.to_string();
        let pos = cursor.min(v.len());
        v.insert(pos, '|');
        v
    } else {
        value.to_string()
    };

    let value_text = Paragraph::new(display_value).style(value_style);
    frame.render_widget(value_text, cols[1]);
}

fn draw_slider(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: f64,
    min: f64,
    max: f64,
    focused: bool,
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

    let label_text = Paragraph::new(format!("{label}:")).style(label_style);
    frame.render_widget(label_text, cols[0]);

    // Calculate slider position
    let slider_width = cols[1].width.saturating_sub(8) as f64;
    let ratio = (value - min) / (max - min);
    let pos = (ratio * slider_width) as usize;

    // Build slider string
    let mut slider = String::new();
    slider.push('[');
    for i in 0..slider_width as usize {
        if i == pos {
            slider.push('|');
        } else {
            slider.push('=');
        }
    }
    slider.push(']');
    slider.push_str(&format!(" {value:.1}"));

    let slider_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let slider_text = Paragraph::new(slider).style(slider_style);
    frame.render_widget(slider_text, cols[1]);
}

fn draw_variant_select(frame: &mut Frame, area: Rect, variant: VariantArg, focused: bool) {
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

    let label_text = Paragraph::new("Variant:").style(label_style);
    frame.render_widget(label_text, cols[0]);

    let variants = ["Auto", "Dark", "Light"];
    let current = match variant {
        VariantArg::Auto => 0,
        VariantArg::Dark => 1,
        VariantArg::Light => 2,
        VariantArg::Both => 0,
    };

    let mut spans = Vec::new();
    spans.push(Span::raw("< "));
    for (i, v) in variants.iter().enumerate() {
        if i == current {
            spans.push(Span::styled(
                format!("[{v}]"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {v} "),
                Style::default().add_modifier(Modifier::DIM),
            ));
        }
    }
    spans.push(Span::raw(" >"));

    let value_style = if focused {
        Style::default()
    } else {
        Style::default().add_modifier(Modifier::DIM)
    };

    let value_text = Paragraph::new(Line::from(spans)).style(value_style);
    frame.render_widget(value_text, cols[1]);
}

fn draw_hue_input(
    frame: &mut Frame,
    area: Rect,
    name: &str,
    value: Option<f32>,
    default: f32,
    focused: bool,
) {
    let actual = value.unwrap_or(default);
    let is_override = value.is_some();

    let style = if focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else if is_override {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().add_modifier(Modifier::DIM)
    };

    let display = format!("{}: {:.0}°", &name[..2], actual);
    let text = Paragraph::new(display).style(style);
    frame.render_widget(text, area);
}
