//! Simple line-by-line markdown renderer for SKILL.md preview.
//!
//! Handles headers (`#`, `##`, `###`), horizontal rules (`---`),
//! and inline spans (`**bold**`, `*italic*`, `` `code` ``).
//! No nested delimiter support -- keeps the parser simple.

use std::sync::OnceLock;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Theme as SyntectTheme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};

use super::theme::Theme;

struct HighlightAssets {
    syntaxes: SyntaxSet,
    themes: ThemeSet,
}

impl HighlightAssets {
    fn load() -> Self {
        Self {
            syntaxes: SyntaxSet::load_defaults_nonewlines(),
            themes: ThemeSet::load_defaults(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodeColorLevel {
    None,
    Ansi,
    TrueColor,
}

static HIGHLIGHT_ASSETS: OnceLock<HighlightAssets> = OnceLock::new();

/// Render raw markdown text into styled `Line`s for the preview panel.
pub fn render_markdown(raw: &str, theme: &Theme) -> Vec<Line<'static>> {
    render_markdown_with_color_level(raw, theme, detect_code_color_level())
}

fn render_markdown_with_color_level(
    raw: &str,
    theme: &Theme,
    color_level: CodeColorLevel,
) -> Vec<Line<'static>> {
    let mut rendered = Vec::new();
    let mut in_code_block: Option<CodeBlockState> = None;

    for line in raw.lines() {
        if let Some(fence) = parse_fence(line) {
            match in_code_block.take() {
                Some(state) if state.fence_marker == fence.marker => {
                    rendered.extend(highlight_code_block(
                        &state.lines,
                        state.syntax,
                        theme,
                        color_level,
                    ));
                    rendered.push(render_code_fence(line, theme, color_level));
                }
                Some(state) => {
                    let mut state = state;
                    state.lines.push(line.to_string());
                    in_code_block = Some(state);
                }
                None => {
                    let syntax = resolve_syntax(fence.info_string);
                    rendered.push(render_code_fence(line, theme, color_level));
                    in_code_block = Some(CodeBlockState {
                        fence_marker: fence.marker,
                        syntax,
                        lines: Vec::new(),
                    });
                }
            }
            continue;
        }

        if let Some(state) = in_code_block.as_mut() {
            state.lines.push(line.to_string());
        } else {
            rendered.push(render_line(line, theme));
        }
    }

    if let Some(state) = in_code_block.take() {
        rendered.extend(highlight_code_block(
            &state.lines,
            state.syntax,
            theme,
            color_level,
        ));
    }

    rendered
}

fn render_line(line: &str, theme: &Theme) -> Line<'static> {
    // Headers: # / ## / ###
    if let Some(rest) = line.strip_prefix("### ") {
        return Line::from(Span::styled(rest.to_string(), theme.preview_header()));
    }
    if let Some(rest) = line.strip_prefix("## ") {
        return Line::from(Span::styled(rest.to_string(), theme.preview_header()));
    }
    if let Some(rest) = line.strip_prefix("# ") {
        return Line::from(Span::styled(rest.to_string(), theme.preview_header()));
    }

    // Horizontal rule
    if line.starts_with("---") {
        return Line::from(Span::styled(
            "\u{2500}".repeat(40),
            Style::default().fg(theme.muted),
        ));
    }

    // Inline markdown
    render_inline_markdown(line, theme)
}

/// Scan left-to-right for delimiter pairs: `**`, `*`, backtick.
fn render_inline_markdown(line: &str, theme: &Theme) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut plain_start = 0;

    while i < len {
        // Check for ** (bold)
        if i + 1 < len
            && chars[i] == '*'
            && chars[i + 1] == '*'
            && let Some(end) = find_double_star(&chars, i + 2)
        {
            // Flush plain text before this
            if plain_start < i {
                let text: String = chars[plain_start..i].iter().collect();
                spans.push(Span::raw(text));
            }
            let content: String = chars[i + 2..end].iter().collect();
            spans.push(Span::styled(content, theme.preview_bold()));
            i = end + 2;
            plain_start = i;
            continue;
        }

        // Check for backtick (code)
        if chars[i] == '`'
            && let Some(end) = find_char(&chars, '`', i + 1)
        {
            if plain_start < i {
                let text: String = chars[plain_start..i].iter().collect();
                spans.push(Span::raw(text));
            }
            let content: String = chars[i + 1..end].iter().collect();
            spans.push(Span::styled(content, theme.preview_code()));
            i = end + 1;
            plain_start = i;
            continue;
        }

        // Check for single * (italic) -- must not be **
        if chars[i] == '*'
            && !(i + 1 < len && chars[i + 1] == '*')
            && let Some(end) = find_single_star(&chars, i + 1)
        {
            if plain_start < i {
                let text: String = chars[plain_start..i].iter().collect();
                spans.push(Span::raw(text));
            }
            let content: String = chars[i + 1..end].iter().collect();
            spans.push(Span::styled(content, theme.preview_italic()));
            i = end + 1;
            plain_start = i;
            continue;
        }

        i += 1;
    }

