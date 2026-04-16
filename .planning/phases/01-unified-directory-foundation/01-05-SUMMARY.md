---
plan: "01-05"
phase: "01-unified-directory-foundation"
status: complete
started: "2026-04-14T07:00:00Z"
completed: "2026-04-14T08:30:00Z"
duration: "~35min"
tasks_completed: 2
tasks_total: 2
---

# Plan 01-05 Summary: Integration Wiring & Final Assembly

## What Was Built
Wired all remaining modules to the unified directory model, removed all deprecated
compatibility shims, rewrote integration tests for the new TOML format, and verified
the full CI suite passes (fmt-check + clippy -D warnings + 338 unit + 89 integration tests).

## Key Changes

### Task 1: Wire remaining modules and remove deprecated shims
- **lib.rs**: Sync pipeline uses `config.distribution_dirs()` and `distribute_to_directory()`
- **eject.rs**: Full rewrite using `DirectoryName` and `config.distribution_dirs()`
- **install.rs**: Uses `config.directories.values()` with `DirectoryType::ClaudePlugins`
- **relocate.rs**: Uses `DirectoryName`, `DirectoryConfig`, `config.distribution_dirs()`
- **config.rs**: Removed all deprecated types (Source, SourceType, TargetName, TargetConfig,
  TargetMethod) and deprecated fields (sources, targets) from Config struct
- **discover.rs**: Removed `discover_source` compat shim
- **distribute.rs**: Removed `distribute_to_target` compat shim, fixed collapsible-if lint

### Task 2: Rewrite integration tests and CHANGELOG
- **cli.rs**: Converted `TestEnvBuilder` and `write_config` helpers to emit new `[directories.*]` format
- **cli.rs**: Converted all ~30 inline TOML string literals from old to new format
- **cli.rs**: Updated `disabled_targets` → `disabled_directories` in machine.toml generation
- **cli.rs**: Updated JSON field assertions and warning message assertions
- **Snapshots**: Updated `doctor_clean` and `status_empty_library` snapshots
- **CHANGELOG.md**: Added v0.6 migration instructions with before/after examples

## Key Files

### Modified
- `crates/tome/src/lib.rs` — Distribution loop + cleanup loop converted
- `crates/tome/src/config.rs` — 150+ lines of deprecated shims removed
- `crates/tome/src/eject.rs` — Full rewrite to unified types
- `crates/tome/src/install.rs` — Plugin discovery via directory values
- `crates/tome/src/relocate.rs` — Target symlink recreation via directories
- `crates/tome/src/discover.rs` — Dead compat shim removed
- `crates/tome/src/distribute.rs` — Compat shim removed, lint fixed
- `crates/tome/tests/cli.rs` — ~30 TOML format conversions
- `CHANGELOG.md` — Migration documentation

## Deviations
- Agent partially completed task 1 but left deprecated types in eject.rs, install.rs,
  relocate.rs, and lib.rs. Orchestrator completed the remaining conversions manually.
- Agent did not convert most integration test TOML strings. Orchestrator delegated
  mass conversion to a second agent.

## Self-Check: PASSED
- [x] All deprecated types removed from config.rs
- [x] No deprecated field references anywhere in codebase
- [x] cargo fmt -- --check passes
- [x] cargo clippy --all-targets -- -D warnings passes (zero warnings)
- [x] 338 unit tests pass
- [x] 89 integration tests pass
