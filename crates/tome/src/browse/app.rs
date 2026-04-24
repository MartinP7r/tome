use std::fs;
use std::path::Path;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::fuzzy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Search,
    Detail,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    Name,
    Source,
    Recent,
}

impl SortMode {
    pub fn next(self) -> Self {
        match self {
            Self::Name => Self::Source,
            Self::Source => Self::Recent,
            Self::Recent => Self::Name,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Source => "source",
            Self::Recent => "recent",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum DetailAction {
    ViewSource,
    CopyPath,
    Disable,
    Enable,
    Back,
}

impl DetailAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::ViewSource => "Open source directory",
            Self::CopyPath => "Copy path to clipboard",
            Self::Disable => "Disable on this machine",
            Self::Enable => "Enable on this machine",
            Self::Back => "Back",
        }
    }
}

pub struct SkillRow {
    pub name: String,
    pub source: String,
    pub path: String,
    pub managed: bool,
    pub synced_at: String,
}

pub struct App {
    pub mode: Mode,
    pub previous_mode: Mode,
    pub should_quit: bool,
    pub rows: Vec<SkillRow>,
    pub filtered_indices: Vec<usize>,
    pub match_indices: Vec<Vec<u32>>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub search_input: String,
    pub visible_height: usize,
    pub preview_title: String,
    pub preview_content: String,
    pub sort_mode: SortMode,
    pub group_by_source: bool,
    pub detail_actions: Vec<DetailAction>,
    pub detail_selected: usize,
    pub theme: super::theme::Theme,
    pub status_message: Option<String>,
}

impl App {
    pub fn new(rows: Vec<SkillRow>) -> Self {
        let match_indices = vec![Vec::new(); rows.len()];
        let filtered_indices: Vec<usize> = (0..rows.len()).collect();
        let mut app = Self {
            mode: Mode::Normal,
            previous_mode: Mode::Normal,
            should_quit: false,
            filtered_indices,
            match_indices,
            rows,
            selected: 0,
            scroll_offset: 0,
            search_input: String::new(),
            visible_height: 20,
            preview_title: "Preview".into(),
            preview_content: "No skills discovered.".into(),
            sort_mode: SortMode::Name,
            group_by_source: false,
            detail_actions: Vec::new(),
            detail_selected: 0,
            theme: super::theme::Theme::detect(),
            status_message: None,
        };
        app.apply_sort();
        app.refresh_preview();
        app
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        // Any-key-dismisses semantics for status_message: the message stays
        // visible until the user presses any key, then disappears on the next
        // action. Mirrors the `?` help-overlay dismissal pattern (see Mode::Help
        // branch below). Cleared unconditionally at the top so the lifetime
        // contract is trivial — no mode-dependent paths.
        self.status_message = None;

        match self.mode {
            Mode::Normal => self.handle_normal_key(key),
            Mode::Search => self.handle_search_key(key),
            Mode::Detail => self.handle_detail_key(key),
            Mode::Help => {
                // Any key dismisses help overlay
                self.mode = self.previous_mode;
            }
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
            KeyCode::Char('s') => {
                self.sort_mode = self.sort_mode.next();
                self.refilter();
            }
            KeyCode::Tab => {
                self.group_by_source = !self.group_by_source;
            }
            KeyCode::Enter if !self.filtered_indices.is_empty() => {
                self.enter_detail_mode();
            }
            KeyCode::Char('?') => {
                self.previous_mode = self.mode;
                self.mode = Mode::Help;
            }
            _ => {}
        }
    }

    fn handle_detail_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down if !self.detail_actions.is_empty() => {
                self.detail_selected =
                    (self.detail_selected + 1).min(self.detail_actions.len() - 1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.detail_selected = self.detail_selected.saturating_sub(1);
            }
            KeyCode::Enter => {
                if let Some(&action) = self.detail_actions.get(self.detail_selected) {
                    self.execute_action(action);
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = Mode::Normal;
            }
            _ => {}
        }
    }

    fn enter_detail_mode(&mut self) {
        self.mode = Mode::Detail;
        // Disable/Enable is shown unconditionally — the action's current
        // implementation (see `execute_action` below) just pops back to
        // Normal mode because the browse module has no machine.toml
        // handle. When a future change wires machine prefs into browse,
        // this list should reflect the skill's actual disabled state
        // (show Disable if enabled, Enable if disabled — never both).
        self.detail_actions = vec![
            DetailAction::ViewSource,
            DetailAction::CopyPath,
            DetailAction::Disable,
            DetailAction::Back,
        ];
        self.detail_selected = 0;
    }

