# Phase 8: Safety Refactors (Partial-Failure Visibility & Cross-Platform) - Context

**Gathered:** 2026-04-24
**Status:** Ready for planning

<domain>
## Phase Boundary

Close three P0/P1 safety gaps surfaced by `/pr-review-toolkit` audits on v0.7 that didn't fit the v0.7.1 hotfix. All three share one theme: **when a destructive command fails partially, the user must see it — no silent success.**

1. **SAFE-01** (#413) — `remove::execute` aggregates partial filesystem failures (dist symlinks, library dirs, library symlinks, git cache) into `RemoveResult` so the caller can surface them. Destructive commands cannot report success while partial cleanup failed.
2. **SAFE-02** (#414) — Browse UI's `DetailAction::ViewSource` and `CopyPath` work on Linux (currently macOS-only via `open` + `pbcopy`-via-`sh -c`), drop the `sh -c` command-injection vector, and surface success/failure in the TUI status bar instead of `let _ = ...` silent drops.
3. **SAFE-03** (#449) — `relocate.rs:93` replaces `std::fs::read_link(..).ok()` with an `eprintln!` warning that names the path and error, mirroring the pattern shipped in #417's fix in `lib.rs::resolve_git_directories`.

Out of scope for this phase:
- New commands, UI surfaces, or capabilities beyond the three sites above.
- Cross-machine config portability (PORT-01..04) — deferred to v0.9 per epic #459.
- Rewriting the browse TUI beyond adding the new `status_message` feedback surface.
- Tests for `arboard`'s internals or `xdg-open`'s behavior — trust upstream + CI matrix.
- Windows support — Unix-only remains a hard constraint (see PROJECT.md Constraints).
- Backporting the warning pattern to every other `.ok()` site in the codebase — audit deliberately narrowed to the sites flagged by the review agents.

</domain>

<decisions>
## Implementation Decisions

### SAFE-01: `remove::execute` Partial-Failure Aggregation

**Error type shape**

- **D-01:** `RemoveResult.failures: Vec<RemoveFailure>` where `RemoveFailure { path: PathBuf, op: FailureKind, error: io::Error }` and `FailureKind` is an enum with variants `Symlink` (distribution-dir symlinks removed in step 1), `LibraryDir` (local library directories removed in step 2a), `LibrarySymlink` (managed-skill library symlinks removed in step 2b), and `GitCache` (git repo cache removed in step 4). Rationale: typed op-kind enables grouped summary lines, clean test assertions (`result.failures.iter().any(|f| f.op == FailureKind::Symlink)`), and future `tome doctor` routing into its existing issue categories. ~15 LoC more than `Vec<(PathBuf, io::Error)>`; worth it for the signal.
- **D-02:** Keep the existing success counts (`symlinks_removed`, `library_entries_removed`, `git_cache_removed`) alongside `failures`. Counts remain the success-path summary; `failures` is purely additive. Drop `#[allow(dead_code)]` on `git_cache_removed` now that the caller will surface it.
- **D-03:** Drop the existing `eprintln!("warning: ...")` lines inside each `execute` loop body. They are redundant once failures are aggregated, and keeping them alongside the aggregated summary would double-print. The caller becomes the single source of truth for user-facing warning output.

**Exit code & caller behavior**

- **D-04:** `Command::Remove` in `lib.rs`: on non-empty `result.failures`, print the existing `✓ Removed directory 'X': N library entries, M symlinks` line AND a follow-up `⚠ K operations failed — run tome doctor` line (grouped by `FailureKind` with per-path detail), then return `Err(anyhow::anyhow!("remove completed with K failures"))`. Shell scripts see exit ≠ 0; humans see both partial-success scope and the warning. Matches issue #413 alternative 2 ("Prints '⚠ N operations failed' AND exits non-zero").
- **D-05:** The error-line format: group by `FailureKind` first, then list per-path entries under each group. Example:
  ```
  ⚠ 2 operations failed — run `tome doctor`:
    Distribution symlinks (1):
      ~/.claude/skills/my-skill: Permission denied (os error 13)
    Library entries (1):
      ~/.tome/skills/my-skill: Directory not empty (os error 66)
  ```
  Color: `⚠` in yellow via `console::style`, preserving the repo's existing style vocabulary. Paths rendered via `paths::collapse_home()` for shorter display (same convention as `status.rs` and Phase 6's tabled summary).

**State-save ordering**

- **D-06:** Keep the current save order: save config → save manifest → regenerate and save lockfile → print summary → return Err if failures. Rationale: the *plan* still mostly executed (directory removed from config, manifest cleaned, lockfile regenerated). Leftover filesystem artifacts become `tome doctor`'s territory, and the next `tome sync` cleanup pass removes them automatically. Matches the user's mental model — "I asked for X to be removed, most of it happened, the tool told me which bits didn't." The alternative (skip save on any failure) creates an inconsistent half-state where config still references a now-partially-removed directory.

### SAFE-02: Cross-Platform Browse Actions + Status Bar Feedback

**Clipboard strategy**

- **D-07:** Add `arboard` as a direct dependency (latest 3.x at time of planning; pin via `cargo add arboard` to get current minor). Replace the `sh -c "echo -n '${path}' | pbcopy"` invocation with `arboard::Clipboard::new()?.set_text(path)?`. Removes the `sh -c` command-injection footgun (#414) and handles Wayland/X11/macOS internally. Trade-off accepted: ~4–6 transitive deps on Linux (x11rb + wayland-client families). The maintenance win over a hand-rolled fallback chain justifies the dep weight for this phase.

**Open strategy**

- **D-08:** Replace `Command::new("open").arg(&path).spawn()` with a `cfg!`-based dispatch:
  ```rust
  let binary = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
  Command::new(binary).arg(&path).spawn()
  ```
  No new crate. `xdg-open` is the de-facto standard on Linux desktops (GNOME, KDE, XFCE). If it's not installed on a minimal system, the spawn fails and the error surfaces via the status bar (per D-09..D-11).

**Status-bar plumbing (the new TUI feedback surface)**

- **D-09:** Add `status_message: Option<String>` field to `browse::app::App`. Set by `execute_action` on both success and failure paths. No timers, no `Instant`, no background tick.
- **D-10:** Clear `status_message` at the top of `handle_key` **before** dispatching to the mode-specific handlers. Effect: the message is visible until the user presses any key, then disappears on the next action. Single-key lifetime — simplest possible lifecycle, no event-loop changes.
- **D-11:** `ui.rs` bottom-bar render: when `app.status_message` is `Some`, render the message *in place of* the static keybind line. When `None`, render keybinds as today. Works identically in `Mode::Normal` and `Mode::Detail`. One local change in `ui.rs`, no layout restructuring, no extra `Constraint::Length(1)`.
- **D-12:** Message styling:
  - **Success:** `✓ Copied: ~/.tome/skills/my-skill` in `theme.accent` color (repo's existing green-family accent).
  - **Failure:** `⚠ Could not open: xdg-open not found` in a yellow-family color (use whatever `theme` exposes; if nothing fits, add a `theme.warning` field).
  Success messages exist because the current silent-success behavior is the real UX bug — users press "copy path," nothing visibly happens, and they wonder whether it worked. The styled ✓ is the positive confirmation that the action completed.

### SAFE-03: Relocate Symlink-Read Warning

- **D-13:** Replace `std::fs::read_link(&link_path).ok()` at `relocate.rs:93` with an explicit `match` that on `Err(e)` emits `eprintln!("warning: could not read symlink at {}: {e}", link_path.display())` and returns `None`. Literal pattern from issue #449 (which itself mirrors #417's `lib.rs::resolve_git_directories` fix). Do not gate on `!cli.quiet` — `relocate` is interactive-first and the caller does not have an easy `cli` handle at this call site. A future refactor can add quiet-gating uniformly; not part of this phase.
- **D-14:** SAFE-03 is the only site changed in this plan. Other `.ok()` occurrences in the codebase are deliberate (e.g. `theme.rs::115-117` env-parse fallback, `git.rs::69` unused-variable suppression) — do NOT expand the audit here. The review agents explicitly flagged only `relocate.rs:93`.

### Packaging & Tests

**Plan breakdown**

- **D-15:** Three plans, one per SAFE-XX requirement, matching Phase 7's per-requirement pattern:
  - `08-01-safe-01-remove-partial-failure-aggregation-PLAN.md` — `RemoveResult` + caller wiring + tests. ~60 LoC + tests. Biggest plan.
  - `08-02-safe-02-browse-cross-platform-status-bar-PLAN.md` — `arboard` dep, `cfg!` open dispatch, `status_message` field, `ui.rs` render, `execute_action` rewire, tests. ~80 LoC. Medium.
  - `08-03-safe-03-relocate-read-link-warning-PLAN.md` — one-line match replacement + unit test covering the error path. ~10 LoC + 1 test. Smallest; can ship first as a warmup or last as a follow-up.
- **D-16:** No cross-plan ordering constraint. Each plan is independent at the code level. Shipping order is planner/executor's choice; SAFE-03 is a natural warmup because it's trivially small and validates the `eprintln!` pattern that the others reference.

**Test strategy**

- **D-17:** **No test abstraction traits.** Do NOT introduce `trait Opener` or `trait ClipboardBackend` for testability. The truly platform-specific code surface after the above decisions is ~4 lines (`cfg!` branch + `arboard` call). The GitHub Actions matrix already runs `ubuntu-latest` + `macos-latest` per CI convention — that's the honest way to exercise the platform branch. Abstraction would be more scaffold than value.
- **D-18:** SAFE-01 tests:
  - **Unit** (in `remove.rs` `#[cfg(test)] mod tests`): add a test that pre-deletes a dist-dir symlink (so `remove_file` returns `ENOENT`) and asserts `result.failures` contains a `RemoveFailure` with `op == FailureKind::Symlink` and the matching path. Also: extend `make_test_setup` if needed to support the failure-injection fixture.
  - **Integration** (in `tests/cli.rs`): add a test that runs `tome remove <name>` with `--force` against a fixture where at least one artifact is un-removable (e.g., a symlink in a read-only parent dir) and asserts exit ≠ 0 AND `⚠` marker present in stderr.
- **D-19:** SAFE-02 tests (platform-agnostic):
  - Unit test `status_message` lifecycle: trigger `execute_action` for `CopyPath` with a valid path, assert `app.status_message == Some("✓ Copied: …")`; then feed a `KeyEvent` via `handle_key`, assert `status_message == None` afterwards.
  - Unit test `execute_action` error handling: force a failure (e.g., inject an invalid path or mock `arboard` via the test-time feature flag if arboard exposes one; otherwise skip and rely on CI to exercise the success path).
  - Do NOT add `#[cfg(target_os = "linux")]` direct tests. The existing CI matrix validates the cfg branch end-to-end on Linux; re-asserting that at the unit level would duplicate CI coverage with no added signal.
- **D-20:** SAFE-03 test: unit test in `relocate.rs` tests module that calls `plan()` against a manifest with a managed skill whose symlink has been replaced with a regular file (so `read_link` errors). Assert the warning is printed (capture stderr via `gag` or similar; if adding a test helper is heavy, assert via observable side-effect — `source_path` stays `None`). Verify the plan itself still succeeds (the `None` fallback still executes the rest of the walk).

### Claude's Discretion

- Exact text of the `⚠ K operations failed` summary line (wording may evolve during planning to align with copy conventions elsewhere in the CLI).
- Whether the `FailureKind` enum lives in `remove.rs` (private to the module) or moves to `crate::result` / a shared `errors` module. Default: keep local to `remove.rs` unless `reassign.rs` or `relocate.rs` grows the same pattern.
- Whether the `arboard` version is pinned loosely (`3`) or tightly (`3.4`) in `Cargo.toml`. Default: follow existing conventions — most direct deps pin to major only.
- Specific color choice for the failure message if `theme` doesn't already expose a warning color (adding a `theme.warning` field vs reusing `theme.accent` with modifier). Default: reuse what exists; add a `warning` field only if visual feedback looks wrong.
- Whether to also touch the `DetailAction::Disable`/`Enable` silent-noop path (currently returns to Normal mode without doing anything per app.rs:216-220). Default: leave alone — those are stubs waiting on machine.toml integration, not silent-failure bugs.
- The exact placement order of the new `arboard` dep in `Cargo.toml` (alphabetical slot between existing deps).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & Roadmap

- `.planning/REQUIREMENTS.md` — SAFE-01, SAFE-02, SAFE-03 definitions and Phase 8 traceability table. Maps each requirement to its GitHub issue (#413, #414, #449).
- `.planning/ROADMAP.md` §"Phase 8: Safety Refactors (Partial-Failure Visibility & Cross-Platform)" — four success criteria that must be TRUE after this phase. Criterion #4 explicitly allows either `#[cfg(target_os = "linux")]` tests OR platform-agnostic abstractions; D-17 picks the CI-matrix path per ROADMAP's "OR" wording.
- `.planning/PROJECT.md` §"Current Milestone: v0.8 Wizard UX & Safety Hardening" — scope-anchor context; confirms these three are P0/P1 carryovers from the v0.7 codebase audit, not new capabilities.

### GitHub Issues (external specs — read these; they contain concrete fix directions)

- **#413** — `bug: remove command reports success even when partial cleanup fails`. Body specifies the `Vec<(PathBuf, io::Error)>` proposal (D-01 extends to a typed struct), names `crates/tome/src/remove.rs:191-231` and `crates/tome/src/lib.rs:312-325` as the sites, and lists the two exit alternatives explicitly.
- **#414** — `bug: browse UI 'open'/'copy path' actions fail silently on Linux + command-injection risk`. Body specifies `crates/tome/src/browse/app.rs:200-215` as the site, flags the `sh -c` injection vector, offers `arboard` as an alternative to hand-rolled fallback chains (D-07 picks `arboard`), and requires status-bar surfacing.
- **#449** — `bug: relocate.rs silently drops fs::read_link errors (sibling pattern to #417)`. Body includes a literal code snippet (mirrored in D-13) and points at `relocate.rs:93`. References **PR #417** as the sibling-pattern fix that this phase replicates.

### Reference PR (the pattern SAFE-03 inherits)

- **PR #417** — Patched `lib.rs::resolve_git_directories` to replace silent `.ok()` drops with `eprintln!("warning: ...")`. SAFE-03's D-13 literally mirrors this pattern. When planner reviews, check the PR's commit diff for the exact warning format — D-13 should produce prose that matches.

### Prior Phase Context (decisions carried forward)

- `.planning/phases/04-wizard-correctness/04-CONTEXT.md` — Phase 4 D-08 (`Config::save_checked` fail-loud-not-silent discipline). Directly relevant because SAFE-01 D-04 extends the same philosophy: destructive runtime ops fail loud too.
- `.planning/phases/05-wizard-test-coverage/05-CONTEXT.md` — Phase 5 D-09 (substring matching > snapshots for polish-layer assertions; crate-boundary `pub(crate)` visibility rule). Relevant to D-18 integration-test assertions and to `RemoveFailure` visibility.
- `.planning/phases/06-display-polish-docs/06-CONTEXT.md` — Phase 6 D-01..D-06 (styled CLI output conventions: `console::style`, `paths::collapse_home()`, terminal-width awareness). D-05's error-line format follows these conventions directly.

### Key Source Files

**SAFE-01 sites:**

- `crates/tome/src/remove.rs:46-51` — `RemoveResult` struct definition. Target for D-01/D-02 expansion (add `failures: Vec<RemoveFailure>` + new `RemoveFailure` struct + `FailureKind` enum).
- `crates/tome/src/remove.rs:181-265` — `execute()` body. Four partial-failure loops (symlinks: 192-204; library dirs: 207-231; manifest: 234-237; git cache: 241-253). Per D-03, replace each loop's `eprintln!` with `result.failures.push(RemoveFailure { ... })`.
- `crates/tome/src/remove.rs:267-404` — existing tests. D-18 adds failure-injection test(s) here.
- `crates/tome/src/lib.rs:362-414` — `Command::Remove` handler. D-04/D-05 updates: after `remove::execute` returns, inspect `result.failures`; if non-empty, print grouped summary and return `Err`.
- `crates/tome/tests/cli.rs` — integration tests. D-18 adds a `tome remove` test that asserts exit ≠ 0 on partial failure.

**SAFE-02 sites:**

- `crates/tome/src/browse/app.rs:41-61` — `DetailAction` enum + `label()`. No changes expected; referenced for context.
- `crates/tome/src/browse/app.rs:71-89` — `App` struct definition. Add `status_message: Option<String>` field here per D-09.
- `crates/tome/src/browse/app.rs:91-117` — `App::new`. Initialize `status_message: None`.
- `crates/tome/src/browse/app.rs:119-163` — `handle_key` + `handle_normal_key`. Clear `status_message` at top of `handle_key` per D-10, before dispatching to mode handlers.
- `crates/tome/src/browse/app.rs:198-225` — `execute_action`. Primary rewrite target for D-07/D-08/D-12 (arboard + cfg dispatch + status_message set on success and failure paths).
- `crates/tome/src/browse/ui.rs:190-192` — Normal-mode bottom-bar `Line::from(spans)` assembly. D-11 makes this conditional on `app.status_message`.
- `crates/tome/src/browse/ui.rs:197-200` — Detail-mode bottom-bar layout. Same D-11 conditional applies.
- `crates/tome/src/browse/theme.rs:21-27` — Theme colors. D-12: if adding a `warning` color, it goes here alongside `accent` and the existing status-bar fields.
- `crates/tome/src/browse/theme.rs:115-117` — `env!` parse fallback via `.ok()`. **Deliberate** — do NOT change per D-14.
- `Cargo.toml` — workspace root. Add `arboard` to `[workspace.dependencies]` per D-07. Check alphabetical ordering.
- `crates/tome/Cargo.toml` — binary crate. Add `arboard = { workspace = true }` entry.

**SAFE-03 site:**

- `crates/tome/src/relocate.rs:89-100` — managed-skill symlink-read block inside `plan()`. D-13 replaces line 93's `.ok()` with explicit `match`. The surrounding logic (`entry.managed` check, `resolve_symlink_target` call) stays identical.
- `crates/tome/src/relocate.rs:477-818` — existing tests. D-20 adds a new test fixture with a corrupted managed-skill symlink that triggers `read_link` error.
- `crates/tome/src/git.rs:69` — `let _ = rev;` unused-variable suppression. **Deliberate** — do NOT change per D-14.

### Documentation Files

- `CHANGELOG.md` — under the v0.8 unreleased section, add three bullets (one per SAFE-XX) with issue references. Planner may group under a single "Safety Refactors" subheader.
- No other doc touches required. The `docs/src/` architecture/commands pages do not describe the internals of `remove`/`browse`/`relocate` at a level that would be affected by these refactors.

### Test Coverage Expectations

- **SAFE-01:** Unit test in `remove.rs` (failure injection) + integration test in `tests/cli.rs` (exit code + stderr marker) per D-18.
- **SAFE-02:** Unit tests for `status_message` lifecycle and `execute_action` dispatch, platform-agnostic per D-17/D-19. No `#[cfg(target_os = "linux")]` direct tests; trust the `ubuntu-latest` + `macos-latest` CI matrix for actual platform branch coverage.
- **SAFE-03:** Unit test in `relocate.rs` asserting the warning path is taken when `read_link` fails, and the plan still succeeds with `source_path: None` per D-20.
- Total new test count: expected 4–6 tests across the three plans. No snapshot tests (D-18 rules them out for SAFE-01; D-19/D-20 stay substring/behavioral per Phase 5 D-09 precedent).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **Plan/render/execute pattern** (`remove.rs`, `reassign.rs`, `relocate.rs`, `fork` in `reassign.rs`) — SAFE-01 extends the existing `RemoveResult` without changing the pattern. No architectural change; pure additive.
- **`paths::collapse_home()`** — already used in `status.rs:181` and Phase 6's tabled summary. Reuse for D-05's per-path error rendering so the user sees `~/…` rather than `/Users/martin/…`.
- **`console::style()` + repo color vocabulary** (`style().green()` for success ticks, `style().yellow()` for warnings, `style().bold()` for labels) — established across `lib.rs`, `remove.rs`, `reassign.rs`. D-05's summary line uses these verbatim.
- **`eprintln!("warning: ...")` + optional `!cli.quiet` gating** — pattern used in `lib.rs:350-353` (discover warnings), `lib.rs:402` (lockfile regen warnings). SAFE-03 mirrors this without the quiet gate (D-13 rationale).
- **`ratatui::text::Line::from(spans)` for keybind rendering** (`browse/ui.rs:190-192`) — existing template. D-11's conditional render replaces the Line build with a `Line::from(vec![Span::styled(...)])` for the status message when present.
- **Existing `theme::Theme::detect()` auto-detection** (`browse/theme.rs`) — provides the color palette. D-12 reuses existing fields; add `warning` color only if needed.

### Established Patterns

- `anyhow::Result<T>` throughout; `.with_context(|| format!(...))` at I/O boundaries. SAFE-01's `execute` keeps returning `Result<RemoveResult>` — the `Result` layer catches unexpected I/O (e.g., `read_dir` in `plan()`), while `RemoveResult.failures` captures the *expected* per-item failures.
- CI matrix: `.github/workflows/` runs `ubuntu-latest` + `macos-latest`. D-17 leans on this for platform coverage without adding test scaffolding.
- Cargo.toml alphabetical ordering of `[dependencies]` / `[workspace.dependencies]`. Insert `arboard` in its alphabetical slot (before `assert_cmd`, after `anyhow`).
- Test co-location: unit tests in `#[cfg(test)] mod tests` in the same file as the production code (D-18 Unit + D-19 + D-20 all follow this). Integration tests in `crates/tome/tests/cli.rs` (D-18 Integration).

### Integration Points

- `lib.rs::Command::Remove` (362-414) is the sole caller of `remove::execute`. Single-site wire-up for D-04/D-05.
- `browse::browse()` is the TUI entry point from `lib.rs::Command::Browse` (355-361). No signature changes; `App` owns all new state.
- `relocate.rs::plan()` is called from `lib.rs::Command::Relocate` (not shown in context above but present). D-13 is internal to `plan()` — no signature change.
- `Cargo.toml` root — `[workspace.dependencies]` is the single-source-of-truth for dep versions; the per-crate `Cargo.toml` uses `{ workspace = true }`.

### Blast Radius

- **SAFE-01:** `remove.rs` (struct additions + loop body edits + tests) + `lib.rs` (Remove handler) + `tests/cli.rs` (integration test). ~60 LoC production, ~40 LoC tests.
- **SAFE-02:** `browse/app.rs` (status_message field + execute_action rewrite + handle_key clear) + `browse/ui.rs` (conditional render) + `browse/theme.rs` (optional warning color) + `Cargo.toml` + `crates/tome/Cargo.toml`. ~80 LoC production, ~30 LoC tests.
- **SAFE-03:** `relocate.rs` (one-block edit + one new test). ~10 LoC production, ~20 LoC test.
- **No changes to:** `config.rs`, `discover.rs`, `library.rs`, `distribute.rs`, `manifest.rs`, `cleanup.rs`, `doctor.rs`, `lockfile.rs`, `wizard.rs`, `machine.rs`, `git.rs` (other than leaving `:69` alone per D-14).

</code_context>

<specifics>
## Specific Ideas

- **Color vocabulary consistency:** The `✓` + green for success / `⚠` + yellow for warning pattern is already used in `remove.rs`, `lib.rs`, and `relocate.rs`. SAFE-01 D-05 and SAFE-02 D-12 should use the identical glyphs and color families — if they diverge, it will read as inconsistent.
- **arboard's error type:** `arboard::Error` is a custom enum. For SAFE-02 D-12's failure message, format as `format!("{e}")` — the `Display` impl produces user-friendly text like `"The clipboard contents were not available in the requested format."` rather than Rust-debug output.
- **FailureKind::GitCache is rare:** Only git-type directories have a git cache to remove (remove.rs:241), so this variant will almost never populate. Keep it for completeness; don't optimize it away.
- **#417's fix pattern (reference):** Before writing D-13's match block, re-read the exact format used in the shipped #417 fix to match word choice. The issue body's snippet is illustrative, not canonical — the PR is.
- **Status-message clear-on-key edge case:** If `handle_key` clears `status_message` at the top but the key is a no-op in the current mode (e.g., pressing `h` in Normal mode), the message still disappears. This is fine — it's the "any-key-dismisses" semantic, matching `?` help overlay dismissal at `app.rs:125-128`.
- **`arboard::Clipboard::new()` can fail at construction** on systems without a clipboard service (headless Linux over SSH, for example). D-12's failure path must handle this — `Clipboard::new()?` propagates the error, which becomes the status-bar warning.
- **Integration test fixture for SAFE-01:** The easiest "un-removable symlink" fixture is a symlink whose parent dir has had permissions stripped to `000` or a dir owned by another user. On CI, `chmod 000` inside a `TempDir` is the portable trick. Remember to `chmod 755` before `TempDir::drop` or test cleanup panics.
- **D-17 vs D-19 consistency:** D-17 says "no test abstraction traits." D-19 says "force a failure (e.g., inject an invalid path or mock `arboard` via the test-time feature flag if arboard exposes one; otherwise skip)." If arboard exposes no test hook, skip the forced-failure unit test — do NOT walk back D-17 by inventing a trait. The CI matrix covers the success path; the failure-handling *logic* is still testable via path-invalidation (`Clipboard::new()` inside a `cfg(test)` block that swaps to a stub writer — only if zero-scaffold).

</specifics>

<deferred>
## Deferred Ideas

- **Backport eprintln-warning pattern to every `.ok()` site** — The review agents flagged only `relocate.rs:93`. Systematic audit of all `.ok()` drops across the codebase would be its own phase (and might be unnecessary — most `.ok()` sites are deliberate fallbacks). Don't expand scope here.
- **`tome doctor` integration for `RemoveFailure`** — D-01's `FailureKind` is designed to route into doctor's existing issue categories, but wiring doctor to *read* leftover RemoveFailures (e.g., via a `.tome/pending-failures.json` dropbox) is a separate feature. Mentioned as future value; not in this phase.
- **`--quiet` / `--verbose` flag handling for SAFE-03's warning** — D-13 explicitly skips this. A unified warning-output layer with consistent quiet/verbose gating across the whole CLI is post-v0.8 polish.
- **Windows support** — Constraint violation per PROJECT.md. `arboard` happens to support Windows, but `std::os::unix::fs::symlink` elsewhere rules it out. Revisiting is a milestone-sized change, not an in-phase concern.
- **`DetailAction::Disable`/`Enable` machine.toml wiring** — Currently a stub (`app.rs:216-220`). Real implementation requires passing machine.toml handle into browse. Out of scope; SAFE-02 narrowly covers `ViewSource` + `CopyPath` per #414.
- **Cross-machine portability (PORT-01..04)** — Already deferred to v0.9 per epic #459.
- **Snapshot-testing the new `⚠` summary line** — D-18 rules this out. Substring assertions follow Phase 5 D-09 precedent.
- **A `theme.warning` color field** — D-12 leaves this as discretion. If reusing existing fields looks wrong, add it during implementation; otherwise leave for a future theming pass.
- **Refactoring `lib.rs::Command::Remove`'s 50-line body** — The handler has grown (save config + save manifest + regen lockfile + print). Extracting helpers is tempting but out of scope; D-04's wiring is purely additive to the existing shape.

</deferred>

---

*Phase: 08-safety-refactors-partial-failure-visibility-cross-platform*
*Context gathered: 2026-04-24*
