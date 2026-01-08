//! Code preview activity - displays syntax-highlighted code snippets.

use std::io::Stdout;
use std::time::Duration;

use color_eyre::eyre::Result;
use crossterm_actions::{NavigationEvent, SelectionEvent, TuiEvent};
use ratatui::{
    Terminal,
    layout::{Constraint, Direction, Layout, Rect},
    prelude::CrosstermBackend,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use tuirealm::{
    Application, Component, Event, EventListenerCfg, MockComponent, PollStrategy, State,
    StateValue,
    command::{Cmd, CmdResult, Direction as CmdDirection},
    props::{AttrValue, Attribute, Props},
};

use crate::tui::activity::{Activity, Context, ExitReason};
use crate::tui::components::{CODE_PREVIEW_FOOTER_ACTIONS, format_footer};
use crate::tui::highlighting::Highlighter;
use crate::tui::snippets::SNIPPETS;
use crate::tui::{AppAction, dispatcher, handle_global_app_events};

// ============================================================================
// Component identifiers (scoped to CodePreviewActivity)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Id {
    CodeView,
}

// ============================================================================
// Messages (scoped to CodePreviewActivity)
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Msg {
    Quit,
    Back,
    NextLanguage,
    PrevLanguage,
    ScrollUp,
    ScrollDown,
}

// ============================================================================
// User events (required by tui-realm)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserEvent {}

// ============================================================================
// CodeView Component
// ============================================================================

/// Colors for the code view from the theme.
pub struct CodeViewColors {
    pub background: Color,
    pub gutter_fg: Color,
    pub border: Color,
}

pub struct CodeView {
    props: Props,
    lines: Vec<Line<'static>>,
    scroll: usize,
    visible_height: usize,
    colors: CodeViewColors,
}

impl CodeView {
    pub fn new(colors: CodeViewColors) -> Self {
        Self {
            props: Props::default(),
            lines: Vec::new(),
            scroll: 0,
            visible_height: 20,
            colors,
        }
    }

    pub fn set_lines(&mut self, lines: Vec<Line<'static>>) {
        self.lines = lines;
        self.scroll = 0;
    }

    fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    fn scroll_down(&mut self) {
        let max_scroll = self.lines.len().saturating_sub(self.visible_height);
        self.scroll = (self.scroll + 1).min(max_scroll);
    }
}

impl MockComponent for CodeView {
    fn view(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        let bg_style = Style::default().bg(self.colors.background);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(bg_style.fg(self.colors.border))
            .style(bg_style);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        self.visible_height = inner.height as usize;

        // Render highlighted lines with line numbers
        let visible_lines: Vec<Line> = self
            .lines
            .iter()
            .enumerate()
            .skip(self.scroll)
            .take(inner.height as usize)
            .map(|(i, line)| {
                let line_num = format!("{:4} ", i + 1);
                let mut spans = vec![ratatui::text::Span::styled(
                    line_num,
                    Style::default()
                        .fg(self.colors.gutter_fg)
                        .bg(self.colors.background),
                )];
                spans.extend(line.spans.clone());
                Line::from(spans)
            })
            .collect();

        let code_widget = Paragraph::new(visible_lines).style(bg_style);
        frame.render_widget(code_widget, inner);

        // Scrollbar
        if self.lines.len() > inner.height as usize {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
            let mut scrollbar_state = ScrollbarState::new(self.lines.len()).position(self.scroll);
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
        State::One(StateValue::Usize(self.scroll))
    }

    fn perform(&mut self, cmd: Cmd) -> CmdResult {
        match cmd {
            Cmd::Scroll(CmdDirection::Up) => {
                self.scroll_up();
                CmdResult::Changed(self.state())
            }
            Cmd::Scroll(CmdDirection::Down) => {
                self.scroll_down();
                CmdResult::Changed(self.state())
            }
            _ => CmdResult::None,
        }
    }
}

impl Component<Msg, UserEvent> for CodeView {
    fn on(&mut self, ev: Event<UserEvent>) -> Option<Msg> {
        // Extract keyboard event
        let Event::Keyboard(key_event) = ev else {
            return None;
        };

        // Handle Esc for going back (not mapped in dispatcher)
        if key_event.code == tuirealm::event::Key::Esc {
            return Some(Msg::Back);
        }

        // Use dispatcher to convert to semantic action
        let action = dispatcher().dispatch(&key_event)?;

        if let Some(msg) = handle_global_app_events(&action) {
            // Convert global Msg to our local Msg
            return match msg {
                crate::tui::activities::Msg::Quit => Some(Msg::Quit),
                crate::tui::activities::Msg::SwitchToCodePreview => Some(Msg::Back), // Toggle back
                _ => None,
            };
        }

        match action {
            // Scrolling
            AppAction::Tui(TuiEvent::Navigation(NavigationEvent::Up)) => {
                self.perform(Cmd::Scroll(CmdDirection::Up));
                Some(Msg::ScrollUp)
            }
            AppAction::Tui(TuiEvent::Navigation(NavigationEvent::Down)) => {
                self.perform(Cmd::Scroll(CmdDirection::Down));
                Some(Msg::ScrollDown)
            }

            // Language switching (Tab/Shift+Tab)
            AppAction::Tui(TuiEvent::Selection(SelectionEvent::Next)) => Some(Msg::NextLanguage),
            AppAction::Tui(TuiEvent::Selection(SelectionEvent::Prev)) => Some(Msg::PrevLanguage),

            _ => None,
        }
    }
}

// ============================================================================
// CodePreviewActivity
// ============================================================================

#[derive(Default)]
pub struct CodePreviewActivity {
    app: Option<Application<Id, Msg, UserEvent>>,
    context: Option<Context>,
    exit_reason: Option<ExitReason>,
    highlighter: Option<Highlighter>,
    current_language: usize,
    needs_clear: bool,
}

impl CodePreviewActivity {
    fn create_application() -> Application<Id, Msg, UserEvent> {
        Application::init(
            EventListenerCfg::default()
                .crossterm_input_listener(Duration::from_millis(20), 10)
                .poll_timeout(Duration::from_millis(50)),
        )
    }