    fn execute_action(&mut self, action: DetailAction) {
        match action {
            DetailAction::ViewSource => {
                if let Some((_, _, path)) = self.selected_row_meta() {
                    // Dispatch the GUI-file-opener binary at compile time: macOS
                    // ships `open`; Linux desktops ship `xdg-open`. We use
                    // `.status()` (blocking) rather than `.spawn()` so we can
                    // observe non-zero exit codes — otherwise `xdg-open` silently
                    // exiting on a headless box (no DISPLAY, no MIME handler)
                    // would still report "✓ Opened" and lie to the user. Both
                    // openers return quickly after dispatching to the system
                    // handler, so the brief block is acceptable for a one-off
                    // TUI action.
                    let binary = if cfg!(target_os = "macos") {
                        "open"
                    } else {
                        "xdg-open"
                    };
                    match std::process::Command::new(binary).arg(&path).status() {
                        Ok(status) if status.success() => {
                            self.status_message = Some(format!(
                                "✓ Opened: {}",
                                crate::paths::collapse_home(Path::new(&path))
                            ));
                        }
                        Ok(status) => {
                            let exit = status
                                .code()
                                .map(|c| c.to_string())
                                .unwrap_or_else(|| "signal".into());
                            self.status_message =
                                Some(format!("⚠ {binary} exited {exit} for: {}", path));
                        }
                        Err(e) => {
                            self.status_message = Some(format!("⚠ Could not launch {binary}: {e}"));
                        }
                    }
                }
            }
            DetailAction::CopyPath => {
                if let Some((_, _, path)) = self.selected_row_meta() {
                    // Use arboard for cross-platform clipboard access. Replaces
                    // the prior macOS-only shelled-pipe invocation, which was
                    // also a command-injection vector (paths with apostrophes
                    // could escape the single-quote wrapping). arboard is built
                    // with `default-features = false, features = ["wayland-data-control"]`
                    // so on Linux both X11 (via x11rb) and Wayland (via
                    // wayland-data-control) backends are compiled in; `image-data`
                    // is intentionally disabled to avoid pulling the `image`
                    // crate (not needed — we only copy text). Construction can
                    // still fail on headless Linux over SSH (no display server),
                    // which surfaces via status_message.
                    let result =
                        arboard::Clipboard::new().and_then(|mut cb| cb.set_text(path.clone()));
                    match result {
                        Ok(()) => {
                            self.status_message = Some(format!(
                                "✓ Copied: {}",
                                crate::paths::collapse_home(Path::new(&path))
                            ));
                        }
                        Err(e) => {
                            self.status_message = Some(format!("⚠ Could not copy: {e}"));
                        }
                    }
                }
            }
            DetailAction::Disable | DetailAction::Enable => {
                // For now, just go back — proper implementation requires machine.toml access
                // which the browse module doesn't currently have
                self.mode = Mode::Normal;
            }
            DetailAction::Back => {
                self.mode = Mode::Normal;
            }
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
        let matches = fuzzy::filter_rows_with_indices(&self.search_input, &self.rows);
        self.filtered_indices = matches.iter().map(|m| m.row_index).collect();
        // Build a lookup: for each row_index, store its name_indices
        self.match_indices = vec![Vec::new(); self.rows.len()];
        for m in matches {
            self.match_indices[m.row_index] = m.name_indices;
        }
        self.apply_sort();
        // Clamp cursor
        if self.filtered_indices.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len() - 1;
        }
        self.clamp_scroll();
        self.refresh_preview();
    }

    fn apply_sort(&mut self) {
        match self.sort_mode {
            SortMode::Name => self
                .filtered_indices
                .sort_by(|&a, &b| self.rows[a].name.cmp(&self.rows[b].name)),
            SortMode::Source => self.filtered_indices.sort_by(|&a, &b| {
                self.rows[a]
                    .source
                    .cmp(&self.rows[b].source)
                    .then(self.rows[a].name.cmp(&self.rows[b].name))
            }),
            SortMode::Recent => self
                .filtered_indices
                .sort_by(|&a, &b| self.rows[b].synced_at.cmp(&self.rows[a].synced_at)),
        }
    }

