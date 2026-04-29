---
phase: 10-phase-8-review-tail
plan: 02
subsystem: testing
tags: [remove, partial-failure, test-coverage, debug-assert, compile-time-guard, regression-test]

# Dependency graph
requires:
  - phase: 08-cli-safety-refactors
    provides: I2/I3 retention contract on `tome remove` partial-failure path
provides:
  - Compile-time drift guard for `FailureKind::ALL` (POLISH-04 option c)
  - Debug-only `path.is_absolute()` invariant on `RemoveFailure::new` (POLISH-05 option a)
  - Deferred `regen_warnings` ordering on happy-path `tome remove` (TEST-04 option a)
  - Success-banner-absence assertion on partial-failure path (TEST-01)
  - End-to-end retry-after-fix integration test (TEST-02)
  - Source-byte ordering regression test anchored to `Command::Remove` region (TEST-04 regression)
affects: [10-03-arboard-pin-and-relocate-deadcode, future-remove-refactors]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Exhaustive-match sentinel + const_assert for enum-array drift-proofing"
    - "debug_assert! constructor invariants on shared-construction newtypes"
    - "Source-byte regression tests anchored to handler-region for false-positive resistance"

key-files:
  created: []
  modified:
    - "crates/tome/src/remove.rs (compile-time sentinel + const-assert + debug_assert + 4 unit tests)"
    - "crates/tome/src/lib.rs (Command::Remove happy-path: success banner before regen_warnings)"
    - "crates/tome/tests/cli.rs (banner-absence asserts + retry e2e test + source-order regression)"

key-decisions:
  - "POLISH-04 option c (exhaustive-match sentinel) — no new dependency; smaller blast-radius than strum::EnumIter (option a)"
  - "POLISH-05 option a (keep new() + add debug_assert!) — smaller blast-radius than replacing 4 call sites (option b)"
  - "TEST-04 option a (defer warnings until after success banner) — banner is the user's anchor; option b (`[lockfile regen]` prefix) adds visual noise on the happy path"
  - "Source-order regression test anchored to `Command::Remove` region via `lib_rs.find(...)` — prevents false-positive failures when unrelated handlers (Reassign / Fork) reorder their own regen_warnings loops"

patterns-established:
  - "Compile-time enum-array drift guard: `_ensure_kind_all_exhaustive` const fn + `const _: () = { assert!(ALL.len() == N); }` block"
  - "Anchored source-byte regression tests: locate handler-region first, then offset all `find()` calls from there"

requirements-completed: [POLISH-04, POLISH-05, TEST-01, TEST-02, TEST-04]

# Metrics
duration: 11min
completed: 2026-04-29
---

# Phase 10 Plan 02: Remove Correctness & Test Coverage Summary

**Compile-enforced `FailureKind::ALL` exhaustiveness via const-eval sentinel, debug-only absolute-path invariant on `RemoveFailure::new`, deferred regen_warnings ordering on `tome remove`, plus banner-absence + retry-after-fix end-to-end coverage closing the v0.8 review tail.**

## Performance

- **Duration:** 11 min
- **Started:** 2026-04-29T02:48:30Z
- **Completed:** 2026-04-29T02:59:53Z
- **Tasks:** 4 (+ 1 fmt fixup)
- **Files modified:** 3

## Accomplishments

- `FailureKind::ALL` cannot drift from the enum: `_ensure_failure_kind_all_exhaustive` const fn + `const _: () = { assert!(FailureKind::ALL.len() == 4); };` block compile-enforce that adding a variant without growing `ALL` (or vice versa) fails to build. Drift-guard manually verified by adding a temporary `Bogus` variant and observing E0004 ("non-exhaustive patterns") plus revert.
- `RemoveFailure::new` carries `debug_assert!(path.is_absolute(), "RemoveFailure::path must be absolute, got: {}", path.display())`. Zero release-build cost; existing `partial_failure_aggregates_*` tests confirm all four `execute()` call sites pass absolute paths today.
- `tome remove` happy-path: green `✓ Removed directory` success banner now appears BEFORE the `for w in &regen_warnings { eprintln!("warning: ...") }` loop. The banner is the user's anchor; multi-warning regen no longer buries it.
- `remove_partial_failure_exits_nonzero_with_warning_marker` gains stdout-AND-stderr banner-absence assertions on the partial-failure path.
- New `remove_retry_succeeds_after_failure_resolved` exercises the full I2/I3 retention contract end-to-end: chmod 0o500 → first `tome remove` fails (config + manifest preserved) → chmod 0o755 → second `tome remove` succeeds with empty failures, config entry gone, manifest empty, library dir gone.
- New `lib_rs_remove_handler_prints_success_banner_before_regen_warnings` pins the source-byte ordering, anchored to `Command::Remove` so unrelated reorders of `Command::Reassign` / `Command::Fork` (each with its own regen_warnings loop) cannot produce false positives.

