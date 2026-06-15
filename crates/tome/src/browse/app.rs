use std::fs;
use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::fuzzy;
use crate::config::DirectoryName;
use crate::discover::SkillName;
use crate::machine::{self, MachinePrefs};

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

/// Try `Clipboard::new().set_text(text)`. On `ClipboardOccupied`, sleep 100ms
/// and retry exactly once before returning the error. POLISH-03 / #463 D3:
/// `ClipboardOccupied` is the most common transient failure (another app
/// holding the clipboard mid-paste); a single 100ms backoff resolves the
/// vast majority of real-world cases without escalating a Warning to the
/// user.
///
/// All other `arboard::Error` variants return immediately — they are NOT
/// transient (`ClipboardNotSupported` is a session-level limitation;
/// `ContentNotAvailable` is a programming error; etc.) so retrying would
/// just delay the inevitable Warning.
///
/// Per #463 D-17/D-19, we do NOT introduce a trait abstraction here — the
/// retry is hard-coded against the real `arboard::Clipboard` API. The
/// retry shape is verified by source-grep + manual UAT, not by an
/// injected fake.
fn try_clipboard_set_text_with_retry(text: &str) -> Result<(), arboard::Error> {
    fn attempt(text: &str) -> Result<(), arboard::Error> {
        arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text.to_owned()))
    }

    match attempt(text) {
        Ok(()) => Ok(()),
        Err(arboard::Error::ClipboardOccupied) => {
            std::thread::sleep(std::time::Duration::from_millis(100));
            attempt(text)
        }
        Err(other) => Err(other),
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
pub enum DetailAction {
    ViewSource,
    CopyPath,
    Disable,
    Enable,
    Back,
}

impl DetailAction {
    /// HARD-21 D-BROWSE-2 — context-sensitive action-menu label.
    ///
    /// The label combines the verb (Disable / Enable) with the *scope*
    /// the toggle would mutate, but NEVER includes the skill name —
    /// the skill name appears in the StatusMessage body produced by
    /// `App::apply_toggle` (D-BROWSE-3 step 4), not here.
    ///
    /// Verbatim shapes:
    ///   - Global toggle:           "Disable on this machine"   /  "Enable on this machine"
    ///   - Per-directory blocklist: "Disable for <dir-name>"    /  "Enable for <dir-name>"
    ///   - Per-directory allowlist: "Disable for <dir-name>"    /  "Enable for <dir-name>"
    ///
    /// (Allowlist label is identical to the blocklist case — semantics
    /// differ but the user's mental model is "I'm disabling this for
    /// that directory", which is correct in both cases.)
    ///
    /// `row` and `prefs` are needed for D-BROWSE-1 scope detection
    /// (which list, if any, holds the skill's parent directory).
    /// Static fallback label used by `ui::render_detail` when either no
    /// row is selected or `MachinePrefs` aren't wired. Mirrors the
    /// pre-HARD-21 behavior so legacy test fixtures (no prefs) still
    /// render a sensible action menu.
    pub fn fallback_label(self) -> &'static str {
        match self {
            Self::ViewSource => "Open source directory",
            Self::CopyPath => "Copy path to clipboard",
            Self::Disable => "Disable on this machine",
            Self::Enable => "Enable on this machine",
            Self::Back => "Back",
        }
    }

    pub fn label(self, row: &SkillRow, prefs: &MachinePrefs) -> String {
        match self {
            Self::ViewSource => "Open source directory".to_string(),
            Self::CopyPath => "Copy path to clipboard".to_string(),
            Self::Disable | Self::Enable => {
                let verb = if matches!(self, Self::Disable) {
                    "Disable"
                } else {
                    "Enable"
                };
                let scope = ToggleScope::resolve(row, prefs);
                let scope_str = match scope {
                    ToggleScope::Global => "on this machine".to_string(),
                    ToggleScope::PerDirBlocklist(d) | ToggleScope::PerDirAllowlist(d) => {
                        format!("for {}", d.as_str())
                    }
                };
                format!("{verb} {scope_str}")
            }
            Self::Back => "Back".to_string(),
        }
    }
}

/// HARD-21 D-BROWSE-1 — which list does an Enable/Disable keystroke mutate?
///
/// Resolution order (most specific wins; MACH-04 invariant guarantees only
/// one of `disabled` / `enabled` is set per directory at a time):
///
///   1. Parent directory has a `disabled` blocklist set in `machine.toml` →
///      toggle that list.
///   2. Parent directory has an `enabled` allowlist set in `machine.toml` →
///      toggle that allowlist (inverted polarity: membership = "include").
///   3. Otherwise → toggle the global `MachinePrefs.disabled` set.
///
/// Unowned skills (`SkillRow.source_directory == None`) always fall to
/// `Global` — they have no parent directory in `tome.toml::directories`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToggleScope {
    Global,
    PerDirBlocklist(DirectoryName),
    PerDirAllowlist(DirectoryName),
}

