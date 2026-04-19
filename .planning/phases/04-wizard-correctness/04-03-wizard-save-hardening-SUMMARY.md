---
phase: 04-wizard-correctness
plan: 03
subsystem: config
tags: [config, wizard, validation, toml, round-trip, serde]

requires:
  - phase: 04-wizard-correctness/04-01
    provides: D-10 Conflict+Why+Suggestion error template in Config::validate()
  - phase: 04-wizard-correctness/04-02
    provides: library_dir/distribution overlap detection in Config::validate()
provides:
  - Config::save_checked method: expand → validate → TOML round-trip → write (D-03/D-07)
  - pub(crate) Config::expand_tildes (visibility bump so wizard can run identical pipeline)
  - Wizard save block hardened to use save_checked
  - Wizard --dry-run branch validates + round-trips before printing preview
affects: [05-wizard-test-coverage, 06-display-polish-docs]

tech-stack:
  added: []
  patterns:
    - "Checked save pattern: clone → expand → validate → round-trip → write; caller's Config untouched"
    - "Defense-in-depth TOML round-trip (D-03) catches silent serde drift beyond structural validation"

key-files:
  created: []
  modified:
    - crates/tome/src/config.rs
    - crates/tome/src/wizard.rs

key-decisions:
  - "Round-trip check compares re-emitted TOML strings for byte equality rather than deriving PartialEq on Config (avoids cascading PartialEq on BTreeMap<DirectoryName, DirectoryConfig> and every leaf type)"
  - "save_checked clones self and expands tildes on the clone; the caller's tilde-shaped paths are preserved (important for returning the Config to callers that re-render it)"
  - "Dry-run branch previews the EXPANDED form (post-tilde) — intentional, because that's exactly what save_checked would validate and persist"
  - "Validation failure returns Err via ? — no retry loop (D-08/D-09). User re-runs tome init."

patterns-established:
  - "Save-with-rails: any code that builds a Config in memory (wizard today; import commands tomorrow) should call Config::save_checked rather than Config::save"
  - "pub(crate) for cross-module pipeline primitives: expand_tildes is now shared between Config::load and wizard dry-run"

requirements-completed: [WHARD-01]

duration: 6m
completed: 2026-04-19
---

# Phase 04-wizard-correctness Plan 03: Wizard Save Hardening Summary

**Config::save_checked enforces expand → validate → TOML round-trip → write; wizard save + dry-run now share the same pipeline so invalid configs never reach disk**

## Performance

- **Duration:** ~6 min
- **Started:** 2026-04-19T05:43:58Z
- **Completed:** 2026-04-19T05:49:28Z
- **Tasks:** 1 (TDD: RED → GREEN)
- **Files modified:** 2

## Accomplishments

- Closed WHARD-01: wizard's save path can no longer write an invalid config
- Added `Config::save_checked` that mirrors `Config::load` (expand + validate) and adds a TOML round-trip equality check as defense-in-depth (D-03)
- Bumped `Config::expand_tildes` to `pub(crate)` so the wizard dry-run branch can run the same pipeline as the real save
- Hardened the wizard's `--dry-run` branch to validate + reparse before printing, so the preview matches the real save outcome
- Save-failure path returns `Err` via `?` — no retry loop (D-08/D-09)

## Task Commits

1. **Task 1 (RED):** `0a0a815` — tests for `Config::save_checked` landed via 04-02's commit (see Deviations #1)
2. **Task 1 (GREEN – config.rs):** `c94d81c` — `feat(04-03): add Config::save_checked for wizard save path`
3. **Task 1 (wizard.rs):** `79f8b3f` — `feat(04-03): wire wizard save + dry-run through save_checked (WHARD-01)`

_Note: TDD RED commit was absorbed into 04-02's work due to a parallel-execution race while staging; see Deviations._

## Files Created/Modified

- `crates/tome/src/config.rs` — New `Config::save_checked` method (~45 lines); `expand_tildes` visibility bumped from private to `pub(crate)`; four new `save_checked_*` unit tests
- `crates/tome/src/wizard.rs` — Save block now calls `save_checked`; dry-run branch clones + expands + validates + reparses before printing

## Decisions Made