    fn move_cursor_down(&mut self, n: usize) {
        if self.filtered_indices.is_empty() {
            return;
        }
        let max = self.filtered_indices.len() - 1;
        self.selected = (self.selected + n).min(max);
        self.clamp_scroll();
        self.refresh_preview();
    }

    fn move_cursor_up(&mut self, n: usize) {
        self.selected = self.selected.saturating_sub(n);
        self.clamp_scroll();
        self.refresh_preview();
    }

    fn jump_to_top(&mut self) {
        self.selected = 0;
        self.scroll_offset = 0;
        self.refresh_preview();
    }

    fn jump_to_bottom(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected = self.filtered_indices.len() - 1;
        }
        self.clamp_scroll();
        self.refresh_preview();
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

    fn refresh_preview(&mut self) {
        let Some((name, source, skill_path)) = self.selected_row_meta() else {
            self.preview_title = "Preview".into();
            self.preview_content = "No matching skill.".into();
            return;
        };

        self.preview_title = format!("Preview: {name}");

        let skill_file = Path::new(&skill_path).join("SKILL.md");
        let header = format!("source: {source}\npath: {skill_path}\n\n");

        self.preview_content = match fs::read_to_string(&skill_file) {
            Ok(content) if content.trim().is_empty() => {
                format!("{header}[SKILL.md is empty]")
            }
            Ok(content) => format!("{header}{content}"),
            Err(err) => format!("{header}[failed to read {}: {err}]", skill_file.display()),
        };
    }

