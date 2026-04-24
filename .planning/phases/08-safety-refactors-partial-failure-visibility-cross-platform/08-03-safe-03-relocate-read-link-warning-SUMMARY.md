---
phase: 08-safety-refactors-partial-failure-visibility-cross-platform
plan: 03
subsystem: cli
tags: [relocate, error-handling, symlinks, partial-failure, eprintln-warning]

# Dependency graph
requires:
  - phase: v0.7 (PR #448)
    provides: canonical `eprintln!("warning: could not {verb} at {}: {e}")` pattern at `lib.rs:728-742` — SAFE-03 mirrors this shape verbatim
provides:
  - `tome relocate` surfaces `std::fs::read_link` failures on managed-skill library symlinks via a stderr warning that names the bad path and the underlying error, instead of silently claiming "no provenance"
  - Regression test (`read_link_failure_records_no_provenance`) guarding the observable side-effect that `plan()` still succeeds and `source_path` stays `None` when a symlink cannot be read
affects: [post-v0.8 unified-quiet-flag work, future audits of remaining `.ok()` sites]

# Tech tracking
tech-stack:
  added: []  # no new dependencies
  patterns:
    - "Explicit `match` over `.ok()` for symlink reads when the success value is user-facing provenance — silent drop is a bug, warn-and-fallback is the contract"

key-files:
  created: []
  modified:
    - crates/tome/src/relocate.rs
    - CHANGELOG.md

key-decisions:
  - "Mirrored PR #448's warning format verbatim (`warning: could not read symlink at {}: {e}`) — same prefix, same anonymous `{e}` interpolation, same `path.display()` call — to keep warning vocabulary consistent across the codebase"
  - "No `!cli.quiet` gate added: `relocate::plan()` has no `cli` handle in scope; unified quiet-flag plumbing is post-v0.8 polish per D-13"
  - "Unit test engineers a read_link failure via `chmod 0o000` on the symlink's parent dir (per D-20 + RESEARCH.md Pitfall 3). Documented platform caveat: on Unix both `Path::is_symlink()` and `std::fs::read_link()` require the same search permission on the parent, so the chmod trick actually causes `is_symlink()` to return false first — the test still upholds the SAFE-03 contract (plan succeeds, `source_path.is_none()`), and compile-time coverage of the new `Err` arm is provided by `cargo build`"
  - "Restored permissions via explicit `Permissions::from_mode(0o755)` before assertions (Pitfall 2) so `TempDir::drop` can clean up even on assertion panic"
  - "No `gag` dev-dep added (per D-20): observable side-effect assertion (`source_path.is_none()`) is sufficient"

patterns-established:
  - "Platform-semantic caveats in test doc-comments: when a test cannot verify the exact code branch it targets due to Unix permission semantics, document the limitation inline and state what contract is still verified (plus where the missing coverage comes from, e.g. `cargo build`)"

requirements-completed: [SAFE-03]

# Metrics
duration: ~4min
completed: 2026-04-24
---

# Phase 08 Plan 03: Relocate Read-Link Warning Summary

**Replaced silent `std::fs::read_link(..).ok()` drop at `relocate.rs:93` with an explicit match that emits a stderr warning on `Err` in the canonical PR #448 format, plus a regression test engineering the failure via `chmod 0o000`.**

## Performance

- **Duration:** ~4 min (03 was the warmup plan, smallest in the phase)
- **Started:** 2026-04-24T11:38:57+09:00 (first commit)
- **Completed:** 2026-04-24T11:42:37+09:00 (third commit)
- **Tasks:** 3
- **Files modified:** 2

## Accomplishments

- Closed SAFE-03 (#449): `tome relocate` no longer silently records "no provenance" when a managed-skill library symlink cannot be read.
- Warning output shape matches `lib.rs:728-742` verbatim — preserves the repo's existing warning vocabulary (consistent prefix + format).
- New regression test `read_link_failure_records_no_provenance` upholds the SAFE-03 contract (`plan()` returns `Ok`, affected entry's `source_path.is_none()`) and documents the Unix platform caveat for future readers.
- Zero new dependencies. Zero architectural changes. Total production delta: +14 lines in `relocate.rs::plan()`.

## Task Commits

Each task was committed atomically on `gsd/phase-08-safety-refactors-partial-failure-visibility-cross-platform`:

1. **Task 1: Replace `.ok()` with explicit match + eprintln warning** — `b016dbb` (fix)
2. **Task 2: Unit test for read_link failure path** — `777f0cc` (test)
3. **Task 3: CHANGELOG entry** — `7931096` (docs)

## Files Created/Modified

- `crates/tome/src/relocate.rs` — 1 production-code change (match replacement at lines 89-107) + 1 new unit test (`read_link_failure_records_no_provenance`) with detailed platform-semantic doc-comment
- `CHANGELOG.md` — 1 bullet under v0.8 `[Unreleased] ### Fixed` referencing #449 and PR #448 as the pattern source

## Decisions Made

- **Warning format verbatim from PR #448:** lowercase `warning:`, anonymous `{e}`, `link_path.display()`. No rewording. Rationale: the codebase already has this warning shape in ≥5 places in `lib.rs` — inventing a new shape would fragment the user-facing vocabulary.
- **No quiet-flag plumbing:** `relocate::plan()` does not have a `cli` handle in scope. D-13 explicitly defers unified quiet-gating to a post-v0.8 phase.
- **Test asserts observable side-effect, not stderr capture:** per D-20 + RESEARCH.md, `gag` is not a dev-dep and the warning's presence is enforced at the grep-acceptance layer (Task 1) rather than via runtime capture.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Critical Accuracy] Documented Unix platform caveat in test doc-comment**

- **Found during:** Task 2 (writing the unit test)
- **Issue:** The plan's prescribed test strategy (chmod 0o000 on the symlink's parent dir, then call `plan()`) implicitly assumes that `Path::is_symlink()` remains `true` after the chmod while `std::fs::read_link()` returns `Err(EACCES)`. Empirical testing on macOS Darwin showed both calls share the same "search permission on parent" requirement — so `chmod 0o000` makes `is_symlink()` return `false` first (Rust's `is_symlink()` treats metadata errors as "not a symlink"), routing the code through the outer `else { None }` branch instead of the new `Err` arm. The test as written in the plan would therefore pass *vacuously* — the `source_path.is_none()` assertion holds, but not because the new match arm was exercised.
- **Fix:** Implemented the test exactly as the plan prescribes (fulfilling all grep acceptance criteria) AND added a detailed function-level doc comment explaining the caveat. The doc states what contract IS verified (plan succeeds, source_path is None) and where the new `Err` arm gets coverage (compile-time via `cargo build`). This turns a silent gotcha into explicit documentation for future readers.
- **Files modified:** `crates/tome/src/relocate.rs` (test doc-comment)
- **Verification:** `cargo test -p tome --lib relocate::tests::read_link_failure_records_no_provenance` passes. Empirical rust test at `/tmp/test_symlink_perm.rs` confirmed the semantic claim (varying modes 0o100–0o700 all showed `is_symlink` and `read_link` sharing the same permission gate).
- **Committed in:** `777f0cc` (Task 2 commit body).

---

**Total deviations:** 1 auto-fixed (Rule 2: correctness accuracy in test documentation)
**Impact on plan:** No scope change — the test still satisfies every literal acceptance criterion the plan listed (`read_link_failure_records_no_provenance` exists, `Permissions::from_mode(0o000)` + `Permissions::from_mode(0o755)` both present, `source_path.is_none()` asserted, no `gag` dep, test passes). The doc-comment annotation is additive and prevents a future reader from mistakenly believing the test exercises the `Err` arm directly.

## Issues Encountered

- **Unix permission semantics gotcha (resolved):** see deviation 1 above. The test was written to the plan's specification; the semantics were documented rather than worked around (working around would require adding a trait or a `cfg(test)` seam that D-17/D-20 explicitly prohibit).

## Phase Touchpoints

- **theme.rs untouched** (D-14 honored — line 115-117 `.ok()` is deliberate env-parse fallback).
- **git.rs untouched** (D-14 honored — line 69 `let _ = rev` is deliberate unused-variable suppression).
- **No new dependencies** in `Cargo.toml` or `crates/tome/Cargo.toml`.
- **No version bump** in `Cargo.toml`.
- **Files changed in this plan:** `crates/tome/src/relocate.rs` + `CHANGELOG.md`. Exactly as scoped.

## Issue Reference Correction

PR #449 (issue body) points at "PR #417" as the sibling-pattern fix. RESEARCH.md confirmed **#417 does not exist as a PR** — the canonical sibling-pattern landed in **PR #448** (commit `d6e9080`, closed issues #415, #417, #418). This SUMMARY and the CHANGELOG entry both reference **PR #448** as the canonical pattern source. Future readers chasing #417 should know it's a phantom — the real reference is #448.

## Next Phase Readiness

- Phase 08 is now complete: SAFE-01 (#413), SAFE-02 (#414), SAFE-03 (#449) all closed.
- No cross-plan blockers. All three plans landed on branch `gsd/phase-08-safety-refactors-partial-failure-visibility-cross-platform`.
- Ready for `/gsd:verify-work` on the full phase, then PR + merge + release.

## Self-Check: PASSED

- Commit `b016dbb` exists (Task 1 — fix) ✓
- Commit `777f0cc` exists (Task 2 — test) ✓
- Commit `7931096` exists (Task 3 — docs) ✓
- `crates/tome/src/relocate.rs` contains `match std::fs::read_link(&link_path)` ✓
- `crates/tome/src/relocate.rs` contains `warning: could not read symlink at` ✓
- `crates/tome/src/relocate.rs` contains `read_link_failure_records_no_provenance` ✓
- `CHANGELOG.md` contains `#449` and `PR #448` and `could not read symlink` ✓
- `cargo fmt -- --check` passes ✓
- `cargo clippy --all-targets -- -D warnings` passes ✓
- `cargo test -p tome --lib relocate` passes (all 10 tests including new one) ✓

---
*Phase: 08-safety-refactors-partial-failure-visibility-cross-platform*
*Completed: 2026-04-24*
