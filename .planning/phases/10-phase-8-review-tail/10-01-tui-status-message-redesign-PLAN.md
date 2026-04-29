---
phase: 10-phase-8-review-tail
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/tome/src/browse/app.rs
  - crates/tome/src/browse/ui.rs
  - crates/tome/src/browse/mod.rs
autonomous: true
requirements: [POLISH-01, POLISH-02, POLISH-03, TEST-03]
issue: "https://github.com/MartinP7r/tome/issues/463 + https://github.com/MartinP7r/tome/issues/462"

must_haves:
  truths:
    - "User pressing Enter on the `Open source directory` action sees an `⏳ Opening: <collapsed-path>...` status message rendered BEFORE the `xdg-open`/`open` blocking call returns; the message is visible during the block."
    - "Any keystrokes typed into the tty during the `xdg-open`/`open` block are drained from the crossterm event queue afterwards and do NOT replay as DetailAction inputs."
    - "`StatusMessage` is a single enum (`Success(String) | Warning(String) | Pending(String)`) with `body() -> &str`, `glyph() -> char`, and `severity() -> StatusSeverity` accessors. The struct form (`pub struct StatusMessage { severity, text }`) and the `text` field with pre-formatted glyph are GONE."
    - "UI rendering composes `format!(\"{glyph} {body}\", ...)` at render time in both `render_status_bar` and the Detail-mode status branch — neither call site reaches into a pre-formatted `text` field."
    - "`StatusMessage` and `StatusSeverity` are both `pub(super)` (browse-module-internal) — not `pub`."
    - "`arboard::Error::ClipboardOccupied` triggers exactly one retry after a 100ms `std::thread::sleep` before a Warning ever reaches `app.status_message`. A successful retry produces a Success message; a still-occupied retry produces the existing `⚠ Clipboard busy ...` Warning."
    - "`ViewSource` `.status()` arms (Ok+success, Ok+non-zero exit, Err) are factored into `status_message_from_open_result(binary: &str, path: &Path, raw: std::io::Result<std::process::ExitStatus>) -> StatusMessage` with unit tests for all three arms using `std::os::unix::process::ExitStatusExt::from_raw`."
    - "The `Pending`-message visibility is achieved by threading a redraw closure (`&mut dyn FnMut(&App)`) through `App::handle_key` → `handle_detail_key` → a new `App::execute_action_with_redraw` method. Production `run_loop` passes a closure that calls `terminal.draw(|f| ui::render(f, app))`; tests pass a no-op closure. `App` itself remains independent of `ratatui::DefaultTerminal` (preserving unit-test isolation)."
  artifacts:
    - path: "crates/tome/src/browse/app.rs"
      provides: "Refactored `StatusMessage` enum (`Success(String) | Warning(String) | Pending(String)`) with `pub(super)` visibility, `body()`/`glyph()`/`severity()` accessors. Free function `status_message_from_open_result(binary, path, raw)` returning a `StatusMessage`. New method `execute_action_with_redraw(&mut self, action, redraw: &mut dyn FnMut(&App))` that handles ViewSource (sets Pending → calls redraw → calls .status() → drains events) and delegates other actions to `execute_action`. `handle_key` and `handle_detail_key` gain the redraw-closure parameter. `try_clipboard_set_text_with_retry` retries `ClipboardOccupied` once after a 100ms backoff."
      contains: "status_message_from_open_result"
    - path: "crates/tome/src/browse/ui.rs"
      provides: "`render_status_bar` and the Detail-mode status branch in `render_detail` both compose the rendered string via `format!(\"{} {}\", msg.glyph(), msg.body())` and dispatch fg color via `match msg.severity()` — no `msg.text` access remains. Both call sites gain a `StatusSeverity::Pending` arm using `theme.muted`."
      contains: "msg.severity()"
    - path: "crates/tome/src/browse/mod.rs"
      provides: "`run_loop` constructs a real redraw closure `|a: &App| { let _ = terminal.draw(|f| ui::render(f, a)); }` and passes it to `app.handle_key(key, &mut redraw)`. The redraw closure is what enables the Pending-message visibility before `.status()` blocks (POLISH-01)."
      contains: "handle_key"
  key_links:
    - from: "crates/tome/src/browse/app.rs::execute_action_with_redraw ViewSource branch"
      to: "crates/tome/src/browse/app.rs::status_message_from_open_result"
      via: "called with the `.status()` Result to convert to a StatusMessage"
      pattern: "status_message_from_open_result"
    - from: "crates/tome/src/browse/app.rs::execute_action_with_redraw ViewSource branch"
      to: "redraw + drain"
      via: "set Pending message → call redraw(self) → run `.status()` → call drain_pending_events()"
      pattern: "Pending\\("
    - from: "crates/tome/src/browse/app.rs::execute_action CopyPath arm"
      to: "ClipboardOccupied retry"
      via: "match arboard::Error::ClipboardOccupied → sleep 100ms → retry once (in `try_clipboard_set_text_with_retry`)"
      pattern: "ClipboardOccupied"
    - from: "crates/tome/src/browse/ui.rs::render_status_bar + render_detail status branch"
      to: "StatusMessage accessors"
      via: "msg.glyph() + msg.body() + msg.severity()"
      pattern: "msg\\.severity\\(\\)"
    - from: "crates/tome/src/browse/mod.rs::run_loop"
      to: "App::handle_key redraw closure"
      via: "&mut |a: &App| { let _ = terminal.draw(|f| ui::render(f, a)); }"
      pattern: "handle_key.*&mut redraw"
---

<objective>
Redesign `StatusMessage` from a struct-with-pre-formatted-text to a single enum with semantic constructors and accessors. Route the `ViewSource` `.status()` dispatch through a testable free helper. Add an `Opening: <path>...` Pending status BEFORE the `xdg-open`/`open` block, drain the crossterm event queue after the block to swallow keystrokes typed during it, and auto-retry `ClipboardOccupied` once with a 100ms backoff before any Warning reaches the status bar.

