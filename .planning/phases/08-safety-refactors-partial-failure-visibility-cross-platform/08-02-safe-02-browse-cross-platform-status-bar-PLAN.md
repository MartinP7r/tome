---
phase: 08-safety-refactors-partial-failure-visibility-cross-platform
plan: 02
type: execute
wave: 1
depends_on: []
files_modified:
  - Cargo.toml
  - crates/tome/Cargo.toml
  - crates/tome/src/browse/app.rs
  - crates/tome/src/browse/ui.rs
  - CHANGELOG.md
autonomous: true
requirements:
  - SAFE-02
must_haves:
  truths:
    - "User on Linux pressing the `open` action in `tome browse` has the skill opened via `xdg-open` (and `copy path` via a cross-platform clipboard crate); any failure appears in the TUI status bar instead of being silently discarded by `let _ = ...`"
    - "Success messages (✓) and failure messages (⚠) for ViewSource and CopyPath are styled and rendered in place of the keybind line until any key is pressed"
  artifacts:
    - path: "Cargo.toml"
      provides: "arboard workspace dependency with default-features = false"
      contains: "arboard = { version"
    - path: "crates/tome/Cargo.toml"
      provides: "arboard binary-crate dep entry"
      contains: "arboard = { workspace = true }"
    - path: "crates/tome/src/browse/app.rs"
      provides: "App.status_message field, status_message clear at top of handle_key, execute_action rewrite using cfg!-dispatched open + arboard clipboard"
      contains: "status_message: Option<String>"
    - path: "crates/tome/src/browse/ui.rs"
      provides: "Conditional status-bar render — substitute single-line styled status_message for keybind line in both Normal and Detail modes"
      contains: "app.status_message"
  key_links:
    - from: "crates/tome/src/browse/app.rs execute_action"
      to: "self.status_message = Some(format!(...))"
      via: "match arms on Ok / Err for both ViewSource (open|xdg-open) and CopyPath (arboard)"
      pattern: "self\\.status_message\\s*=\\s*Some"
    - from: "crates/tome/src/browse/app.rs handle_key"
      to: "self.status_message = None"
      via: "first statement before mode dispatch"
      pattern: "self\\.status_message\\s*=\\s*None"
    - from: "crates/tome/src/browse/ui.rs render_status_bar (Normal) + inline Line::from(...) at 310-329 (Detail)"
      to: "Line::from(vec![Span::styled(msg, ...)])"
      via: "if let Some(msg) = &app.status_message { ... }"
      pattern: "if let Some\\(msg\\)\\s*=\\s*&app\\.status_message"
---

<objective>
Make `tome browse`'s `DetailAction::ViewSource` (open file) and `DetailAction::CopyPath` (clipboard) work cross-platform on Linux + macOS, replace the `sh -c | pbcopy` command-injection vector with the `arboard` crate, and surface success/failure as a styled status-bar message instead of `let _ = ...` silent drops. Covers SAFE-02 (#414).

Purpose: Today `execute_action` calls `Command::new("open")` (macOS-only) and `sh -c "echo -n '${path}' | pbcopy"` (macOS-only AND command-injection unsafe). Linux users get nothing; macOS users get no feedback. This plan adds `arboard` as a workspace dep with `default-features = false` (text-only — avoids ~10 transitive image-processing deps), uses `cfg!(target_os = "macos")` to dispatch `open` vs `xdg-open`, and routes both success and failure into a new `App.status_message: Option<String>` rendered in place of the keybind line until the next keypress.

Output: `arboard` workspace + per-crate dep entries; `App.status_message` field; status-bar clear in `handle_key`; rewritten `execute_action`; conditional render in `ui.rs` at both Normal-mode and Detail-mode status-bar sites; unit test for the status_message lifecycle; CHANGELOG bullet under v0.8 unreleased.
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
@.planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-CONTEXT.md
@.planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-RESEARCH.md
@Cargo.toml
@crates/tome/Cargo.toml
@crates/tome/src/browse/app.rs
@crates/tome/src/browse/ui.rs
@crates/tome/src/browse/theme.rs