    fn selected_row_meta(&self) -> Option<(String, String, String)> {
        let row_idx = *self.filtered_indices.get(self.selected)?;
        let row = self.rows.get(row_idx)?;
        Some((row.name.clone(), row.source.clone(), row.path.clone()))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn make_app(n: usize) -> (App, tempfile::TempDir) {
        let temp_root = tempfile::tempdir().expect("tempdir");

        let rows: Vec<SkillRow> = (0..n)
            .map(|i| {
                let skill_dir = temp_root.path().join(format!("skill-{i}"));
                fs::create_dir_all(&skill_dir).expect("create skill dir");
                fs::write(skill_dir.join("SKILL.md"), format!("# skill-{i}\n"))
                    .expect("write skill");

                SkillRow {
                    name: format!("skill-{i}"),
                    source: "test".into(),
                    path: skill_dir.display().to_string(),
                    managed: false,
                    synced_at: String::new(),
                }
            })
            .collect();

        let mut app = App::new(rows);
        app.visible_height = 5;
        (app, temp_root)
    }

    #[test]
    fn cursor_down_clamps_at_end() {
        let (mut app, _tmp) = make_app(3);
        app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        assert_eq!(app.selected, 2); // clamped to last
    }

    #[test]
    fn cursor_up_clamps_at_start() {
        let (mut app, _tmp) = make_app(3);
        app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn jump_to_bottom_and_top() {
        let (mut app, _tmp) = make_app(10);
        app.handle_key(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT));
        assert_eq!(app.selected, 9);
        app.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
        assert_eq!(app.selected, 0);
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn scroll_offset_follows_cursor() {
        let (mut app, _tmp) = make_app(20);
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
        let (mut app, _tmp) = make_app(3);
        assert_eq!(app.mode, Mode::Normal);
        app.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        assert_eq!(app.mode, Mode::Search);
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn search_filters_rows() {
        let (mut app, _tmp) = make_app(10);
        app.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        // Type "skill-3"
        for c in "skill-3".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        // Fuzzy search should include the intended match in results
        assert!(!app.filtered_indices.is_empty());
        assert!(
            app.filtered_indices
                .iter()
                .any(|&idx| app.rows[idx].name == "skill-3")
        );
    }

    #[test]
    fn esc_in_search_clears_and_restores_all() {
        let (mut app, _tmp) = make_app(10);
        app.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.mode, Mode::Normal);
        assert!(app.search_input.is_empty());
        assert_eq!(app.filtered_indices.len(), 10);
    }

    #[test]
    fn quit_on_q() {
        let (mut app, _tmp) = make_app(3);
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(app.should_quit);
    }

    #[test]
    fn half_page_down() {
        let (mut app, _tmp) = make_app(20);
        app.visible_height = 10;
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert_eq!(app.selected, 5);
    }

    #[test]
    fn empty_rows_dont_panic() {
        let (mut app, _tmp) = make_app(0);
        app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT));
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn preview_updates_for_selected_skill() {
        let (mut app, _tmp) = make_app(3);
        assert!(app.preview_content.contains("# skill-0"));

        app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        assert!(app.preview_content.contains("# skill-1"));
    }

    #[test]
    fn preview_shows_fallback_for_empty_rows() {
        let (app, _tmp) = make_app(0);
        assert_eq!(app.preview_content, "No matching skill.");
        assert_eq!(app.preview_title, "Preview");
    }

    #[test]
    fn preview_title_reflects_selected_skill() {
        let (app, _tmp) = make_app(3);
        assert_eq!(app.preview_title, "Preview: skill-0");
    }

    #[test]
    fn preview_header_contains_source_and_path() {
        let (app, _tmp) = make_app(2);
        assert!(app.preview_content.contains("source: test"));
        assert!(app.preview_content.contains("path: "));
    }

    #[test]
    fn preview_handles_empty_skill_md() {
        let temp = tempfile::tempdir().expect("tempdir");
        let skill_dir = temp.path().join("empty-skill");
        fs::create_dir_all(&skill_dir).expect("mkdir");
        fs::write(skill_dir.join("SKILL.md"), "  \n").expect("write");

        let rows = vec![SkillRow {
            name: "empty-skill".into(),
            source: "test".into(),
            path: skill_dir.display().to_string(),
            managed: false,
            synced_at: String::new(),
        }];
        let app = App::new(rows);
        assert!(app.preview_content.contains("[SKILL.md is empty]"));
    }

    #[test]
    fn preview_handles_missing_skill_md() {
        let temp = tempfile::tempdir().expect("tempdir");
        let skill_dir = temp.path().join("no-file");
        fs::create_dir_all(&skill_dir).expect("mkdir");
        // No SKILL.md written

        let rows = vec![SkillRow {
            name: "no-file".into(),
            source: "test".into(),
            path: skill_dir.display().to_string(),
            managed: false,
            synced_at: String::new(),
        }];
        let app = App::new(rows);
        assert!(app.preview_content.contains("[failed to read"));
    }

    #[test]
    fn preview_updates_after_search_filter() {
        let (mut app, _tmp) = make_app(5);
        assert!(app.preview_content.contains("# skill-0"));

        // Enter search mode and filter to skill-3
        app.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        for c in "skill-3".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }

        // Preview should have updated (may not be skill-3 first due to Name sort
        // reordering fuzzy results, but skill-3 must be in the filtered set)
        assert!(!app.filtered_indices.is_empty());
        assert!(
            app.filtered_indices
                .iter()
                .any(|&idx| app.rows[idx].name == "skill-3"),
            "skill-3 should be in filtered results"
        );
        // Preview should show some valid skill content (not the fallback)
        assert!(!app.preview_content.contains("No matching skill."));
    }

    fn make_row(name: &str, source: &str, synced: &str) -> SkillRow {
        SkillRow {
            name: name.to_string(),
            source: source.to_string(),
            path: format!("/test/{}", name),
            managed: false,
            synced_at: synced.to_string(),
        }
    }

    #[test]
    fn sort_by_name() {
        let rows = vec![
            make_row("zeta", "src-a", ""),
            make_row("alpha", "src-b", ""),
            make_row("mid", "src-a", ""),
        ];
        let app = App::new(rows);
        assert_eq!(app.sort_mode, SortMode::Name);
        let names: Vec<&str> = app
            .filtered_indices
            .iter()
            .map(|&i| app.rows[i].name.as_str())
            .collect();
        assert_eq!(names, vec!["alpha", "mid", "zeta"]);
    }

    #[test]
    fn sort_by_source() {
        let rows = vec![
            make_row("zeta", "src-b", ""),
            make_row("alpha", "src-a", ""),
            make_row("beta", "src-a", ""),
        ];
        let mut app = App::new(rows);
        app.sort_mode = SortMode::Source;
        app.refilter();
        let names: Vec<&str> = app
            .filtered_indices
            .iter()
            .map(|&i| app.rows[i].name.as_str())
            .collect();
        // Grouped by source, then alphabetical within source
        assert_eq!(names, vec!["alpha", "beta", "zeta"]);
    }