impl ToggleScope {
    pub fn resolve(row: &SkillRow, prefs: &MachinePrefs) -> Self {
        let Some(dir) = row.source_directory.as_ref() else {
            return Self::Global;
        };
        let entry = match machine::directory_prefs(prefs, dir) {
            Some(e) => e,
            None => return Self::Global,
        };
        // A directory entry exists; route based on whether it carries a
        // blocklist (non-empty `disabled`), an allowlist (any `enabled`),
        // or neither. Empty-`disabled` + None-`enabled` is treated as
        // "no per-directory list set" and falls through to global.
        if !entry.disabled_set().is_empty() {
            Self::PerDirBlocklist(dir.clone())
        } else if entry.enabled_set().is_some() {
            Self::PerDirAllowlist(dir.clone())
        } else {
            Self::Global
        }
    }
}

/// HARD-21 D-BROWSE-3 step 3 — should the action menu show "Disable" or
/// "Enable" for this row, given current `MachinePrefs`?
///
/// The verb is determined by the skill's *current* state in the resolved
/// scope:
/// - `PerDirBlocklist`: skill IN blocklist → currently disabled → show Enable.
/// - `PerDirAllowlist`: skill NOT in allowlist → currently disabled → show Enable.
/// - `Global`:          skill IN global disabled set → show Enable.
///
/// Otherwise show Disable.
pub fn current_toggle_action(row: &SkillRow, prefs: &MachinePrefs) -> DetailAction {
    let scope = ToggleScope::resolve(row, prefs);
    let currently_disabled = match &scope {
        ToggleScope::Global => prefs.is_disabled(&row.name),
        ToggleScope::PerDirBlocklist(dir) => machine::directory_prefs(prefs, dir)
            .map(|d| d.disabled_set().iter().any(|s| s.as_str() == row.name))
            .unwrap_or(false),
        ToggleScope::PerDirAllowlist(dir) => machine::directory_prefs(prefs, dir)
            .and_then(|d| d.enabled_set())
            .map(|set| !set.iter().any(|s| s.as_str() == row.name))
            .unwrap_or(false),
    };
    if currently_disabled {
        DetailAction::Enable
    } else {
        DetailAction::Disable
    }
}