**CRITICAL drift corrections from RESEARCH.md (override CONTEXT.md where they conflict):**
1. CONTEXT.md says the bottom-bar render lives at `ui.rs:190-200` — that is WRONG. Lines 180-192 are the `highlight_name` fuzzy-match helper. The REAL status-bar render sites are `render_status_bar` at `ui.rs:332+` (Normal mode) and the inline `Line::from(...)` block at `ui.rs:310-329` (Detail mode). All `<action>` blocks in this plan use the CORRECTED locations.
2. CONTEXT.md leaves "add `theme.warning`" as Claude's discretion — RESEARCH.md confirms `theme.alert` (yellow) already exists at `theme.rs:15`. **Reuse `theme.alert` for `⚠` messages; do NOT add a new `theme.warning` field.** (Pitfall 5.)
3. `arboard` MUST be added with `default-features = false` to avoid pulling the `image` crate + ~10 transitive deps for a text-only use case (Pitfall 1).
4. Per D-17/D-19, do NOT introduce `trait Opener` / `trait ClipboardBackend` / `#[cfg(target_os = "linux")]` direct tests. CI matrix exercises the platform branch at compile time. `arboard` exposes no test hook → skip the forced-failure unit test per D-19.
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add arboard workspace dep + per-crate entry (D-07)</name>
  <files>Cargo.toml, crates/tome/Cargo.toml</files>
  <read_first>
    - .planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-CONTEXT.md (D-07)
    - .planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-RESEARCH.md (Standard Stack > Installation block; Pitfall 1)
    - Cargo.toml (verify alphabetical slot — `arboard` falls after `anyhow`, before `assert_cmd` in `[workspace.dependencies]`)
    - crates/tome/Cargo.toml (verify alphabetical slot — after `anyhow.workspace = true`)
  </read_first>
  <action>
    Step 1 — Root `Cargo.toml`: under `[workspace.dependencies]`, insert (in the alphabetical slot between `anyhow` and `assert_cmd`):

    ```toml
    arboard = { version = "3", default-features = false }
    ```

    `default-features = false` is REQUIRED — it strips the `image-data` feature which would otherwise pull `image 0.25` + ~10 transitive image-processing deps that tome does not use (Pitfall 1 from RESEARCH.md).

    Step 2 — `crates/tome/Cargo.toml`: under `[dependencies]`, insert (alphabetical slot, after `anyhow = { workspace = true }` or wherever the existing 'a' deps sit):

    ```toml
    arboard = { workspace = true }
    ```

    Verify the result: `cargo build -p tome` must complete without pulling `image` (sanity check via `cargo tree -p tome -i image` returning nothing OR exit 1).

    Do NOT pin to `3.6.1` tightly — repo convention is loose major-pin (`anyhow = "1"`, `clap = "4"`, `serde = "1"`).
  </action>
  <verify>
    <automated>cargo build -p tome 2>&1 | tail -10 && cargo tree -p tome -i image 2>&1 | head -5</automated>
  </verify>
  <acceptance_criteria>
    - `grep -q 'arboard = { version = "3", default-features = false }' Cargo.toml`
    - `grep -q 'arboard = { workspace = true }' crates/tome/Cargo.toml`
    - `cargo build -p tome 2>&1 | grep -q 'error' && exit 1 || exit 0`
    - `cargo tree -p tome -i image 2>&1 | grep -qE 'package ID|ERROR' || true` (image crate not pulled — `cargo tree -i image` exits non-zero or prints "package ID specification not found")
    - `! grep -q 'arboard = "3"' Cargo.toml` (no naked version — must include default-features = false)
  </acceptance_criteria>
  <done>arboard is in workspace deps with default-features = false; binary crate references it via workspace; `image` crate is NOT pulled into the dep tree.</done>
</task>