    // Flush remaining plain text
    if plain_start < len {
        let text: String = chars[plain_start..].iter().collect();
        spans.push(Span::raw(text));
    }

    if spans.is_empty() {
        Line::from("")
    } else {
        Line::from(spans)
    }
}

struct Fence<'a> {
    marker: char,
    info_string: Option<&'a str>,
}

struct CodeBlockState {
    fence_marker: char,
    syntax: Option<&'static SyntaxReference>,
    lines: Vec<String>,
}

fn parse_fence(line: &str) -> Option<Fence<'_>> {
    let trimmed = line.trim_start();
    for marker in ['`', '~'] {
        if let Some(rest) = trimmed.strip_prefix(marker)
            && let Some(rest) = rest.strip_prefix(marker)
            && let Some(rest) = rest.strip_prefix(marker)
        {
            return Some(Fence {
                marker,
                info_string: {
                    let info = rest.trim();
                    (!info.is_empty()).then_some(info)
                },
            });
        }
    }
    None
}

fn render_code_fence(line: &str, theme: &Theme, color_level: CodeColorLevel) -> Line<'static> {
    if matches!(color_level, CodeColorLevel::None) {
        Line::from(line.to_string())
    } else {
        Line::from(Span::styled(line.to_string(), theme.preview_code_fence()))
    }
}

fn resolve_syntax(info_string: Option<&str>) -> Option<&'static SyntaxReference> {
    let token = info_string
        .map(normalize_info_token)
        .filter(|token| !token.is_empty())?
        .to_ascii_lowercase();
    let assets = HIGHLIGHT_ASSETS.get_or_init(HighlightAssets::load);
    let alias = syntax_alias(&token);
    assets
        .syntaxes
        .find_syntax_by_token(alias)
        .or_else(|| assets.syntaxes.find_syntax_by_token(&token))
}

fn normalize_info_token(info_string: &str) -> &str {
    info_string
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim_matches(|c: char| matches!(c, '{' | '}' | ',' | '.'))
}

fn syntax_alias(token: &str) -> &str {
    match token {
        "js" => "javascript",
        "ts" => "typescript",
        "py" => "python",
        "rb" => "ruby",
        "rs" => "rust",
        "sh" | "shell" | "zsh" => "bash",
        "yml" => "yaml",
        "md" => "markdown",
        _ => token,
    }
}

fn highlight_code_block(
    lines: &[String],
    syntax: Option<&'static SyntaxReference>,
    theme: &Theme,
    color_level: CodeColorLevel,
) -> Vec<Line<'static>> {
    match (syntax, color_level) {
        (_, CodeColorLevel::None) => lines.iter().cloned().map(Line::from).collect(),
        (Some(syntax), _) => {
            let assets = HIGHLIGHT_ASSETS.get_or_init(HighlightAssets::load);
            let mut highlighter = HighlightLines::new(syntax, syntax_theme(theme, &assets.themes));
            lines
                .iter()
                .map(
                    |line| match highlighter.highlight_line(line, &assets.syntaxes) {
                        Ok(ranges) => {
                            let spans: Vec<Span<'static>> = ranges
                                .into_iter()
                                .filter(|(_, text)| !text.is_empty())
                                .map(|(style, text)| {
                                    Span::styled(
                                        text.to_string(),
                                        syntect_style_to_ratatui(style, color_level),
                                    )
                                })
                                .collect();
                            if spans.is_empty() {
                                Line::from("")
                            } else {
                                Line::from(spans)
                            }
                        }
                        Err(_) => Line::from(Span::styled(line.clone(), theme.preview_code())),
                    },
                )
                .collect()
        }
        (None, _) => lines
            .iter()
            .map(|line| Line::from(Span::styled(line.clone(), theme.preview_code())))
            .collect(),
    }
}

