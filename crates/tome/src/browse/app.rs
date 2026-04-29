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

/// Severity for ephemeral status-bar messages surfaced by DetailAction
/// handlers. The variant set is closed — adding a new variant requires
/// updates to `glyph()`, `severity()`, the ui.rs color dispatch, and any
/// tests that exhaustively match on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StatusSeverity {
    Success,
    Warning,
    /// Informational "operation in progress" message. Used for the
    /// `Opening: <path>...` status surfaced before blocking on
    /// `xdg-open` / `open` (POLISH-01). Rendered with `theme.muted`.
    Pending,
}

/// A one-shot toast rendered in the status bar until the next keypress.
///
/// Stored as `Option<StatusMessage>` on `App`. The variant carries the raw
/// body string only — the glyph (`✓` / `⚠` / `⏳`) is composed at render
/// time in `ui.rs` via `msg.glyph()`. This eliminates the stringly-typed
/// pre-formatted-text design that made the type fragile to refactor and
/// duplicated the severity signal between `severity` and the leading
/// glyph in `text`.
// Derives note: Clone + PartialEq + Eq are kept for the existing assert_eq!-based
// test surface (handle_key / execute_action lifecycle). Debug is for {:?} in test
// failure messages. NO Hash / Ord / Default — this type has no key/sort/empty
// semantics. Audited 2026 (POLISH-02).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum StatusMessage {
    Success(String),
    Warning(String),
    /// Constructed in `handle_view_source` (POLISH-01) to surface
    /// "Opening: <path>..." before `xdg-open`/`open` blocks.
    Pending(String),
}

impl StatusMessage {
    /// Raw body without any glyph prefix.
    pub(super) fn body(&self) -> &str {
        match self {
            StatusMessage::Success(s) | StatusMessage::Warning(s) | StatusMessage::Pending(s) => {
                s.as_str()
            }
        }
    }

    /// Severity glyph rendered before the body. UI composes
    /// `format!("{} {}", msg.glyph(), msg.body())` at render time.
    pub(super) fn glyph(&self) -> char {
        match self {
            StatusMessage::Success(_) => '✓',
            StatusMessage::Warning(_) => '⚠',
            StatusMessage::Pending(_) => '⏳',
        }
    }

    pub(super) fn severity(&self) -> StatusSeverity {
        match self {
            StatusMessage::Success(_) => StatusSeverity::Success,
            StatusMessage::Warning(_) => StatusSeverity::Warning,
            StatusMessage::Pending(_) => StatusSeverity::Pending,
        }
    }
}