<task type="auto">
  <name>Task 2: Add App.status_message field + initialize None (D-09)</name>
  <files>crates/tome/src/browse/app.rs</files>
  <read_first>
    - .planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-CONTEXT.md (D-09)
    - crates/tome/src/browse/app.rs (App struct ~lines 71-89; App::new ~lines 91-117)
  </read_first>
  <action>
    In `crates/tome/src/browse/app.rs`:

    1. App struct (~lines 71-89): append a new field after the existing fields:
    ```rust
    pub status_message: Option<String>,
    ```
    Visibility `pub` so `ui.rs` can read it; field ordering: append at the end of the struct.

    2. `App::new` (~lines 91-117): in the struct-literal returned at the end, add (alphabetical or appended — match existing field-init style):
    ```rust
    status_message: None,
    ```

    Do not add any imports yet (no `arboard` import here — Task 4 handles that). Do not add any setter — direct field assignment from `execute_action` is the API.
  </action>
  <verify>
    <automated>cargo build -p tome 2>&1 | tail -5</automated>
  </verify>
  <acceptance_criteria>
    - `grep -q 'pub status_message: Option<String>' crates/tome/src/browse/app.rs`
    - `grep -q 'status_message: None' crates/tome/src/browse/app.rs`
    - `cargo build -p tome 2>&1 | grep -q 'error\[' && exit 1 || exit 0`
  </acceptance_criteria>
  <done>App struct gains `status_message: Option<String>` field; App::new initializes it to None; crate compiles.</done>
</task>

<task type="auto">
  <name>Task 3: Clear status_message at top of handle_key (D-10)</name>
  <files>crates/tome/src/browse/app.rs</files>
  <read_first>
    - .planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-CONTEXT.md (D-10)
    - .planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-RESEARCH.md (Pitfall 4 — clear unconditionally at top, mirrors the `?` help-overlay any-key-dismisses pattern at app.rs:125-128)
    - crates/tome/src/browse/app.rs (handle_key ~lines 119-163)
  </read_first>
  <action>
    In `crates/tome/src/browse/app.rs::handle_key` (~lines 119-163), as the FIRST statement of the function body (before the existing `match self.mode { ... }` dispatch):

    ```rust
    self.status_message = None;
    ```

    This unconditional clear gives "any-key-dismisses" semantics — the message is visible until the user presses any key. Mirrors the `?` help-overlay pattern.

    Do NOT clear inside individual mode branches — that creates mode-dependent message lifetime. Single point of clearing keeps the lifecycle contract trivial.
  </action>
  <verify>
    <automated>cargo build -p tome 2>&1 | tail -5</automated>
  </verify>
  <acceptance_criteria>
    - `grep -A2 'pub fn handle_key' crates/tome/src/browse/app.rs | grep -q 'self.status_message = None'` (clear is the first statement after the fn signature, possibly with a doc comment in between — accept if it appears in the first 5 lines after the signature)
    - `grep -c 'self.status_message = None' crates/tome/src/browse/app.rs` returns 1 (single point of clearing)
    - `cargo build -p tome 2>&1 | grep -q 'error\[' && exit 1 || exit 0`
  </acceptance_criteria>
  <done>`handle_key` clears `self.status_message = None` as its first statement; no other clear sites exist.</done>
</task>

