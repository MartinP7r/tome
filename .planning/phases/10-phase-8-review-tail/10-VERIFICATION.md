---
phase: 10-phase-8-review-tail
verified: 2026-04-26T00:00:00Z
status: passed
score: 11/11 must-haves verified
re_verification:
  previous_status: none
  previous_score: n/a
  gaps_closed: []
  gaps_remaining: []
  regressions: []
---

# Phase 10: Phase 8 Review Tail — Type Design, TUI Polish & Test Coverage — Verification Report

**Phase Goal:** Close the 11 post-merge review items from #462 (P1-P5) and #463 (D1-D6) so the v0.8 review tail is fully cleared in one cut.

**Verified:** 2026-04-26
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (per requirement-id contract)

| #   | Requirement | Truth                                                                                                       | Status     | Evidence                                                                                                                                                                                                       |
| --- | ----------- | ----------------------------------------------------------------------------------------------------------- | ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | POLISH-01   | `tome browse` ViewSource paints `Pending("Opening: <path>...")` BEFORE `xdg-open`/`open` blocks; keystrokes during the block are drained afterward. | VERIFIED   | `app.rs:473` sets `StatusMessage::Pending(format!("Opening: ..."))` then `redraw(self)` BEFORE `Command::status()`. `app.rs:500` `drain_pending_events()`. Closure threaded through `handle_key:247` → `handle_detail_key:306` → `execute_action_with_redraw:434` → `handle_view_source:450`. |
| 2   | POLISH-02   | `StatusMessage` is `pub(super)` enum (`Success(String) \| Warning(String) \| Pending(String)`) with `body()`/`glyph()`/`severity()` accessors; UI formats `"{glyph} {body}"` at render time. | VERIFIED   | `app.rs:43-77` defines `pub(super) enum StatusMessage` + `pub(super) enum StatusSeverity` with all three accessors. UI `ui.rs` consumes `msg.glyph()`/`msg.body()`/`msg.severity()`. Test `status_message_body_does_not_contain_glyph` (passing) pins the invariant. |
| 3   | POLISH-03   | `ClipboardOccupied` errors auto-retry once with 100ms backoff before any warning surfaces.                  | VERIFIED   | `app.rs:127-140` `try_clipboard_set_text_with_retry` matches `ClipboardOccupied` arm, sleeps 100ms, retries once. Test `copy_path_retry_helper_returns_within_bound` (passing) verifies wall-clock bound.        |
| 4   | TEST-03     | `status_message_from_open_result(...)` factored as `pub(super)` helper with 3 unit tests via `ExitStatusExt::from_raw`. | VERIFIED   | `app.rs:91` `pub(super) fn status_message_from_open_result(...)`. Tests at `app.rs:1269` (Ok+success), `app.rs:1289` (Ok+nonzero), `app.rs:1308` (Err) — all passing. Used by `handle_view_source` (browse module).        |
| 5   | POLISH-04   | `FailureKind::ALL` compile-enforced via exhaustive-match sentinel + `const _: () = { assert!(...len() == 4); };`. | VERIFIED   | `remove.rs:103` `_ensure_failure_kind_all_exhaustive` const fn with 4-arm exhaustive match. `remove.rs:112-117` `const _: () = { assert!(FailureKind::ALL.len() == 4); };`. Tests `failure_kind_all_length_matches_variant_count` + `failure_kind_all_ordering_pinned` pass.    |
| 6   | POLISH-05   | `RemoveFailure::new` body has `debug_assert!(path.is_absolute(), ...)`.                                     | VERIFIED   | `remove.rs:140-147` `pub(crate) fn new(...)` with `debug_assert!(path.is_absolute(), "RemoveFailure::path must be absolute, got: {}", path.display())`. Tests `remove_failure_new_relative_path_panics_in_debug` + `remove_failure_new_absolute_path_succeeds` pass.    |
| 7   | TEST-01     | `remove_partial_failure_exits_nonzero_with_warning_marker` asserts `Removed directory` is **absent** from stdout AND stderr on partial failure. | VERIFIED   | `tests/cli.rs:3467-3474` two banner-absence asserts (stdout AND stderr) with comment citing TEST-01 / P1 contract. Test passes.                                                                                |
| 8   | TEST-02     | End-to-end retry-after-fix test: partial failure → fix → second `tome remove` succeeds with empty failures, config gone, manifest empty, library dir gone. | VERIFIED   | `tests/cli.rs:3586-3710` `remove_retry_succeeds_after_failure_resolved` exercises chmod 0o500 → fail → 0o755 → succeed with all four post-conditions checked (success status, banner present, config entry gone, manifest skill gone, library dir gone). Test passes.                       |
| 9   | TEST-04     | `regen_warnings` deferred until AFTER success banner; source-byte regression test anchored to `Command::Remove` region. | VERIFIED   | `lib.rs:471-485` banner `println!` precedes `for w in &regen_warnings { eprintln!(...) }` loop inside `Command::Remove` block. `tests/cli.rs:3713-3757` `lib_rs_remove_handler_prints_success_banner_before_regen_warnings` anchors `String::find()` to `region_start` (Command::Remove) per TEST-04 anchoring contract. Test passes. |
| 10  | POLISH-06   | `arboard` pinned to `>=3.6, <3.7` in `Cargo.toml` with bump-review comment.                                 | VERIFIED   | `Cargo.toml:13-22` workspace dep `arboard = { version = ">=3.6, <3.7", ... }` with 7-line bump-review comment citing CHANGELOG audit, exhaustive match-arm requirement, and #463 / POLISH-06.                            |
| 11  | TEST-05     | `SkillMoveEntry.source_path` removed; `#[allow(dead_code)]` gone; `provenance_from_link_result` retained for SAFE-03 stderr side-effect. | VERIFIED   | `rg "source_path" relocate.rs` returns 0. `rg "#\[allow\(dead_code\)\]" relocate.rs` returns 0. `relocate.rs:98` `let _ = provenance_from_link_result(...)` retained for stderr side-effect. Test `provenance_from_link_result_warns_and_returns_none_on_err` (passing) regression-guards SAFE-03.    |

