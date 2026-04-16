---
phase: 02-git-sources-selection
plan: 02
subsystem: machine-prefs
tags: [machine-prefs, per-directory-filtering, skill-selection]
dependency_graph:
  requires: []
  provides: [DirectoryPrefs, is_skill_allowed, validate]
  affects: [distribute.rs]
tech_stack:
  added: []
  patterns: [locality-principle, allowlist-blocklist, serde-default]
key_files:
  created: []
  modified:
    - crates/tome/src/machine.rs
decisions:
  - "enabled field is Option<BTreeSet> (None = no allowlist, Some = exclusive allowlist)"
  - "is_skill_allowed marked #[allow(dead_code)] until wired in Plan 02-03"
  - "validate() called in load() to fail fast on invalid machine.toml"
metrics:
  duration: "4 minutes"
  completed: "2026-04-15"
requirements: [MACH-02, MACH-03, MACH-04, MACH-05]
---

# Phase 2 Plan 02: Per-Directory Skill Filtering Summary

Per-directory skill filtering in MachinePrefs with DirectoryPrefs struct, locality-based resolution (per-dir enabled > per-dir disabled > global disabled), and validation rejecting simultaneous disabled+enabled.

## What Was Done

### Task 1: Add DirectoryPrefs struct, per-directory field, validation, and is_skill_allowed (TDD)

**RED:** Wrote 13 failing tests covering all filtering combinations, TOML round-trips, validation, and backward compatibility.

**GREEN:** Implemented:
- `DirectoryPrefs` struct with `disabled: BTreeSet<SkillName>` (blocklist) and `enabled: Option<BTreeSet<SkillName>>` (allowlist)
- Extended `MachinePrefs` with `directory: BTreeMap<DirectoryName, DirectoryPrefs>` field
- `validate()` method that rejects directories with both disabled and enabled set (MACH-04)
- `is_skill_allowed()` method implementing D-08 locality principle resolution
- Updated `load()` to call `validate()` after TOML parsing

**Commit:** `b7acd2e`

## Verification

- `cargo test -p tome -- machine::tests` -- 25 tests pass (12 existing + 13 new)
- `cargo test -p tome -q` -- 351 unit + 89 integration tests pass
- `cargo clippy -p tome --all-targets -- -D warnings` -- clean
- `cargo fmt -p tome -- --check` -- clean

## Deviations from Plan

None -- plan executed exactly as written.

## Known Stubs

- `is_skill_allowed()` is marked `#[allow(dead_code)]` -- it will be wired into `distribute.rs` in Plan 02-03.
