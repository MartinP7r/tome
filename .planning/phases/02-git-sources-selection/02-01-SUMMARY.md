---
phase: 02-git-sources-selection
plan: 01
subsystem: git
tags: [git, sha256, subprocess, config, paths]

# Dependency graph
requires:
  - phase: 01-unified-directory-foundation
    provides: DirectoryConfig with DirectoryType::Git, branch/tag/rev fields, validation
provides:
  - git.rs module with clone, update, SHA-reading, URL hashing, env clearing
  - DirectoryConfig.subdir field for monorepo skill subdirectories
  - TomePaths.repos_dir() for git cache path
affects: [02-git-sources-selection plans 02-04 that wire git into sync pipeline]

# Tech tracking
tech-stack:
  added: []
  patterns: [git subprocess with env_remove for GIT_DIR/GIT_WORK_TREE/GIT_INDEX_FILE, SHA-256 URL hashing for cache dirs, shallow clone with depth 1]

key-files:
  created: [crates/tome/src/git.rs]
  modified: [crates/tome/src/config.rs, crates/tome/src/paths.rs, crates/tome/src/wizard.rs, crates/tome/src/discover.rs, crates/tome/src/distribute.rs, crates/tome/src/doctor.rs, crates/tome/src/eject.rs, crates/tome/src/relocate.rs, crates/tome/src/status.rs]

key-decisions:
  - "Used #![allow(dead_code)] on git.rs since functions are wired in by Plan 03"
  - "SHA-256 URL hashing uses per-byte format!(\"{:02x}\") matching manifest.rs pattern"

patterns-established:
  - "Git subprocess env clearing: every Command::new(\"git\") chains .env_remove for GIT_DIR, GIT_WORK_TREE, GIT_INDEX_FILE"
  - "Shallow clone + fetch/reset pattern: clone --depth 1, update via fetch --depth 1 origin <ref> + reset --hard FETCH_HEAD"

requirements-completed: [GIT-01, GIT-02, GIT-03, GIT-04, GIT-05, GIT-06]

# Metrics
duration: 9min
completed: 2026-04-15
---

# Phase 2 Plan 1: Git Module Foundation Summary

**Self-contained git.rs module with clone/update/SHA-reading plus subdir config field and repos_dir path method**

## Performance

- **Duration:** 9 min
- **Started:** 2026-04-15T13:31:22Z
- **Completed:** 2026-04-15T13:40:17Z
- **Tasks:** 2
- **Files modified:** 10

## Accomplishments
- Created git.rs module with 7 pub(crate) functions covering clone, update, SHA-reading, URL hashing, effective path, ref spec resolution, and git availability check
- All git subprocess calls clear GIT_DIR, GIT_WORK_TREE, GIT_INDEX_FILE environment variables (9 total env_remove calls)
- Added subdir field to DirectoryConfig with git-only validation
- Added repos_dir() to TomePaths returning tome_home/repos/
- 11 unit tests covering pure functions and read_head_sha integration

## Task Commits

Each task was committed atomically:

1. **Task 1: Create git.rs module with clone, update, SHA-reading, and URL hashing** - `45f223a` (feat)
2. **Task 2: Add subdir field to DirectoryConfig and repos_dir() to TomePaths** - `84316d1` (feat)

## Files Created/Modified
- `crates/tome/src/git.rs` - New module: git subprocess operations (clone, fetch, update, SHA reading, URL hashing)
- `crates/tome/src/config.rs` - Added subdir: Option<String> to DirectoryConfig with git-only validation
- `crates/tome/src/paths.rs` - Added repos_dir() method to TomePaths
- `crates/tome/src/lib.rs` - Added pub(crate) mod git declaration
- `crates/tome/src/wizard.rs` - Updated DirectoryConfig construction with subdir: None
- `crates/tome/src/discover.rs` - Updated DirectoryConfig construction with subdir: None
- `crates/tome/src/distribute.rs` - Updated DirectoryConfig construction with subdir: None
- `crates/tome/src/doctor.rs` - Updated DirectoryConfig construction with subdir: None
- `crates/tome/src/eject.rs` - Updated DirectoryConfig construction with subdir: None
- `crates/tome/src/relocate.rs` - Updated DirectoryConfig construction with subdir: None
- `crates/tome/src/status.rs` - Updated DirectoryConfig construction with subdir: None

## Decisions Made
- Used `#![allow(dead_code)]` module-level attribute on git.rs since all functions are pub(crate) but not yet wired into the sync pipeline (Plan 03 does that). This avoids clippy -D warnings without individual annotations.
- SHA-256 byte formatting uses the `.iter().map(|b| format!("{:02x}", b)).collect()` pattern matching manifest.rs, since sha2 0.11's `finalize()` output doesn't implement LowerHex.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed SHA-256 formatting for sha2 0.11 API**
- **Found during:** Task 1 (git.rs module creation)
- **Issue:** Plan specified `format!("{:x}", hasher.finalize())` but sha2 0.11's Array type doesn't implement LowerHex
- **Fix:** Used per-byte formatting matching existing manifest.rs pattern
- **Files modified:** crates/tome/src/git.rs
- **Verification:** Tests pass, hash output is correct 64-char hex
- **Committed in:** 45f223a (Task 1 commit)

**2. [Rule 3 - Blocking] Updated all DirectoryConfig construction sites for new subdir field**
- **Found during:** Task 2 (subdir field addition)
- **Issue:** DirectoryConfig struct doesn't derive Default, so all 29 construction sites across 8 files needed `subdir: None` added
- **Fix:** Added subdir: None to all construction sites using automated regex replacement
- **Files modified:** config.rs, wizard.rs, discover.rs, distribute.rs, doctor.rs, eject.rs, relocate.rs, status.rs
- **Verification:** 349 unit tests + 89 integration tests pass
- **Committed in:** 84316d1 (Task 2 commit)

**3. [Rule 3 - Blocking] Rebased worktree onto main to get Phase 1 changes**
- **Found during:** Task 2 start
- **Issue:** Worktree was created from a pre-Phase-1 commit; DirectoryConfig didn't exist
- **Fix:** Rebased onto main which has all Phase 1 unified directory changes
- **Verification:** DirectoryConfig and all Phase 1 types available after rebase

---

**Total deviations:** 3 auto-fixed (3 blocking)
**Impact on plan:** All auto-fixes necessary for compilation and correctness. No scope creep.

## Issues Encountered
None beyond the auto-fixed deviations.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- git.rs module ready for Plan 03 to wire into sync pipeline
- subdir field ready for effective_path() usage during git resolution
- repos_dir() ready for clone destination computation
- All existing tests pass (349 unit + 89 integration)

---
*Phase: 02-git-sources-selection*
*Completed: 2026-04-15*
