---
phase: 19-doctor-status-surface-bugfix-bundle
plan: 04
subsystem: testing
tags: [flake, arboard, clipboard, git, backup, browse, hard-14, fix-02]

# Dependency graph
requires:
  - phase: 19-doctor-status-surface-bugfix-bundle/01
    provides: "Doctor categorization + RepairKind substrate (W1) — no direct code coupling; only branch-state dependency"
provides:
  - "Browse copy-path retry test bound relaxed 600ms → 2000ms with multi-line FLAKE-FIX root-cause comment (closes #511)"
  - "FLAKE-WATCH defensive comment on backup::tests::push_and_pull_roundtrip documenting flake history + reproduction attempts + future retry-wrapper mitigation path (HARD-14 carry-over)"
  - "D-FLAKE-3 honored: clock injection rejected for v0.11 scope, mentioned only in rejected-alternative comment"
affects: [19-07-changelog-and-phase-verification, future-phase-if-flake-recurs]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "FLAKE-FIX (#511 / HARD-14): canonical comment template for relaxed-bound test guards naming root-cause class + rejected alternative"
    - "FLAKE-WATCH (HARD-14 / FIX-02 / #511): defensive comment template for non-reproducible flakes documenting history + recommended next-mitigation path"

key-files:
  created:
    - .planning/phases/19-doctor-status-surface-bugfix-bundle/19-04-SUMMARY.md
  modified:
    - crates/tome/src/browse/app.rs
    - crates/tome/src/backup.rs

key-decisions:
  - "Outcome C for backup test: flake NOT reproducible locally (50/50 isolated, 10/10 module, 5/5 full-lib runs all pass at --test-threads=8 on M1 macOS); defensive FLAKE-WATCH comment shipped in lieu of speculative retry wrapper"
  - "D-FLAKE-2 honored explicitly: backup test investigated per its actual root-cause class (no timing assertion → no D-FLAKE-1 relaxed-bound treatment); recommended next mitigation (per-call git subprocess retry wrapper with exponential backoff) documented inline for future-phase pickup if flake recurs in CI"
  - "D-FLAKE-3 honored: clock-injection (trait Clock in browse::app) NOT introduced in code; only referenced in rejected-alternative comment block per v0.11 polish-scope discipline"
  - "Browse-test bound chosen at 2000ms (per CONTEXT.md D-FLAKE-1 + RESEARCH.md empirical breakdown): catches 10×-retry regression (~1100ms) while tolerating realistic parallel-test arboard contention; verified 100/100 consecutive passes at --test-threads=8"

patterns-established:
  - "FLAKE-FIX comment shape: bound relaxation + root-cause class (named) + assertion-purpose clarification (guards hangs, not perf) + rejected-alternative note. Multi-line, placed immediately above the relaxed assert."
  - "FLAKE-WATCH comment shape: history (issue refs + prior mitigation) + reproduction attempts table (commands + outcome) + recommended next-mitigation code sketch + decision references (D-FLAKE-2 / D-FLAKE-3). Placed above #[test] annotation."

requirements-completed:
  - FIX-02

# Metrics
duration: 18min
completed: 2026-05-13
---

# Phase 19 Plan 04: Flake Bounds Relaxation Summary

**Browse copy-path retry test bound relaxed 600ms→2000ms with arboard-rooted FLAKE-FIX comment; backup roundtrip flake documented via defensive FLAKE-WATCH comment (Outcome C — non-reproducible locally) per D-FLAKE-2's investigation-first re-opening clause**

## Performance

- **Duration:** ~18 min
- **Started:** 2026-05-13T07:04:00Z (approximate, based on cargo compile cache state)
- **Completed:** 2026-05-13T07:22:00Z
- **Tasks:** 2/2 completed
- **Files modified:** 2 (`browse/app.rs`, `backup.rs`)

## Accomplishments