**Score:** 11/11 truths verified

### Required Artifacts (Level 1+2+3 verification)

| Artifact                                                              | Expected                                                                              | Status     | Details                                                                                                |
| --------------------------------------------------------------------- | ------------------------------------------------------------------------------------- | ---------- | ------------------------------------------------------------------------------------------------------ |
| `crates/tome/src/browse/app.rs`                                       | `StatusMessage` enum, `status_message_from_open_result`, `try_clipboard_set_text_with_retry`, `execute_action_with_redraw`, `handle_view_source`, `drain_pending_events`, redraw closure params | VERIFIED   | All symbols present; widely used (line counts confirm substantive). 56 browse tests pass.              |
| `crates/tome/src/browse/ui.rs`                                        | `msg.glyph()`/`msg.body()`/`msg.severity()` accessors; `ui::render(&App)`; `body_height_for_area` helper | VERIFIED   | Wired correctly — `mod.rs::run_loop` calls `body_height_for_area(area)`. Render takes `&App`.        |
| `crates/tome/src/browse/mod.rs`                                       | `run_loop` constructs redraw closure, passes to `app.handle_key(...)`                  | VERIFIED   | Closure construction matches plan exactly; `Pending("Opening: ...")` flow wired end-to-end.            |
| `crates/tome/src/remove.rs`                                           | `_ensure_failure_kind_all_exhaustive` const fn, `const _: () = { assert!(...) };` block, `RemoveFailure::new` debug_assert | VERIFIED   | All present (lines 103, 112-117, 140-147). 10 remove::tests pass including 4 new ones.                  |
| `crates/tome/src/lib.rs`                                              | `Command::Remove` block: success banner BEFORE `for w in &regen_warnings` loop         | VERIFIED   | Banner at line 471, regen warnings at line 483. Source-byte test passes.                                |
| `crates/tome/tests/cli.rs`                                            | TEST-01 banner-absence asserts, `remove_retry_succeeds_after_failure_resolved`, `lib_rs_remove_handler_prints_success_banner_before_regen_warnings` | VERIFIED   | All three test bodies verified; 136 cli tests pass.                                                    |
| `Cargo.toml`                                                          | `arboard = ">=3.6, <3.7"` workspace dep with bump-review comment                       | VERIFIED   | Lines 13-22 match exactly.                                                                             |
| `crates/tome/src/relocate.rs`                                         | `source_path` field gone; `#[allow(dead_code)]` gone; `provenance_from_link_result` retained for stderr side-effect | VERIFIED   | All three checks pass; 12 relocate::tests pass.                                                        |