## Task Commits

Each task was committed atomically with `--no-verify` (parallel wave protocol):

1. **Task 1: POLISH-04 + POLISH-05 (drift-proof + invariant)** — `a38fba6` (feat)
2. **Task 2: TEST-04 reorder (deferred warnings)** — `5c45a5c` (fix)
3. **Task 3: TEST-01 banner-absence asserts** — `5b4e2c4` (test)
4. **Task 4: TEST-02 retry e2e + TEST-04 regression test** — `39535ab` (test)
5. **Fmt fixup on Task 1 tests** — `14e5a93` (style) — `cargo fmt` collapsed a 4-line `assert_ne!` to 1 line; committed as a separate fixup per parallel-wave protocol (no amending)

## Files Created/Modified

- `crates/tome/src/remove.rs` — Added `_ensure_failure_kind_all_exhaustive` const fn + `const _: () = { assert!(FailureKind::ALL.len() == 4); };` block. Added `debug_assert!(path.is_absolute(), ...)` to `RemoveFailure::new`. Added 4 unit tests: `failure_kind_all_length_matches_variant_count`, `failure_kind_all_ordering_pinned`, `remove_failure_new_relative_path_panics_in_debug`, `remove_failure_new_absolute_path_succeeds`.
- `crates/tome/src/lib.rs` — Reordered `Command::Remove` happy-path: success banner `println!` now precedes `for w in &regen_warnings { eprintln!(...) }` loop. Added comment block citing TEST-04 option a + the regression test name.
- `crates/tome/tests/cli.rs` — Extended `remove_partial_failure_exits_nonzero_with_warning_marker` with stdout/stderr `Removed directory` absence asserts. Added `remove_retry_succeeds_after_failure_resolved` (e2e retry). Added `lib_rs_remove_handler_prints_success_banner_before_regen_warnings` (source-order regression, anchored to `Command::Remove` region).

## Decisions Made

- **POLISH-04 option c (exhaustive-match sentinel)** chosen over (a) `strum::EnumIter`. No new dependency; smaller blast-radius. The `const _: () = { assert!(...) };` block additionally catches the symmetric drift (match arm added without growing ALL).
- **POLISH-05 option a (keep new() + add debug_assert!)** chosen over (b) replacing 4 call sites with struct literals. Option a is a single-site edit; option b would touch 4 production call sites in `execute()`.
- **TEST-04 option a (defer warnings)** chosen over (b) `[lockfile regen]` prefix. Option a is a pure reorder (zero rendered-output change on the no-warnings happy path — which is the common case); option b adds visual noise on every line even when there are no warnings to surface.
- **Source-order regression test anchoring** — anchor `String::find()` calls to `Command::Remove` region first because `lib.rs` contains three `for w in &regen_warnings` loops (Remove, Reassign, Fork). Without anchoring, a future reorder of unrelated handlers would create a false-positive failure unrelated to the Remove ordering contract.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Switched `BTreeSet` to hand-rolled pairwise check in `failure_kind_all_length_matches_variant_count`**
- **Found during:** Task 1 RED (initial test compilation)
- **Issue:** Plan body suggested `BTreeSet<FailureKind>` for uniqueness check, but `FailureKind` only derives `(Debug, Clone, Copy, PartialEq, Eq)` — no `Ord`/`Hash`. `BTreeSet::contains` requires `Ord`.
- **Fix:** Replaced `BTreeSet` with a pairwise `assert_ne!` loop (uses only `PartialEq`, which is already derived). Equivalent semantics — pins uniqueness and length. Adding `Ord`/`Hash` to `FailureKind` would be unnecessary surface-area expansion for one test.
- **Files modified:** `crates/tome/src/remove.rs` (test only)
- **Verification:** `cargo test -p tome --lib remove::tests::failure_kind_all_length_matches_variant_count` passes
- **Committed in:** `a38fba6`