- Closed [#511](https://github.com/MartinP7r/tome/issues/511) (`browse::app::tests::copy_path_retry_helper_returns_within_bound` timing flake) via a 5-LOC bound relaxation + 13-line multi-line root-cause comment block naming `arboard` clipboard contention (NSPasteboard / X11 / WinClipboard arbitration) as the unfixable root-cause class. Comment also documents the rejected clock-injection alternative (D-FLAKE-3) so future engineers know it was considered.
- Addressed the HARD-14 `backup::tests::push_and_pull_roundtrip` carry-over per D-FLAKE-2's actual-root-cause-investigation clause: 50 isolated + 10 backup-module + 5 full-lib-suite runs all passed cleanly at `--test-threads=8` on M1 macOS — flake does NOT reproduce locally. Shipped a 41-line defensive FLAKE-WATCH comment block documenting (a) history (HARD-14 added `setup_git_config`), (b) reproduction attempts and outcomes, (c) the recommended next mitigation (per-call git subprocess retry wrapper with 50ms exponential backoff) as inline pseudocode for future-phase pickup if the flake recurs in CI.
- Verified Success Criterion 3 first bullet: 100/100 consecutive `cargo test -p tome --lib browse::app::tests::copy_path_retry_helper_returns_within_bound -- --test-threads=8` runs pass after the bound bump.

## Task Commits

Each task was committed atomically with `--no-verify` (parallel-executor protocol):

1. **Task 1: Relax browse test bound 600ms → 2000ms + add multi-line root-cause comment** — `2531d0e` (fix)
2. **Task 2: Reproduce-first then fix `backup::tests::push_and_pull_roundtrip` per its actual root-cause class** — `30ed61e` (docs — Outcome C: defensive comment only, no test logic change)

**Plan metadata commit:** (will be created after this SUMMARY lands)

## Files Modified

- `crates/tome/src/browse/app.rs` — Relaxed `assert!(elapsed < Duration::from_millis(2000), ...)` from previous 600ms; inserted 13-line FLAKE-FIX comment block naming arboard contention + rejected clock-injection alternative. The existing empirical-breakdown comment at `:1790-1795` is preserved (it still accurately describes the happy-path 5–500ms / retry-path 100–600ms timing model).
- `crates/tome/src/backup.rs` — Inserted 41-line FLAKE-WATCH comment above `#[test] fn push_and_pull_roundtrip` documenting flake history, reproduction-attempt log, and recommended next-mitigation retry-wrapper pattern. No test logic changed.

## Decisions Made

### Outcome A/B/C selection for the backup test (per Task 2 ambiguity)

**Selected: Outcome C — flake NOT reproducible locally; defensive FLAKE-WATCH comment shipped.**

Reproduction attempts on M1 macOS, 2026-05-13:

| Command | Iterations | Result |
| --- | --- | --- |
| `cargo test -p tome backup::tests::push_and_pull_roundtrip -- --test-threads=8` | 50 | 50/50 pass |
| `cargo test -p tome backup -- --test-threads=8` (full backup module under parallelism) | 10 | 10/10 pass |
| `cargo test -p tome --lib -- --test-threads=8` (full unit-test suite) | 5 | 5/5 pass |

Local hardware (M1 macOS) cannot reproduce the flake; the historical failures are CI-environment-specific (Linux GHA runner — likely shared-filesystem / scheduler contention or fresh-worker identity-config absence). Per D-FLAKE-2's investigation-first clause, applying a speculative retry wrapper without a reproducible failure mode would be cargo-cult engineering and risks hiding a real bug. The FLAKE-WATCH comment ships the recommended next mitigation as inline pseudocode so a future-phase pickup is mechanical if the flake recurs.

### D-FLAKE-3 (clock-injection) explicitly NOT tempted

The browse test could in principle be made bound-free by injecting a `trait Clock` into `browse::app`, replacing wall-clock `Instant::now()` calls with a test-controlled clock. This was considered and rejected:

- It would require restructuring `try_clipboard_set_text_with_retry` to accept a clock parameter
- The blast radius would touch every caller, not just the test
- The current bound-relaxation pattern is ≤20 LOC; the abstraction is ~200 LOC
- v0.11 is explicitly "polish + observability" — out of scope for a structural refactor

The rejection is documented in the FLAKE-FIX comment itself (last paragraph) so future engineers know it was considered.

### Browse-test bound value: 2000ms (per CONTEXT.md + RESEARCH.md empirical analysis)

| Bound | Catches | Tolerates |
| --- | --- | --- |
| 600ms (previous) | Second-retry regression (+100ms) | Local single-threaded happy path only |
| **2000ms (new)** | **10×-retry regression (~1100ms)** | **Realistic parallel-test arboard contention** |
| 3000–5000ms (alternatives) | Same as 2000ms | Same | Strictly weaker regression catch |

Picked 2000ms per the plan's locked recommendation: it preserves regression-catching for any reasonable retry-count regression while tolerating the variance we actually see in CI. Local 100-run check confirmed.

## Deviations from Plan

**None — plan executed exactly as written.**

The plan explicitly listed three possible outcomes (A/B/C) for Task 2 and authorized Outcome C as an acceptable conclusion when the flake cannot be reproduced. Selecting Outcome C is following the plan, not deviating from it.

## Issues Encountered

- **Worktree branch divergence at executor start:** The parallel-executor worktree was forked from commit `197933d` (3 commits behind the phase branch) and was missing Phase 18 + Phase 19 Wave 1 commits. Resolved by fetching `gsd/phase-19-doctor-status-surface-bugfix-bundle` into the worktree as `tmp-phase-19` and resetting the worktree branch to its tip. This is standard parallel-executor topology setup, not a deviation from the plan.

## Verification

- `cargo test -p tome --lib browse::app::tests::copy_path_retry_helper_returns_within_bound` — passes
- `cargo test -p tome --lib backup::tests::push_and_pull_roundtrip -- --test-threads=8` — passes
- `cargo test -p tome --lib -- --test-threads=8` (full unit suite, 5 runs) — 5/5 pass
- 100-iteration browse-test stability run — 100/100 pass (Success Criterion 3 first bullet ✓)
- 50-iteration backup-test reproduction attempt — 50/50 pass (Outcome C confirmed)
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo fmt -- --check` — clean

Acceptance-criteria grep checks:

| Check | Expected | Actual |
| --- | --- | --- |
| `rg "Duration::from_millis\(2000\)" crates/tome/src/browse/app.rs` | ≥1 | 1 ✓ |
| `rg "Duration::from_millis\(600\)" crates/tome/src/browse/app.rs` (inside copy_path_retry_helper_returns_within_bound) | 0 | 0 ✓ |
| `rg "FLAKE-FIX \(#511 / HARD-14\)" crates/tome/src/browse/app.rs` | 1 | 1 ✓ |
| `rg "FLAKE-WATCH \(HARD-14" crates/tome/src/backup.rs` | 1 | 1 ✓ |
| `rg "trait Clock" crates/tome/src/browse/app.rs` (non-comment) | 0 | 0 ✓ |

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- FIX-02 closed in full: #511 closeable via the browse fix; HARD-14 carry-over folded into FIX-02 per D-FLAKE-2 with the documented re-opening clause invoked → Outcome C selected.
- Plan 07 (final verification + CHANGELOG) can now reference FIX-02 as shipped. The CHANGELOG entry should mention both the browse-bound fix AND the backup-test FLAKE-WATCH documentation so a reader of the v0.11 release notes can find the rationale without re-reading the plan.
- If the backup flake recurs in CI post-v0.11, the next mitigation step is already sketched inline at `backup.rs::push_and_pull_roundtrip` — a future-phase ticket only needs to swap the test's `git_success`/`git_helper` calls for the retry wrapper. No new design work required.

## Self-Check: PASSED

**Files verified to exist:**

- `crates/tome/src/browse/app.rs` — FOUND (modified, FLAKE-FIX comment present)
- `crates/tome/src/backup.rs` — FOUND (modified, FLAKE-WATCH comment present)
- `.planning/phases/19-doctor-status-surface-bugfix-bundle/19-04-SUMMARY.md` — FOUND (this file)

**Commits verified to exist:**

- `2531d0e` — FOUND (Task 1: browse bound relaxation)
- `30ed61e` — FOUND (Task 2: backup FLAKE-WATCH comment)

---
*Phase: 19-doctor-status-surface-bugfix-bundle*
*Completed: 2026-05-13*
