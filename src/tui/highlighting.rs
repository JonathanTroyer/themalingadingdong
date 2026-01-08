//! Syntax highlighting engine using syntect with Base24 theme generation.

use std::str::FromStr;
use std::sync::LazyLock;

use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::{
    Color, FontStyle, ScopeSelectors, StyleModifier, Theme, ThemeItem, ThemeSettings,
};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use syntect_tui::into_span;
use tinted_builder::Base16Scheme;

/// Cached syntax set - expensive to load, so we cache it globally.
pub static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);

/// Syntax highlighter that generates themes from Base24 schemes.
pub struct Highlighter {
    theme: Theme,
}

impl Highlighter {
    /// Create a new highlighter from a Base24 color scheme.
    pub fn new(scheme: &Base16Scheme) -> Self {
        Self {
            theme: build_theme(scheme),
        }
    }

    /// Get the background color as ratatui Color.
    pub fn background_color(&self) -> ratatui::style::Color {
        self.theme
            .settings
            .background
            .map(|c| ratatui::style::Color::Rgb(c.r, c.g, c.b))
            .unwrap_or(ratatui::style::Color::Reset)
    }

    /// Highlight code and return ratatui Lines.
    pub fn highlight(&self, code: &str, extension: &str) -> Vec<Line<'static>> {
        let syntax = SYNTAX_SET
            .find_syntax_by_extension(extension)
            .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

        let mut highlighter = HighlightLines::new(syntax, &self.theme);

        // Use LinesWithEndings to preserve newlines for proper syntax state tracking.
        // Some syntaxes (like Bash) need newlines to correctly end comment scopes.
        LinesWithEndings::from(code)
            .map(|line| {
                let ranges = highlighter
                    .highlight_line(line, &SYNTAX_SET)
                    .unwrap_or_default();
                let spans: Vec<Span<'static>> = ranges
                    .into_iter()
                    .filter_map(|seg| {
                        into_span(seg).ok().map(|span| {
                            // Strip trailing newline and convert to owned span
                            let content = span.content.trim_end_matches('\n').to_string();
                            // Remove background from style so spans inherit widget background.
                            // into_span sets explicit backgrounds which cause visual artifacts
                            // on whitespace characters (tabs appearing as grey blocks).
                            // We patch the style to have bg: None rather than an explicit color.
                            let mut patched = ratatui::style::Style::new();
                            if let Some(fg) = span.style.fg {
                                patched = patched.fg(fg);
                            }
                            if span
                                .style
                                .add_modifier
                                .contains(ratatui::style::Modifier::BOLD)
                            {
                                patched = patched.add_modifier(ratatui::style::Modifier::BOLD);
                            }
                            if span
                                .style
                                .add_modifier
                                .contains(ratatui::style::Modifier::ITALIC)
                            {
                                patched = patched.add_modifier(ratatui::style::Modifier::ITALIC);
                            }
                            if span
                                .style
                                .add_modifier
                                .contains(ratatui::style::Modifier::UNDERLINED)
                            {
                                patched =
                                    patched.add_modifier(ratatui::style::Modifier::UNDERLINED);
                            }
                            Span::styled(content, patched)
                        })
                    })
                    // Filter out empty spans that may result from stripped newlines
                    .filter(|span| !span.content.is_empty())
                    .collect();
                Line::from(spans)
            })
            .collect()
    }
}

/// Build a syntect Theme from a Base24 scheme.
fn build_theme(scheme: &Base16Scheme) -> Theme {
    // Helper to get RGB from scheme
    let get_color = |name: &str| -> Color {
        scheme
            .palette
            .get(name)
            .map(|c| Color {
                r: c.rgb.0,
                g: c.rgb.1,
                b: c.rgb.2,
                a: 255,
            })
            .unwrap_or(Color::BLACK)
    };

    // Theme settings (editor chrome)
    let settings = ThemeSettings {
        foreground: Some(get_color("base05")),
        background: Some(get_color("base00")),
        caret: Some(get_color("base05")),
        selection: Some(get_color("base02")),
        line_highlight: Some(get_color("base01")),
        gutter: Some(get_color("base01")),
        gutter_foreground: Some(get_color("base04")),
        ..Default::default()
    };

    // Scope rules (syntax highlighting)
    let scopes = build_scope_rules(scheme);

    Theme {
        name: Some(scheme.name.clone()),
        author: Some(scheme.author.clone()),
        settings,
        scopes,
    }
}

/// Build scope rules mapping Base24 colors to syntax scopes.
/// Based on the official base16-textmate template.
fn build_scope_rules(scheme: &Base16Scheme) -> Vec<ThemeItem> {
    let get_color = |name: &str| -> Color {
        scheme
            .palette
            .get(name)
            .map(|c| Color {
                r: c.rgb.0,
                g: c.rgb.1,
                b: c.rgb.2,
                a: 255,
            })
            .unwrap_or(Color::BLACK)
    };

    // Helper to create a foreground-only ThemeItem
    let rule = |scope: &str, color: Color, font_style: Option<FontStyle>| -> ThemeItem {
        ThemeItem {
            scope: ScopeSelectors::from_str(scope).unwrap_or_default(),
            style: StyleModifier {
                foreground: Some(color),
                background: None,
                font_style,
            },
        }
    };

    vec![
        // base03: Comments
        rule(
            "comment, punctuation.definition.comment",
            get_color("base03"),
            Some(FontStyle::ITALIC),
        ),
        // base05: Default text, operators, punctuation, delimiters
        rule(
            "punctuation, meta.brace, keyword.operator, variable.parameter.function",
            get_color("base05"),
            None,
        ),
        // base07: Meta class (lightest foreground)
        rule("meta.class", get_color("base07"), None),
        // base08: Variables, XML tags, markup links/lists, diff deleted
        rule(
            "variable, entity.name.tag, markup.deleted, markup.list, string.other.link",
            get_color("base08"),
            None,
        ),
        // base09: Constants, numbers, booleans, attributes, units
        rule(
            "constant, constant.numeric, constant.language, constant.character, \
             entity.other.attribute-name, keyword.other.unit, meta.link, markup.quote",
            get_color("base09"),
            None,
        ),
        // base0A: Classes, types, markup bold
        rule(
            "entity.name.type, entity.name.class, support.type, support.class, markup.bold",
            get_color("base0A"),
            None,
        ),
        // base0B: Strings, inherited class, markup code/inserted
        rule(
            "string, constant.other.symbol, entity.other.inherited-class, \
             markup.inserted, markup.raw.inline",
            get_color("base0B"),
            None,
        ),
        // base0C: Support functions, regex, escape chars, colors
        rule(
            "support.function, string.regexp, constant.character.escape, constant.other.color",
            get_color("base0C"),
            None,
        ),
        // base0D: Functions, methods, attribute IDs, headings
        rule(
            "entity.name.function, meta.require, support.function.any-method, \
             variable.function, variable.annotation, support.macro, \
             keyword.other.special-method, entity.other.attribute-name.id, \
             punctuation.definition.entity, markup.heading, entity.name.section",
            get_color("base0D"),
            None,
        ),
        // base0E: Keywords, storage, selectors, markup italic/changed, interpolation
        rule(
            "keyword, storage, storage.type, storage.modifier, meta.selector, \
             markup.italic, markup.changed, punctuation.section.embedded, variable.interpolation",
            get_color("base0E"),
            Some(FontStyle::BOLD),
        ),
        // base0F: Labels, deprecated, embedded language tags
        rule(
            "entity.name.label, invalid.deprecated",
            get_color("base0F"),
            None,
        ),
    ]
}
