---
phase: 02-git-sources-selection
plan: 04
subsystem: cli
tags: [rust, clap, remove-command, cleanup]

# Dependency graph
requires:
  - phase: 01-unified-directory-foundation
    provides: Config, Manifest, TomePaths, eject pattern
provides:
  - "tome remove <name> command with plan/render/execute pattern"
  - "Full artifact cleanup: symlinks, library dirs, manifest, config, lockfile"
  - "Interactive confirmation, --dry-run, --force flags"
affects: [wizard, config-migration]

# Tech tracking
tech-stack:
  added: []
  patterns: [plan-render-execute for destructive CLI commands]

key-files:
  created:
    - crates/tome/src/remove.rs
  modified:
    - crates/tome/src/cli.rs
    - crates/tome/src/lib.rs
    - crates/tome/tests/cli.rs

key-decisions:
  - "Adapted remove to source-based config (sources Vec) since unified directory model not yet on main"
  - "Followed eject.rs plan/render/execute pattern for consistency"
  - "Remove saves config via Config::save to paths.config_path(), requiring --tome-home in integration tests"

patterns-established:
  - "Plan/render/execute pattern for destructive commands with --force and --dry-run"

requirements-completed: [CLI-01]

# Metrics
duration: 7min
completed: 2026-04-15
---

# Phase 2 Plan 4: tome remove Summary

**`tome remove` command with full source cleanup: symlinks, library dirs, manifest entries, config save, and lockfile regeneration**

## Performance

- **Duration:** 7 min
- **Started:** 2026-04-15T13:56:06Z
- **Completed:** 2026-04-15T14:02:48Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- Implemented `tome remove <name>` command removing source entries and all associated artifacts
- Plan/render/execute pattern with cleanup ordering: symlinks, library, manifest, config, lockfile
- TTY confirmation prompt, --force to skip, --no-input without --force fails safely
- 4 unit tests and 4 integration tests covering key scenarios

## Task Commits

Each task was committed atomically:

1. **Task 1: Create remove.rs module with plan/preview/execute and add CLI subcommand** - `64b57f7` (feat)
2. **Task 2: Add integration tests for tome remove** - `424295b` (test)

## Files Created/Modified
- `crates/tome/src/remove.rs` - Remove command: plan(), render_plan(), execute() with cleanup ordering
- `crates/tome/src/cli.rs` - Remove subcommand with NAME arg and --force flag
- `crates/tome/src/lib.rs` - Module declaration and Command::Remove dispatch with TTY confirmation
- `crates/tome/tests/cli.rs` - 4 integration tests: nonexistent source, local directory cleanup, dry-run, no-input

## Decisions Made
- Adapted the plan's "directories" references to work with current source-based config model (sources Vec, not directories BTreeMap) since the unified directory model from prior waves is not yet merged to main
- Used `--tome-home` in integration tests instead of `--config` to ensure config save path matches load path

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Adapted to source-based config model**
- **Found during:** Task 1
- **Issue:** Plan referenced `config.directories` and git-type cleanup, but worktree is on main with source-based config (Vec<Source>)
- **Fix:** Implemented remove against `config.sources` instead of `config.directories`; deferred git cache cleanup since git.rs module doesn't exist yet
- **Files modified:** crates/tome/src/remove.rs
- **Verification:** All tests pass, clippy clean
- **Committed in:** 64b57f7

**2. [Rule 1 - Bug] Fixed config save path mismatch in integration tests**
- **Found during:** Task 2
- **Issue:** TestEnvBuilder writes config to `config.toml` but `Config::save` writes to `tome.toml` via `paths.config_path()`
- **Fix:** Created `remove_test_env` helper writing config as `tome.toml` and used `--tome-home` flag
- **Files modified:** crates/tome/tests/cli.rs
- **Verification:** All 4 integration tests pass
- **Committed in:** 424295b

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 bug)
**Impact on plan:** Both necessary for correctness. Git cache cleanup deferred to when git.rs is merged.

## Issues Encountered
None beyond the deviations above.

## Known Stubs
None - all functionality is fully wired.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- `tome remove` is functional and tested
- Git-type directory cache cleanup should be added when git.rs module lands on main

---
*Phase: 02-git-sources-selection*
*Completed: 2026-04-15*
