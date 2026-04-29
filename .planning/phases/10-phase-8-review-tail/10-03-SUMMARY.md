---
phase: 10-phase-8-review-tail
plan: 03
subsystem: build-hygiene + relocate
tags: [arboard, cargo-toml, dead-code, relocate, safe-03, polish-06, test-05]

requires:
  - phase: 08-safety-refactors
    provides: SAFE-03 stderr warning surface for unreadable managed symlinks (provenance_from_link_result)
provides:
  - "arboard workspace dependency pinned to >=3.6, <3.7 with documented bump-review policy"
  - "SkillMoveEntry no longer carries the dead source_path field"
  - "#[allow(dead_code)] removed from relocate.rs"
  - "SAFE-03 contract preserved by standalone unit test"
affects: [phase-11+]

tech-stack:
  added: []
  patterns:
    - "Patch-pin + bump-review-comment as silent-bump-prevention pattern (alternative to a cfg(test) enum-growth canary)"
    - "let _ = side_effecting_helper(...) for retained helpers whose return value has lost its consumer but whose stderr/log side-effect is the contract"

key-files:
  created:
    - .planning/phases/10-phase-8-review-tail/10-03-SUMMARY.md
  modified:
    - Cargo.toml
    - crates/tome/src/relocate.rs

key-decisions:
  - "POLISH-06 option (a) — patch-version pin (>=3.6, <3.7) with multi-line bump-review comment — chosen over (b) cfg(test) enum-growth canary. Simpler, more obvious, no test-runtime overhead."
  - "TEST-05 option (a) — REMOVE the SkillMoveEntry.source_path field — chosen over (b) wire-it. copy_library and recreate_target_symlinks already use direct read_link calls; wiring would be redundant."
  - "provenance_from_link_result retained (option β: side-effect-only call, return value discarded with `let _ = ...`). The stderr WARNING is the SAFE-03 contract; deleting the helper would regress SAFE-03."

patterns-established:
  - "Bump-review-comment pattern: when a dependency exposes a non-exhaustive error/event enum that we match against, pin to a patch range and document the audit trigger inline — silent variant additions become impossible without a CHANGELOG.md review."
  - "Side-effect-only retained helper: when a helper's return value loses its consumer but its stderr/log side effect IS the contract, keep the helper and discard with `let _ = ...`; document the dual purpose in the doc comment so future maintainers don't 'clean it up'."

requirements-completed: [POLISH-06, TEST-05]

duration: 6 min
completed: 2026-04-29
---

# Phase 10 Plan 03: arboard pin + relocate dead-code Summary

**Pinned `arboard` to `>=3.6, <3.7` with a bump-review comment in `Cargo.toml`, and removed the dead `SkillMoveEntry.source_path` field (plus three test-side assertions and `#[allow(dead_code)]`) from `crates/tome/src/relocate.rs` while preserving the SAFE-03 stderr warning surface.**

## Performance