<task type="auto">
  <name>Task 4: Rewrite execute_action — cfg-dispatched open + arboard + status_message on both paths (D-07, D-08, D-12)</name>
  <files>crates/tome/src/browse/app.rs</files>
  <read_first>
    - .planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-CONTEXT.md (D-07, D-08, D-12)
    - .planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-RESEARCH.md (Pattern 3 — TUI Status-Bar Feedback; arboard Error Display strings)
    - crates/tome/src/browse/app.rs (execute_action ~lines 198-224)
    - crates/tome/src/paths.rs (verify the `paths::collapse_home` signature so the formatted path display is consistent with the rest of the CLI)
  </read_first>
  <action>
    In `crates/tome/src/browse/app.rs::execute_action` (~lines 198-224), rewrite the function body. Replace the existing `Command::new("open").arg(&path).spawn()` and `sh -c | pbcopy` invocations entirely.

    Required imports (add at top of the file alphabetically, only if not already present):
    ```rust
    use std::process::Command;
    ```
    (`arboard::Clipboard` is referenced via fully qualified path inside the function — no top-level import required, but adding `use arboard;` is also fine if it sorts cleanly.)

    New function body — ViewSource arm:
    ```rust
    DetailAction::ViewSource => {
        if let Some((_, _, path)) = self.selected_row_meta() {
            let binary = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
            match Command::new(binary).arg(&path).spawn() {
                Ok(_) => {
                    self.status_message = Some(format!(
                        "✓ Opened: {}",
                        crate::paths::collapse_home(&path).display()
                    ));
                }
                Err(e) => {
                    self.status_message = Some(format!("⚠ Could not open: {e}"));
                }
            }
        }
    }
    ```

    CopyPath arm:
    ```rust
    DetailAction::CopyPath => {
        if let Some((_, _, path)) = self.selected_row_meta() {
            let path_string = path.to_string_lossy().into_owned();
            let result = arboard::Clipboard::new()
                .and_then(|mut cb| cb.set_text(path_string));
            match result {
                Ok(()) => {
                    self.status_message = Some(format!(
                        "✓ Copied: {}",
                        crate::paths::collapse_home(&path).display()
                    ));
                }
                Err(e) => {
                    self.status_message = Some(format!("⚠ Could not copy: {e}"));
                }
            }
        }
    }
    ```

    Disable / Enable / Back arms: unchanged (per CONTEXT.md "leave alone" discretion — they are stubs awaiting machine.toml integration and remain out of scope for SAFE-02).

    `selected_row_meta()` already exists on `App`. Use the same pattern as the original `execute_action` for accessing the path. If the existing destructuring shape differs, adapt it minimally — do NOT introduce a new helper.

    NO `let _ = ...` pattern. NO `sh -c`. NO `pbcopy`. After this rewrite, those substrings must be absent from `app.rs`.
  </action>
  <verify>
    <automated>cargo build -p tome 2>&1 | tail -10 && cargo clippy -p tome --all-targets -- -D warnings 2>&1 | tail -10</automated>
  </verify>
  <acceptance_criteria>
    - `grep -q 'cfg!(target_os = "macos")' crates/tome/src/browse/app.rs`
    - `grep -q '"xdg-open"' crates/tome/src/browse/app.rs`
    - `grep -q 'arboard::Clipboard::new()' crates/tome/src/browse/app.rs`
    - `grep -q '✓ Opened:' crates/tome/src/browse/app.rs`
    - `grep -q '✓ Copied:' crates/tome/src/browse/app.rs`
    - `grep -q '⚠ Could not open:' crates/tome/src/browse/app.rs`
    - `grep -q '⚠ Could not copy:' crates/tome/src/browse/app.rs`
    - `! grep -q 'pbcopy' crates/tome/src/browse/app.rs` (sh-c invocation gone)
    - `! grep -q 'sh -c' crates/tome/src/browse/app.rs` (sh-c invocation gone)
    - `! grep -nE 'let _ =.*Command::new\("open"\)\.spawn' crates/tome/src/browse/app.rs` (silent drop pattern gone)
    - `cargo clippy -p tome --all-targets -- -D warnings 2>&1 | grep -q 'warning\|error' && exit 1 || exit 0`
  </acceptance_criteria>
  <done>execute_action uses cfg!-dispatched open binary + arboard for clipboard; both Ok and Err paths set status_message; sh -c / pbcopy / let _ = silent-drop patterns are gone; clippy clean.</done>
</task>

