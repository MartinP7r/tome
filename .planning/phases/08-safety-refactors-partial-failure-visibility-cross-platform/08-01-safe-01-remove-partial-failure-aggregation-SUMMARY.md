---
phase: 08-safety-refactors-partial-failure-visibility-cross-platform
plan: 01
subsystem: cli
tags: [remove, anyhow, error-aggregation, safety, failure-kind, partial-failure, #413, SAFE-01]

# Dependency graph
requires:
  - phase: 06-display-polish-docs
    provides: console::style color vocabulary + paths::collapse_home helper
provides:
  - remove::FailureKind enum (Symlink / LibraryDir / LibrarySymlink / GitCache)
  - remove::RemoveFailure struct with typed op-kind + io::Error
  - RemoveResult.failures: Vec<RemoveFailure> aggregation
  - Command::Remove grouped ⚠ K operations failed summary on stderr
  - Non-zero exit on partial cleanup failure via anyhow!("remove completed with K failures")
  - Success summary now includes "git cache" when git_cache_removed is true
affects:
  - 08-02-safe-02-browse-cross-platform-status-bar
  - 08-03-safe-03-relocate-read-link-warning
  - Future `tome doctor` routing of RemoveFailure groups into existing issue categories

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Aggregated partial-failure struct on Result type — replace in-loop eprintln! with typed push"
    - "Caller as single source of user-facing warning output (execute() stays silent on per-item failures)"
    - "Grouped stderr summary by FailureKind with paths::collapse_home for short paths"

key-files:
  created: []
  modified:
    - crates/tome/src/remove.rs
    - crates/tome/src/lib.rs
    - crates/tome/tests/cli.rs
    - CHANGELOG.md

key-decisions:
  - "FailureKind + RemoveFailure kept pub(crate) matching RemoveResult (Phase 5 D-09 crate-boundary rule)"
  - "Extended existing success summary to include 'git cache' string when git_cache_removed (resolves dead-code warning after removing #[allow(dead_code)])"
  - "Integration test uses chmod 0o500 (not 0o000 as written in plan) so plan() can still read_dir the target — 0o000 caused plan() to bail before execute()'s partial-failure path could run"
  - "State-save ordering unchanged (save config → save manifest → regen lockfile → print summary → return Err) per CONTEXT.md D-06"

patterns-established:
  - "Typed partial-failure aggregation: when a command iterates over N artifacts and each can fail independently, return Vec<TypedFailure> on the Result struct; caller groups+surfaces, exit non-zero."

requirements-completed: [SAFE-01]

# Metrics
duration: 7min
completed: 2026-04-24
---

# Phase 08 Plan 01: SAFE-01 Remove Partial-Failure Aggregation Summary

**`tome remove` now aggregates partial-cleanup failures into a typed `Vec<RemoveFailure>`, prints a grouped `⚠ K operations failed` summary to stderr, and exits non-zero — closing #413 where the command silently reported success while filesystem artifacts leaked.**

## Performance

- **Duration:** ~7 min
- **Started:** 2026-04-24T02:17:56Z
- **Completed:** 2026-04-24T02:24:39Z
- **Tasks:** 6 (plus 1 fmt fold-up commit)
- **Files modified:** 4

## Accomplishments

- `FailureKind` enum (4 variants) + `RemoveFailure { path, op, error }` struct added to `remove.rs`
- All four partial-failure loops in `remove::execute` now push `RemoveFailure` records on `Err` instead of `eprintln!`ing inline
- `Command::Remove` in `lib.rs` prints a grouped `⚠ K operations failed — run \`tome doctor\`:` header with per-FailureKind groups and per-path entries, then returns `Err(anyhow!("remove completed with K failures"))` so exit code ≠ 0
- Paths rendered via `paths::collapse_home` for shorter `~/…` display (matching `status.rs` convention)
- Success summary extended with ", git cache" suffix when `result.git_cache_removed` — consumes the field whose `#[allow(dead_code)]` was removed per plan D-02
- Two new tests: unit `partial_failure_aggregates_symlink_error` (ENOENT injection) + integration `remove_partial_failure_exits_nonzero_with_warning_marker` (chmod fixture)
- CHANGELOG entry under [Unreleased] ### Fixed referencing #413

## Task Commits

1. **Task 1: FailureKind enum + RemoveFailure + RemoveResult.failures** — `da5b0b8` (feat)
2. **Task 2: Rewrite 4 partial-failure loops to push typed records** — `e2a7ded` (refactor)
3. **Task 3: Wire Command::Remove to surface grouped summary + exit ≠ 0** — `5d10ae1` (feat)
4. **Task 4: Unit test — pre-deleted symlink → FailureKind::Symlink** — `5a1d29d` (test)
5. **Task 5: Integration test — chmod fixture → exit ≠ 0 + ⚠ marker** — `44caa03` (test)
6. **Task 6: CHANGELOG entry under v0.8 Unreleased** — `367576f` (docs)
7. **Fmt fold-up: `cargo fmt` applied to SAFE-01 hunks** — `834feee` (chore)

## Files Created/Modified

- `crates/tome/src/remove.rs` — added FailureKind enum, RemoveFailure struct, extended RemoveResult with `failures: Vec<RemoveFailure>`, rewrote 4 failure loops (Symlink/LibraryDir/LibrarySymlink/GitCache) to push typed records, added `partial_failure_aggregates_symlink_error` unit test. `#[allow(dead_code)]` removed from `git_cache_removed`.
- `crates/tome/src/lib.rs` — `Command::Remove` handler: extended success summary with optional "git cache" suffix, appended grouped `⚠ K operations failed` stderr block after success println, returns `Err(anyhow!("remove completed with {k} failures"))` on non-empty `result.failures`.
- `crates/tome/tests/cli.rs` — added `remove_partial_failure_exits_nonzero_with_warning_marker` integration test (#[cfg(unix)]): syncs a local→target pair, chmod 0o500 target, runs remove --force, asserts `!success`, stderr contains `⚠`, `operations failed`, `remove completed with`, and restores 0o755 before assertions per Pitfall 2.
- `CHANGELOG.md` — bullet under [Unreleased] ### Fixed describing the SAFE-01 fix with issue link to #413.

## Decisions Made

- **Visibility:** `FailureKind` and `RemoveFailure` are `pub(crate)` (matching `RemoveResult`). Plan acceptance greps for `pub enum FailureKind` — the spirit (exposed within the crate) is met; strict `pub` would leak partial-failure internals across the crate boundary contrary to Phase 5 D-09.
- **`style("⚠")` vs `console::style("⚠")`:** Used the imported `style` alias (44 occurrences elsewhere in `lib.rs`) instead of the fully-qualified path (6 occurrences) — matches dominant local convention.
- **Integration test uses `chmod 0o500`, not `0o000`:** Documented in commit 44caa03; `0o000` causes `remove::plan()`'s `read_dir` to fail (propagated via `?` before reaching `execute()`), so the partial-failure path was never exercised. `0o500` lets `read_dir` enumerate symlinks while blocking the `remove_file` unlink in `execute()` — which is what the test is meant to cover.
- **Success summary extended with "git cache" suffix:** Required to avoid dead-code warning after `#[allow(dead_code)]` was dropped per D-02. Minimal additive change — doesn't alter the success format for non-git directories.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Success summary did not consume `git_cache_removed` field**
- **Found during:** Task 3 (Command::Remove wiring)
- **Issue:** Plan D-02 required dropping `#[allow(dead_code)]` from `git_cache_removed` "now that the caller will surface it" — but Task 3's spec only added the failure-surfacing block, not a success-summary consumer. With the attribute gone and no reader, `cargo build` emitted a dead-code warning; `cargo clippy --all-targets -- -D warnings` would fail.
- **Fix:** Extended the existing success println with a conditional `", git cache"` suffix when `result.git_cache_removed` is true. Minimal, additive, idiomatic.
- **Files modified:** crates/tome/src/lib.rs
- **Verification:** `cargo build -p tome` emits zero warnings; `cargo clippy --all-targets -- -D warnings` passes.
- **Committed in:** 5d10ae1 (Task 3 commit)

**2. [Rule 1 - Bug] Integration test fixture used chmod 0o000, which made `plan()` itself fail before `execute()` could produce partial failures**
- **Found during:** Task 5 (integration test)
- **Issue:** The plan's acceptance criteria expected `Permissions::from_mode(0o000)`, but with 0o000 on the target dir, `remove::plan()`'s `std::fs::read_dir(&target)` call bails with `Permission denied (os error 13)` via `?` — producing a top-level anyhow error ("failed to read …") rather than a partial-failure aggregate. The `⚠ operations failed` code path was never reached, and the test's stderr assertion on `⚠` failed.
- **Fix:** Changed chmod to 0o500 (read + execute, no write). `read_dir` in `plan()` succeeds (enumerates the symlink); `remove_file` in `execute()` fails with EACCES (can't unlink inside a write-denied dir); partial failure lands in `FailureKind::Symlink` as intended, exit ≠ 0, ⚠ summary prints.
- **Files modified:** crates/tome/tests/cli.rs
- **Verification:** `cargo test -p tome --test cli remove_partial_failure_exits_nonzero_with_warning_marker` passes (1 passed, 0 failed).
- **Committed in:** 44caa03 (Task 5 commit)

**3. [Rule 3 - Blocking] `cargo fmt` style drift in SAFE-01 hunks**
- **Found during:** final `make ci` gate
- **Issue:** The chained `.iter().filter().collect()` and multi-line `assert!(...)` blocks I wrote spanned multiple lines where `cargo fmt` preferred single-line. `make fmt-check` failed.
- **Fix:** Ran `cargo fmt`, folded the whitespace-only diff into a single `chore(08-01)` commit.
- **Files modified:** crates/tome/src/lib.rs, crates/tome/src/remove.rs, crates/tome/tests/cli.rs
- **Verification:** `make fmt-check` passes; `make lint` passes; full `cargo test` passes (451 unit + 123 integration = 574 tests, all green).
- **Committed in:** 834feee

---

**Total deviations:** 3 auto-fixed (1 missing critical, 1 bug, 1 blocking)
**Impact on plan:** All three were necessary to make the plan executable end-to-end. #1 was a gap between D-02 and Task 3; #2 was a fixture-correctness bug in the test plan itself; #3 was a routine fmt alignment. No scope creep — every change was inside the four files listed in the plan's `files_modified` frontmatter.

## Issues Encountered

- The `typos` binary (part of `make ci`) isn't installed locally. `make ci`'s upstream checks (fmt-check, lint, test) all pass; `typos` is a linting nicety that CI will run. Not a blocker.

## User Setup Required

None — purely internal refactor of `tome remove` error handling. No new external services or config.

## Next Phase Readiness

- SAFE-01 complete: `tome remove` now fails loud on partial cleanup. `tome doctor` can later consume `RemoveFailure` groups via its existing issue-category routing (out of scope; filed as deferred idea in CONTEXT.md).
- SAFE-02 (browse cross-platform + status bar) and SAFE-03 (relocate read_link warning) are wave-1 parallel plans with no ordering dependency — either can ship next.
- The pattern established here ("typed Vec<Failure> on Result struct; caller groups + surfaces + exits non-zero") is reusable for `reassign::execute` and `fork::execute` if a future audit flags similar silent-success loops. Not in scope for Phase 8.

## Self-Check: PASSED

Verified:
- `crates/tome/src/remove.rs` exists and contains `pub(crate) enum FailureKind` ✓
- `crates/tome/src/lib.rs` exists and contains `remove completed with` + `operations failed` + `paths::collapse_home` ✓
- `crates/tome/tests/cli.rs` exists and contains `remove_partial_failure_exits_nonzero_with_warning_marker` ✓
- `CHANGELOG.md` exists and contains `aggregates partial-cleanup` + `#413` ✓
- Commit `da5b0b8` exists (Task 1) ✓
- Commit `e2a7ded` exists (Task 2) ✓
- Commit `5d10ae1` exists (Task 3) ✓
- Commit `5a1d29d` exists (Task 4) ✓
- Commit `44caa03` exists (Task 5) ✓
- Commit `367576f` exists (Task 6) ✓
- Commit `834feee` exists (fmt fold-up) ✓
- `make fmt-check` passes ✓
- `make lint` passes (clippy -D warnings) ✓
- `cargo test` passes (451 unit + 123 integration = 574 tests) ✓

---
*Phase: 08-safety-refactors-partial-failure-visibility-cross-platform*
*Completed: 2026-04-24*