- **Round-trip compares TOML strings, not Configs:** Deriving `PartialEq` on `Config` would cascade to every leaf type (`DirectoryConfig`, `DirectoryRole`, `BackupConfig`, `DirectoryType`, `DirectoryName`, `SkillName`) and to `BTreeMap<DirectoryName, DirectoryConfig>`. Comparing `toml::to_string_pretty` output bytes is simpler and catches the same class of bug (field accidentally dropped by `skip_serializing_if`, enum variant renamed, etc.).
- **`save_checked` operates on a clone:** The wizard builds a `Config` in memory and returns it from `run()`. If `save_checked` expanded tildes in place, the returned `Config` would have absolute paths rather than `~`-shaped paths, which would leak into callers that re-render or re-save it. Cloning keeps the caller's Config intact.
- **Dry-run preview shows the expanded form:** This is intentional — the preview is meant to show exactly what would be validated and written. Showing the pre-expansion form would give a false impression that the saved file would contain `~`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 – Blocking] RED test commit merged into 04-02's commit**
- **Found during:** Task 1 (RED phase)
- **Issue:** Plan 04-02 ran in parallel and touched `crates/tome/src/config.rs`. I added my four RED tests while the file was clean on my end; however, the parallel agent staged and committed the file (including my unstaged edits) before my own `git commit` for RED ran. Result: my RED tests were actually committed under `ed32dad feat(04-02): reject library/distribution path overlaps`, not under a 04-03 commit.
- **Fix:** Accepted the state (reverting 04-02's commit to separate the tests would rewrite history and destroy 04-02's work). Proceeded with GREEN directly. Commit `c94d81c` adds the `save_checked` implementation that makes the tests pass. Commit `79f8b3f` wires it into the wizard.
- **Files affected:** commit history only — no code correctness impact.
- **Verification:** All four `save_checked_*` unit tests pass; RED→GREEN transition verified by checking `cargo test` output before and after `c94d81c` (no `save_checked` method before; 4/4 green after).
- **Committed in:** N/A (history deviation, not a code change).

---

**Total deviations:** 1 (parallel-execution race in commit history)
**Impact on plan:** Zero functional impact. The plan's code-level requirements are fully met. Only the commit-attribution structure differs from the TDD RED→GREEN pattern described in the plan.

## Issues Encountered

- Pre-existing flaky test `backup::tests::snapshot_creates_commit` fails because the local git/SSH-signing config refuses signing operations in the temp-dir sandbox. Confirmed pre-existing via `git stash` + re-run on the untouched tree. Out of scope per SCOPE BOUNDARY. Not logged to `deferred-items.md` because it's a known environmental limitation, not a code issue introduced by this plan.

## Acceptance Criteria — Verification

All criteria from plan's `<acceptance_criteria>` block pass:

| # | Check | Result |
|---|-------|--------|
| 1 | `rg "pub fn save_checked" crates/tome/src/config.rs` | 1 hit |
| 2 | `rg "pub\(crate\) fn expand_tildes" crates/tome/src/config.rs` | 1 hit |
| 3 | `rg "save_checked" crates/tome/src/wizard.rs` | 1 hit |
| 4 | `rg "config\.save\(" crates/tome/src/wizard.rs` (must be 0) | 0 hits |
| 5 | `rg "round-trip" crates/tome/src/config.rs` | 6 hits (inside save_checked) |
| 6 | `rg "wizard dry-run: configuration is invalid" crates/tome/src/wizard.rs` | 1 hit |
| 7 | `rg "wizard save aborted" crates/tome/src/wizard.rs` | 1 hit |
| 8 | `cargo test -p tome --lib -- config::tests::save_checked` | 4/4 pass |
| 9 | `cargo build -p tome` | Clean |
| 10 | `cargo clippy --all-targets -- -D warnings` | Clean |
| 11 | `cargo fmt -- --check` | Clean |

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Phase 04 (Wizard Correctness) is functionally complete. Plan 04-01 upgraded error messages, Plan 04-02 added overlap detection, Plan 04-03 wired the wizard through `save_checked`.
- Phase 05 (Wizard Test Coverage) can now rely on `Config::save_checked` as a stable entry point for integration tests that do `tome init --dry-run --no-input` end-to-end.
- No blockers.

## Self-Check: PASSED

- Files modified exist: `crates/tome/src/config.rs` FOUND; `crates/tome/src/wizard.rs` FOUND
- Commits exist:
  - `c94d81c feat(04-03): add Config::save_checked for wizard save path` FOUND
  - `79f8b3f feat(04-03): wire wizard save + dry-run through save_checked (WHARD-01)` FOUND
- Prior-commit reference (`0a0a815`) resolves in `git log --all`: FOUND (landed under 04-02 due to parallel race)

---
*Phase: 04-wizard-correctness*
*Completed: 2026-04-19*
