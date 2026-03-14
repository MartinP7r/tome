use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::fuzzy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Search,
}

pub struct SkillRow {
    pub name: String,
    pub source: String,
    pub path: String,
}

pub struct App {
    pub mode: Mode,
    pub should_quit: bool,
    pub rows: Vec<SkillRow>,
    pub filtered_indices: Vec<usize>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub search_input: String,
    pub visible_height: usize,
}

impl App {
    pub fn new(rows: Vec<SkillRow>) -> Self {
        let filtered_indices: Vec<usize> = (0..rows.len()).collect();
        Self {
            mode: Mode::Normal,
            should_quit: false,
            filtered_indices,
            rows,
            selected: 0,
            scroll_offset: 0,
            search_input: String::new(),
            visible_height: 20,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match self.mode {
            Mode::Normal => self.handle_normal_key(key),
            Mode::Search => self.handle_search_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => self.move_cursor_down(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_cursor_up(1),
            KeyCode::Char('g') => self.jump_to_top(),
            KeyCode::Char('G') => self.jump_to_bottom(),
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_cursor_down(self.visible_height / 2);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_cursor_up(self.visible_height / 2);
            }
            KeyCode::PageDown => self.move_cursor_down(self.visible_height),
            KeyCode::PageUp => self.move_cursor_up(self.visible_height),
            KeyCode::Char('/') => self.mode = Mode::Search,
            _ => {}
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.search_input.clear();
                self.refilter();
                self.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                self.mode = Mode::Normal;
            }
            KeyCode::Backspace => {
                self.search_input.pop();
                self.refilter();
            }
            KeyCode::Char(c) => {
                self.search_input.push(c);
                self.refilter();
            }
            _ => {}
        }
    }

    fn refilter(&mut self) {
        self.filtered_indices = fuzzy::filter_rows(&self.search_input, &self.rows);
        // Clamp cursor
        if self.filtered_indices.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len() - 1;
        }
        self.clamp_scroll();
    }

    fn move_cursor_down(&mut self, n: usize) {
        if self.filtered_indices.is_empty() {
            return;
        }
        let max = self.filtered_indices.len() - 1;
        self.selected = (self.selected + n).min(max);
        self.clamp_scroll();
    }

    fn move_cursor_up(&mut self, n: usize) {
        self.selected = self.selected.saturating_sub(n);
        self.clamp_scroll();
    }

    fn jump_to_top(&mut self) {
        self.selected = 0;
        self.scroll_offset = 0;
    }

    fn jump_to_bottom(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected = self.filtered_indices.len() - 1;
        }
        self.clamp_scroll();
    }

    fn clamp_scroll(&mut self) {
        if self.visible_height == 0 {
            return;
        }
        // Ensure selected row is visible
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + self.visible_height {
            self.scroll_offset = self.selected - self.visible_height + 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_app(n: usize) -> App {
        let rows: Vec<SkillRow> = (0..n)
            .map(|i| SkillRow {
                name: format!("skill-{i}"),
                source: "test".into(),
                path: format!("/path/{i}"),
            })
            .collect();
        let mut app = App::new(rows);
        app.visible_height = 5;
        app
    }

    #[test]
    fn cursor_down_clamps_at_end() {
        let mut app = make_app(3);
        app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        assert_eq!(app.selected, 2); // clamped to last
    }

    #[test]
    fn cursor_up_clamps_at_start() {
        let mut app = make_app(3);
        app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn jump_to_bottom_and_top() {
        let mut app = make_app(10);
        app.handle_key(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT));
        assert_eq!(app.selected, 9);
        app.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
        assert_eq!(app.selected, 0);
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn scroll_offset_follows_cursor() {
        let mut app = make_app(20);
        app.visible_height = 5;
        // Move down past visible area
        for _ in 0..7 {
            app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        }
        assert_eq!(app.selected, 7);
        assert!(app.scroll_offset > 0);
        assert!(app.selected < app.scroll_offset + app.visible_height);
    }

    #[test]
    fn search_mode_toggle() {
        let mut app = make_app(3);
        assert_eq!(app.mode, Mode::Normal);
        app.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        assert_eq!(app.mode, Mode::Search);
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn search_filters_rows() {
        let mut app = make_app(10);
        app.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        // Type "skill-3"
        for c in "skill-3".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        // Should have filtered results
        assert!(!app.filtered_indices.is_empty());
        assert!(app.filtered_indices.len() < 10);
    }

    #[test]
    fn esc_in_search_clears_and_restores_all() {
        let mut app = make_app(10);
        app.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.mode, Mode::Normal);
        assert!(app.search_input.is_empty());
        assert_eq!(app.filtered_indices.len(), 10);
    }

    #[test]
    fn quit_on_q() {
        let mut app = make_app(3);
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(app.should_quit);
    }

    #[test]
    fn half_page_down() {
        let mut app = make_app(20);
        app.visible_height = 10;
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert_eq!(app.selected, 5);
    }

    #[test]
    fn empty_rows_dont_panic() {
        let mut app = make_app(0);
        app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT));
        assert_eq!(app.selected, 0);
    }
}