**2. [Rule 1 - Style] cargo fmt collapsed `assert_ne!` invocation in Task 1 tests**
- **Found during:** Final `make ci` run after all 4 tasks
- **Issue:** `cargo fmt -- --check` failed because `assert_ne!(a, b, "...")` was written across 4 lines; rustfmt prefers single-line for short args.
- **Fix:** Ran `cargo fmt`, committed as a separate `style(10-02)` commit (parallel-wave protocol forbids `--amend`).
- **Files modified:** `crates/tome/src/remove.rs`
- **Verification:** `cargo fmt -- --check` clean
- **Committed in:** `14e5a93`

**Total deviations:** 2 auto-fixed (1 trait-bound bug, 1 style/fmt)
**Impact on plan:** Both deviations were minor. The `BTreeSet` → pairwise switch preserves the test's semantics with smaller dependency surface. The fmt fixup is purely cosmetic.

## Issues Encountered

- **Parallel-wave shared working tree:** During Task 1 RED phase, `cargo test` failed because executor 10-01 had uncommitted in-flight changes to `browse/app.rs` (renaming `StatusMessage.text` to a getter). This is inherent to parallel waves — all executors share the working tree. Resolved by proceeding with source-byte verification (`rg`) until 10-01 committed their changes; final `make ci` after all parallel work converged passes cleanly.
- **One transient flake:** First `make ci` showed `remove_preserves_git_lockfile_entries` failing once (lockfile lacked `git_commit_sha`). Test passes in isolation and on second full-suite run — likely test-parallelism interference with another git-touching test.

## Verification Summary

```
make ci
  cargo fmt -- --check        ✓ clean
  cargo clippy -D warnings    ✓ clean
  cargo test (lib)            ✓ 524 passed
  cargo test (cli)            ✓ 136 passed
  typos                       ✓ All checks passed
```

Drift-guard manual verification: temporarily added `FailureKind::Bogus` → `cargo build -p tome` failed with `E0004: non-exhaustive patterns` pointing at `_ensure_failure_kind_all_exhaustive`. Reverted; build clean.

POLISH-04, POLISH-05, TEST-01, TEST-02, TEST-04 — all closed.

## Self-Check: PASSED

- [x] `crates/tome/src/remove.rs` modified (verified via `git log -1 --stat a38fba6` and `14e5a93`)
- [x] `crates/tome/src/lib.rs` modified (verified via `5c45a5c`)
- [x] `crates/tome/tests/cli.rs` modified (verified via `5b4e2c4` and `39535ab`)
- [x] Commit `a38fba6` exists (Task 1 — POLISH-04 + POLISH-05)
- [x] Commit `5c45a5c` exists (Task 2 — TEST-04 reorder)
- [x] Commit `5b4e2c4` exists (Task 3 — TEST-01 banner-absence)
- [x] Commit `39535ab` exists (Task 4 — TEST-02 retry + TEST-04 regression)
- [x] Commit `14e5a93` exists (fmt fixup)
- [x] All new tests pass: `failure_kind_all_length_matches_variant_count`, `failure_kind_all_ordering_pinned`, `remove_failure_new_relative_path_panics_in_debug`, `remove_failure_new_absolute_path_succeeds`, `remove_retry_succeeds_after_failure_resolved`, `lib_rs_remove_handler_prints_success_banner_before_regen_warnings`
- [x] Existing regression tests still pass: `partial_failure_aggregates_symlink_error`, `partial_failure_aggregates_multiple_kinds`, `remove_partial_failure_does_not_save_disk_state`, `remove_failure_summary_wording`
- [x] `make ci` passes (524 lib + 136 cli + fmt + clippy + typos)

## Next Phase Readiness

Plan 10-03 (arboard pin + relocate deadcode) is independent and already in flight. After all three plans land, Phase 10 verification should run `/gsd:verify-work 10` to close the phase against POLISH-01..06 + TEST-01..05. The remaining 6 review-tail items (POLISH-01, POLISH-02, POLISH-03, POLISH-06, TEST-03, TEST-05) are owned by 10-01 and 10-03.

---
*Phase: 10-phase-8-review-tail*
*Completed: 2026-04-29*
