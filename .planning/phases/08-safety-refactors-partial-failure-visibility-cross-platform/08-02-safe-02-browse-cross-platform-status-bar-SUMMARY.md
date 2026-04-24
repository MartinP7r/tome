---
phase: 08-safety-refactors-partial-failure-visibility-cross-platform
plan: 02
subsystem: tui
tags: [browse, tui, arboard, cross-platform, xdg-open, status-bar, safety, #414, SAFE-02]

# Dependency graph
requires:
  - phase: 08-safety-refactors-partial-failure-visibility-cross-platform
    provides: CHANGELOG [Unreleased] ### Fixed section from plan 08-01 (extended with SAFE-02 bullet rather than starting a new section)
  - phase: 06-display-polish-docs
    provides: paths::collapse_home helper for ~/… display
provides:
  - arboard workspace + binary-crate dependency (default-features = false)
  - browse::app::App.status_message: Option<String> field
  - Clear-on-any-key semantics (first statement of handle_key sets status_message = None)
  - Cross-platform execute_action — cfg!(target_os = "macos") dispatch between `open` and `xdg-open`; arboard::Clipboard for CopyPath
  - Conditional status-bar render in ui.rs for both Normal (render_status_bar) and Detail (inline) modes — glyph-prefix color dispatch (✓ → accent, ⚠ → alert)
  - Unit test status_message_set_by_copy_path_and_cleared_by_any_key
affects:
  - 08-03-safe-03-relocate-read-link-warning (independent wave-1 plan; no code coupling)
  - Future machine.toml wiring for DetailAction::Disable/Enable (still stubs — intentionally untouched per CONTEXT.md discretion)

# Tech tracking
tech-stack:
  added:
    - "arboard 3.6.1 (via workspace dep 'arboard = { version = \"3\", default-features = false }')"
  patterns:
    - "cfg!(target_os = \"...\") compile-time dispatch for platform-varying binary names (instead of trait abstraction — per D-17)"
    - "Single-key-lifetime status message: clear unconditionally at top of input handler; set in action handlers; glyph-prefix (✓ / ⚠) drives color dispatch"
    - "Duplicated if-let Some/else render arms in two status-bar call sites; no shared helper extracted — sites have different surrounding Span construction (count-badge vs Detail-label)"

key-files:
  created: []
  modified:
    - Cargo.toml
    - Cargo.lock
    - crates/tome/Cargo.toml
    - crates/tome/src/browse/app.rs
    - crates/tome/src/browse/ui.rs
    - CHANGELOG.md

key-decisions:
  - "arboard added with default-features = false — strips image-data feature and avoids image 0.25 + transitive image-processing deps (verified: `cargo tree -p tome -i image` returns 'package ID specification did not match')"
  - "Glyph-prefix color dispatch (starts_with('⚠') → theme.alert; else → theme.accent) reuses existing theme fields; no theme.warning field added (Pitfall 5)"
  - "Test accepts either ✓ or ⚠ prefix — headless CI runners cannot be guaranteed a working clipboard service, and per D-17/D-19 no trait ClipboardBackend / mock introduced"
  - "Corrected plan example: crate::paths::collapse_home returns String (not Displayable PathBuf). Removed the plan's `.display()` call — collapse_home already produces a display String"
  - "Detail-mode status bar used if/else instead of early-return because the Line::from value is bound and passed to frame.render_widget later; pattern diverges from render_status_bar's return-early style but is cleaner for the inline site"

patterns-established:
  - "Cross-platform TUI action feedback: pair a cfg!(target_os) dispatch for platform binaries with a platform-agnostic crate (arboard for clipboard); surface both Ok and Err paths via a transient status_message field cleared on next keypress."

requirements-completed: [SAFE-02]

# Metrics
duration: 6min
completed: 2026-04-24
---

# Phase 08 Plan 02: SAFE-02 Browse Cross-Platform + Status Bar Summary

**`tome browse` `open` (ViewSource) and `copy path` (CopyPath) actions now work on Linux via `xdg-open` + `arboard` (replacing the macOS-only `open` + `sh -c … | pbcopy` invocation which was also a command-injection vector), and both success (`✓`) and failure (`⚠`) outcomes appear in the TUI status bar in place of the keybind line until the next keypress — closing #414.**

## Performance

- **Duration:** ~6 min
- **Started:** 2026-04-24T02:28:32Z
- **Completed:** 2026-04-24T02:34:29Z
- **Tasks:** 7 (plus 1 fmt fold-up commit)
- **Files modified:** 6 (Cargo.toml, Cargo.lock, crates/tome/Cargo.toml, crates/tome/src/browse/app.rs, crates/tome/src/browse/ui.rs, CHANGELOG.md)
- **Tests:** 452 unit + 123 integration = 575 total (+1 vs post-08-01 baseline of 574)

## Accomplishments

- `arboard = { version = "3", default-features = false }` added to workspace deps and pulled in via `{ workspace = true }` in the binary crate; `cargo tree -p tome -i image` confirms the `image` crate is NOT in the dep tree (Pitfall 1 satisfied)
- `App.status_message: Option<String>` field added; initialized `None`
- `handle_key` clears `self.status_message = None` as its first statement — any-key-dismisses semantics, single point of clearing
- `execute_action` rewrite:
  - ViewSource: `cfg!(target_os = "macos")` picks `open` (macOS) vs `xdg-open` (Linux/other); both `Ok` and `Err` paths set a formatted `status_message` with `crate::paths::collapse_home` for `~/…` display
  - CopyPath: `arboard::Clipboard::new().and_then(|mut cb| cb.set_text(path))` replaces the `sh -c | pbcopy` invocation; same `Ok`/`Err` → `status_message` pattern
- `ui.rs` status-bar sites:
  - Normal mode (`render_status_bar`): prepended `if let Some(msg) = &app.status_message { … return; }` block before the existing count/hint assembly
  - Detail mode (inline `Line::from(…)` at ~`ui.rs:310`): wrapped existing keybind-line construction in `if let Some … else { … existing … }`
  - Glyph-prefix color: `starts_with('⚠')` → `theme.alert`; else → `theme.accent`. No `theme.warning` field added (Pitfall 5)
- Unit test `status_message_set_by_copy_path_and_cleared_by_any_key` added to `browse::app::tests`; passes on dev machine
- CHANGELOG bullet under `[Unreleased]` `### Fixed` referencing #414, mentioning `xdg-open`, `arboard`, and the status bar; no Cargo.toml version bump

## Task Commits

1. **Task 1: arboard workspace + per-crate deps (default-features = false)** — `954f7ad` (chore)
2. **Task 2: App.status_message field + init None** — `9db6bc5` (feat)
3. **Task 3: Clear status_message at top of handle_key** — `c33be1e` (feat)
4. **Task 4: Rewrite execute_action — cfg-dispatched open + arboard** — `cca0e21` (feat)
5. **Task 5: Conditional status-bar render for Normal + Detail modes** — `27c467a` (feat)
6. **Task 6: Unit test — status_message lifecycle** — `3fbaecd` (test)
7. **Task 7: CHANGELOG entry under [Unreleased] ### Fixed** — `62ad629` (docs)
8. **Fmt fold-up: cargo fmt applied to SAFE-02 hunks** — `d305619` (chore)

## Files Created/Modified

- `Cargo.toml` — added `arboard = { version = "3", default-features = false }` to `[workspace.dependencies]` between `anyhow` and `clap` (alphabetical); pulled in `arboard 3.6.1` + a handful of objc2 deps on macOS via Cargo.lock resolution. No `image` crate.
- `Cargo.lock` — updated by cargo build after the dep add.
- `crates/tome/Cargo.toml` — added `arboard = { workspace = true }` under `[dependencies]`, after `anyhow.workspace = true`.
- `crates/tome/src/browse/app.rs` — added `pub status_message: Option<String>` field to `App`, init in `App::new`, clear at top of `handle_key`, fully rewrote `DetailAction::ViewSource` and `DetailAction::CopyPath` arms of `execute_action`, added `status_message_set_by_copy_path_and_cleared_by_any_key` test to the `#[cfg(test)] mod tests` block. `DetailAction::Disable | Enable | Back` arms untouched per CONTEXT.md "leave alone" discretion.
- `crates/tome/src/browse/ui.rs` — two conditional blocks added:
  - `render_status_bar` (Normal mode): prepended `if let Some(msg) = &app.status_message { … Paragraph::new(Line::from(spans)) … return; }` using `theme.alert` (⚠) or `theme.accent` (else) with `theme.status_bar_bg` background
  - `render_detail` inline bottom bar: wrapped the existing `Line::from(vec![…])` in `if let Some(msg) = &app.status_message { Line::from(vec![msg_span, pad]) } else { /* existing Detail | j/k select … */ }` to preserve the same keybind line when no message is set
- `CHANGELOG.md` — added one bullet to `[Unreleased]` `### Fixed` (below the existing SAFE-01 bullet) describing the cross-platform fix + status-bar surface + command-injection closure, linked to #414.

## Decisions Made

- **`arboard::Clipboard` NOT `Send`:** The `arboard::Clipboard` type is `!Send` on some platforms (e.g. macOS), which is why it's scoped to the `execute_action` body — it's constructed, used, and dropped within a single call. No `tokio::spawn`/async boundary is crossed, so no `Send` bound is triggered. If browse ever goes async (Tokio), clipboard access will need to be marshaled to a dedicated task.
- **`collapse_home` returns `String`, not a `Display`-able PathBuf** (plan had a spurious `.display()` call). Verified in `crates/tome/src/paths.rs:142`; used directly inside `format!("✓ Opened: {}", collapse_home(Path::new(&path)))`. No `.display()` needed.
- **Detail-mode bottom bar uses `if / else`, Normal-mode uses `if let … return;`:** The two sites differ structurally — Normal-mode `render_status_bar` already has `frame.render_widget(…, area)` as its last statement so early return is clean; Detail-mode binds a `Line` value that's passed to `frame.render_widget` outside the block, so an `if/else` expression is cleaner than hoisting an early return. Both sites satisfy the plan's `if let Some(msg) = &app.status_message` acceptance grep (exactly 2 occurrences).
- **Test tolerates both `✓` and `⚠` prefixes:** Per D-19, we don't introduce a `trait ClipboardBackend` to force a specific branch. On the dev machine, `arboard::Clipboard::new()` succeeds and the test exercises the `✓` path; on headless CI runners it may fail and the test exercises the `⚠` path. Either outcome proves the lifecycle contract (`Some(...)` set by action, `None` after next key).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `cargo fmt` preferred single-line `let result = arboard::Clipboard::new().and_then(...)` over the two-line form I initially wrote**
- **Found during:** final `make fmt-check` gate
- **Issue:** `make fmt-check` failed on a whitespace-only diff in the `CopyPath` arm — rustfmt wanted `let result = arboard::Clipboard::new().and_then(|mut cb| cb.set_text(path.clone()));` on one line, not split across two. No semantic change.
- **Fix:** Ran `cargo fmt`, folded into a single `chore(08-02)` commit (d305619).
- **Files modified:** crates/tome/src/browse/app.rs
- **Verification:** `make fmt-check` passes; `make lint` passes; `cargo test` passes (575 total).

**2. [Rule 1 - Bug] Plan's acceptance grep `! grep -q 'pbcopy'` caught my historical-context comment**
- **Found during:** Task 4 post-edit acceptance check
- **Issue:** The comment I originally added inside the `CopyPath` arm read "Replaces the old ``sh -c "echo -n '${path}' | pbcopy"`` invocation …" for documentation. The plan's acceptance criteria intentionally greps for absence of `pbcopy` and `sh -c` — the comment would have failed the criteria.
- **Fix:** Rephrased the comment as "Replaces the prior macOS-only shelled-pipe invocation, which was also a command-injection vector (paths with apostrophes could escape the single-quote wrapping)" — preserves the historical context for future readers without reintroducing the exact substrings the plan greps forbid.
- **Files modified:** crates/tome/src/browse/app.rs
- **Verification:** `grep -n 'pbcopy' crates/tome/src/browse/app.rs` returns empty; `grep -n 'sh -c' crates/tome/src/browse/app.rs` returns empty.
- **Committed in:** `cca0e21` (Task 4 commit; cleanup applied before commit, no separate commit needed)

### Plan-text corrections applied in code

- **Plan's `collapse_home(…)`.display()` call** — incorrect. `crate::paths::collapse_home` returns `String`, not something with a `.display()` method. Code uses `collapse_home(Path::new(&path))` directly in the `format!` as-is. No impact on behavior.

**Total deviations:** 2 auto-fixed (1 bug, 1 blocking) + 1 silent plan-text correction. All within the files listed in the plan's `files_modified` frontmatter.

## Issues Encountered

- Git commit signing agent returned "agent refused operation?" once during the fmt fold-up commit (d305619); a single retry succeeded. Likely the Bitwarden SSH agent had just woken up. Not a blocker. All other commits signed without issue (well — unsigned per the sequential-executor protocol on phase branches).
- Task 1's initial acceptance check appeared to fail due to a bash grep chain short-circuit on a non-zero exit from an earlier `grep -q` invocation — false negative, the actual acceptance was met. Resolved by running the checks individually rather than chaining `&&`.

## User Setup Required

None — purely a source-level refactor. `arboard` resolves transitively via `cargo build` without any system packages on macOS; on Linux, `arboard` will link against system `libxcb` / wayland libs if the user has a desktop session, and fall back to a descriptive error on fully headless systems (which surfaces via the new status bar — exactly the designed UX).

## Next Phase Readiness

- SAFE-02 complete: Linux + macOS both exercise the cross-platform code path at compile time via the CI matrix (`ubuntu-latest` + `macos-latest`). The status-bar surface is reusable for any future TUI action feedback (a natural fit for the eventual machine.toml `Disable`/`Enable` wiring).
- SAFE-03 (relocate read_link warning) is the remaining wave-1 plan — no dependency on SAFE-02. Pattern to mirror: PR #417's `eprintln!("warning: …")` shape.
- No new abstractions introduced (no `trait Opener`, no `trait ClipboardBackend`, no `#[cfg(target_os = "linux")]` direct tests) — D-17 discipline held.

## Self-Check: PASSED

Verified:
- `Cargo.toml` contains `arboard = { version = "3", default-features = false }` ✓
- `crates/tome/Cargo.toml` contains `arboard = { workspace = true }` ✓
- `cargo tree -p tome -i image` returns no match (image crate NOT in tree) ✓
- `crates/tome/src/browse/app.rs` contains `pub status_message: Option<String>` ✓
- `crates/tome/src/browse/app.rs` contains `self.status_message = None` (count = 1, in handle_key) ✓
- `crates/tome/src/browse/app.rs` contains `cfg!(target_os = "macos")` ✓
- `crates/tome/src/browse/app.rs` contains `"xdg-open"` ✓
- `crates/tome/src/browse/app.rs` contains `arboard::Clipboard::new()` ✓
- `crates/tome/src/browse/app.rs` contains `✓ Opened:` and `✓ Copied:` ✓
- `crates/tome/src/browse/app.rs` contains `⚠ Could not open:` and `⚠ Could not copy:` ✓
- `crates/tome/src/browse/app.rs` does NOT contain `pbcopy` or `sh -c` ✓
- `crates/tome/src/browse/app.rs` does NOT contain `let _ = …Command::new("open").spawn` ✓
- `crates/tome/src/browse/ui.rs` contains `if let Some(msg) = &app.status_message` (count = 2) ✓
- `crates/tome/src/browse/ui.rs` contains `theme.alert` and `theme.accent` ✓
- `crates/tome/src/browse/ui.rs` contains `starts_with('⚠')` ✓
- `crates/tome/src/browse/theme.rs` does NOT contain `pub warning:` ✓
- `CHANGELOG.md` contains `#414`, `xdg-open`, `arboard`, `status bar` ✓
- Commit `954f7ad` exists (Task 1) ✓
- Commit `9db6bc5` exists (Task 2) ✓
- Commit `c33be1e` exists (Task 3) ✓
- Commit `cca0e21` exists (Task 4) ✓
- Commit `27c467a` exists (Task 5) ✓
- Commit `3fbaecd` exists (Task 6) ✓
- Commit `62ad629` exists (Task 7) ✓
- Commit `d305619` exists (fmt fold-up) ✓
- `make fmt-check` passes ✓
- `make lint` passes (clippy -D warnings) ✓
- `cargo test` passes (452 unit + 123 integration = 575 tests) ✓
- New test `status_message_set_by_copy_path_and_cleared_by_any_key` passes ✓

---
*Phase: 08-safety-refactors-partial-failure-visibility-cross-platform*
*Completed: 2026-04-24*