- **Duration:** ~6 min
- **Started:** 2026-04-29T02:46Z
- **Completed:** 2026-04-29T02:52Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- POLISH-06 (D6) closed: `arboard` pinned to a patch-version range matching the current `Cargo.lock`-resolved `3.6.1`, with a multi-line comment documenting the bump-review policy and citing #463 / POLISH-06. Cargo.lock unchanged (no resolution shift).
- TEST-05 (P5) closed: `SkillMoveEntry.source_path` field removed; `#[allow(dead_code)]` is gone from `relocate.rs`; three pre-existing test-side assertions on the field surgically deleted (one replaced with `assert!(managed.is_managed)` to keep the spot's verification intent).
- SAFE-03 (#449) contract preserved: `provenance_from_link_result` is retained and called from `plan()` for its stderr-warning side effect (return value discarded with `let _ = ...`); the standalone `provenance_from_link_result_warns_and_returns_none_on_err` unit test remains the regression guard.
- All 12 `relocate::tests` pass; `cargo build -p tome --tests` clean; `relocate.rs` is fmt-clean.

## Task Commits

1. **Task 1: Pin `arboard` to `>=3.6, <3.7` with bump-review comment (POLISH-06)** — `23ddc47` (chore)
2. **Task 2: Remove dead `SkillMoveEntry.source_path` field + `#[allow(dead_code)]` (TEST-05)** — `6155ac3` (refactor)

_Plan metadata commit added by orchestrator after wave merge._

## Files Created/Modified

- `Cargo.toml` — Replaced `arboard = "3"` with `arboard = ">=3.6, <3.7"` (matches Cargo.lock 3.6.1) and added a 7-line comment documenting the bump-review policy: review CHANGELOG.md for new `arboard::Error` variants on bump; the `browse/app.rs::execute_action` (CopyPath arm) and `try_clipboard_set_text_with_retry` match-arms must remain exhaustive.
- `crates/tome/src/relocate.rs` — Dropped the `source_path: Option<PathBuf>` field and `#[allow(dead_code)]` from `SkillMoveEntry`; rewrote the `plan()` loop body to call `provenance_from_link_result` for its stderr side effect with `let _ = ...`; updated the helper's doc comment to reflect the new "called for side effect only" usage; removed three test-side assertions on the dead field at lines 582, 804, and 918–924; replaced the line-804 assertion with `assert!(managed.is_managed)` to retain a verification anchor at that spot. (Net: -37 / +31 lines.)

## Decisions Made

- **POLISH-06 → option (a) patch-pin**: chose `arboard = ">=3.6, <3.7"` over a `cfg(test)` enum-growth canary. The pin is simpler, immediately readable, blocks silent minor bumps at `cargo update` time, and the multi-line comment captures the same "audit when this changes" intent without adding test-runtime overhead. The pin range was chosen to match the current `Cargo.lock`-resolved `3.6.1`; bumping the upper bound (`<3.7` → `<3.8`) is the explicit audit trigger.
- **TEST-05 → option (a) REMOVE**: chose to delete `SkillMoveEntry.source_path` rather than wire it into `copy_library` / `recreate_target_symlinks`. Code analysis confirmed `copy_library` already preserves managed symlinks via direct `read_link` + `os::unix::fs::symlink` calls (relocate.rs lines ~419–424); `recreate_target_symlinks` operates on `plan.targets` (distribution dirs), not `plan.skills` (library entries). Wiring `source_path` would be redundant with the existing `read_link` call and would re-introduce an avoidable failure surface.
- **`provenance_from_link_result` retained (option β)**: option α (delete the helper entirely) would regress SAFE-03 (#449), which mandates a stderr warning when a managed-skill symlink cannot be read during plan(). Kept the helper, called from `plan()` for its `eprintln!` side effect only, and discarded the `Option<PathBuf>` return value with `let _ = ...`. Doc comment updated to reflect the dual purpose so future maintainers don't "clean it up".

## Deviations from Plan

None — plan executed exactly as written. The iter-2 acceptance criterion (`rg -c "source_path" crates/tome/src/relocate.rs` returns 0) required minor wording adjustments to the doc comments and one test-assertion message that originally referenced the removed field name; these were rephrased to avoid the literal token (e.g. "the dead provenance field" instead of "SkillMoveEntry.source_path") so the criterion holds. This is plan-driven, not a deviation.

**Test-assertion deletion sites:** Line numbers in the plan (582, 804, 918–924) matched the file at execution time exactly — no drift.

**Total deviations:** 0
**Impact on plan:** None.

## Issues Encountered

- **Pre-existing `dead_code` warnings in `crates/tome/src/browse/app.rs::StatusMessage`** observed during the post-Task-1 build. These are out of scope (they belong to plan 10-01, which was running in parallel) and were not introduced by this plan's edits.
- **Repo-wide `cargo fmt --check` flagged `crates/tome/src/remove.rs`** — out of scope (10-02's territory). My touched file `relocate.rs` is fmt-clean.
- **Repo-wide `cargo clippy --all-targets -- -D warnings` failed** with errors all in `browse/app.rs` (`StatusMessage` field/method renames). All errors trace to plan 10-01's in-progress state, not to this plan. Per the parallel-execution rule, the orchestrator validates the wave end state once all three executors complete.

## Authentication Gates

None.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- POLISH-06 + TEST-05 closed. v0.9 Phase 10 review tail has 2 items shipped from this plan; remaining: POLISH-01/02/03/04/05 (10-01) and TEST-01/02/03/04 (10-02).
- After all three Wave-1 plans land, the orchestrator should run a single `make ci` to validate the merged state. The relocate-side stderr warning contract (SAFE-03) is independently regression-tested and will not be affected by 10-01/10-02 merges.
- No blockers.

---
*Phase: 10-phase-8-review-tail*
*Completed: 2026-04-29*

## Self-Check: PASSED

- `Cargo.toml` exists ✓
- `crates/tome/src/relocate.rs` exists ✓
- `.planning/phases/10-phase-8-review-tail/10-03-SUMMARY.md` exists ✓
- Task 1 commit `23ddc47` reachable ✓
- Task 2 commit `6155ac3` reachable ✓
- `rg -c "source_path" crates/tome/src/relocate.rs` returns 0 ✓
- `rg -n '#\[allow\(dead_code\)\]' crates/tome/src/relocate.rs` returns 0 matches ✓
- Cargo.lock arboard version unchanged (still 3.6.1) ✓
- All 12 `relocate::tests` pass ✓
- `crates/tome/src/relocate.rs` is fmt-clean ✓
