---
phase: 01-unified-directory-foundation
plan: 03
subsystem: state
tags: [manifest, lockfile, machine, status, doctor, directory-model, serde]

requires:
  - phase: 01-01
    provides: DirectoryName, DirectoryType, DirectoryRole, DirectoryConfig types, Config.directories, convenience iterators
provides:
  - MachinePrefs with disabled_directories (DirectoryName) replacing disabled_targets (TargetName)
  - DirectoryStatus struct replacing SourceStatus + TargetStatus
  - Directory-aware doctor diagnostics using distribution_dirs()
  - Updated manifest/lockfile doc comments for directory terminology
affects: [01-02, 01-04, 01-05]

tech-stack:
  added: []
  patterns: [unified-directory-status, role-based-counting]

key-files:
  created: []
  modified:
    - crates/tome/src/manifest.rs
    - crates/tome/src/lockfile.rs
    - crates/tome/src/machine.rs
    - crates/tome/src/status.rs
    - crates/tome/src/doctor.rs
    - crates/tome/src/lib.rs
    - crates/tome/src/relocate.rs

key-decisions:
  - "Updated lib.rs and relocate.rs callers inline rather than adding compat shims for machine.rs field rename -- cleaner than adding temporary redirects"
  - "status.rs count_skill_dirs and count_symlinks as separate helpers -- discovery dirs count subdirectories, target-only dirs count symlinks"

patterns-established:
  - "DirectoryStatus.role field includes human-readable description from DirectoryRole::description()"
  - "Doctor uses config.distribution_dirs() for checking distribution directories instead of iterating targets directly"

requirements-completed: [MACH-01, STATE-01, STATE-02, STATE-03]

duration: 9min
completed: 2026-04-12
---

# Phase 1 Plan 3: State/Reporting Modules Summary

**Unified directory terminology in manifest, lockfile, machine prefs, status, and doctor -- disabled_directories replaces disabled_targets, DirectoryStatus replaces SourceStatus/TargetStatus**

## Performance

- **Duration:** 9 min
- **Started:** 2026-04-12T08:21:16Z
- **Completed:** 2026-04-12T08:30:13Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments
- machine.rs: disabled_targets -> disabled_directories with DirectoryName type, methods renamed to is_directory_disabled/disable_directory
- status.rs: SourceStatus + TargetStatus merged into DirectoryStatus with role description, gather() iterates config.directories
- doctor.rs: diagnostics use config.directories and distribution_dirs(), messages reference "directory" not "source"/"target"
- manifest.rs and lockfile.rs: doc comments updated for directory terminology (field names preserved for serialization compat)
- 66 unit tests pass across all five modules

## Task Commits

Each task was committed atomically:

1. **Task 1: Update manifest.rs, lockfile.rs, and machine.rs for directory naming** - `73135b8` (feat)
2. **Task 2: Merge SourceStatus/TargetStatus into DirectoryStatus, update doctor.rs** - `3323835` (feat)

## Files Created/Modified
- `crates/tome/src/manifest.rs` - Updated doc comments (source_name field preserved, docs say "directory")
- `crates/tome/src/lockfile.rs` - Updated doc comments (source_name maps to [directories.*])
- `crates/tome/src/machine.rs` - Renamed disabled_targets -> disabled_directories, TargetName -> DirectoryName
- `crates/tome/src/status.rs` - Replaced SourceStatus + TargetStatus with DirectoryStatus, unified render
- `crates/tome/src/doctor.rs` - Directory-aware diagnostics, check_distribution_dir, directory_issues
- `crates/tome/src/lib.rs` - Updated callers: warn_unknown_disabled_directories, is_directory_disabled
- `crates/tome/src/relocate.rs` - Updated field reference: target_issues -> directory_issues

## Decisions Made
- Updated lib.rs and relocate.rs callers inline for machine.rs field rename instead of adding compat shims -- two callers is simple enough to fix directly.
- Separated count_skill_dirs (for discovery directories) and count_symlinks (for target-only directories) as distinct counting strategies in status.rs.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated lib.rs callers for machine.rs rename**
- **Found during:** Task 1
- **Issue:** lib.rs referenced `disabled_targets`, `is_target_disabled`, and `warn_unknown_disabled_targets` which no longer exist after machine.rs rename
- **Fix:** Updated to `disabled_directories`, `is_directory_disabled`, `warn_unknown_disabled_directories`
- **Files modified:** crates/tome/src/lib.rs
- **Verification:** cargo test passes
- **Committed in:** 73135b8 (part of Task 1 commit)

**2. [Rule 3 - Blocking] Updated relocate.rs field reference**
- **Found during:** Task 2
- **Issue:** relocate.rs referenced `report.target_issues` which was renamed to `directory_issues` in DoctorReport
- **Fix:** Updated field reference
- **Files modified:** crates/tome/src/relocate.rs
- **Verification:** cargo test passes
- **Committed in:** 3323835 (part of Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 blocking)
**Impact on plan:** Both fixes required for compilation. No scope creep.

## Issues Encountered
None beyond the blocking fixes documented above.

## User Setup Required
None - no external service configuration required.

## Known Stubs
None - all types and functions are fully implemented with real logic.

## Next Phase Readiness
- State/reporting modules fully migrated to directory terminology
- Deprecated compat types (Source, SourceType, TargetConfig, TargetMethod, TargetName) still referenced by discover.rs, wizard.rs, distribute.rs, lib.rs sync pipeline
- Plans 01-02, 01-04, 01-05 will convert remaining modules

## Self-Check: PASSED

All 7 modified files verified present. Both task commits (73135b8, 3323835) verified in git log.

---
*Phase: 01-unified-directory-foundation*
*Completed: 2026-04-12*