/// Convert the result of `Command::new(opener).arg(path).status()` into the
/// matching `StatusMessage`. Factored out of `App::execute_action` so the
/// three arms (Ok+success, Ok+non-zero exit, Err) are unit-testable with
/// synthetic `ExitStatus` values via `ExitStatusExt::from_raw` — engineering
/// a real opener failure on a CI runner is racy (depends on whether
/// `xdg-open`/`open` is installed and what the OS does on missing MIME
/// handlers).
///
/// `binary` is the opener name ("open" on macOS, "xdg-open" on Linux);
/// `path` is the file path passed to it. Both appear in error/success
/// messages so the user can tell which file failed and which opener tried.
pub(super) fn status_message_from_open_result(
    binary: &str,
    path: &std::path::Path,
    raw: std::io::Result<std::process::ExitStatus>,
) -> StatusMessage {
    match raw {
        Ok(status) if status.success() => {
            StatusMessage::Success(format!("Opened: {}", crate::paths::collapse_home(path)))
        }
        Ok(status) => {
            let exit = status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "signal".into());
            StatusMessage::Warning(format!("{binary} exited {exit} for: {}", path.display()))
        }
        Err(e) => StatusMessage::Warning(format!("Could not launch {binary}: {e}")),
    }
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
    pub(super) status_message: Option<StatusMessage>,
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

    pub(super) fn handle_key(&mut self, key: KeyEvent, redraw: &mut dyn FnMut(&App)) {
        // Any-key-dismisses semantics for status_message: the message stays
        // visible until the user presses any key, then disappears on the next
        // action. Mirrors the `?` help-overlay dismissal pattern (see Mode::Help
        // branch below). Cleared unconditionally at the top so the lifetime
        // contract is trivial — no mode-dependent paths.
        self.status_message = None;

        match self.mode {
            Mode::Normal => self.handle_normal_key(key),
            Mode::Search => self.handle_search_key(key),
            // Detail mode threads the redraw closure through to
            // `execute_action_with_redraw` so the ViewSource branch can
            // surface a `Pending("Opening: ...")` status BEFORE the
            // blocking `.status()` call (POLISH-01). Production
            // `run_loop` constructs a closure that calls
            // `terminal.draw(...)`; tests pass `&mut |_| {}`.
            Mode::Detail => self.handle_detail_key(key, redraw),
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

    fn handle_detail_key(&mut self, key: KeyEvent, redraw: &mut dyn FnMut(&App)) {
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
                    // ViewSource needs the redraw closure for POLISH-01 (so
                    // `Pending("Opening: ...")` paints before `.status()`
                    // blocks); other actions don't block, so the legacy
                    // `execute_action` path is sufficient.
                    if matches!(action, DetailAction::ViewSource) {
                        self.execute_action_with_redraw(action, redraw);
                    } else {
                        self.execute_action(action);
                    }
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
                // Production callers should use `execute_action_with_redraw`
                // directly so the Pending("Opening: ...") status renders
                // before `.status()` blocks (POLISH-01). This arm is kept
                // so callers/tests that construct an App and call
                // `execute_action(ViewSource)` directly still work — they
                // pass through to `handle_view_source` with a no-op redraw
                // closure, identical to legacy behavior.
                self.handle_view_source(&mut |_| {});
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
                            self.status_message = Some(StatusMessage::Success(format!(
                                "Copied: {}",
                                crate::paths::collapse_home(Path::new(&path))
                            )));
                        }
                        Err(e) => {
                            // Targeted remediation hints for the most common
                            // failure modes (S6 from phase-8 PR review).
                            // `ClipboardNotSupported` happens on headless
                            // Linux (SSH without DISPLAY) and on platforms
                            // lacking a clipboard backend at compile time.
                            // Everything else falls through to raw Display.
                            let msg = match &e {
                                arboard::Error::ClipboardNotSupported => {
                                    "Clipboard unavailable (headless or unsupported session)"
                                        .to_string()
                                }
                                arboard::Error::ClipboardOccupied => {
                                    "Clipboard busy (another app is holding it); try again"
                                        .to_string()
                                }
                                other => format!("Could not copy: {other}"),
                            };
                            self.status_message = Some(StatusMessage::Warning(msg));
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

    /// Executes a detail action with the ability to redraw before any blocking
    /// operation (e.g. `xdg-open`/`open` `.status()` calls). Production callers
    /// (the run_loop in browse/mod.rs) should use this method; tests that don't
    /// need the pre-block redraw can call `execute_action(action)` directly.
    ///
    /// The `redraw` closure should call `terminal.draw(...)` so the user sees
    /// `Pending` status before the block. Failures inside the closure are
    /// silently dropped — a draw error must not abort the action.
    pub(super) fn execute_action_with_redraw(
        &mut self,
        action: DetailAction,
        redraw: &mut dyn FnMut(&App),
    ) {
        match action {
            DetailAction::ViewSource => self.handle_view_source(redraw),
            other => self.execute_action(other),
        }
    }

    /// Implements the `ViewSource` action. Sets a `Pending("Opening: ...")`
    /// status, calls the redraw closure so the message paints BEFORE the
    /// blocking `.status()` call, runs the opener, replaces the status with
    /// the result via `status_message_from_open_result`, and finally drains
    /// any tty events that arrived during the block (POLISH-01).
    fn handle_view_source(&mut self, redraw: &mut dyn FnMut(&App)) {
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
            let path_buf = std::path::PathBuf::from(&path);

            // POLISH-01: surface "Opening: <path>..." BEFORE the blocking
            // `.status()` call. The redraw closure synchronously calls
            // `terminal.draw(...)` so the user sees the Pending message
            // BEFORE the block (in production); test sites pass a no-op
            // closure and skip the visual side-effect.
            self.status_message = Some(StatusMessage::Pending(format!(
                "Opening: {}...",
                crate::paths::collapse_home(&path_buf)
            )));
            redraw(self);

            let raw = std::process::Command::new(binary).arg(&path).status();
            self.status_message = Some(status_message_from_open_result(binary, &path_buf, raw));

            // POLISH-01 drain step: keystrokes that arrived in the crossterm
            // event queue while `.status()` was blocking were aimed at the
            // GUI file opener, not the TUI — replaying them as DetailAction
            // inputs would cause phantom navigation. Drain them.
            self.drain_pending_events();
        }
    }

    /// Drain any crossterm events that accumulated in the queue without
    /// dispatching them. Called after a long-blocking operation (e.g.
    /// `xdg-open`'s `.status()`) so keystrokes typed while the operation
    /// was running don't replay as DetailAction inputs after it returns
    /// (POLISH-01).
    ///
    /// Uses `event::poll(Duration::ZERO)` which returns immediately without
    /// blocking. The `unwrap_or(false)` collapses the rare poll error to
    /// "queue empty, stop draining" — losing one event under a poll error
    /// is no worse than the phantom-replay we're already avoiding.
    fn drain_pending_events(&self) {
        while crossterm::event::poll(std::time::Duration::ZERO).unwrap_or(false) {
            let _ = crossterm::event::read();
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
        app.handle_key(
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
            &mut |_| {},
        );
        app.handle_key(
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
            &mut |_| {},
        );
        app.handle_key(
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
            &mut |_| {},
        );
        assert_eq!(app.selected, 2); // clamped to last
    }

    #[test]
    fn cursor_up_clamps_at_start() {
        let (mut app, _tmp) = make_app(3);
        app.handle_key(
            KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE),
            &mut |_| {},
        );
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn jump_to_bottom_and_top() {
        let (mut app, _tmp) = make_app(10);
        app.handle_key(
            KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT),
            &mut |_| {},
        );
        assert_eq!(app.selected, 9);
        app.handle_key(
            KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
            &mut |_| {},
        );
        assert_eq!(app.selected, 0);
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn scroll_offset_follows_cursor() {
        let (mut app, _tmp) = make_app(20);
        app.visible_height = 5;
        // Move down past visible area
        for _ in 0..7 {
            app.handle_key(
                KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
                &mut |_| {},
            );
        }
        assert_eq!(app.selected, 7);
        assert!(app.scroll_offset > 0);
        assert!(app.selected < app.scroll_offset + app.visible_height);
    }

    #[test]
    fn search_mode_toggle() {
        let (mut app, _tmp) = make_app(3);
        assert_eq!(app.mode, Mode::Normal);
        app.handle_key(
            KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE),
            &mut |_| {},
        );
        assert_eq!(app.mode, Mode::Search);
        app.handle_key(
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            &mut |_| {},
        );
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn search_filters_rows() {
        let (mut app, _tmp) = make_app(10);
        app.handle_key(
            KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE),
            &mut |_| {},
        );
        // Type "skill-3"
        for c in "skill-3".chars() {
            app.handle_key(
                KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
                &mut |_| {},
            );
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
        app.handle_key(
            KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE),
            &mut |_| {},
        );
        app.handle_key(
            KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
            &mut |_| {},
        );
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &mut |_| {});
        assert_eq!(app.mode, Mode::Normal);
        assert!(app.search_input.is_empty());
        assert_eq!(app.filtered_indices.len(), 10);
    }

    #[test]
    fn quit_on_q() {
        let (mut app, _tmp) = make_app(3);
        app.handle_key(
            KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
            &mut |_| {},
        );
        assert!(app.should_quit);
    }

    #[test]
    fn half_page_down() {
        let (mut app, _tmp) = make_app(20);
        app.visible_height = 10;
        app.handle_key(
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
            &mut |_| {},
        );
        assert_eq!(app.selected, 5);
    }

    #[test]
    fn empty_rows_dont_panic() {
        let (mut app, _tmp) = make_app(0);
        app.handle_key(
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
            &mut |_| {},
        );
        app.handle_key(
            KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT),
            &mut |_| {},
        );
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn preview_updates_for_selected_skill() {
        let (mut app, _tmp) = make_app(3);
        assert!(app.preview_content.contains("# skill-0"));

        app.handle_key(
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
            &mut |_| {},
        );
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
        app.handle_key(
            KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE),
            &mut |_| {},
        );
        for c in "skill-3".chars() {
            app.handle_key(
                KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
                &mut |_| {},
            );
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
        app.handle_key(
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
            &mut |_| {},
        );
        let selected_name = app.rows[app.filtered_indices[app.selected]].name.clone();
        assert_eq!(selected_name, "beta");
    }

    #[test]
    fn enter_detail_mode() {
        let (mut app, _tmp) = make_app(3);
        app.handle_key(
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            &mut |_| {},
        );
        assert_eq!(app.mode, Mode::Detail);
        assert!(!app.detail_actions.is_empty());
        assert_eq!(app.detail_selected, 0);
    }

    #[test]
    fn detail_mode_navigation() {
        let (mut app, _tmp) = make_app(3);
        app.handle_key(
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            &mut |_| {},
        );
        assert_eq!(app.mode, Mode::Detail);
        let num_actions = app.detail_actions.len();

        // Move down
        app.handle_key(
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
            &mut |_| {},
        );
        assert_eq!(app.detail_selected, 1);

        // Move up
        app.handle_key(
            KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE),
            &mut |_| {},
        );
        assert_eq!(app.detail_selected, 0);

        // Clamp at bottom
        for _ in 0..num_actions + 2 {
            app.handle_key(
                KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
                &mut |_| {},
            );
        }
        assert_eq!(app.detail_selected, num_actions - 1);
    }

    #[test]
    fn detail_mode_back() {
        let (mut app, _tmp) = make_app(3);
        app.handle_key(
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            &mut |_| {},
        );
        assert_eq!(app.mode, Mode::Detail);
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &mut |_| {});
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn group_by_source_toggle() {
        let (mut app, _tmp) = make_app(3);
        assert!(!app.group_by_source);
        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE), &mut |_| {});
        assert!(app.group_by_source);
        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE), &mut |_| {});
        assert!(!app.group_by_source);
    }

    #[test]
    fn search_then_sort() {
        let (mut app, _tmp) = make_app(10);
        // Enter search mode
        app.handle_key(
            KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE),
            &mut |_| {},
        );
        for c in "skill".chars() {
            app.handle_key(
                KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
                &mut |_| {},
            );
        }
        app.handle_key(
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            &mut |_| {},
        );

        // All 10 should match "skill"
        assert_eq!(app.filtered_indices.len(), 10);

        // Cycle sort and verify it still has the right count
        app.handle_key(
            KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
            &mut |_| {},
        );
        assert_eq!(app.sort_mode, SortMode::Source);
        assert_eq!(app.filtered_indices.len(), 10);
    }

    #[test]
    fn status_message_set_by_copy_path_and_cleared_by_any_key() {
        // Lifecycle contract for the SAFE-02 status_message surface:
        //   1. Executing a DetailAction (CopyPath here) must set
        //      app.status_message to Some(StatusMessage::Success(_) | Warning(_))
        //      — we accept either because headless CI runners (Linux over
        //      SSH, etc.) may not have a clipboard service available, and
        //      per D-17/D-19 we do NOT introduce a `trait ClipboardBackend`
        //      to force one branch.
        //   2. Feeding any KeyEvent through handle_key must clear
        //      status_message to None — the any-key-dismisses semantic
        //      handle_key enforces as its first statement.
        let (mut app, _tmp) = make_app(3);

        // Step 1: execute CopyPath. arboard::Clipboard::new() may succeed or
        // fail depending on the host environment; both paths set status_message.
        app.execute_action(DetailAction::CopyPath);

        let msg = app
            .status_message
            .as_ref()
            .cloned()
            .expect("status_message must be Some after CopyPath action");
        assert!(
            matches!(
                msg.severity(),
                StatusSeverity::Success | StatusSeverity::Warning
            ),
            "expected Success or Warning severity, got: {:?}",
            msg.severity()
        );
        // The glyph belongs in ui.rs at render time, not in the body string.
        assert!(
            msg.glyph() == '✓' || msg.glyph() == '⚠',
            "expected ✓ or ⚠ glyph; got: {}",
            msg.glyph()
        );
        assert!(
            !msg.body().starts_with('✓')
                && !msg.body().starts_with('⚠')
                && !msg.body().starts_with('⏳'),
            "body must not embed a glyph prefix; got: {}",
            msg.body()
        );

        // Step 2: any key clears the message.
        app.handle_key(
            KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE),
            &mut |_| {},
        );
        assert!(
            app.status_message.is_none(),
            "status_message must be None after any key; was: {:?}",
            app.status_message
        );
    }

    #[test]
    fn status_message_glyph_dispatch_for_each_variant() {
        // Each variant's `body()` must return the raw inner string with no
        // leading glyph or space, while `glyph()` and `severity()` reflect
        // the variant. UI composition (`{glyph} {body}`) lives in ui.rs.
        let ok = StatusMessage::Success("Copied: /tmp/foo".into());
        assert_eq!(ok.severity(), StatusSeverity::Success);
        assert_eq!(ok.glyph(), '✓');
        assert_eq!(ok.body(), "Copied: /tmp/foo");

        let warn = StatusMessage::Warning("Could not copy: permission denied".into());
        assert_eq!(warn.severity(), StatusSeverity::Warning);
        assert_eq!(warn.glyph(), '⚠');
        assert_eq!(warn.body(), "Could not copy: permission denied");

        let pending = StatusMessage::Pending("Opening: ~/foo...".into());
        assert_eq!(pending.severity(), StatusSeverity::Pending);
        assert_eq!(pending.glyph(), '⏳');
        assert_eq!(pending.body(), "Opening: ~/foo...");
    }

    #[test]
    fn status_message_body_does_not_contain_glyph() {
        // Belt-and-braces invariant: regardless of variant, body() is the raw
        // inner string and never starts with a glyph or leading space. UI
        // rendering composes the glyph at display time, so any leading-glyph
        // bytes here would render double-glyphed (`✓ ✓ Copied...`).
        for msg in [
            StatusMessage::Success("Copied: /tmp/x".into()),
            StatusMessage::Warning("Could not copy: permission denied".into()),
            StatusMessage::Pending("Opening: ~/foo...".into()),
        ] {
            assert!(
                !msg.body().starts_with('✓')
                    && !msg.body().starts_with('⚠')
                    && !msg.body().starts_with('⏳'),
                "body() must not start with a glyph; got: {}",
                msg.body()
            );
            assert!(
                !msg.body().starts_with(' '),
                "body() must not start with a space; got: {:?}",
                msg.body()
            );
        }
    }

    #[test]
    fn status_message_set_by_view_source_and_cleared_by_any_key() {
        // Lifecycle contract for DetailAction::ViewSource — symmetric to the
        // CopyPath test above but exercises the `open`/`xdg-open` dispatch
        // path. We accept any of the three severities because:
        //   - Success/Warning depends on whether the opener succeeds or
        //     fails on this host (open/xdg-open presence, path validity,
        //     display server).
        //   - Pending is the transient state set BEFORE `.status()` blocks
        //     (POLISH-01) — in `execute_action` (no redraw closure), the
        //     `handle_view_source` dispatch overwrites it before returning,
        //     but the exhaustive arm guards against future refactors that
        //     might leave the message as Pending.
        let (mut app, _tmp) = make_app(3);

        app.execute_action(DetailAction::ViewSource);

        let msg = app
            .status_message
            .as_ref()
            .cloned()
            .expect("status_message must be Some after ViewSource action");
        match msg.severity() {
            StatusSeverity::Success => assert_eq!(
                msg.glyph(),
                '✓',
                "Success severity must produce ✓ glyph; got: {}",
                msg.glyph()
            ),
            StatusSeverity::Warning => assert_eq!(
                msg.glyph(),
                '⚠',
                "Warning severity must produce ⚠ glyph; got: {}",
                msg.glyph()
            ),
            StatusSeverity::Pending => assert_eq!(
                msg.glyph(),
                '⏳',
                "Pending severity must produce ⏳ glyph; got: {}",
                msg.glyph()
            ),
        }

        app.handle_key(
            KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE),
            &mut |_| {},
        );
        assert!(
            app.status_message.is_none(),
            "status_message must be None after any key; was: {:?}",
            app.status_message
        );
    }

    // POLISH-01 / TEST-03: status_message_from_open_result is the
    // factored helper for the three `Command::status()` arms. The
    // synthetic-ExitStatus tests use ExitStatusExt::from_raw on Unix
    // (see #462). Engineering a real opener failure on a CI runner is
    // racy — depends on whether xdg-open/open is installed and what the
    // OS does on missing MIME handlers — so we drive the helper with a
    // pre-built `Result<ExitStatus, io::Error>` instead.

    #[cfg(unix)]
    #[test]
    fn status_message_from_open_result_ok_success() {
        use std::os::unix::process::ExitStatusExt;
        // raw 0 = exit code 0 = success on Unix
        let status = std::process::ExitStatus::from_raw(0);
        let path = std::path::PathBuf::from("/tmp/foo");
        let msg = status_message_from_open_result("xdg-open", &path, Ok(status));
        assert!(
            matches!(msg, StatusMessage::Success(_)),
            "Ok+success must produce Success variant; got: {:?}",
            msg
        );
        assert!(
            msg.body().contains("Opened:"),
            "Success body must contain 'Opened:'; got: {}",
            msg.body()
        );
    }

    #[cfg(unix)]
    #[test]
    fn status_message_from_open_result_ok_nonzero_exit() {
        use std::os::unix::process::ExitStatusExt;
        // raw 0x100 = exit code 1 in the high byte (Unix wait status)
        let status = std::process::ExitStatus::from_raw(0x100);
        let path = std::path::PathBuf::from("/tmp/foo");
        let msg = status_message_from_open_result("xdg-open", &path, Ok(status));
        assert!(
            matches!(msg, StatusMessage::Warning(_)),
            "Ok+nonzero must produce Warning variant; got: {:?}",
            msg
        );
        assert!(
            msg.body().starts_with("xdg-open exited 1 for: "),
            "Warning body must include opener and exit code; got: {}",
            msg.body()
        );
    }

    #[test]
    fn status_message_from_open_result_err() {
        let err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let path = std::path::PathBuf::from("/tmp/foo");
        let msg = status_message_from_open_result("xdg-open", &path, Err(err));
        assert!(
            matches!(msg, StatusMessage::Warning(_)),
            "Err must produce Warning variant; got: {:?}",
            msg
        );
        assert_eq!(
            msg.body(),
            "Could not launch xdg-open: not found",
            "Err body must include opener and error display; got: {}",
            msg.body()
        );
    }

    #[test]
    fn view_source_invokes_redraw_callback_for_pending_status() {
        // POLISH-01: the Pending("Opening: ...") message must be painted
        // BEFORE `.status()` blocks. The contract that supports this is
        // that handle_view_source calls the redraw closure at least once.
        // We don't assert WHAT was drawn (the closure is opaque) — we
        // just count invocations. A missing call would mean the user
        // waits for the opener to return before seeing any feedback.
        let (mut app, _tmp) = make_app(3);
        let mut redraw_calls: u32 = 0;
        let mut redraw_cb = |_app: &App| {
            redraw_calls += 1;
        };
        app.execute_action_with_redraw(DetailAction::ViewSource, &mut redraw_cb);
        assert!(
            redraw_calls >= 1,
            "redraw must be called at least once for the Pending status (POLISH-01); got {} calls",
            redraw_calls
        );
    }

    #[test]
    fn drain_pending_events_returns_when_queue_empty() {
        // POLISH-01: drain_pending_events must terminate promptly when
        // the crossterm event queue is empty. A regression that blocks
        // on poll(non-zero) or read() with no event would hang the TUI
        // after every ViewSource action. The 100ms upper bound is safe —
        // an empty queue should drain in <1ms.
        let (app, _tmp) = make_app(3);
        let start = std::time::Instant::now();
        app.drain_pending_events();
        let elapsed = start.elapsed();
        assert!(
            elapsed < std::time::Duration::from_millis(100),
            "drain_pending_events must return promptly on empty queue; took {:?}",
            elapsed
        );
    }
}
