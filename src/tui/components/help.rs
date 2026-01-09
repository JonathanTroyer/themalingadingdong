//! Help modal component displaying keybindings.

use crossterm_actions::{AppEvent, NavigationEvent, SelectionEvent, TuiEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::tui::{AppAction, dispatcher};

/// Actions to display in a keybinding group.
struct KeybindingGroup {
    title: &'static str,
    actions: &'static [AppAction],
}

/// Groups of actions for the help modal, with semantic organization.
const HELP_GROUPS: &[KeybindingGroup] = &[
    KeybindingGroup {
        title: "Global",
        actions: &[
            AppAction::Tui(TuiEvent::App(AppEvent::Quit)),
            AppAction::Tui(TuiEvent::App(AppEvent::Help)),
            AppAction::Tui(TuiEvent::App(AppEvent::Refresh)),
        ],
    },
    KeybindingGroup {
        title: "Views",
        actions: &[AppAction::CodePreview],
    },
    KeybindingGroup {
        title: "Focus Navigation",
        actions: &[
            AppAction::Tui(TuiEvent::Selection(SelectionEvent::Next)),
            AppAction::Tui(TuiEvent::Selection(SelectionEvent::Prev)),
        ],
    },
    KeybindingGroup {
        title: "Component Navigation",
        actions: &[
            AppAction::Tui(TuiEvent::Navigation(NavigationEvent::Up)),
            AppAction::Tui(TuiEvent::Navigation(NavigationEvent::Down)),
            AppAction::Tui(TuiEvent::Navigation(NavigationEvent::Left)),
            AppAction::Tui(TuiEvent::Navigation(NavigationEvent::Right)),
        ],
    },
    KeybindingGroup {
        title: "Value Adjustment",
        actions: &[
            AppAction::ValueDecrementSmall,
            AppAction::ValueIncrementSmall,
            AppAction::ValueDecrementLarge,
            AppAction::ValueIncrementLarge,
        ],
    },
];

/// Extra keybindings not in dispatcher (component-specific behaviors).
/// Format: (description, keys)
const EXTRA_BINDINGS: &[(&str, &str)] = &[
    ("Edit hue value", "Enter"),
    ("Quick edit", "0-9"),
    ("Cancel edit", "Esc"),
    ("Delete character", "Backspace"),
];

/// Actions shown in the main activity footer.
pub const MAIN_FOOTER_ACTIONS: &[AppAction] = &[
    AppAction::Tui(TuiEvent::Selection(SelectionEvent::Next)),
    AppAction::CodePreview,
    AppAction::Tui(TuiEvent::App(AppEvent::Help)),
    AppAction::Tui(TuiEvent::App(AppEvent::Quit)),
    AppAction::Tui(TuiEvent::App(AppEvent::Refresh)),
];

/// Actions shown in the code preview footer.
pub const CODE_PREVIEW_FOOTER_ACTIONS: &[AppAction] = &[
    AppAction::Tui(TuiEvent::Selection(SelectionEvent::Next)),
    AppAction::CodePreview,
    AppAction::Tui(TuiEvent::App(AppEvent::Help)),
    AppAction::Tui(TuiEvent::App(AppEvent::Quit)),
];

/// Format a footer string from a list of actions.
/// Format: "desc: key | desc: key" (description first, single key binding)
pub fn format_footer(actions: &[AppAction], extras: &[(&str, &str)]) -> String {
    let help_entries = dispatcher().config().help_entries();
    let mut parts: Vec<String> = Vec::new();

    for action in actions {
        if let Some(entry) = help_entries.get(action)
            && let (Some(key), Some(desc)) = (entry.keys.first(), entry.description)
        {
            let short_desc = desc
                .split_whitespace()
                .next()
                .unwrap_or(desc)
                .to_lowercase();
            parts.push(format!("{short_desc}: {key}"));
        }
    }

    for (desc, key) in extras {
        parts.push(format!("{desc}: {key}"));
    }

    parts.join(" | ")
}

/// Calculate a centered popup area with given width/height percentages.
fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

/// Render the help modal overlay.
pub fn render_help(frame: &mut Frame) {
    let area = popup_area(frame.area(), 50, 70);
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split inner area: content at top, footer at bottom
    let layout = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(inner);
    let content_area = layout[0];
    let footer_area = layout[1];

    let bold = Style::default().add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::Gray);

    let mut lines = vec![
        Line::from(Span::styled("Keybindings", bold)),
        Line::from(""),
    ];

    // Get help entries from dispatcher
    let help_entries = dispatcher().config().help_entries();

    for group in HELP_GROUPS {
        lines.push(Line::from(Span::styled(group.title, bold)));
        for action in group.actions {
            if let Some(entry) = help_entries.get(action) {
                let keys_str = entry
                    .keys
                    .iter()
                    .map(|k| k.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                let desc = entry.description.unwrap_or("(no description)");
                lines.push(Line::from(vec![
                    Span::styled(format!("  {desc:<20}"), Style::default()),
                    Span::styled(keys_str, dim),
                ]));
            }
        }
        lines.push(Line::from(""));
    }

    // Extra component-specific bindings
    lines.push(Line::from(Span::styled("Hue Override Editing", bold)));
    for (description, keys) in EXTRA_BINDINGS {
        lines.push(Line::from(vec![
            Span::styled(format!("  {description:<20}"), Style::default()),
            Span::styled(*keys, dim),
        ]));
    }
    lines.push(Line::from(""));

    let content = Paragraph::new(lines);
    frame.render_widget(content, content_area);

    let footer = Paragraph::new(Line::from(Span::styled(
        "Press Esc, ?, or Enter to close",
        Style::default()
            .fg(Color::Gray)
            .add_modifier(Modifier::ITALIC),
    )))
    .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(footer, footer_area);
}