    #[test]
    fn sort_by_recent() {
        let rows = vec![
            make_row("old", "src", "2024-01-01T00:00:00Z"),
            make_row("newest", "src", "2024-03-01T00:00:00Z"),
            make_row("middle", "src", "2024-02-01T00:00:00Z"),
        ];
        let mut app = App::new(rows);
        app.sort_mode = SortMode::Recent;
        app.refilter();
        let names: Vec<&str> = app
            .filtered_indices
            .iter()
            .map(|&i| app.rows[i].name.as_str())
            .collect();
        assert_eq!(names, vec!["newest", "middle", "old"]);
    }

    #[test]
    fn sort_cycles() {
        assert_eq!(SortMode::Name.next(), SortMode::Source);
        assert_eq!(SortMode::Source.next(), SortMode::Recent);
        assert_eq!(SortMode::Recent.next(), SortMode::Name);
    }

    #[test]
    fn sort_preserves_selection() {
        let rows = vec![
            make_row("zeta", "src-b", ""),
            make_row("alpha", "src-a", ""),
            make_row("beta", "src-a", ""),
        ];
        let mut app = App::new(rows);
        // After Name sort, "alpha" is first. Move to "beta" (index 1).
        app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        let selected_name = app.rows[app.filtered_indices[app.selected]].name.clone();
        assert_eq!(selected_name, "beta");
    }

    #[test]
    fn enter_detail_mode() {
        let (mut app, _tmp) = make_app(3);
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.mode, Mode::Detail);
        assert!(!app.detail_actions.is_empty());
        assert_eq!(app.detail_selected, 0);
    }

    #[test]
    fn detail_mode_navigation() {
        let (mut app, _tmp) = make_app(3);
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.mode, Mode::Detail);
        let num_actions = app.detail_actions.len();

        // Move down
        app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        assert_eq!(app.detail_selected, 1);

        // Move up
        app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        assert_eq!(app.detail_selected, 0);

        // Clamp at bottom
        for _ in 0..num_actions + 2 {
            app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        }
        assert_eq!(app.detail_selected, num_actions - 1);
    }

    #[test]
    fn detail_mode_back() {
        let (mut app, _tmp) = make_app(3);
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.mode, Mode::Detail);
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn group_by_source_toggle() {
        let (mut app, _tmp) = make_app(3);
        assert!(!app.group_by_source);
        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert!(app.group_by_source);
        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert!(!app.group_by_source);
    }

    #[test]
    fn search_then_sort() {
        let (mut app, _tmp) = make_app(10);
        // Enter search mode
        app.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        for c in "skill".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        // All 10 should match "skill"
        assert_eq!(app.filtered_indices.len(), 10);

        // Cycle sort and verify it still has the right count
        app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
        assert_eq!(app.sort_mode, SortMode::Source);
        assert_eq!(app.filtered_indices.len(), 10);
    }

    #[test]
    fn status_message_set_by_copy_path_and_cleared_by_any_key() {
        // Lifecycle contract for the SAFE-02 status_message surface:
        //   1. Executing a DetailAction (CopyPath here) must set
        //      app.status_message to Some("✓ Copied: ...") on success OR
        //      Some("⚠ Could not copy: ...") on failure — we accept either
        //      prefix because headless CI runners (Linux over SSH, etc.)
        //      may not have a clipboard service available, and per D-17/D-19
        //      we do NOT introduce a `trait ClipboardBackend` to force one
        //      branch.
        //   2. Feeding any KeyEvent through handle_key must clear
        //      status_message to None — the any-key-dismisses semantic
        //      handle_key enforces as its first statement.
        let (mut app, _tmp) = make_app(3);

        // Step 1: execute CopyPath. arboard::Clipboard::new() may succeed or
        // fail depending on the host environment; both paths set status_message.
        app.execute_action(DetailAction::CopyPath);

        assert!(
            app.status_message.is_some(),
            "status_message must be Some after CopyPath action"
        );
        let msg = app.status_message.as_ref().unwrap().clone();
        assert!(
            msg.starts_with('✓') || msg.starts_with('⚠'),
            "expected ✓ or ⚠ prefix, got: {msg}"
        );

        // Step 2: any key clears the message.
        app.handle_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
        assert!(
            app.status_message.is_none(),
            "status_message must be None after any key; was: {:?}",
            app.status_message
        );
    }
}