This is the TUI bundle from the Phase 8 review tail — closes 4 of the 11 review-tail items in one cut, all centred on `crates/tome/src/browse/app.rs`, `crates/tome/src/browse/ui.rs`, and `crates/tome/src/browse/mod.rs`.

**Closes:** POLISH-01 (D1, blocking-open UX), POLISH-02 (D2, StatusMessage type redesign), POLISH-03 (D3, ClipboardOccupied auto-retry), TEST-03 (P3, status_message_from_open_result helper + unit tests).

**Decision pinned (POLISH-01 redraw threading):** Use the **redraw-closure** approach — `App::handle_key` gains a `redraw: &mut dyn FnMut(&App)` parameter, threaded down to a new `App::execute_action_with_redraw` method. Production `run_loop` passes a closure that calls `terminal.draw(...)`; tests pass a no-op closure. This keeps `App` independent of `ratatui::DefaultTerminal` (preserving unit-test isolation) and allows the redraw to fire BEFORE `.status()` blocks. Rejected alternatives: (a) a `pending_redraw: bool` flag on `App` — only redraws AFTER `.status()` returns, which is too late; (b) threading `&mut ratatui::DefaultTerminal` directly through `handle_key` — couples `App` to the ratatui type and breaks unit-test isolation.

Purpose: Make the `tome browse` `open`/`copy` paths feel responsive (Pending message before block, no replayed keystrokes), eliminate the stringly-typed glyph-prefix in `StatusMessage::text`, and make the three `.status()` arms unit-testable on synthetic exit codes.

Output: Refactored `StatusMessage` enum + accessors, new `status_message_from_open_result` helper, redraw-closure threading through `handle_key`, drain-after-block in ViewSource, retry-once in CopyPath, ui.rs callsites updated.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/REQUIREMENTS.md

@crates/tome/src/browse/app.rs
@crates/tome/src/browse/ui.rs
@crates/tome/src/browse/mod.rs
@crates/tome/src/browse/theme.rs

<interfaces>
<!-- Key types and contracts the executor needs. Extracted from codebase. -->

