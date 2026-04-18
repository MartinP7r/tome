//! Terminal-adaptive color theme for the browse TUI.
//!
//! Provides a `Theme` struct that bundles all color and style values used
//! by the renderer. `Theme::detect()` picks dark or light palette based on
//! the `$COLORFGBG` environment variable.

use ratatui::style::{Color, Modifier, Style};

/// All colors consumed by `ui::render`. Derived styles are computed
/// via methods to ensure they always stay consistent with the base colors.
#[derive(Clone)]
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
    /// Magenta family -- inline code.
    pub code_fg: Color,
    /// Background color for the status bar count badge (e.g. "100 skills").
    /// Separated from `accent` so the badge can stand out visually without
    /// affecting other accent-colored elements.
    pub badge_bg: Color,
    /// Foreground color for high-contrast badges on the `badge_bg` background.
    /// Must contrast with `badge_bg`.
    pub badge_fg: Color,
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
            code_fg: Color::Magenta,
            badge_bg: Color::Green,
            badge_fg: Color::Black, // black on green = high contrast
        }
    }

    /// Light palette for light-background terminals.
    pub fn light() -> Self {
        Self {
            accent: Color::Indexed(30),       // dark cyan
            alert: Color::Indexed(136),       // dark yellow
            muted: Color::Indexed(243),       // medium gray
            selected_bg: Color::Indexed(254), // near-white gray
            status_bar_bg: Color::Indexed(254),
            status_bar_fg: Color::Indexed(243),
            code_fg: Color::Indexed(133), // dark magenta
            badge_bg: Color::Indexed(28), // dark green
            badge_fg: Color::White,       // white on dark-green = high contrast
        }
    }

    /// Bold + accent fg -- markdown `#` headers in preview.
    pub fn preview_header(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Bold modifier only -- `**bold**` in preview.
    pub fn preview_bold(&self) -> Style {
        Style::default().add_modifier(Modifier::BOLD)
    }

    /// Italic modifier only -- `*italic*` in preview.
    pub fn preview_italic(&self) -> Style {
        Style::default().add_modifier(Modifier::ITALIC)
    }

    /// code_fg color -- `` `code` `` in preview.
    pub fn preview_code(&self) -> Style {
        Style::default().fg(self.code_fg)
    }

    /// alert fg + Bold -- fuzzy match character highlights.
    pub fn match_highlight(&self) -> Style {
        Style::default().fg(self.alert).add_modifier(Modifier::BOLD)
    }

    /// alert fg + Bold -- source group headers.
    pub fn group_header(&self) -> Style {
        Style::default().fg(self.alert).add_modifier(Modifier::BOLD)
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
    }

    #[test]
    fn test_light_theme_indexed_colors() {
        let theme = Theme::light();
        assert_eq!(theme.accent, Color::Indexed(30));
        assert_eq!(theme.alert, Color::Indexed(136));
        assert_eq!(theme.muted, Color::Indexed(243));
        assert_eq!(theme.selected_bg, Color::Indexed(254));
        assert_eq!(theme.code_fg, Color::Indexed(133));
    }

    #[test]
    fn test_detect_defaults_to_dark() {
        // In test environment, COLORFGBG is typically not set,
        // so detect() should return dark theme.
        let theme = Theme::detect();
        assert_eq!(theme.accent, Color::Cyan);
    }

    #[test]
    fn test_derived_styles_use_base_colors() {
        let theme = Theme::dark();
        assert_eq!(theme.preview_header().fg, Some(Color::Cyan));
        assert!(theme.preview_header().add_modifier.contains(Modifier::BOLD));
        assert_eq!(theme.preview_code().fg, Some(Color::Magenta));
        assert_eq!(theme.match_highlight().fg, Some(Color::Yellow));
        assert!(
            theme
                .match_highlight()
                .add_modifier
                .contains(Modifier::BOLD)
        );
    }
}
