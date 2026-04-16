//! Simple line-by-line markdown renderer for SKILL.md preview.
//!
//! Handles headers (`#`, `##`, `###`), horizontal rules (`---`),
//! and inline spans (`**bold**`, `*italic*`, `` `code` ``).
//! No nested delimiter support -- keeps the parser simple.

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use super::theme::Theme;

/// Render raw markdown text into styled `Line`s for the preview panel.
pub fn render_markdown<'a>(raw: &'a str, theme: &Theme) -> Vec<Line<'a>> {
    raw.lines().map(|line| render_line(line, theme)).collect()
}

fn render_line<'a>(line: &'a str, theme: &Theme) -> Line<'a> {
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
fn render_inline_markdown<'a>(line: &'a str, theme: &Theme) -> Line<'a> {
    let mut spans: Vec<Span<'a>> = Vec::new();
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
        assert!(lines[0].spans[0]
            .style
            .add_modifier
            .contains(Modifier::BOLD));
    }

    #[test]
    fn test_h2_header() {
        let theme = Theme::dark();
        let lines = render_markdown("## Sub", &theme);
        assert_eq!(lines[0].spans[0].content, "Sub");
        assert!(lines[0].spans[0]
            .style
            .add_modifier
            .contains(Modifier::BOLD));
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
}
