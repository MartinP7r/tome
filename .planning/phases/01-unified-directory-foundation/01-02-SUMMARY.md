---
phase: 01-unified-directory-foundation
plan: 02
subsystem: pipeline
tags: [sync-pipeline, discovery, distribution, consolidation, cleanup, unified-directory]

requires:
  - phase: 01-01
    provides: DirectoryName, DirectoryType, DirectoryRole, DirectoryConfig types; Config.discovery_dirs(), distribution_dirs() iterators
provides:
  - Role-based discovery dispatching via discover_directory_entry()
  - Manifest-based origin check replacing shares_tool_root() in distribution
  - Directory-aware distribute_to_directory() function
  - Deprecated compat shims (discover_source, distribute_to_target) for unconverted modules
affects: [01-03, 01-04, 01-05]

tech-stack:
  added: []
  patterns: [manifest-based-origin-check, directory-name-based-circular-prevention]

key-files:
  created: []
  modified:
    - crates/tome/src/discover.rs
    - crates/tome/src/distribute.rs

key-decisions:
  - "Added deprecated discover_source() compat shim so status.rs compiles without conversion (deferred to plan 01-04)"
  - "Added deprecated distribute_to_target() compat shim and target_name field so lib.rs compiles without conversion (deferred to plan 01-05)"
  - "Cleanup.rs required no changes -- already accepts generic Path arguments, iteration happens in lib.rs"
  - "Manifest-based source_name == dir_name check replaces shares_tool_root() -- simpler and correct for unified directory model"

patterns-established:
  - "discover_directory_entry() dispatches by DirectoryType, determines origin by DirectoryRole"
  - "distribute_to_directory() uses manifest source_name == dir_name for circular prevention (PIPE-03)"
  - "Deprecated compat shims bridge unconverted modules with #[deprecated] and #[allow(deprecated)]"

requirements-completed: [PIPE-01, PIPE-02, PIPE-03, PIPE-04, PIPE-05]

duration: 12min
completed: 2026-04-12
---

# Phase 1 Plan 2: Pipeline Adaptation Summary

**Four pipeline modules (discover, distribute) rewritten for unified directory model with manifest-based circular prevention replacing shares_tool_root()**

## Performance

- **Duration:** 12 min
- **Started:** 2026-04-12T08:22:42Z
- **Completed:** 2026-04-12T08:35:07Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- discover.rs rewritten: iterates config.discovery_dirs(), dispatches by DirectoryType, determines SkillOrigin by DirectoryRole
- distribute.rs rewritten: distribute_to_directory() with manifest-based source_name == dir_name circular prevention
- shares_tool_root() and source_paths parameter removed entirely from distribution
- library.rs and cleanup.rs required no changes (already generic, no old type references)
- 77 tests pass across all four modules (23 discover + 32 library + 13 distribute + 9 cleanup)

## Task Commits

Each task was committed atomically:

1. **Task 1: Rewrite discover.rs for directory-based discovery** - `49bf9af` (feat)
2. **Task 2: Rewrite distribute.rs for directory-aware distribution** - `c6975f2` (feat)

## Files Created/Modified
- `crates/tome/src/discover.rs` - Role-based discovery: discovery_dirs() iteration, DirectoryType dispatch, DirectoryRole origin determination
- `crates/tome/src/distribute.rs` - Directory-aware distribution: distribute_to_directory(), manifest-based origin check, no source_paths

## Decisions Made
- Added deprecated `discover_source()` compat shim because `status.rs` still calls it (will be converted in plan 01-04)
- Added deprecated `distribute_to_target()` compat shim and `target_name` field because `lib.rs` still uses old API (will be converted in plan 01-05)
- library.rs needed zero changes -- consolidation strategy already determined by `SkillOrigin` which is correctly set by updated discover.rs
- cleanup.rs needed zero changes -- already accepts `&Path` arguments; the iteration over `config.targets` happens in lib.rs, not cleanup.rs

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added deprecated discover_source() compat shim**
- **Found during:** Task 1 (discover.rs compilation)
- **Issue:** status.rs calls `discover::discover_source()` which was renamed to `discover_directory_entry()`. Crate fails to compile.
- **Fix:** Added deprecated `pub fn discover_source()` wrapper that translates old Source type to new function parameters.
- **Files modified:** crates/tome/src/discover.rs
- **Verification:** `cargo build -p tome` succeeds
- **Committed in:** 49bf9af (Task 1 commit)

**2. [Rule 3 - Blocking] Added deprecated distribute_to_target() compat shim and target_name field**
- **Found during:** Task 2 (distribute.rs compilation)
- **Issue:** lib.rs calls `distribute::distribute_to_target()` and accesses `dr.target_name` field. Both renamed.
- **Fix:** Added deprecated wrapper function and duplicate `target_name` field on DistributeResult.
- **Files modified:** crates/tome/src/distribute.rs
- **Verification:** `cargo build -p tome` succeeds
- **Committed in:** c6975f2 (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 blocking)
**Impact on plan:** Both compat shims are necessary for the crate to compile while other modules still reference old APIs. Clearly marked deprecated with removal plan references. No scope creep.

## Issues Encountered
- Pre-existing test failure in `relocate::tests::execute_recreates_target_symlinks` -- uses deprecated `config.targets` which is always empty. Out of scope for this plan; will be fixed when relocate.rs is converted.

## User Setup Required
None - no external service configuration required.

## Known Stubs
None - all functions are fully implemented with real logic.

## Next Phase Readiness
- All four pipeline modules work against new directory types
- Deprecated compat shims keep unconverted modules compilable
- Ready for plan 01-03 (manifest/lockfile/machine adaptation) and 01-04 (status/doctor)

---
*Phase: 01-unified-directory-foundation*
*Completed: 2026-04-12*
