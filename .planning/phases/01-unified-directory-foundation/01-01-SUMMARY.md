---
phase: 01-unified-directory-foundation
plan: 01
subsystem: config
tags: [serde, toml, newtype, validation, unified-directory]

requires: []
provides:
  - DirectoryName, DirectoryType, DirectoryRole, DirectoryConfig types
  - Config struct with directories BTreeMap
  - deny_unknown_fields with old-format migration hint
  - discovery_dirs(), distribution_dirs(), managed_dirs() convenience iterators
  - Config.validate() for role/type combo checking
  - Deprecated compat shims for Source, SourceType, TargetConfig, TargetMethod, TargetName
affects: [01-02, 01-03, 01-04, 01-05]

tech-stack:
  added: []
  patterns: [unified-directory-model, role-based-filtering, deprecated-compat-shims]

key-files:
  created: []
  modified: [crates/tome/src/config.rs]

key-decisions:
  - "Added deprecated compat shims instead of removing old types outright -- required for crate to compile while other modules still reference old types"
  - "Used #[serde(skip)] on deprecated sources/targets fields so deny_unknown_fields still rejects old TOML format"
  - "DirectoryType uses #[default] derive attribute instead of manual Default impl (clippy suggestion)"

patterns-established:
  - "DirectoryName newtype: same validation pattern as SkillName via validate_identifier()"
  - "Role defaulting chain: DirectoryType.default_role() called when role field is None"
  - "is_discovery()/is_distribution() methods on DirectoryRole for pipeline filtering"

requirements-completed: [CFG-01, CFG-02, CFG-03, CFG-04, CFG-05, CFG-06]

duration: 8min
completed: 2026-04-12
---

# Phase 1 Plan 1: Config Type System Summary

**Unified directory type system (DirectoryName/Type/Role/Config) replacing Source/TargetName/TargetConfig with deny_unknown_fields, migration hint, validation, and convenience iterators**

## Performance

- **Duration:** 8 min
- **Started:** 2026-04-12T08:08:46Z
- **Completed:** 2026-04-12T08:16:50Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments
- Complete config.rs rewrite with new unified directory type system
- 39 unit tests covering all new types, TOML parsing, validation, and convenience iterators
- Old-format TOML files rejected with clear migration hint
- Deprecated backward-compat shims keep the full crate compilable during migration

## Task Commits

Each task was committed atomically:

1. **Task 1: Define new directory types and rewrite Config struct** - `37b904c` (feat)

## Files Created/Modified
- `crates/tome/src/config.rs` - Complete rewrite: new DirectoryName, DirectoryType, DirectoryRole, DirectoryConfig types; updated Config struct with directories BTreeMap; deny_unknown_fields; validation; convenience iterators; deprecated compat shims

## Decisions Made
- Added deprecated compat shims (Source, SourceType, TargetConfig, TargetMethod, TargetName as type alias) with `#[deprecated]` and `#[serde(skip)]` so other modules compile during migration. Without this, `cargo test -p tome config::tests` cannot run because the whole crate must compile.
- Used `#[default]` derive on DirectoryType::Directory variant instead of manual Default impl (per clippy suggestion).
- Kept `sources` and `targets` as `#[serde(skip)]` fields on Config so field access in other modules compiles. These are always empty and will be removed in plans 01-02 through 01-05.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added deprecated compatibility shims for old types**
- **Found during:** Task 1 (compilation of full crate)
- **Issue:** Removing Source, SourceType, TargetConfig, TargetMethod, TargetName caused 48 compilation errors across 10+ modules. `cargo test -p tome config::tests` requires the full crate to compile.
- **Fix:** Added deprecated type stubs with `#[deprecated]`, `#[allow(deprecated)]`, and `#[serde(skip)]` annotations. TargetName became a type alias for DirectoryName.
- **Files modified:** crates/tome/src/config.rs
- **Verification:** `cargo test -p tome config::tests` passes (39 tests), `cargo clippy` shows no config.rs errors
- **Committed in:** 37b904c (part of task commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Required for compilation. No scope creep. Deprecated types clearly marked for removal in subsequent plans.

## Issues Encountered
None beyond the compilation blocking issue addressed above.

## User Setup Required
None - no external service configuration required.

## Known Stubs
None - all types are fully implemented with real logic.

## Next Phase Readiness
- New type system is complete and tested
- All other modules compile against deprecated compat shims
- Plans 01-02 through 01-05 will convert each module to use new types and remove compat shims

---
*Phase: 01-unified-directory-foundation*
*Completed: 2026-04-12*