<task type="auto">
  <name>Task 5: Conditional status-bar render in ui.rs — substitute styled status_message for keybind line (D-11, D-12)</name>
  <files>crates/tome/src/browse/ui.rs</files>
  <read_first>
    - .planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-CONTEXT.md (D-11, D-12 — note CONTEXT.md's `ui.rs:190-200` line ref is WRONG)
    - .planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-RESEARCH.md (Pattern 3 conditional render block; "Verify CONTEXT.md line references" drift table; Pitfall 5 — reuse theme.alert, do NOT add theme.warning)
    - crates/tome/src/browse/ui.rs (CORRECTED locations: render_status_bar at ~ui.rs:332+ for Normal mode; inline Line::from(...) block at ~ui.rs:310-329 for Detail mode)
    - crates/tome/src/browse/theme.rs (verify `accent` and `alert` fields exist at ~theme.rs:14-33; do NOT add new fields)
  </read_first>
  <action>
    Two render sites need the conditional. Use RESEARCH.md's CORRECTED locations (NOT CONTEXT.md's `ui.rs:190-200` ref which points at `highlight_name`).

    **Site A — Normal-mode status bar in `render_status_bar` (~`ui.rs:332+`):**
    At the START of `render_status_bar` (before the existing `hint_pairs` assembly), add:
    ```rust
    if let Some(msg) = &app.status_message {
        let style = if msg.starts_with('⚠') {
            ratatui::style::Style::default().fg(theme.alert)
        } else {
            ratatui::style::Style::default().fg(theme.accent)
        };
        let line = ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(format!(" {} ", msg), style),
        ]);
        frame.render_widget(ratatui::widgets::Paragraph::new(line), area);
        return;
    }
    ```
    Use whatever short-form import paths the file already has (most of `ratatui::text::*`, `ratatui::style::*`, `ratatui::widgets::*` are already imported at the top of `ui.rs`). Drop the `ratatui::` prefix where the bare types are already in scope.

    **Site B — Detail-mode status bar inline at `~ui.rs:310-329`:**
    Find the inline `Line::from(vec![Span::styled(" Detail ", ...), ...])` assembly that builds the Detail-mode bottom bar. Wrap the existing keybind-line construction in an `if let Some(msg) = &app.status_message { ... } else { /* existing code */ }` such that when `Some`, the `Line::from` is built from a single `Span::styled(msg, style)` (same `theme.alert` vs `theme.accent` selection on `⚠` prefix), and when `None` the existing keybind line renders unchanged.

    **Color rule (D-12 + Pitfall 5):**
    - Success (`✓` prefix) → `theme.accent` (already exists; cyan/green-family)
    - Failure (`⚠` prefix) → `theme.alert` (already exists at `theme.rs:15`; yellow)
    - Detection: substring check `msg.starts_with('⚠')` is acceptable (D-12 explicitly says "styled ✓"/"styled ⚠" — startsWith on glyph is the simplest detection). Do NOT add a new `theme.warning` field.

    No layout restructuring; no extra `Constraint::Length(1)`. The status_message takes the existing keybind line's space.
  </action>
  <verify>
    <automated>cargo build -p tome 2>&1 | tail -5 && cargo clippy -p tome --all-targets -- -D warnings 2>&1 | tail -5</automated>
  </verify>
  <acceptance_criteria>
    - `grep -c 'if let Some(msg) = &app.status_message' crates/tome/src/browse/ui.rs` returns 2 (one for Normal, one for Detail)
    - `grep -q 'theme.alert' crates/tome/src/browse/ui.rs` (failure styling reuses existing theme field)
    - `grep -q 'theme.accent' crates/tome/src/browse/ui.rs` (success styling reuses existing theme field)
    - `grep -q "starts_with('⚠')" crates/tome/src/browse/ui.rs` (color dispatch by glyph prefix)
    - `! grep -q 'pub warning:' crates/tome/src/browse/theme.rs` (no new theme.warning field added — D-12 + Pitfall 5)
    - `! grep -q 'theme.warning' crates/tome/src/browse/ui.rs` (consequence of above)
    - `cargo clippy -p tome --all-targets -- -D warnings 2>&1 | grep -qE 'warning\[|error\[' && exit 1 || exit 0`
  </acceptance_criteria>
  <done>Both Normal-mode (render_status_bar) and Detail-mode (inline) status-bar render sites have the conditional `if let Some(msg) = &app.status_message { ... }` block; ✓ messages render in theme.accent, ⚠ in theme.alert; no theme.warning field added.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 6: Unit test — status_message lifecycle (set on action, clear on next key) (D-19)</name>
  <files>crates/tome/src/browse/app.rs</files>
  <behavior>
    - Calling `execute_action(DetailAction::CopyPath)` on an `App` with a valid selected row sets `app.status_message` to `Some(...)` matching either `✓ Copied:` (success path on dev machines with a clipboard) OR `⚠ Could not copy:` (success path on headless CI). Test asserts `is_some()` AND `starts_with("✓") || starts_with("⚠")` — accepts either glyph because we cannot guarantee clipboard availability.
    - After feeding ANY `KeyEvent` (e.g., `KeyCode::Char('h')`) to `handle_key`, `app.status_message` is `None`.
  </behavior>
  <read_first>
    - .planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-CONTEXT.md (D-19, D-17)
    - .planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-RESEARCH.md (Pitfall 4 — clear-on-any-key semantics)
    - crates/tome/src/browse/app.rs (existing `#[cfg(test)] mod tests` block — extend it; do NOT introduce trait abstractions per D-17)
  </read_first>
  <action>
    Add a unit test `status_message_set_by_copy_path_and_cleared_by_any_key` to the existing `#[cfg(test)] mod tests` block at the end of `crates/tome/src/browse/app.rs`.

    1. Use any existing test fixture for building an `App` with at least one selectable row. If none exists, build one inline minimally — do not introduce new abstractions.
    2. Call `app.execute_action(DetailAction::CopyPath)`.
    3. Assert: `assert!(app.status_message.is_some(), "status_message must be set after CopyPath action");`
    4. Assert: `let msg = app.status_message.as_ref().unwrap();`
       `assert!(msg.starts_with("✓") || msg.starts_with("⚠"), "expected ✓ or ⚠ prefix, got: {msg}");`
       (We can't guarantee clipboard works on headless CI — accept either success or failure form.)
    5. Feed any key: build a `crossterm::event::KeyEvent` with `KeyCode::Char('h')` and pass to `app.handle_key(...)`.
    6. Assert: `assert!(app.status_message.is_none(), "status_message must clear after any key");`

    Per D-19 + RESEARCH.md: do NOT attempt to force an arboard failure via mocking. arboard exposes no test hook (verified). Skip the forced-failure unit test and rely on the substring-tolerant assertion above + CI matrix. Per D-17: do NOT introduce `trait Opener` / `trait ClipboardBackend`.
  </action>
  <verify>
    <automated>cargo test -p tome --lib browse::app::tests::status_message_set_by_copy_path_and_cleared_by_any_key 2>&1 | tail -15</automated>
  </verify>
  <acceptance_criteria>
    - `grep -q 'status_message_set_by_copy_path_and_cleared_by_any_key' crates/tome/src/browse/app.rs`
    - `grep -q 'app.status_message.is_some()' crates/tome/src/browse/app.rs`
    - `grep -q 'app.status_message.is_none()' crates/tome/src/browse/app.rs`
    - `cargo test -p tome --lib browse::app::tests::status_message_set_by_copy_path_and_cleared_by_any_key 2>&1 | grep -q 'test result: ok. 1 passed'`
    - No new trait introduced in the test block (compare diff — should be only #[test] fn additions)
  </acceptance_criteria>
  <done>Unit test asserts status_message is Some after CopyPath, None after any key; tolerates either ✓/⚠ prefix to work on headless CI; no abstraction layer.</done>
</task>

<task type="auto">
  <name>Task 7: CHANGELOG entry under v0.8 unreleased (SAFE-02 / #414)</name>
  <files>CHANGELOG.md</files>
  <read_first>
    - CHANGELOG.md (top ~50 lines)
  </read_first>
  <action>
    In `CHANGELOG.md`, under v0.8 unreleased `### Fixed` (or `### Changed` if `Fixed` is already used by SAFE-01 — pick the more accurate semantic), add:

    ```
    - `tome browse` actions `open` (ViewSource) and `copy path` (CopyPath) now work on Linux (`xdg-open` and the `arboard` crate respectively). Both success (`✓`) and failure (`⚠`) outcomes appear in the TUI status bar in place of the keybind line until the next keypress, replacing the prior macOS-only silent-drop behavior. The `sh -c "echo -n ${path} | pbcopy"` invocation is removed (eliminates a command-injection vector). ([#414](https://github.com/MartinP7r/tome/issues/414))
    ```

    Do NOT bump the version number in `Cargo.toml`.
  </action>
  <verify>
    <automated>grep -q 'tome browse.*xdg-open' CHANGELOG.md && grep -q '#414' CHANGELOG.md</automated>
  </verify>
  <acceptance_criteria>
    - `grep -q '#414' CHANGELOG.md`
    - `grep -q 'xdg-open' CHANGELOG.md`
    - `grep -q 'arboard' CHANGELOG.md`
    - `grep -q 'status bar' CHANGELOG.md`
    - `git diff Cargo.toml 2>&1 | grep -q '^+version' && exit 1 || exit 0` (no version bump)
  </acceptance_criteria>
  <done>CHANGELOG.md has a bullet for SAFE-02 referencing #414, mentioning xdg-open + arboard + status bar; no Cargo.toml version bump.</done>
</task>

</tasks>

<verification>
- `cargo fmt -- --check` passes
- `cargo clippy --all-targets -- -D warnings` passes (no new warnings; `arboard::Clipboard` is `!Send` on some platforms but stays scoped to `execute_action` body — no Send-bound errors expected)
- `cargo test` passes (the new lifecycle unit test green)
- `cargo build -p tome` succeeds; `cargo tree -p tome -i image` reports no `image` crate (verifies `default-features = false` worked — Pitfall 1)
- Linux platform-branch coverage: `cargo check` on local macOS verifies the macOS branch compiles; the `cfg!(target_os = "linux")` branch is exercised at compile time by the existing GitHub Actions `ubuntu-latest` matrix runner (per RESEARCH.md, the runner does not have `xdg-utils` installed at runtime, but D-17/D-19 explicitly do not exercise the binary at runtime — only compile-time platform-branch coverage is required)
- No `theme.warning` field added (Pitfall 5 — reuse `theme.alert`)
- No `trait Opener` / `trait ClipboardBackend` / `#[cfg(target_os = "linux")]` direct test abstractions (D-17)
</verification>

<success_criteria>
- SAFE-02 requirement satisfied: Linux + macOS branches dispatched at compile time; status-bar surfaces ✓/⚠ outcomes in place of keybind line.
- `App.status_message: Option<String>` field exists; cleared at top of `handle_key` (any-key-dismisses); set by both Ok/Err paths in `execute_action`.
- `arboard` workspace dep present with `default-features = false` — `image` crate NOT in dep tree.
- `sh -c | pbcopy` invocation gone — command-injection vector closed.
- One new unit test (`status_message_set_by_copy_path_and_cleared_by_any_key`); no platform-specific tests; no test abstractions.
- Untouched per RESEARCH.md blast radius: `theme.rs:115-117` `.ok()` (D-14), `git.rs:69` `let _ = rev` (D-14), `DetailAction::Disable`/`Enable` arms (Claude's discretion default).
</success_criteria>

<output>
After completion, create `.planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-02-safe-02-browse-cross-platform-status-bar-SUMMARY.md` capturing:
- arboard workspace + per-crate dep entries (with default-features = false noted)
- App.status_message field + handle_key clear + execute_action rewrite
- ui.rs Normal-mode + Detail-mode conditional render sites (note CORRECTED locations vs CONTEXT.md drift)
- Theme reuse decision (theme.alert; no new theme.warning)
- Unit test name + pass status
- CHANGELOG bullet
- Any deviations from CONTEXT.md decisions (expected: none, but document the corrected ui.rs line refs)
</output>