### Key Link Verification

| From                                | To                                              | Via                                          | Status | Details                                                                                                                  |
| ----------------------------------- | ----------------------------------------------- | -------------------------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------ |
| `mod.rs::run_loop`                  | `app.handle_key`                                | redraw closure construction + arg            | WIRED  | Closure built as `let mut redraw = \|a: &App\| { let _ = terminal.draw(\|frame\| ui::render(frame, a)); };` and passed.    |
| `handle_key`                        | `handle_view_source`                            | `handle_detail_key` → `execute_action_with_redraw` → `handle_view_source` | WIRED  | Verified by `view_source_invokes_redraw_callback_for_pending_status` test (passing).                                     |
| `handle_view_source`                | `Pending("Opening: ...")` → `Command::status()` | `redraw(self)` between status set and block  | WIRED  | Source-grep confirms ordering.                                                                                           |
| `handle_view_source`                | `drain_pending_events()`                        | post-block call                              | WIRED  | Source line `app.rs:500` shows method definition; called after `.status()` returns.                                     |
| `execute_action::CopyPath`          | `try_clipboard_set_text_with_retry`             | direct call                                  | WIRED  | Single source-of-truth helper at `app.rs:127`; matched against `arboard::Error` variants.                              |
| `lib.rs::Command::Remove`           | success banner → regen_warnings loop            | sequential code                              | WIRED  | Banner at line 471, loop at line 483 (banner-first).                                                                     |
| `relocate::plan`                    | `provenance_from_link_result`                   | `let _ = provenance_from_link_result(...)`   | WIRED  | Retained for stderr side-effect; SAFE-03 contract preserved.                                                             |

### Data-Flow Trace (Level 4)

| Artifact                          | Data Variable          | Source                                                                  | Produces Real Data | Status   |
| --------------------------------- | ---------------------- | ----------------------------------------------------------------------- | ------------------ | -------- |
| `StatusMessage::Pending`          | `app.status_message`   | Set by `handle_view_source` from real `path` arg of selected skill      | Yes                | FLOWING  |
| `lib.rs` success banner           | `result.library_entries_removed`, `result.symlinks_removed`, `result.git_cache_removed` | Real `RemoveResult` from `remove::execute()`                            | Yes                | FLOWING  |
| `RemoveFailure::path`             | path field             | Real PathBuf from execute() call sites (always absolute, debug-asserted) | Yes                | FLOWING  |
| Retry test banner assertion       | second_stdout          | Real `tome remove` invocation                                            | Yes                | FLOWING  |

### Behavioral Spot-Checks

| Behavior                                       | Command                                                                                              | Result                          | Status |
| ---------------------------------------------- | ---------------------------------------------------------------------------------------------------- | ------------------------------- | ------ |
| Browse module compiles & all tests pass        | `cargo test -p tome --lib browse::`                                                                  | 56 passed; 0 failed             | PASS   |
| Remove module compiles & all tests pass        | `cargo test -p tome --lib remove::`                                                                  | 10 passed; 0 failed             | PASS   |
| Relocate module compiles & all tests pass      | `cargo test -p tome --lib relocate::`                                                                | 12 passed; 0 failed             | PASS   |
| TEST-01 banner-absence assertions pass         | `cargo test -p tome --test cli remove_partial_failure`                                               | 2 passed; 0 failed              | PASS   |
| TEST-02 end-to-end retry test passes           | `cargo test -p tome --test cli remove_retry_succeeds_after_failure_resolved`                         | 1 passed; 0 failed              | PASS   |
| TEST-04 source-byte ordering test passes       | `cargo test -p tome --test cli lib_rs_remove_handler_prints_success_banner_before_regen_warnings`    | 1 passed; 0 failed              | PASS   |
| `make ci` (fmt + clippy + test + typos) green  | `make ci`                                                                                            | 526 lib + 136 cli + 0 doc; clean | PASS   |

Note: One transient flake observed on first `make ci` run (`remove_preserves_git_lockfile_entries` failed once due to test-parallelism interference with another git-touching test, as documented in 10-02-SUMMARY.md). Subsequent runs are clean. Not a regression — pre-existing intermittent flake unrelated to phase 10 changes.