    fn code_view_colors(&self) -> CodeViewColors {
        self.context
            .as_ref()
            .and_then(|ctx| ctx.model.current_scheme.as_ref())
            .map(|scheme| {
                let get_color = |name: &str| {
                    scheme
                        .palette
                        .get(name)
                        .map(|c| Color::Rgb(c.rgb.0, c.rgb.1, c.rgb.2))
                        .unwrap_or(Color::Reset)
                };
                CodeViewColors {
                    background: get_color("base00"),
                    gutter_fg: get_color("base04"),
                    border: get_color("base01"),
                }
            })
            .unwrap_or(CodeViewColors {
                background: Color::Reset,
                gutter_fg: Color::DarkGray,
                border: Color::DarkGray,
            })
    }

    fn highlight_current_snippet(&self) -> Vec<Line<'static>> {
        if let Some(ref highlighter) = self.highlighter {
            let snippet = &SNIPPETS[self.current_language];
            highlighter.highlight(snippet.code, snippet.extension)
        } else {
            Vec::new()
        }
    }

    fn next_language(&mut self) {
        self.current_language = (self.current_language + 1) % SNIPPETS.len();
        self.needs_clear = true;
        self.update_code_view();
    }

    fn prev_language(&mut self) {
        self.current_language = (self.current_language + SNIPPETS.len() - 1) % SNIPPETS.len();
        self.needs_clear = true;
        self.update_code_view();
    }

    fn update_code_view(&mut self) {
        let lines = self.highlight_current_snippet();
        let colors = self.code_view_colors();
        if let Some(ref mut app) = self.app {
            let _ = app.umount(&Id::CodeView);
            let mut code_view = CodeView::new(colors);
            code_view.set_lines(lines);
            let _ = app.mount(Id::CodeView, Box::new(code_view), vec![]);
            let _ = app.active(&Id::CodeView);
        }
    }
}

impl Activity for CodePreviewActivity {
    fn on_create(&mut self, context: Context) {
        // Create highlighter from current scheme
        if let Some(ref scheme) = context.model.current_scheme {
            self.highlighter = Some(Highlighter::new(scheme));
        }
        self.context = Some(context);

        // Create application and mount component
        let mut app = Self::create_application();
        let mut code_view = CodeView::new(self.code_view_colors());
        code_view.set_lines(self.highlight_current_snippet());
        let _ = app.mount(Id::CodeView, Box::new(code_view), vec![]);
        let _ = app.active(&Id::CodeView);

        self.app = Some(app);
    }

    fn on_draw(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        // Force full redraw when switching languages to prevent ghost text
        if self.needs_clear {
            terminal.clear()?;
            self.needs_clear = false;
        }

        let app = self.app.as_mut().expect("app should be initialized");
        let snippet = &SNIPPETS[self.current_language];

        // Draw UI
        terminal.draw(|frame| {
            let area = frame.area();

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // Title
                    Constraint::Min(10),   // Code
                    Constraint::Length(1), // Status
                ])
                .split(area);

            // Title bar with language tabs
            let title = format!(" Code Preview - [{}]", snippet.display_name);
            let title_widget =
                Paragraph::new(title).style(Style::default().add_modifier(Modifier::BOLD));
            frame.render_widget(title_widget, rows[0]);

            // Code view
            app.view(&Id::CodeView, frame, rows[1]);

            // Status bar
            let status = format_footer(CODE_PREVIEW_FOOTER_ACTIONS, &[]);
            let status_widget =
                Paragraph::new(status).style(Style::default().add_modifier(Modifier::DIM));
            frame.render_widget(status_widget, rows[2]);
        })?;

        // Process events through tui-realm
        match app.tick(PollStrategy::Once) {
            Ok(messages) => {
                for msg in messages {
                    match msg {
                        Msg::Quit => {
                            self.exit_reason = Some(ExitReason::Quit);
                            return Ok(());
                        }
                        Msg::Back => {
                            self.exit_reason = Some(ExitReason::SwitchToMain);
                            return Ok(());
                        }
                        Msg::NextLanguage => self.next_language(),
                        Msg::PrevLanguage => self.prev_language(),
                        Msg::ScrollUp | Msg::ScrollDown => {
                            // Already handled in component
                        }
                    }
                }
            }
            Err(_) => {
                // Timeout, continue
            }
        }

        Ok(())
    }

    fn will_umount(&self) -> Option<&ExitReason> {
        self.exit_reason.as_ref()
    }

    fn on_destroy(&mut self) -> Option<Context> {
        self.app = None;
        self.context.take()
    }
}
