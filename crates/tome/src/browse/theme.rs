//! Terminal-adaptive color theme for the browse TUI.
//!
//! Provides a `Theme` struct that bundles all color and style values used
//! by the renderer. `Theme::detect()` picks dark or light palette based on
//! the `$COLORFGBG` environment variable.

use ratatui::style::{Color, Modifier, Style};

/// All colors and derived styles consumed by `ui::render`.
pub struct Theme {
    /// Cyan family -- headers, badges, selection indicators.
    pub accent: Color,
    /// Yellow family -- group headers, search display, match highlights.
    pub alert: Color,
    /// Gray family -- hints, inactive text, metadata labels.
    pub muted: Color,
    /// Dark gray family -- selected row background.
    pub selected_bg: Color,
    /// Dark gray family -- status bar background.
    pub status_bar_bg: Color,
    /// Gray family -- status bar text.
    pub status_bar_fg: Color,
    /// Red family -- errors.
    pub destructive: Color,
    /// Green family -- confirmations.
    pub success: Color,
    /// Magenta family -- inline code.
    pub code_fg: Color,
    /// Bold + accent fg -- markdown `#` headers in preview.
    pub preview_header: Style,
    /// Bold modifier only -- `**bold**` in preview.
    pub preview_bold: Style,
    /// Italic modifier only -- `*italic*` in preview.
    pub preview_italic: Style,
    /// code_fg color -- `` `code` `` in preview.
    pub preview_code: Style,
    /// alert fg + Bold -- fuzzy match character highlights.
    pub match_highlight: Style,
    /// alert fg + Bold -- source group headers.
    pub group_header: Style,
}

impl Theme {
    /// Detect terminal background brightness and return the appropriate theme.
    pub fn detect() -> Self {
        if is_light_terminal() {
            Self::light()
        } else {
            Self::dark()
        }
    }

    /// Dark palette (default) for dark-background terminals.
    pub fn dark() -> Self {
        Self {
            accent: Color::Cyan,
            alert: Color::Yellow,
            muted: Color::Gray,
            selected_bg: Color::DarkGray,
            status_bar_bg: Color::DarkGray,
            status_bar_fg: Color::Gray,
            destructive: Color::Red,
            success: Color::Green,
            code_fg: Color::Magenta,
            preview_header: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            preview_bold: Style::default().add_modifier(Modifier::BOLD),
            preview_italic: Style::default().add_modifier(Modifier::ITALIC),
            preview_code: Style::default().fg(Color::Magenta),
            match_highlight: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            group_header: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        }
    }

    /// Light palette for light-background terminals.
    pub fn light() -> Self {
        let accent = Color::Indexed(30); // dark cyan
        let alert = Color::Indexed(136); // dark yellow
        let muted = Color::Indexed(243); // medium gray
        let code_fg = Color::Indexed(133); // dark magenta

        Self {
            accent,
            alert,
            muted,
            selected_bg: Color::Indexed(254), // near-white gray
            status_bar_bg: Color::Indexed(254),
            status_bar_fg: Color::Indexed(243),
            destructive: Color::Indexed(124), // dark red
            success: Color::Indexed(28),      // dark green
            code_fg,
            preview_header: Style::default()
                .fg(accent)
                .add_modifier(Modifier::BOLD),
            preview_bold: Style::default().add_modifier(Modifier::BOLD),
            preview_italic: Style::default().add_modifier(Modifier::ITALIC),
            preview_code: Style::default().fg(code_fg),
            match_highlight: Style::default()
                .fg(alert)
                .add_modifier(Modifier::BOLD),
            group_header: Style::default()
                .fg(alert)
                .add_modifier(Modifier::BOLD),
        }
    }
}

/// Check `$COLORFGBG` to determine if the terminal has a light background.
///
/// The variable has format `"fg;bg"` where bg is an ANSI color index.
/// If `bg >= 9` (excluding 8 which is dark gray), the background is light.
fn is_light_terminal() -> bool {
    std::env::var("COLORFGBG")
        .ok()
        .and_then(|v| v.rsplit(';').next().map(String::from))
        .and_then(|bg| bg.parse::<u8>().ok())
        .is_some_and(|bg| bg >= 9)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dark_theme_default_colors() {
        let theme = Theme::dark();
        assert_eq!(theme.accent, Color::Cyan);
        assert_eq!(theme.alert, Color::Yellow);
        assert_eq!(theme.muted, Color::Gray);
        assert_eq!(theme.selected_bg, Color::DarkGray);
        assert_eq!(theme.code_fg, Color::Magenta);
        assert_eq!(theme.destructive, Color::Red);
        assert_eq!(theme.success, Color::Green);
    }

    #[test]
    fn test_light_theme_indexed_colors() {
        let theme = Theme::light();
        assert_eq!(theme.accent, Color::Indexed(30));
        assert_eq!(theme.alert, Color::Indexed(136));
        assert_eq!(theme.muted, Color::Indexed(243));
        assert_eq!(theme.selected_bg, Color::Indexed(254));
        assert_eq!(theme.code_fg, Color::Indexed(133));
        assert_eq!(theme.destructive, Color::Indexed(124));
        assert_eq!(theme.success, Color::Indexed(28));
    }

    #[test]
    fn test_detect_defaults_to_dark() {
        // In test environment, COLORFGBG is typically not set,
        // so detect() should return dark theme.
        let theme = Theme::detect();
        assert_eq!(theme.accent, Color::Cyan);
    }
}