pub struct SkillRow {
    pub name: String,
    pub source: String,
    pub path: String,
    pub managed: bool,
    pub synced_at: String,
    /// Parent directory this skill came from in `tome.toml::directories`.
    /// `None` = Unowned (Phase 14 D-C1) — falls through to global toggle
    /// scope per D-BROWSE-1.
    pub source_directory: Option<DirectoryName>,
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
    /// HARD-21 — per-machine preferences mutated by `apply_toggle`. Held
    /// in-memory across the lifetime of the browse session; persisted
    /// to disk after every toggle via `machine::save(&prefs, &path)`
    /// (D-BROWSE-3 step 2). `None` for legacy callers that haven't
    /// wired prefs through (existing unit tests, CLI-smoke paths) —
    /// in that case Disable/Enable becomes a no-op that surfaces a
    /// Warning, preserving the v0.9 behavior byte-for-byte.
    pub(super) machine_prefs: Option<MachinePrefs>,
    /// Filesystem path for `machine::save` after a toggle. Companion to
    /// `machine_prefs`; both are `Some` together or `None` together.
    pub(super) machine_path: Option<PathBuf>,
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
            machine_prefs: None,
            machine_path: None,
        };
        app.apply_sort();
        app.refresh_preview();
        app
    }

    /// HARD-21 — wire per-machine prefs into the App so `Disable/Enable`
    /// toggles persist via `machine::save(&prefs, &path)` (D-BROWSE-3
    /// step 2). The browse module owns the prefs struct for the lifetime
    /// of the session; mutations happen in-memory first, then the entire
    /// struct is written via the existing atomic temp+rename pattern.
    pub fn with_machine_prefs(mut self, prefs: MachinePrefs, path: PathBuf) -> Self {
        self.machine_prefs = Some(prefs);
        self.machine_path = Some(path);
        self
    }

    /// Construct an `App` with deterministic state for snapshot tests
    /// (HARD-12). Avoids `Theme::detect()`'s `$COLORFGBG` env-var read
    /// (would make snapshots flake under different terminals) and
    /// pre-fills a stable preview body so the snapshot doesn't depend
    /// on filesystem reads.
    ///
    /// `filter` simulates an active fuzzy filter (`Some("foo")` would
    /// route through `refilter()` exactly like the user-typed flow).
    /// `theme` is injected so tests can explicitly cover both light
    /// and dark palettes.
    #[cfg(any(test, feature = "test-support"))]
    pub fn for_snapshot(
        rows: Vec<SkillRow>,
        theme: super::theme::Theme,
        filter: Option<&str>,
    ) -> Self {
        let match_indices = vec![Vec::new(); rows.len()];
        let filtered_indices: Vec<usize> = (0..rows.len()).collect();
        let preview_content = if rows.is_empty() {
            "No matching skill.".into()
        } else {
            // Stable, filesystem-independent preview body so the snapshot
            // doesn't depend on whether the rows' `path` directories
            // contain a real `SKILL.md` on the test runner.
            format!(
                "source: {}\npath: {}\n\n# {}\nA test skill body.",
                rows[0].source, rows[0].path, rows[0].name
            )
        };
        let preview_title = if rows.is_empty() {
            "Preview".into()
        } else {
            format!("Preview: {}", rows[0].name)
        };
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
            preview_title,
            preview_content,
            sort_mode: SortMode::Name,
            group_by_source: false,
            detail_actions: Vec::new(),
            detail_selected: 0,
            theme,
            status_message: None,
            machine_prefs: None,
            machine_path: None,
        };
        app.apply_sort();
        if let Some(f) = filter {
            app.search_input = f.to_string();
            // Re-run the fuzzy filter so filtered_indices / match_indices
            // reflect the filter exactly as if the user had typed it.
            // refresh_preview() runs in here too; it'll touch the
            // filesystem looking for SKILL.md, but we re-stamp
            // preview_content below so the snapshot is filesystem-stable.
            app.refilter();
        }
        // Force a stable preview body regardless of any filesystem
        // walk that `refilter()` may have triggered. This keeps the
        // snapshot deterministic across CI runners.
        if !app.filtered_indices.is_empty() {
            let row_idx = app.filtered_indices[app.selected];
            let row = &app.rows[row_idx];
            app.preview_title = format!("Preview: {}", row.name);
            app.preview_content = format!(
                "source: {}\npath: {}\n\n# {}\nA test skill body.",
                row.source, row.path, row.name
            );
        } else {
            app.preview_title = "Preview".into();
            app.preview_content = "No matching skill.".into();
        }
        app
    }

    /// Test-support: enter detail mode with the canonical action list
    /// so snapshot tests can render the detail pane without simulating
    /// an Enter keypress.
    #[cfg(any(test, feature = "test-support"))]
    pub fn enter_detail_mode_for_snapshot(&mut self) {
        self.enter_detail_mode();
    }

    /// Test-support: enter help mode so snapshot tests can render the
    /// help overlay without simulating a `?` keypress.
    #[cfg(any(test, feature = "test-support"))]
    pub fn enter_help_mode_for_snapshot(&mut self) {
        self.previous_mode = self.mode;
        self.mode = Mode::Help;
    }

    /// Test-support: invoke the production `execute_action` path so
    /// HARD-21 snapshot tests exercise the full toggle flow (apply
    /// plus label flip plus status surface) the way the keypress
    /// handler would. This is a thin wrapper because `execute_action`
    /// is `pub(super)` to keep the production surface tight.
    #[cfg(any(test, feature = "test-support"))]
    pub fn execute_action_for_snapshot(&mut self, action: DetailAction) {
        self.execute_action(action);
    }

    /// Test-support: re-run the fuzzy filter pipeline so a snapshot
    /// fixture that mutates `sort_mode` / `group_by_source` after
    /// construction picks up the new sort / grouping. Internal
    /// `refilter()` is private and threads through `refresh_preview()`
    /// which would touch the filesystem; this wrapper restamps the
    /// preview body afterwards to keep the snapshot deterministic.
    #[cfg(any(test, feature = "test-support"))]
    pub fn refilter_for_snapshot(&mut self) {
        self.refilter();
        // Re-stamp preview to the same filesystem-independent body as
        // `for_snapshot` so the post-refilter render stays stable.
        if !self.filtered_indices.is_empty() {
            let row_idx = self.filtered_indices[self.selected];
            let row = &self.rows[row_idx];
            self.preview_title = format!("Preview: {}", row.name);
            self.preview_content = format!(
                "source: {}\npath: {}\n\n# {}\nA test skill body.",
                row.source, row.path, row.name
            );
        } else {
            self.preview_title = "Preview".into();
            self.preview_content = "No matching skill.".into();
        }
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
        // HARD-21 D-BROWSE-3 step 3: surface either Disable or Enable
        // (never both) reflecting the skill's current state in the
        // smart-routed scope. When `machine_prefs` is `None` (legacy
        // callers / unit tests that haven't wired prefs), default to
        // showing Disable so the action menu structure stays stable.
        let toggle_action = self
            .selected_skill_row()
            .and_then(|row| {
                self.machine_prefs
                    .as_ref()
                    .map(|prefs| current_toggle_action(row, prefs))
            })
            .unwrap_or(DetailAction::Disable);

        self.detail_actions = vec![
            DetailAction::ViewSource,
            DetailAction::CopyPath,
            toggle_action,
            DetailAction::Back,
        ];
        self.detail_selected = 0;
    }

    /// Borrow the currently-selected `SkillRow`, if any. Companion to
    /// `selected_row_meta` (which clones the metadata into owned strings)
    /// for HARD-21 paths that need a reference to the row itself.
    fn selected_skill_row(&self) -> Option<&SkillRow> {
        let row_idx = *self.filtered_indices.get(self.selected)?;
        self.rows.get(row_idx)
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
                    // POLISH-03: `try_clipboard_set_text_with_retry` retries
                    // `ClipboardOccupied` once after a 100ms backoff before
                    // surfacing a Warning. All other arboard errors return
                    // immediately. The CopyPath message body strings are
                    // unchanged so existing UAT/snapshot expectations don't
                    // drift; `ClipboardOccupied` now fires only after the
                    // retry has also failed.
                    let result = try_clipboard_set_text_with_retry(&path);
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
                // HARD-21 D-BROWSE-1/-2/-3: toggle the skill in the
                // resolved scope, save machine.toml atomically, and
                // surface a scope-explicit StatusMessage::Success.
                // Errors surface as Warning per the existing pattern.
                if let Err(e) = self.apply_toggle(action) {
                    self.status_message = Some(StatusMessage::Warning(format!(
                        "Could not save machine.toml: {e}"
                    )));
                }
                // Re-render the action label by rebuilding the action
                // list against the now-mutated prefs (D-BROWSE-3 step 3).
                // Stay in Detail mode so the user can immediately undo
                // (press the inverse to flip back).
                self.refresh_detail_actions();
            }
            DetailAction::Back => {
                self.mode = Mode::Normal;
            }
        }
    }

    /// Rebuild `detail_actions` against the current `machine_prefs`,
    /// preserving the user's selection cursor on whichever toggle
    /// (Disable | Enable) is now appropriate. Called after `apply_toggle`
    /// so the row's action label flips immediately (D-BROWSE-3 step 3).
    fn refresh_detail_actions(&mut self) {
        let toggle_action = self
            .selected_skill_row()
            .and_then(|row| {
                self.machine_prefs
                    .as_ref()
                    .map(|prefs| current_toggle_action(row, prefs))
            })
            .unwrap_or(DetailAction::Disable);
        // Slot 2 is the toggle (matches `enter_detail_mode` ordering).
        if let Some(slot) = self.detail_actions.get_mut(2) {
            *slot = toggle_action;
        }
    }

    /// HARD-21 — apply the user's Disable/Enable keystroke per
    /// D-BROWSE-1 smart-routing, save `machine.toml` atomically per
    /// step 2, and surface a `StatusMessage::Success` per step 4.
    ///
    /// Steps:
    ///   1. Resolve scope (PerDirBlocklist | PerDirAllowlist | Global).
    ///   2. Mutate `MachinePrefs` in-memory.
    ///   3. Save `machine.toml` atomically (existing temp+rename).
    ///   4. Stamp a scope-explicit `StatusMessage::Success` body —
    ///      "Disabled <skill> on this machine" / "Enabled <skill> for <dir>"
    ///      etc. This body is DISTINCT from the action-menu label
    ///      (label has no skill name; body does).
    pub(crate) fn apply_toggle(&mut self, action: DetailAction) -> anyhow::Result<()> {
        let was_disable = matches!(action, DetailAction::Disable);
        let row = self
            .selected_skill_row()
            .ok_or_else(|| anyhow::anyhow!("no skill selected"))?;
        let row_name = row.name.clone();
        let row_dir = row.source_directory.clone();
        let scope = match self.machine_prefs.as_ref() {
            Some(prefs) => ToggleScope::resolve(row, prefs),
            None => {
                anyhow::bail!("machine prefs not wired into browse session");
            }
        };

        // Build a SkillName with the lenient validator (rejects empty +
        // path separators); browse rows always carry well-formed names
        // by construction (DiscoveredSkill::name was validated upstream),
        // so this is belt-and-braces.
        let skill = SkillName::new(&row_name)
            .map_err(|e| anyhow::anyhow!("invalid skill name '{row_name}': {e}"))?;

        // The Global scope routes through the shared `tome::actions` helper
        // — the same code path the GUI's `set_skill_disabled` Tauri command
        // uses (Phase 26 plan 26-03 / D-06). PerDir scopes stay inline
        // because their semantics (per-directory blocklist / allowlist) are
        // TUI-only and not part of the shared GUI surface today.
        let path = self
            .machine_path
            .clone()
            .ok_or_else(|| anyhow::anyhow!("machine path not wired into browse session"))?;

        match &scope {
            ToggleScope::Global => {
                // Shared helper does load-mutate-save in one atomic
                // temp+rename. We then re-sync the in-memory prefs from
                // disk so the next render and the `current_toggle_action`
                // lookup observe the same state the GUI would.
                crate::actions::set_skill_disabled(&skill, was_disable, &path)?;
                let reloaded = machine::load(&path)?;
                *self
                    .machine_prefs
                    .as_mut()
                    .expect("machine_prefs must be Some after the Some-arm above") = reloaded;
            }
            ToggleScope::PerDirBlocklist(dir) => {
                // PerDir arms still mutate in-memory + atomic save inline
                // (HARD-21 D-BROWSE-1 routing — not shared with the GUI's
                // global-only D-06 surface).
                {
                    let prefs = self
                        .machine_prefs
                        .as_mut()
                        .expect("machine_prefs must be Some after the Some-arm above");
                    prefs.toggle_per_dir_blocklist(dir, skill, was_disable);
                }
                let prefs_immut = self.machine_prefs.as_ref().expect("Some after mutation");
                machine::save(prefs_immut, &path)?;
            }
            ToggleScope::PerDirAllowlist(dir) => {
                {
                    let prefs = self
                        .machine_prefs
                        .as_mut()
                        .expect("machine_prefs must be Some after the Some-arm above");
                    prefs.toggle_per_dir_allowlist(dir, skill, was_disable);
                }
                let prefs_immut = self.machine_prefs.as_ref().expect("Some after mutation");
                machine::save(prefs_immut, &path)?;
            }
        }

        // Step 4: scope-explicit StatusMessage::Success body. Skill name
        // appears in the body (NOT in the action-menu label per D-BROWSE-2).
        let verb_past = if was_disable { "Disabled" } else { "Enabled" };
        let body = match &scope {
            ToggleScope::Global => format!("{verb_past} {row_name} on this machine"),
            ToggleScope::PerDirBlocklist(d) | ToggleScope::PerDirAllowlist(d) => {
                format!("{verb_past} {row_name} for {}", d.as_str())
            }
        };
        self.status_message = Some(StatusMessage::Success(body));
        // Suppress unused-variable warning when row_dir isn't read in
        // the future (it's already implicit in `scope`); kept as a
        // breadcrumb for downstream UI work.
        let _ = row_dir;
        Ok(())
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
                    source_directory: None,
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
            source_directory: None,
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
            source_directory: None,
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
            source_directory: None,
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

    // POLISH-03 / #463 D3: ClipboardOccupied auto-retry. Per the design
    // doc (D-17/D-19), arboard is NOT abstracted behind a trait — the
    // retry is hard-coded against the real `arboard::Clipboard` API.
    // That means we cannot mock the two attempts; the test surface is:
    //   (a) the wall-clock bound (one fast attempt + at most one 100ms
    //       backoff = <250ms total),
    //   (b) the helper signature compiles,
    //   (c) the existing CopyPath lifecycle test continues to pass.
    // Manual UAT covers the actual two-attempt behavior.

    #[test]
    fn copy_path_retry_helper_returns_within_bound() {
        // The retry contract: at most one fast attempt + one 100ms-delayed
        // attempt on `ClipboardOccupied`. The wall-clock budget below
        // catches the regression we care about — a SECOND 100ms sleep
        // creeping in (e.g. a `loop` instead of one retry) — without
        // flaking on macOS arboard's variable per-call latency under
        // parallel test execution.
        //
        // Empirical breakdown (macOS, parallel `cargo test`):
        //   - happy path: arboard::Clipboard::new() + set_text() ≈ 5–500ms
        //     (NSPasteboard contention, system load)
        //   - ClipboardOccupied path: first attempt + 100ms sleep + second
        //     attempt ≈ 100–600ms
        //   - regression (double retry): adds another ~100ms ⇒ ~700ms+
        //
        // 600ms catches the regression while tolerating the parallel-test
        // contention we observe in practice. A future refactor that adds
        // a SECOND retry hop (or replaces the sleep with a longer one)
        // would push past this bound and surface the regression.
        let start = std::time::Instant::now();
        let _ = super::try_clipboard_set_text_with_retry("test-payload");
        let elapsed = start.elapsed();
        // FLAKE-FIX (#511 / HARD-14): bound relaxed from 600ms to 2000ms.
        // arboard clipboard contention under --test-threads=N can pause threads
        // ≫ 600ms regardless of helper performance — NSPasteboard / X11 clipboard
        // server / WinClipboard arbitration is opaque to user code. This assertion
        // guards against actual hangs (an unbounded retry `loop`), NOT perf
        // regressions. A 2000ms bound catches a 10×-retry regression while
        // tolerating realistic parallel-test contention.
        //
        // Deterministic clock injection (trait Clock in browse::app) was
        // considered but rejected for v0.11 scope (D-FLAKE-3). If this bound
        // flakes again post-fix, the abstraction can be introduced.
        assert!(
            elapsed < std::time::Duration::from_millis(2000),
            "retry helper must complete within 2000ms (one fast + one 100ms backoff, even under \
             parallel-test clipboard contention); took {:?}",
            elapsed
        );
    }

    #[test]
    fn copy_path_retry_helper_signature_compiles() {
        // Smoke test: ensures the helper exists with the documented
        // signature `fn(&str) -> Result<(), arboard::Error>`. If a
        // future refactor changes the type (e.g. accepts a different
        // error type, or returns Result<bool, _>), this fails to
        // compile and the issue surfaces before the source-grep
        // checks in acceptance_criteria run.
        let _: fn(&str) -> Result<(), arboard::Error> = super::try_clipboard_set_text_with_retry;
    }

    // ===========================================================================
    // HARD-21 — DetailAction::{Disable, Enable} wiring tests
    //
    // Coverage matrix:
    //   - Smart-routing (D-BROWSE-1): global / per-dir blocklist /
    //     per-dir allowlist (inverted polarity) / undo via inverse.
    //   - Action-menu LABEL (D-BROWSE-2): verb + scope, NO skill name.
    //   - Status-message BODY (D-BROWSE-3 step 4): verb + skill + scope.
    //   - 4-step toggle flow assertions (D-BROWSE-3 steps 1–4).
    //
    // Both label() and apply_toggle() are tested against the real
    // MachinePrefs surface — no mocking — so the contract is enforced
    // end-to-end through machine.rs.
    // ===========================================================================

    use crate::config::DirectoryName;
    use crate::discover::SkillName;
    use crate::machine::{self, DirectoryOverride, MachinePrefs};

    /// Build a SkillRow that points at directory `dir` (None = Unowned).
    fn toggle_row(name: &str, dir: Option<&str>) -> SkillRow {
        SkillRow {
            name: name.to_string(),
            source: dir.unwrap_or("(unowned)").to_string(),
            path: format!("/library/{}", name),
            managed: false,
            synced_at: String::new(),
            source_directory: dir.map(|d| DirectoryName::new(d).unwrap()),
        }
    }

    /// Build an App fixture for HARD-21 tests with a single skill row,
    /// machine_prefs wired to a tmpdir-local machine.toml. The path is
    /// returned via the TempDir guard so tests can re-load and inspect
    /// the on-disk content (D-BROWSE-3 step 2 round-trip).
    fn toggle_app(rows: Vec<SkillRow>, prefs: MachinePrefs) -> (App, tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let machine_path = tmp.path().join("machine.toml");
        let mut app = App::new(rows).with_machine_prefs(prefs, machine_path.clone());
        app.visible_height = 5;
        (app, tmp, machine_path)
    }

    /// Insert a per-directory blocklist for `dir` containing `skills`.
    fn seed_blocklist(prefs: &mut MachinePrefs, dir: &str, skills: &[&str]) {
        for s in skills {
            prefs.toggle_per_dir_blocklist(
                &DirectoryName::new(dir).unwrap(),
                SkillName::new(*s).unwrap(),
                true,
            );
        }
    }

    /// Insert a per-directory allowlist for `dir` containing `skills`.
    fn seed_allowlist(prefs: &mut MachinePrefs, dir: &str, skills: &[&str]) {
        for s in skills {
            prefs.toggle_per_dir_allowlist(
                &DirectoryName::new(dir).unwrap(),
                SkillName::new(*s).unwrap(),
                false, // false = enable (insert into allowlist)
            );
        }
    }

    // -------- D-BROWSE-1 smart-routing --------

    #[test]
    fn apply_toggle_global_when_no_per_dir_list() {
        // Skill foo is in directory bar, but bar has neither a blocklist
        // nor an allowlist set in machine.toml. Disabling foo must mutate
        // the GLOBAL `disabled` set.
        let prefs = MachinePrefs::default();
        let (mut app, _tmp, _path) = toggle_app(vec![toggle_row("foo", Some("bar"))], prefs);
        app.apply_toggle(DetailAction::Disable).unwrap();
        let prefs = app.machine_prefs.as_ref().unwrap();
        assert!(
            prefs.is_disabled("foo"),
            "global disabled set must contain foo"
        );
    }

    #[test]
    fn apply_toggle_per_dir_blocklist() {
        // Bar already has a blocklist (containing baz). Disabling foo (in
        // bar) inserts foo into bar's blocklist — never touches global.
        let mut prefs = MachinePrefs::default();
        seed_blocklist(&mut prefs, "bar", &["baz"]);
        let (mut app, _tmp, _path) = toggle_app(vec![toggle_row("foo", Some("bar"))], prefs);
        app.apply_toggle(DetailAction::Disable).unwrap();
        let prefs = app.machine_prefs.as_ref().unwrap();
        let dir = machine::directory_prefs(prefs, &DirectoryName::new("bar").unwrap()).unwrap();
        assert!(
            dir.disabled_set().iter().any(|s| s.as_str() == "foo"),
            "per-dir blocklist must contain foo"
        );
        assert!(
            !prefs.is_disabled("foo"),
            "global disabled set must NOT contain foo (per-dir blocklist scope)"
        );
    }

    #[test]
    fn apply_toggle_per_dir_allowlist_inverted_polarity() {
        // Bar has an allowlist (containing foo). Disabling foo REMOVES it
        // from the allowlist (inverted polarity); the disabled blocklist
        // stays None (MACH-04 invariant preserved).
        let mut prefs = MachinePrefs::default();
        seed_allowlist(&mut prefs, "bar", &["foo"]);
        let (mut app, _tmp, _path) = toggle_app(vec![toggle_row("foo", Some("bar"))], prefs);
        app.apply_toggle(DetailAction::Disable).unwrap();
        let prefs = app.machine_prefs.as_ref().unwrap();
        let dir = machine::directory_prefs(prefs, &DirectoryName::new("bar").unwrap()).unwrap();
        let allowlist = dir
            .enabled_set()
            .expect("allowlist must remain set after Disable on allowlist scope");
        assert!(
            !allowlist.iter().any(|s| s.as_str() == "foo"),
            "Disable on allowlist scope must REMOVE foo from allowlist (inverted polarity)"
        );
        assert!(
            dir.disabled_set().is_empty(),
            "MACH-04: disabled set must stay empty when allowlist is in use"
        );
    }

    #[test]
    fn apply_toggle_undo_via_inverse() {
        // Disable then Enable round-trips to the original state.
        let prefs = MachinePrefs::default();
        let (mut app, _tmp, _path) = toggle_app(vec![toggle_row("foo", Some("bar"))], prefs);

        app.apply_toggle(DetailAction::Disable).unwrap();
        assert!(app.machine_prefs.as_ref().unwrap().is_disabled("foo"));

        app.apply_toggle(DetailAction::Enable).unwrap();
        assert!(!app.machine_prefs.as_ref().unwrap().is_disabled("foo"));
    }

    // -------- D-BROWSE-2 action-menu label (verb + scope, NO skill name) --------

    #[test]
    fn label_global_scope_disable() {
        let prefs = MachinePrefs::default();
        let row = toggle_row("foo", Some("bar"));
        assert_eq!(
            DetailAction::Disable.label(&row, &prefs),
            "Disable on this machine"
        );
    }

    #[test]
    fn label_global_scope_enable() {
        let prefs = MachinePrefs::default();
        let row = toggle_row("foo", Some("bar"));
        assert_eq!(
            DetailAction::Enable.label(&row, &prefs),
            "Enable on this machine"
        );
    }

    #[test]
    fn label_per_dir_blocklist() {
        let mut prefs = MachinePrefs::default();
        seed_blocklist(&mut prefs, "my-dir", &["baz"]);
        let row = toggle_row("foo", Some("my-dir"));
        assert_eq!(
            DetailAction::Disable.label(&row, &prefs),
            "Disable for my-dir"
        );
        assert_eq!(
            DetailAction::Enable.label(&row, &prefs),
            "Enable for my-dir"
        );
    }

    #[test]
    fn label_per_dir_allowlist() {
        let mut prefs = MachinePrefs::default();
        seed_allowlist(&mut prefs, "my-dir", &["foo"]);
        let row = toggle_row("foo", Some("my-dir"));
        assert_eq!(
            DetailAction::Disable.label(&row, &prefs),
            "Disable for my-dir"
        );
    }

    #[test]
    fn label_does_not_contain_skill_name() {
        // D-BROWSE-2: the action-menu label NEVER contains the skill name.
        // The skill name appears in the StatusMessage body (D-BROWSE-3
        // step 4), not in the label.
        let mut prefs = MachinePrefs::default();
        seed_blocklist(&mut prefs, "my-dir", &["other-skill"]);
        let row = toggle_row("hardrocket", Some("my-dir"));
        for variant in [DetailAction::Disable, DetailAction::Enable] {
            let label = variant.label(&row, &prefs);
            assert!(
                !label.contains("hardrocket"),
                "label MUST NOT contain skill name; got: {label}"
            );
        }
    }

    // -------- D-BROWSE-3 status-message body (verb + skill + scope) --------

    #[test]
    fn apply_toggle_status_message_global_disable() {
        let prefs = MachinePrefs::default();
        let (mut app, _tmp, _path) = toggle_app(vec![toggle_row("foo", Some("bar"))], prefs);
        app.apply_toggle(DetailAction::Disable).unwrap();
        let msg = app
            .status_message
            .as_ref()
            .expect("status_message must be Some");
        assert_eq!(msg.body(), "Disabled foo on this machine");
    }

    #[test]
    fn apply_toggle_status_message_global_enable() {
        let mut prefs = MachinePrefs::default();
        prefs.toggle_global_disabled(SkillName::new("foo").unwrap(), true);
        let (mut app, _tmp, _path) = toggle_app(vec![toggle_row("foo", Some("bar"))], prefs);
        app.apply_toggle(DetailAction::Enable).unwrap();
        let msg = app
            .status_message
            .as_ref()
            .expect("status_message must be Some");
        assert_eq!(msg.body(), "Enabled foo on this machine");
    }

    #[test]
    fn apply_toggle_status_message_per_dir_disable() {
        let mut prefs = MachinePrefs::default();
        seed_blocklist(&mut prefs, "my-dir", &["other"]);
        let (mut app, _tmp, _path) = toggle_app(vec![toggle_row("foo", Some("my-dir"))], prefs);
        app.apply_toggle(DetailAction::Disable).unwrap();
        let msg = app
            .status_message
            .as_ref()
            .expect("status_message must be Some");
        assert_eq!(msg.body(), "Disabled foo for my-dir");
    }

    #[test]
    fn apply_toggle_status_message_per_dir_enable() {
        let mut prefs = MachinePrefs::default();
        seed_blocklist(&mut prefs, "my-dir", &["foo"]);
        let (mut app, _tmp, _path) = toggle_app(vec![toggle_row("foo", Some("my-dir"))], prefs);
        app.apply_toggle(DetailAction::Enable).unwrap();
        let msg = app
            .status_message
            .as_ref()
            .expect("status_message must be Some");
        assert_eq!(msg.body(), "Enabled foo for my-dir");
    }

    // -------- D-BROWSE-3 4-step flow assertions --------

    #[test]
    fn apply_toggle_step1_mutates_in_memory() {
        // Step 1: in-memory MachinePrefs reflects the toggle BEFORE save.
        let prefs = MachinePrefs::default();
        let (mut app, _tmp, _path) = toggle_app(vec![toggle_row("foo", Some("bar"))], prefs);
        assert!(!app.machine_prefs.as_ref().unwrap().is_disabled("foo"));
        app.apply_toggle(DetailAction::Disable).unwrap();
        assert!(
            app.machine_prefs.as_ref().unwrap().is_disabled("foo"),
            "step 1: in-memory MachinePrefs.is_disabled must flip"
        );
    }

    #[test]
    fn apply_toggle_step2_atomic_save_round_trip() {
        // Step 2: machine.toml on disk reflects the toggle (load + re-read).
        let prefs = MachinePrefs::default();
        let (mut app, _tmp, path) = toggle_app(vec![toggle_row("foo", Some("bar"))], prefs);
        app.apply_toggle(DetailAction::Disable).unwrap();

        // Reload from disk.
        let reloaded = machine::load(&path).expect("reload machine.toml");
        assert!(
            reloaded.is_disabled("foo"),
            "step 2: on-disk machine.toml must reflect toggle"
        );
    }

    #[test]
    fn apply_toggle_step3_label_flips() {
        // Step 3: DetailAction::label() flips Disable ↔ Enable across the
        // toggle. We probe via current_toggle_action() which is what
        // enter_detail_mode/refresh_detail_actions consult.
        let prefs = MachinePrefs::default();
        let (mut app, _tmp, _path) = toggle_app(vec![toggle_row("foo", Some("bar"))], prefs);

        // Before: foo is enabled → menu would show Disable.
        let row = toggle_row("foo", Some("bar"));
        let before = current_toggle_action(&row, app.machine_prefs.as_ref().unwrap());
        assert_eq!(before, DetailAction::Disable);
        let label_before = before.label(&row, app.machine_prefs.as_ref().unwrap());
        assert_eq!(label_before, "Disable on this machine");

        app.apply_toggle(DetailAction::Disable).unwrap();

        // After: foo is disabled (globally) → menu now shows Enable.
        let after = current_toggle_action(&row, app.machine_prefs.as_ref().unwrap());
        assert_eq!(after, DetailAction::Enable);
        let label_after = after.label(&row, app.machine_prefs.as_ref().unwrap());
        assert_eq!(label_after, "Enable on this machine");
    }

    #[test]
    fn apply_toggle_step4_surfaces_success_status() {
        // Step 4: status_message is Some(StatusMessage::Success { .. })
        // with the verbatim D-BROWSE-3 body shape.
        let prefs = MachinePrefs::default();
        let (mut app, _tmp, _path) = toggle_app(vec![toggle_row("foo", Some("bar"))], prefs);
        app.apply_toggle(DetailAction::Disable).unwrap();
        let msg = app
            .status_message
            .as_ref()
            .expect("step 4: status_message must be Some");
        assert!(
            matches!(msg, StatusMessage::Success(_)),
            "step 4: must be Success variant, got: {:?}",
            msg
        );
        assert_eq!(msg.body(), "Disabled foo on this machine");
        assert_eq!(msg.glyph(), '✓');
    }

    // -------- MACH-04 invariant + miscellaneous regression --------

    #[test]
    fn toggle_never_sets_both_disabled_and_enabled() {
        // MACH-04 regression: regardless of toggle path, MachinePrefs
        // validation must not trip (would fail if a directory had both
        // `disabled` and `enabled` set).
        let mut prefs = MachinePrefs::default();
        seed_blocklist(&mut prefs, "my-dir", &["other"]);
        let (mut app, _tmp, _path) = toggle_app(vec![toggle_row("foo", Some("my-dir"))], prefs);

        app.apply_toggle(DetailAction::Disable).unwrap();
        app.apply_toggle(DetailAction::Enable).unwrap();
        app.machine_prefs
            .as_ref()
            .unwrap()
            .validate()
            .expect("MACH-04: validate must pass after toggle round-trip");
    }

    #[test]
    fn no_dead_code_attr_above_detail_action() {
        // HARD-21 acceptance: `#[allow(dead_code)]` must NOT decorate
        // the DetailAction enum once the variants are wired. We check
        // by reading our own source — a regression that re-adds the
        // attr fails this assertion.
        let src = include_str!("app.rs");
        let snippet = src
            .lines()
            .skip_while(|l| !l.contains("pub enum DetailAction"))
            .take(2)
            .collect::<Vec<_>>()
            .join("\n");
        // The line BEFORE `pub enum DetailAction` is the relevant one;
        // we look at the 5 lines before it for any allow(dead_code).
        let pos = src
            .find("pub enum DetailAction")
            .expect("DetailAction must exist");
        let preceding = &src[..pos];
        let last_block = preceding.lines().rev().take(5).collect::<Vec<_>>();
        for line in &last_block {
            assert!(
                !line.contains("dead_code"),
                "HARD-21: #[allow(dead_code)] must NOT precede DetailAction; offending line: {line}\nSnippet:\n{snippet}"
            );
        }
    }

    // Suppress unused-import warning for DirectoryOverride (it's reachable
    // through the test-support integration but we don't probe it directly).
    #[test]
    fn _imports_compile() {
        let _: Option<DirectoryOverride> = None;
    }
}