Current shape of `StatusMessage` in `crates/tome/src/browse/app.rs` (lines 19–54):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusSeverity {
    Success,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusMessage {
    pub severity: StatusSeverity,
    pub text: String,   // <-- pre-formatted with leading glyph "✓ " / "⚠ "
}

impl StatusMessage {
    pub fn success(body: impl Into<String>) -> Self { /* prefixes "✓ " */ }
    pub fn warning(body: impl Into<String>) -> Self { /* prefixes "⚠ " */ }
}
```

Target shape after this plan:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StatusSeverity { Success, Warning, Pending }

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum StatusMessage {
    Success(String),
    Warning(String),
    Pending(String),
}

impl StatusMessage {
    pub(super) fn body(&self) -> &str;       // returns inner String slice
    pub(super) fn glyph(&self) -> char;      // '✓' / '⚠' / '⏳'
    pub(super) fn severity(&self) -> StatusSeverity;
}
```

Current `ViewSource` arm in `execute_action` (`crates/tome/src/browse/app.rs` lines 252–293):
```rust
match std::process::Command::new(binary).arg(&path).status() {
    Ok(status) if status.success() => self.status_message = Some(StatusMessage::success(format!("Opened: {}", ...))),
    Ok(status) => self.status_message = Some(StatusMessage::warning(format!("{binary} exited {exit} for: {path}"))),
    Err(e)   => self.status_message = Some(StatusMessage::warning(format!("Could not launch {binary}: {e}"))),
}
```

Target shape — the ViewSource branch lives in a new `App::handle_view_source(&mut self, redraw: &mut dyn FnMut(&App))` helper called only from `App::execute_action_with_redraw`. Set Pending → call redraw(self) → call `.status()` → set result message → drain events:

```rust
fn handle_view_source(&mut self, redraw: &mut dyn FnMut(&App)) {
    if let Some((_, _, path)) = self.selected_row_meta() {
        let binary = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
        let path_buf = std::path::PathBuf::from(&path);

        // POLISH-01: surface "Opening: ..." BEFORE blocking on .status()
        self.status_message = Some(StatusMessage::Pending(format!(
            "Opening: {}...",
            crate::paths::collapse_home(&path_buf)
        )));
        redraw(self);   // user sees "Opening: ..." before the block

        let raw = std::process::Command::new(binary).arg(&path).status();
        self.status_message = Some(status_message_from_open_result(binary, &path_buf, raw));

        // POLISH-01: drain queued events typed during the block
        self.drain_pending_events();
    }
}
```

Free function (new, sibling of `App` impl, `pub(super)` so unit tests in the same module see it):
```rust
pub(super) fn status_message_from_open_result(
    binary: &str,
    path: &std::path::Path,
    raw: std::io::Result<std::process::ExitStatus>,
) -> StatusMessage {
    match raw {
        Ok(status) if status.success() => StatusMessage::Success(format!(
            "Opened: {}",
            crate::paths::collapse_home(path)
        )),
        Ok(status) => {
            let exit = status.code().map(|c| c.to_string()).unwrap_or_else(|| "signal".into());
            StatusMessage::Warning(format!("{binary} exited {exit} for: {}", path.display()))
        }
        Err(e) => StatusMessage::Warning(format!("Could not launch {binary}: {e}")),
    }
}
```

Current `CopyPath` arm (`crates/tome/src/browse/app.rs` lines 294–337) returns either Ok or Err; on Err the match maps `ClipboardOccupied` → `"Clipboard busy ..."`. After this plan, the call to `arboard::Clipboard::new().and_then(...)` is replaced by `try_clipboard_set_text_with_retry(&path)`, which retries `ClipboardOccupied` once after a 100ms `std::thread::sleep`.

`render_status_bar` (`crates/tome/src/browse/ui.rs` line ~370) and the Detail-mode status branch (line ~315) currently access `msg.text` and `msg.severity` directly. After this plan, they call `msg.body()`, `msg.glyph()`, `msg.severity()` and compose the rendered span via `format!(" {} {} ", msg.glyph(), msg.body())`.

`Pending` rendering: use `theme.muted` foreground (no `theme.warning` is needed; `Pending` is informational, not alarming).

**Drain semantics:** `crossterm::event::poll(Duration::ZERO)` returns true if at least one event is available without blocking. Looping until it returns false drains the queue. Drop everything (don't dispatch) — keystrokes typed during the block were aimed at "the open dialog" not "the TUI."

**Redraw closure threading depth (the chosen design):**
```text
run_loop (browse/mod.rs)
   └── builds `let mut redraw = |a: &App| { let _ = terminal.draw(|f| ui::render(f, a)); };`
       └── calls `app.handle_key(key, &mut redraw)`
              └── `App::handle_key` matches mode, dispatches to:
                     ├── `handle_normal_key(key)` (unchanged — no redraw needed)
                     ├── `handle_search_key(key)` (unchanged)
                     ├── `handle_detail_key(key, redraw)` (gains redraw param)
                     │      └── on Enter+ViewSource: `self.execute_action_with_redraw(action, redraw)`
                     │      └── on other actions:    `self.execute_action(action)` (legacy path)
                     └── Help mode (unchanged)
```

Test sites: every `app.handle_key(key)` call in `#[cfg(test)] mod tests` becomes `app.handle_key(key, &mut |_| {})`. Mechanical update; ~25-30 call sites in `app.rs::tests`.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Redesign `StatusMessage` as an enum with accessors (POLISH-02)</name>
  <files>crates/tome/src/browse/app.rs, crates/tome/src/browse/ui.rs</files>
  <read_first>
    - crates/tome/src/browse/app.rs lines 17–54 (current StatusMessage struct + StatusSeverity enum)
    - crates/tome/src/browse/app.rs lines 252–337 (execute_action — call sites for StatusMessage::success/warning)
    - crates/tome/src/browse/app.rs lines 853–956 (existing test coverage that asserts on `.text`, `.severity`)
    - crates/tome/src/browse/ui.rs lines 313–353 (render_detail status branch — accesses msg.text + msg.severity)
    - crates/tome/src/browse/ui.rs lines 355–386 (render_status_bar — accesses msg.text + msg.severity)
    - crates/tome/src/browse/theme.rs (theme.accent / theme.alert / theme.muted — Pending uses muted)
  </read_first>
  <behavior>
    - Test 1 (`status_message_success_constructor`): `StatusMessage::Success("Copied: /tmp/x".into())` matches `Success(_)` arm; `.body()` returns `"Copied: /tmp/x"`; `.glyph()` returns `'✓'`; `.severity()` returns `StatusSeverity::Success`.
    - Test 2 (`status_message_warning_constructor`): `StatusMessage::Warning("Could not copy: permission denied".into())`; `.glyph()` returns `'⚠'`; `.severity()` returns `StatusSeverity::Warning`.
    - Test 3 (`status_message_pending_constructor`): `StatusMessage::Pending("Opening: ~/foo...".into())`; `.body()` returns `"Opening: ~/foo..."`; `.glyph()` returns `'⏳'`; `.severity()` returns `StatusSeverity::Pending`.
    - Test 4 (`status_message_body_does_not_contain_glyph`): For all three variants, `body()` returns the raw inner string with NO leading glyph or space. Assert via `assert!(!msg.body().starts_with('✓') && !msg.body().starts_with('⚠') && !msg.body().starts_with('⏳'))` and `assert!(!msg.body().starts_with(' '))`.
    - Test 5 (`status_message_lifecycle_unchanged`): The existing `status_message_set_by_copy_path_and_cleared_by_any_key` and `status_message_set_by_view_source_and_cleared_by_any_key` tests are MIGRATED (not duplicated): assertions like `msg.text.starts_with('✓')` become `msg.glyph() == '✓'`; assertions on `msg.severity` become `msg.severity()`. Behavior under test (set on action / cleared by any key) is unchanged.
  </behavior>
  <action>
**Step 1 — Replace the type declaration in `crates/tome/src/browse/app.rs`** (lines 17–54).

Replace the existing `StatusSeverity` and `StatusMessage` declarations with:

```rust
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
```

**Visibility:** the previous `pub` is narrowed to `pub(super)` so consumers outside the `browse` module cannot reach into the type. Confirm `App.status_message: Option<StatusMessage>` field stays pub-or-pub(super) per the existing visibility — but `StatusMessage` itself does NOT need to be visible outside the module. (`browse/mod.rs` is the only external entry point and it doesn't construct StatusMessages.)

**Step 2 — Update construction sites in `execute_action`** (lines 272, 282, 287, 311, 320–334):

Replace `StatusMessage::success(format!("Opened: {}", ...))` with
`StatusMessage::Success(format!("Opened: {}", ...))`. Same swap for `::warning(...)` → `::Warning(...)`. The body strings are identical to the current code (no leading glyph, no leading space). The `Self::success`/`::warning` constructors themselves are removed in this step — direct variant construction replaces them.

**Step 3 — Update `crates/tome/src/browse/ui.rs` consumers** (lines 313–353 for `render_detail`'s status branch; lines 370–386 for `render_status_bar`):

Wherever the code reads `msg.text` or `msg.severity`, swap to:
- `msg.text` → `format!("{} {}", msg.glyph(), msg.body())`
- `msg.severity` → `msg.severity()`

The match-on-severity block in both call sites GAINS a `StatusSeverity::Pending` arm. Color dispatch:

```rust
let msg_style = match msg.severity() {
    super::app::StatusSeverity::Warning => Style::default().fg(theme.alert).bg(theme.status_bar_bg),
    super::app::StatusSeverity::Success => Style::default().fg(theme.accent).bg(theme.status_bar_bg),
    super::app::StatusSeverity::Pending => Style::default().fg(theme.muted).bg(theme.status_bar_bg),
};
```

In both `render_detail` (line ~315) and `render_status_bar` (line ~370), the `Span::styled(format!(" {} ", msg.text), ...)` becomes `Span::styled(format!(" {} {} ", msg.glyph(), msg.body()), ...)`.

**Step 4 — Migrate the 3 existing StatusMessage tests in `app.rs`** (lines 853–956):
- `status_message_set_by_copy_path_and_cleared_by_any_key`: change `msg.severity` → `msg.severity()`, change `msg.text.starts_with('✓')` → `msg.glyph() == '✓'`. Add the no-leading-glyph-in-body assertion: `assert!(!msg.body().starts_with('✓') && !msg.body().starts_with('⚠'))`.
- `status_message_success_and_warning_constructors_apply_glyph_prefix`: rename to `status_message_glyph_dispatch_for_each_variant`. Replace constructors with direct variants:
  ```rust
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
  ```
- `status_message_set_by_view_source_and_cleared_by_any_key`: same `severity` → `severity()` and `text.starts_with` → `glyph() ==` migration. The `match msg.severity { Success => ..., Warning => ... }` block GAINS a `Pending` arm (assert `msg.glyph() == '⏳'`); behavior is now: ViewSource may emit Success, Warning, OR Pending (Pending here would be transient — the test calls `execute_action(ViewSource)` synchronously; the Pending message is set BEFORE the `.status()` call but overwritten by the result before `execute_action` returns, so in practice the test sees Success or Warning. The Pending arm is included for exhaustiveness — if a future refactor leaves the message as Pending, the test will assert the correct glyph instead of panicking on a non-exhaustive match).

**Step 5 — Add 4 new tests** for the pure type behavior (Tests 1–4 above). Place after `status_message_set_by_view_source_and_cleared_by_any_key` (~line 956).

**Step 6 — Audit derives** (already documented in the comment above the `StatusMessage` derive line in Step 1). Verify the `Copy` derive on `StatusSeverity` is still warranted — it is (used in `match` dispatch and across the `app.rs` ↔ `ui.rs` boundary as a value). No `Hash` / `Ord` / `Default` derives are added.

Run: `cargo test -p tome browse::app::tests::status_message`
  </action>
  <verify>
    <automated>cd /Users/martin/dev/opensource/tome && cargo test -p tome browse::app::tests::status_message 2>&1 | tail -20 && cargo clippy -p tome --all-targets -- -D warnings 2>&1 | tail -5</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "pub\(super\) enum StatusMessage" crates/tome/src/browse/app.rs` returns exactly 1 match.
    - `rg -n "Success\(String\)|Warning\(String\)|Pending\(String\)" crates/tome/src/browse/app.rs` returns at least 3 matches (the variant declarations).
    - `rg -n "fn glyph\(&self\)|fn body\(&self\)|fn severity\(&self\)" crates/tome/src/browse/app.rs` returns exactly 3 matches.
    - `rg -n "msg\.text" crates/tome/src/browse/ui.rs` returns 0 matches (the `.text` field is gone — every consumer composes via `glyph()` + `body()`).
    - `rg -n "msg\.text" crates/tome/src/browse/app.rs` returns 0 matches.
    - `rg -n "msg\.severity\b" crates/tome/src/browse/ui.rs` returns 0 matches (must be the method call `msg.severity()` with parens).
    - `rg -n "msg\.severity\(\)" crates/tome/src/browse/ui.rs` returns at least 2 matches (one per call site).
    - `rg -n "StatusSeverity::Pending" crates/tome/src/browse/ui.rs` returns at least 2 matches (Pending arm in both render_detail status branch and render_status_bar).
    - `rg -n "StatusMessage::success\(|StatusMessage::warning\(" crates/tome/src/browse/app.rs` returns 0 matches (legacy constructors removed).
    - `rg -n "pub StatusMessage|pub fn success|pub fn warning" crates/tome/src/browse/app.rs` returns 0 matches (visibility narrowed).
    - `cargo test -p tome browse::app::tests::status_message` runs ≥ 7 tests, all pass.
    - `cargo clippy -p tome --all-targets -- -D warnings` is clean.
  </acceptance_criteria>
  <done>
    `StatusMessage` is a `pub(super) enum { Success(String) | Warning(String) | Pending(String) }` with `body()`/`glyph()`/`severity()` accessors. ui.rs composes the rendered string via `format!("{} {}", msg.glyph(), msg.body())` and dispatches color via `msg.severity()` with a Pending arm using `theme.muted`. Tests migrated, derives audited.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Factor `status_message_from_open_result` + add Pending status + drain keystrokes (POLISH-01, TEST-03)</name>
  <files>crates/tome/src/browse/app.rs, crates/tome/src/browse/mod.rs</files>
  <read_first>
    - crates/tome/src/browse/app.rs lines 161–212 (`handle_key` and `handle_normal_key` — `handle_key` signature change required)
    - crates/tome/src/browse/app.rs lines 213–250 (`handle_detail_key` — needs the redraw param)
    - crates/tome/src/browse/app.rs lines 252–293 (current ViewSource arm — being refactored into `handle_view_source`)
    - crates/tome/src/browse/app.rs lines 478–957 (entire `#[cfg(test)] mod tests` block — every `app.handle_key(key, ...)` call site must be updated mechanically)
    - crates/tome/src/browse/mod.rs lines 1–60 (`run_loop` — needs to construct and pass the redraw closure)
    - crates/tome/src/paths.rs (`collapse_home` — already used elsewhere)
  </read_first>
  <behavior>
    - Test 1 (`status_message_from_open_result_ok_success`): synthetic `ExitStatus::from_raw(0)` (status 0 = success on Unix) → returns `StatusMessage::Success(s)` where `s.contains("Opened:")` and the body includes `crate::paths::collapse_home(path)` rendering of the path. Use `std::os::unix::process::ExitStatusExt::from_raw` (gated on `#[cfg(unix)]`).
    - Test 2 (`status_message_from_open_result_ok_nonzero_exit`): synthetic `ExitStatus::from_raw(0x100)` (exit code 1 in the high byte on Linux/macOS) → returns `StatusMessage::Warning(s)` where `s.starts_with("xdg-open exited 1 for: ")`. Test with `binary = "xdg-open"` and a path; assert the body contains the path's `Display` form.
    - Test 3 (`status_message_from_open_result_err`): synthetic `std::io::Error::new(std::io::ErrorKind::NotFound, "not found")` → returns `StatusMessage::Warning(s)` where `s == "Could not launch xdg-open: not found"`.
    - Test 4 (`view_source_invokes_redraw_callback_for_pending_status`): drives `App::execute_action_with_redraw(DetailAction::ViewSource, &mut redraw_cb)` with a counting closure; asserts `redraw_calls >= 1`. Proves the redraw is invoked at least once per ViewSource (which is the Pending-message render trip).
    - Test 5 (`drain_pending_events_returns_when_queue_empty`): calls `app.drain_pending_events()` and asserts it returns within ~100ms via `Instant::now()` before/after. Proves the drain loop terminates on an empty queue (no hangs).
  </behavior>
  <action>
**Step 1 — Add `status_message_from_open_result` free function** (after `impl StatusMessage` block in `app.rs`, ~line 95):

```rust
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
        Ok(status) if status.success() => StatusMessage::Success(format!(
            "Opened: {}",
            crate::paths::collapse_home(path)
        )),
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
```

**Step 2 — Add `drain_pending_events` method on `App`** (private helper, place near `selected_row_meta` ~line 471):

```rust
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
```

`&self` (not `&mut`) — we don't store events, just discard them.

**Step 3 — Refactor the ViewSource arm into a `handle_view_source` helper that accepts a redraw closure.**

Remove the entire current `DetailAction::ViewSource => { ... }` arm from `execute_action` (lines 252–293). Add a new private method on `App`:

```rust
fn handle_view_source(&mut self, redraw: &mut dyn FnMut(&App)) {
    if let Some((_, _, path)) = self.selected_row_meta() {
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
```

The `execute_action` body retains the `CopyPath`, `Disable`, `Enable`, `Back` arms unchanged. The `ViewSource` arm of `execute_action` itself becomes:

```rust
DetailAction::ViewSource => {
    // Production callers should use `execute_action_with_redraw` directly so the
    // Pending status renders before `.status()` blocks. This arm is kept so
    // tests that construct an App and call `execute_action(ViewSource)` directly
    // continue to work — they pass through to handle_view_source with a no-op
    // redraw, which is identical to legacy behavior.
    self.handle_view_source(&mut |_| {});
}
```

**Step 4 — Add a redraw-aware variant of `execute_action`** in `crates/tome/src/browse/app.rs`. Place immediately after the existing `execute_action` method:

```rust
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
```

**Step 5 — Update `App::handle_key` to accept and thread the redraw closure** (`crates/tome/src/browse/app.rs` line 161). Replace:

```rust
pub fn handle_key(&mut self, key: KeyEvent) {
```

With:

```rust
pub(super) fn handle_key(&mut self, key: KeyEvent, redraw: &mut dyn FnMut(&App)) {
```

Inside `handle_key`, the `Mode::Detail => self.handle_detail_key(key)` branch becomes `Mode::Detail => self.handle_detail_key(key, redraw)`. The `Mode::Normal`, `Mode::Search`, and `Mode::Help` branches do NOT need the closure (no blocking calls inside them).

**Step 6 — Update `App::handle_detail_key`** (line 214). Replace:

```rust
fn handle_detail_key(&mut self, key: KeyEvent) {
```

With:

```rust
fn handle_detail_key(&mut self, key: KeyEvent, redraw: &mut dyn FnMut(&App)) {
```

Inside `handle_detail_key`, the Enter-action dispatch becomes:

```rust
KeyCode::Enter => {
    if let Some(&action) = self.detail_actions.get(self.detail_selected) {
        // ViewSource needs the redraw closure for POLISH-01; other actions
        // do not block, so the legacy execute_action path is sufficient.
        if matches!(action, DetailAction::ViewSource) {
            self.execute_action_with_redraw(action, redraw);
        } else {
            self.execute_action(action);
        }
    }
}
```

**Step 7 — Update `crates/tome/src/browse/mod.rs::run_loop`** to construct a real redraw closure and pass it. Replace the loop body:

```rust
fn run_loop(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            // POLISH-01: redraw closure threaded into handle_key so the
            // ViewSource arm can surface a `Pending("Opening: ...")` message
            // BEFORE `.status()` blocks. The closure ignores draw errors —
            // a draw failure must not abort the open action; the next
            // `terminal.draw(...)` at the top of this loop will recover.
            let mut redraw = |a: &App| {
                let _ = terminal.draw(|frame| ui::render(frame, a));
            };
            app.handle_key(key, &mut redraw);
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
```

**Step 8 — Update the ~25-30 test sites** that call `app.handle_key(key)` in `crates/tome/src/browse/app.rs::tests`. Find them with:

```bash
rg -c "app\.handle_key\(" crates/tome/src/browse/app.rs
```

Each becomes `app.handle_key(key, &mut |_| {})`. The closure is a no-op `|_: &App| {}` — tests don't care about the redraw side-effect (Pending messages are overwritten by the result before any test inspects `app.status_message`).

**Note on borrow-checker quirk:** The closure-passing pattern `&mut |_| {}` may need a stack binding to satisfy lifetime rules in some test sites. If `app.handle_key(key, &mut |_| {})` triggers `error[E0716]: temporary value dropped while borrowed`, refactor to:

```rust
let mut nr = |_: &App| {};
app.handle_key(key, &mut nr);
```

Run `cargo build -p tome --tests` to chase any sites that need the binding pattern.

**Step 9 — Add the 5 new tests** to `app.rs::mod tests`:
- `status_message_from_open_result_ok_success`
- `status_message_from_open_result_ok_nonzero_exit`
- `status_message_from_open_result_err`
- `view_source_invokes_redraw_callback_for_pending_status`
- `drain_pending_events_returns_when_queue_empty`

For the synthetic-`ExitStatus` tests, use `std::os::unix::process::ExitStatusExt::from_raw`. Gate with `#[cfg(unix)]`:

```rust
#[cfg(unix)]
#[test]
fn status_message_from_open_result_ok_success() {
    use std::os::unix::process::ExitStatusExt;
    let status = std::process::ExitStatus::from_raw(0);
    let path = std::path::PathBuf::from("/tmp/foo");
    let msg = status_message_from_open_result("xdg-open", &path, Ok(status));
    assert!(matches!(msg, StatusMessage::Success(_)));
    assert!(msg.body().contains("Opened:"));
}

#[cfg(unix)]
#[test]
fn status_message_from_open_result_ok_nonzero_exit() {
    use std::os::unix::process::ExitStatusExt;
    let status = std::process::ExitStatus::from_raw(0x100);
    let path = std::path::PathBuf::from("/tmp/foo");
    let msg = status_message_from_open_result("xdg-open", &path, Ok(status));
    assert!(matches!(msg, StatusMessage::Warning(_)));
    assert!(msg.body().starts_with("xdg-open exited 1 for: "));
}

#[test]
fn status_message_from_open_result_err() {
    let err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
    let path = std::path::PathBuf::from("/tmp/foo");
    let msg = status_message_from_open_result("xdg-open", &path, Err(err));
    assert!(matches!(msg, StatusMessage::Warning(_)));
    assert_eq!(msg.body(), "Could not launch xdg-open: not found");
}

#[test]
fn view_source_invokes_redraw_callback_for_pending_status() {
    let (mut app, _tmp) = make_app(3);
    let mut redraw_calls: u32 = 0;
    let mut redraw_cb = |_app: &App| { redraw_calls += 1; };
    app.execute_action_with_redraw(DetailAction::ViewSource, &mut redraw_cb);
    assert!(
        redraw_calls >= 1,
        "redraw must be called at least once for the Pending status (POLISH-01)"
    );
}

#[test]
fn drain_pending_events_returns_when_queue_empty() {
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
```

Run: `cargo build -p tome --tests && cargo test -p tome --lib browse::`
  </action>
  <verify>
    <automated>cd /Users/martin/dev/opensource/tome && cargo build -p tome --tests 2>&1 | tail -10 && cargo test -p tome --lib browse:: 2>&1 | tail -20 && cargo clippy -p tome --all-targets -- -D warnings 2>&1 | tail -5</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "pub\(super\) fn status_message_from_open_result" crates/tome/src/browse/app.rs` returns exactly 1 match.
    - `rg -n "fn drain_pending_events" crates/tome/src/browse/app.rs` returns exactly 1 match.
    - `rg -n "fn execute_action_with_redraw" crates/tome/src/browse/app.rs` returns exactly 1 match.
    - `rg -n "fn handle_view_source" crates/tome/src/browse/app.rs` returns exactly 1 match.
    - `rg -n "redraw: &mut dyn FnMut" crates/tome/src/browse/` returns at least 2 matches (handle_key + execute_action_with_redraw signatures; plus likely handle_detail_key + handle_view_source for 4 total).
    - `rg -n "Opening: " crates/tome/src/browse/app.rs` returns at least 1 match (the Pending body format string).
    - `rg -n "StatusMessage::Pending" crates/tome/src/browse/app.rs` returns at least 1 match (the `handle_view_source` body).
    - `rg -n "crossterm::event::poll\(std::time::Duration::ZERO\)|crossterm::event::poll\(Duration::ZERO\)" crates/tome/src/browse/app.rs` returns at least 1 match (drain loop).
    - `rg -n "let mut redraw = \|a: &App\|" crates/tome/src/browse/mod.rs` returns at least 1 match (the production redraw closure).
    - `rg -n "app\.handle_key\(key, &mut" crates/tome/src/browse/app.rs` returns at least 25 matches (test sites updated).
    - `cargo test -p tome browse::app::tests::status_message_from_open_result_ok_success` passes.
    - `cargo test -p tome browse::app::tests::status_message_from_open_result_ok_nonzero_exit` passes.
    - `cargo test -p tome browse::app::tests::status_message_from_open_result_err` passes.
    - `cargo test -p tome browse::app::tests::view_source_invokes_redraw_callback_for_pending_status` passes.
    - `cargo test -p tome browse::app::tests::drain_pending_events_returns_when_queue_empty` passes.
    - `cargo test -p tome --lib browse::` passes (all pre-existing browse module tests still green after the test-site updates).
    - `cargo build -p tome --tests` is clean.
    - `cargo clippy -p tome --all-targets -- -D warnings` is clean.
  </acceptance_criteria>
  <done>
    `status_message_from_open_result` is a `pub(super)` free function with 3 unit tests covering Ok-success, Ok-nonzero-exit, and Err arms. `App::execute_action_with_redraw` and `App::handle_view_source` exist; the redraw closure threads through `handle_key` → `handle_detail_key` → `execute_action_with_redraw`. `run_loop` constructs the production redraw closure with `terminal.draw(...)`. After `.status()` returns, queued crossterm events are drained via `drain_pending_events()`. Test sites updated mechanically.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Auto-retry `ClipboardOccupied` once with 100ms backoff (POLISH-03)</name>
  <files>crates/tome/src/browse/app.rs</files>
  <read_first>
    - crates/tome/src/browse/app.rs lines 294–337 (current CopyPath arm — Err mapping including ClipboardOccupied)
    - crates/tome/src/browse/app.rs lines 853–912 (existing CopyPath lifecycle test — must continue to pass)
  </read_first>
  <behavior>
    - Test 1 (`copy_path_retries_clipboard_occupied_once`): factor the retry logic into a free helper `try_clipboard_set_text_with_retry(text: &str) -> Result<(), arboard::Error>` that calls `arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text))` once; on `ClipboardOccupied`, sleeps 100ms and retries once. Test the helper directly with a fake by injecting a counter or by relying on the host's actual clipboard. Since arboard uses real OS APIs (un-mockable without trait abstraction, which #463 explicitly rejects), test the SHAPE: a unit test that calls the helper and asserts it returns `Ok(())` or `Err(_)` within ~250ms (one fast attempt + one 100ms-delayed attempt). Use `std::time::Instant` to assert the wall-clock bound.
    - Test 2 (`copy_path_retry_helper_under_250ms_on_success`): On a host where the clipboard is available (most dev machines + CI runners with a display server), the first attempt succeeds and the helper returns in <50ms. Skip the test under `#[cfg(target_os = "linux")]` if `DISPLAY`/`WAYLAND_DISPLAY` is unset (headless CI) — the helper will return `Err(ClipboardNotSupported)` immediately, which is also valid (no retry, no sleep). The bound assertion: `elapsed < Duration::from_millis(250)` covers both fast-success AND fast-fail paths; only ClipboardOccupied followed by ClipboardOccupied would push past 100ms — and CI doesn't reproduce that.
    - Test 3 (`status_message_set_by_copy_path_and_cleared_by_any_key`) — UPDATED: existing test from Task 1's migration. Continues to pass; a host that produces `ClipboardOccupied` once now eventually emits Success or Warning depending on the second attempt.

    **Note on testability:** the retry contract is not directly testable on real-OS arboard without injecting a fake. Per #463 D-17/D-19, we deliberately do NOT introduce a `trait ClipboardBackend`. The test surface is therefore: (a) the wall-clock bound, (b) the lifecycle invariant (existing test), (c) a `rg` assertion in acceptance_criteria that the retry source code is present. Manual UAT covers the actual two-attempt behavior.
  </behavior>
  <action>
**Step 1 — Add the retry helper** (place near `status_message_from_open_result`, ~line 110):

```rust
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
```

**Step 2 — Replace the inlined `arboard::Clipboard::new().and_then(...)` in the CopyPath arm** (around line 308) with a call to the new helper:

```rust
DetailAction::CopyPath => {
    if let Some((_, _, path)) = self.selected_row_meta() {
        let result = try_clipboard_set_text_with_retry(&path);
        match result {
            Ok(()) => {
                self.status_message = Some(StatusMessage::Success(format!(
                    "Copied: {}",
                    crate::paths::collapse_home(Path::new(&path))
                )));
            }
            Err(e) => {
                let msg = match &e {
                    arboard::Error::ClipboardNotSupported => {
                        "Clipboard unavailable (headless or unsupported session)".to_string()
                    }
                    arboard::Error::ClipboardOccupied => {
                        "Clipboard busy (another app is holding it); try again".to_string()
                    }
                    other => format!("Could not copy: {other}"),
                };
                self.status_message = Some(StatusMessage::Warning(msg));
            }
        }
    }
}
```

The `ClipboardOccupied` arm in the match is RETAINED — it now fires only after the retry has also failed. The body string is unchanged so existing snapshot/UAT expectations don't drift.

**Step 3 — Add the 2 new tests** to `app.rs::mod tests`:

```rust
#[test]
fn copy_path_retry_helper_returns_within_bound() {
    // The retry contract: at most one fast attempt + one 100ms-delayed
    // attempt. On a host where the clipboard succeeds or fails-fast
    // (ClipboardNotSupported), the helper returns in <50ms. On a host
    // where the clipboard is occupied repeatedly, it returns in ~100ms.
    // The 250ms upper bound covers both — anything longer indicates a
    // regression (e.g., a second 100ms sleep crept in).
    let start = std::time::Instant::now();
    let _ = super::try_clipboard_set_text_with_retry("test-payload");
    let elapsed = start.elapsed();
    assert!(
        elapsed < std::time::Duration::from_millis(250),
        "retry helper must complete within 250ms (one fast + one 100ms backoff); took {:?}",
        elapsed
    );
}

#[test]
fn copy_path_retry_helper_signature_compiles() {
    // Smoke test: ensures the helper exists with the documented signature.
    // If a future refactor changes the type, this fails to compile and
    // the issue is surfaced before the source-grep checks run.
    let _: fn(&str) -> Result<(), arboard::Error> = super::try_clipboard_set_text_with_retry;
}
```

The first test depends on the host's clipboard state. On macOS dev machines and most Linux CI runners with `DISPLAY` set, the first attempt succeeds in <10ms. On headless CI, `Clipboard::new()` returns `ClipboardNotSupported` (no retry, no sleep). The 250ms upper bound is safe.

**Step 4 — Run the existing `status_message_set_by_copy_path_and_cleared_by_any_key` test** (migrated in Task 1) to confirm the retry path doesn't break it. The test already accepts either Success or Warning severity; the retry adds at most 100ms latency on hosts that produce ClipboardOccupied, which doesn't affect the assertion shape.

Run: `cargo test -p tome browse::app::tests::copy_path_retry browse::app::tests::status_message_set_by_copy_path`
  </action>
  <verify>
    <automated>cd /Users/martin/dev/opensource/tome && cargo test -p tome browse::app::tests::copy_path 2>&1 | tail -15 && cargo clippy -p tome --all-targets -- -D warnings 2>&1 | tail -5</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "fn try_clipboard_set_text_with_retry" crates/tome/src/browse/app.rs` returns exactly 1 match.
    - `rg -n "ClipboardOccupied.*sleep|sleep.*ClipboardOccupied" crates/tome/src/browse/app.rs` returns at least 1 match (the retry pattern: match arm + sleep call adjacent).
    - `rg -n "Duration::from_millis\(100\)" crates/tome/src/browse/app.rs` returns at least 1 match (the 100ms backoff).
    - `rg -n "arboard::Clipboard::new\(\).and_then" crates/tome/src/browse/app.rs` returns 0 matches in the `execute_action` body (the inlined call is replaced by the helper). One match is allowed inside the helper itself.
    - `cargo test -p tome browse::app::tests::copy_path_retry_helper_returns_within_bound` passes.
    - `cargo test -p tome browse::app::tests::copy_path_retry_helper_signature_compiles` passes.
    - `cargo test -p tome browse::app::tests::status_message_set_by_copy_path_and_cleared_by_any_key` passes (regression).
    - `cargo clippy -p tome --all-targets -- -D warnings` is clean.
  </acceptance_criteria>
  <done>
    `try_clipboard_set_text_with_retry` exists as a free helper and is called from the CopyPath arm. ClipboardOccupied triggers exactly one 100ms-delayed retry; all other errors fall through immediately. Existing CopyPath lifecycle test still passes.
  </done>
</task>

</tasks>

<verification>
- `cargo test -p tome browse::app::tests::status_message` — ≥ 7 tests pass (variant constructors, body/glyph/severity, lifecycle migrated).
- `cargo test -p tome browse::app::tests::status_message_from_open_result` — 3 tests pass (Ok-success, Ok-nonzero-exit, Err).
- `cargo test -p tome browse::app::tests::view_source_invokes_redraw_callback_for_pending_status` — passes.
- `cargo test -p tome browse::app::tests::drain_pending_events_returns_when_queue_empty` — passes.
- `cargo test -p tome browse::app::tests::copy_path_retry_helper_returns_within_bound` — passes.
- `cargo test -p tome browse::app::tests::copy_path_retry_helper_signature_compiles` — passes.
- `cargo test -p tome browse::app::tests::status_message_set_by_copy_path_and_cleared_by_any_key` — passes (regression).
- `cargo test -p tome browse::app::tests::status_message_set_by_view_source_and_cleared_by_any_key` — passes (regression, migrated).
- `cargo test -p tome --lib browse::` — passes (all pre-existing browse module tests still green after redraw-closure threading).
- `make ci` — clean.
- `rg -n "msg\.text" crates/tome/src/browse/` — 0 matches.
- `rg -n "StatusMessage::success\(|StatusMessage::warning\(" crates/tome/src/browse/app.rs` — 0 matches (legacy constructors removed).
- `rg -n "redraw: &mut dyn FnMut" crates/tome/src/browse/` — at least 2 matches.
</verification>

<success_criteria>
- `StatusMessage` is a `pub(super) enum` with `Success(String)` / `Warning(String)` / `Pending(String)` variants and `body()`/`glyph()`/`severity()` accessors (POLISH-02).
- `ui.rs` composes the rendered string at render time via `format!("{} {}", msg.glyph(), msg.body())` and dispatches color via `msg.severity()` with a Pending arm using `theme.muted` (POLISH-02).
- Redraw closure (`&mut dyn FnMut(&App)`) threads through `App::handle_key` → `handle_detail_key` → `execute_action_with_redraw` → `handle_view_source`. Production `run_loop` passes a closure that calls `terminal.draw(...)`; tests pass `&mut |_| {}`. `App` itself stays independent of `ratatui::DefaultTerminal` (POLISH-01).
- `ViewSource` action sets `StatusMessage::Pending("Opening: <collapsed-path>...")` and invokes the redraw closure BEFORE `.status()` blocks; after the block returns, queued crossterm events are drained (POLISH-01).
- `status_message_from_open_result(binary, path, raw)` is a unit-tested `pub(super)` free function covering Ok+success / Ok+non-zero / Err (TEST-03).
- `try_clipboard_set_text_with_retry` retries `ClipboardOccupied` once after a 100ms backoff before any Warning reaches `app.status_message` (POLISH-03).
</success_criteria>

<output>
After completion, create `.planning/phases/10-phase-8-review-tail/10-01-SUMMARY.md` recording:
- New `StatusMessage` enum + accessor signatures.
- New `status_message_from_open_result` and `try_clipboard_set_text_with_retry` signatures.
- `App::execute_action_with_redraw` and `App::handle_view_source` signatures.
- `App::handle_key(&mut self, key: KeyEvent, redraw: &mut dyn FnMut(&App))` new signature.
- The exact line edited in `crates/tome/src/browse/mod.rs::run_loop`.
- Test names added (≥ 7 in the StatusMessage cluster, 3 in the open_result cluster, 2 in the retry cluster, 1 for view_source_invokes_redraw, 1 for drain_pending_events).
- Tests migrated (lifecycle / glyph dispatch).
- Test sites updated mechanically: the count of `app.handle_key(...)` call sites converted from 1-arg to 2-arg.
- One-line confirmation: POLISH-01 + POLISH-02 + POLISH-03 + TEST-03 closed.
</output>