fn syntax_theme<'a>(theme: &Theme, themes: &'a ThemeSet) -> &'a SyntectTheme {
    let name = if theme.is_light() {
        "base16-ocean.light"
    } else {
        "base16-ocean.dark"
    };
    themes
        .themes
        .get(name)
        .expect("syntect default theme set should contain browse theme mapping")
}

fn syntect_style_to_ratatui(
    syntect_style: syntect::highlighting::Style,
    color_level: CodeColorLevel,
) -> Style {
    let mut style = Style::default();
    style = match color_level {
        CodeColorLevel::None => style,
        CodeColorLevel::TrueColor => style.fg(Color::Rgb(
            syntect_style.foreground.r,
            syntect_style.foreground.g,
            syntect_style.foreground.b,
        )),
        CodeColorLevel::Ansi => style.fg(ansi_color(syntect_style.foreground)),
    };

    if syntect_style.font_style.contains(FontStyle::BOLD) {
        style = style.add_modifier(Modifier::BOLD);
    }
    if syntect_style.font_style.contains(FontStyle::ITALIC) {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if syntect_style.font_style.contains(FontStyle::UNDERLINE) {
        style = style.add_modifier(Modifier::UNDERLINED);
    }

    style
}

fn detect_code_color_level() -> CodeColorLevel {
    if std::env::var_os("NO_COLOR").is_some() {
        return CodeColorLevel::None;
    }

    let colorterm = std::env::var("COLORTERM").unwrap_or_default();
    if colorterm.contains("truecolor") || colorterm.contains("24bit") {
        CodeColorLevel::TrueColor
    } else {
        CodeColorLevel::Ansi
    }
}

fn ansi_color(color: syntect::highlighting::Color) -> Color {
    const PALETTE: &[(Color, (i16, i16, i16))] = &[
        (Color::Black, (0, 0, 0)),
        (Color::Red, (205, 49, 49)),
        (Color::Green, (13, 188, 121)),
        (Color::Yellow, (229, 229, 16)),
        (Color::Blue, (36, 114, 200)),
        (Color::Magenta, (188, 63, 188)),
        (Color::Cyan, (17, 168, 205)),
        (Color::Gray, (229, 229, 229)),
        (Color::DarkGray, (102, 102, 102)),
        (Color::LightRed, (241, 76, 76)),
        (Color::LightGreen, (35, 209, 139)),
        (Color::LightYellow, (245, 245, 67)),
        (Color::LightBlue, (59, 142, 234)),
        (Color::LightMagenta, (214, 112, 214)),
        (Color::LightCyan, (41, 184, 219)),
        (Color::White, (255, 255, 255)),
    ];

    let (r, g, b) = (i16::from(color.r), i16::from(color.g), i16::from(color.b));
    PALETTE
        .iter()
        .min_by_key(|(_, (pr, pg, pb))| {
            let dr = r - pr;
            let dg = g - pg;
            let db = b - pb;
            dr * dr + dg * dg + db * db
        })
        .map(|(ansi, _)| *ansi)
        .unwrap_or(Color::Reset)
}

/// Find closing `**` starting from position `start`.
fn find_double_star(chars: &[char], start: usize) -> Option<usize> {
    let mut i = start;
    while i + 1 < chars.len() {
        if chars[i] == '*' && chars[i + 1] == '*' {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Find a single `*` that is not part of `**`.
fn find_single_star(chars: &[char], start: usize) -> Option<usize> {
    let mut i = start;
    while i < chars.len() {
        if chars[i] == '*' && !(i + 1 < chars.len() && chars[i + 1] == '*') {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Find a specific character starting from `start`.
fn find_char(chars: &[char], ch: char, start: usize) -> Option<usize> {
    chars[start..]
        .iter()
        .position(|&c| c == ch)
        .map(|p| p + start)
}

#[cfg(test)]
mod tests {
    use ratatui::style::{Color, Modifier};

    use super::*;

    #[test]
    fn test_header_rendering() {
        let theme = Theme::dark();
        let lines = render_markdown("# Hello", &theme);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans.len(), 1);
        assert_eq!(lines[0].spans[0].content, "Hello");
        assert!(
            lines[0].spans[0]
                .style
                .add_modifier
                .contains(Modifier::BOLD)
        );
    }

    #[test]
    fn test_h2_header() {
        let theme = Theme::dark();
        let lines = render_markdown("## Sub", &theme);
        assert_eq!(lines[0].spans[0].content, "Sub");
        assert!(
            lines[0].spans[0]
                .style
                .add_modifier
                .contains(Modifier::BOLD)
        );
    }

    #[test]
    fn test_hr_rendering() {
        let theme = Theme::dark();
        let lines = render_markdown("---", &theme);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].spans[0].content.contains('\u{2500}'));
    }

    #[test]
    fn test_inline_bold() {
        let theme = Theme::dark();
        let lines = render_markdown("hello **world**", &theme);
        assert_eq!(lines.len(), 1);
        // Should have "hello " as raw + "world" as bold
        assert!(lines[0].spans.len() >= 2);
        let bold_span = lines[0]
            .spans
            .iter()
            .find(|s| s.content == "world")
            .expect("should have 'world' span");
        assert!(bold_span.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_inline_code() {
        let theme = Theme::dark();
        let lines = render_markdown("hello `code` end", &theme);
        assert_eq!(lines.len(), 1);
        let code_span = lines[0]
            .spans
            .iter()
            .find(|s| s.content == "code")
            .expect("should have 'code' span");
        assert_eq!(code_span.style.fg, Some(Color::Magenta));
    }

    #[test]
    fn test_inline_italic() {
        let theme = Theme::dark();
        let lines = render_markdown("hello *world* end", &theme);
        let italic_span = lines[0]
            .spans
            .iter()
            .find(|s| s.content == "world")
            .expect("should have 'world' span");
        assert!(italic_span.style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn test_plain_text() {
        let theme = Theme::dark();
        let lines = render_markdown("just plain text", &theme);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans[0].content, "just plain text");
    }

    #[test]
    fn test_multiline() {
        let theme = Theme::dark();
        let lines = render_markdown("# Title\n\nSome text\n---\nMore", &theme);
        assert_eq!(lines.len(), 5);
    }

    #[test]
    fn test_recognized_fenced_code_gets_syntax_highlighted() {
        let theme = Theme::dark();
        let lines = render_markdown_with_color_level(
            "```rust\nfn main() { println!(\"hi\"); }\n```",
            &theme,
            CodeColorLevel::TrueColor,
        );

        assert_eq!(lines[0].spans[0].style.fg, Some(theme.muted));
        assert_eq!(lines[2].spans[0].style.fg, Some(theme.muted));

        let highlighted = &lines[1];
        assert!(
            highlighted.spans.len() > 1,
            "recognized syntax should split the line into styled spans"
        );
        assert!(
            highlighted
                .spans
                .iter()
                .any(|span| matches!(span.style.fg, Some(Color::Rgb(_, _, _)))),
            "expected at least one syntax-colored span, got: {:?}",
            highlighted.spans
        );
    }

    #[test]
    fn test_unknown_fenced_code_falls_back_to_plain_code_style() {
        let theme = Theme::dark();
        let lines = render_markdown_with_color_level(
            "```not-a-language\nfn main() {}\n```",
            &theme,
            CodeColorLevel::TrueColor,
        );

        assert_eq!(lines[1].spans.len(), 1);
        assert_eq!(lines[1].spans[0].content, "fn main() {}");
        assert_eq!(lines[1].spans[0].style.fg, Some(theme.code_fg));
    }

    #[test]
    fn test_untagged_fenced_code_uses_plain_code_style() {
        let theme = Theme::dark();
        let lines = render_markdown_with_color_level(
            "```\nname = \"tome\"\n```",
            &theme,
            CodeColorLevel::TrueColor,
        );

        assert_eq!(lines[1].spans.len(), 1);
        assert_eq!(lines[1].spans[0].content, "name = \"tome\"");
        assert_eq!(lines[1].spans[0].style.fg, Some(theme.code_fg));
    }

    #[test]
    fn test_no_color_mode_keeps_code_readable_without_highlight_colors() {
        let theme = Theme::dark();
        let lines = render_markdown_with_color_level(
            "```rust\nfn main() {}\n```",
            &theme,
            CodeColorLevel::None,
        );

        let code_line = &lines[1];
        assert!(
            code_line
                .spans
                .iter()
                .all(|span| span.style.fg.is_none() && span.style.bg.is_none()),
            "expected monochrome fallback, got: {:?}",
            code_line.spans
        );
    }
}