### Requirements Coverage

| Requirement | Source Plan | Description                                                                                                    | Status     | Evidence                                                                                                |
| ----------- | ----------- | -------------------------------------------------------------------------------------------------------------- | ---------- | ------------------------------------------------------------------------------------------------------- |
| POLISH-01   | 10-01       | `Pending("Opening: ...")` painted before block; tty events drained after.                                     | SATISFIED  | Truth #1 verified.                                                                                      |
| POLISH-02   | 10-01       | `StatusMessage` enum redesign with body/glyph/severity accessors, `pub(super)`.                                 | SATISFIED  | Truth #2 verified.                                                                                      |
| POLISH-03   | 10-01       | `ClipboardOccupied` retry once with 100ms backoff.                                                             | SATISFIED  | Truth #3 verified.                                                                                      |
| POLISH-04   | 10-02       | `FailureKind::ALL` compile-enforced exhaustive.                                                                | SATISFIED  | Truth #5 verified.                                                                                      |
| POLISH-05   | 10-02       | `RemoveFailure::new` debug_assert(path.is_absolute()).                                                          | SATISFIED  | Truth #6 verified.                                                                                      |
| POLISH-06   | 10-03       | `arboard` pinned to `>=3.6, <3.7` with bump-review comment.                                                    | SATISFIED  | Truth #10 verified.                                                                                     |
| TEST-01     | 10-02       | Banner-absence asserts on partial-failure path.                                                                | SATISFIED  | Truth #7 verified.                                                                                      |
| TEST-02     | 10-02       | End-to-end retry-after-fix test.                                                                                | SATISFIED  | Truth #8 verified.                                                                                      |
| TEST-03     | 10-01       | `status_message_from_open_result` helper + 3 unit tests.                                                        | SATISFIED  | Truth #4 verified.                                                                                      |
| TEST-04     | 10-02       | Deferred regen_warnings + source-byte regression test anchored to Command::Remove.                              | SATISFIED  | Truth #9 verified.                                                                                      |
| TEST-05     | 10-03       | `SkillMoveEntry.source_path` removed; `#[allow(dead_code)]` gone; SAFE-03 contract preserved.                   | SATISFIED  | Truth #11 verified.                                                                                     |

**Coverage:** 11/11 requirement IDs satisfied. No orphaned IDs (REQUIREMENTS.md Phase 10 column matches plan claims exactly: POLISH-01..06, TEST-01..05).

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |

None. Anti-pattern scan on `browse/app.rs`, `browse/ui.rs`, `browse/mod.rs`, `remove.rs`, `lib.rs`, `relocate.rs`, `Cargo.toml`, `tests/cli.rs` returned no TODO/FIXME/PLACEHOLDER stubs in phase 10's modified regions, no empty-return stubs, and no console.log-only handlers. The `let _ = provenance_from_link_result(...)` discard at `relocate.rs:98` is intentional (side-effect-only retention for SAFE-03 stderr warning) and is documented in the helper's doc comment, so it is NOT an anti-pattern.

### Human Verification Required

None. The borderline POLISH-01 keystroke-drain on a real Linux desktop is covered by the unit-testable `drain_pending_events()` helper at `app.rs:500` plus its `drain_pending_events_returns_when_queue_empty` test. No items routed to human verification.

### Gaps Summary

No gaps. All 11 must-haves verified end-to-end:

- **TUI polish (10-01):** StatusMessage enum, redraw threading, Pending pre-block paint, clipboard retry — all wired, tested, and behaviorally correct.
- **Remove correctness (10-02):** Compile-time drift guard for FailureKind::ALL, debug-only path-is-absolute invariant, deferred regen_warnings ordering — all enforced at compile-time or runtime, with regression tests.
- **Build hygiene + dead code (10-03):** arboard patch-pin with bump-review comment, SkillMoveEntry.source_path field deletion, SAFE-03 contract preserved via retained side-effect call — all verified.

Phase 10 cleanly closes the v0.8 review tail (#462 P1-P5 + #463 D1-D6).

---

_Verified: 2026-04-26_
_Verifier: Claude (gsd-verifier)_
