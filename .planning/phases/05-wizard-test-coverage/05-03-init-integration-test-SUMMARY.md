---
phase: 05-wizard-test-coverage
plan: 03
subsystem: testing
tags: [assert_cmd, integration-test, wizard, no-input, dry-run, toml-roundtrip]

# Dependency graph
requires:
  - phase: 05-wizard-test-coverage
    provides: "Plan 05-01 --no-input plumbing (wizard::run accepts no_input, lib.rs bail removed) and Config::directories()/library_dir()/exclude() pub accessors"
provides:
  - "End-to-end integration tests for `tome init --dry-run --no-input` covering empty-HOME and seeded-HOME shapes"
  - "`parse_generated_config` helper: splits wizard stdout on the `Generated config:` marker (wizard.rs:324) and parses the trailing TOML as `tome::config::Config`"
  - "`assert_config_roundtrips` helper: mirrors `Config::save_checked`'s round-trip guard (Phase 4 D-03) as a reusable test utility"
  - "Demonstrated pattern for driving wizard from integration tests via HOME + TOME_HOME + NO_COLOR env overrides (no TTY needed)"
affects: [future wizard integration tests, WIZ-01–05 deferred wizard rewrite verification]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Integration-test marker-split parsing: stdout.split_once(\"Generated config:\\n\") → toml::from_str::<Config>"
    - "Env-triple HOME + TOME_HOME + NO_COLOR isolation (no .env_clear() which breaks cargo_bin path resolution)"
    - "Accessor-only Config reads from external test crate (pub fn directories()/library_dir()/exclude() vs pub(crate) field access that would fail to compile)"

key-files:
  created: []
  modified:
    - "crates/tome/tests/cli.rs (+189 lines: 2 tests, 2 helpers, 1 use statement)"

key-decisions:
  - "Tests drive the real binary via assert_cmd rather than calling wizard::run() directly — proves the CLI entry point, global flag plumbing, and stdout emission all work end-to-end"
  - "Parse TOML via toml::from_str::<Config> rather than snapshot-matching — deserialization is the contract; snapshots would be brittle across TempDir paths and BTreeMap ordering"
  - "Set TOME_HOME explicitly alongside HOME to defensively neutralise default_tome_home's XDG fallback even though HOME override already handles it (belt-and-braces determinism)"
  - "Compare against EXPANDED paths (tmp.path().join(\".claude/plugins\")) because wizard.rs:311-317 runs expand_tildes() before emitting — this is a Phase 4 behavior that Plan 05-01's dry-run branch inherits"
  - "Import only the four symbols needed (Config, DirectoryName, DirectoryRole, DirectoryType) rather than tome::* — keeps the new imports localized and explicit"

patterns-established:
  - "Wizard integration test template: TempDir HOME + TOME_HOME + NO_COLOR=1 → parse `Generated config:` marker → assert validate() + round-trip — reusable for future init UX coverage"
  - "Test helpers live as free `fn` at module scope alongside existing `write_config`/`create_skill`/`snapshot_settings`, not inside a child mod — matches the file's established style"

requirements-completed: [WHARD-05]

# Metrics
duration: 10min
completed: 2026-04-20
---

# Phase 05 Plan 03: Init Integration Test Summary

**Two `assert_cmd` integration tests drive `tome init --dry-run --no-input` end-to-end against empty and seeded TempDir HOMEs, proving the generated Config validates and round-trips through TOML byte-equal.**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-04-20T09:18Z (approx)
- **Completed:** 2026-04-20T09:28Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments

- Closed WHARD-05: `tome init --dry-run --no-input` is now covered by real-binary integration tests — no more TTY-only gap.
- Proved Plan 05-01's `--no-input` plumbing works end-to-end: bail removed, wizard reaches the `Generated config:` emission branch, output is valid parseable TOML.
- Validated Plan 05-01 Part D accessors (`Config::directories()`, `library_dir()`, `exclude()`, `DirectoryConfig::role()`) in their intended use site — external-crate consumers can now read Config state without field-visibility hacks.
- Established a reusable pattern for future wizard integration tests: HOME + TOME_HOME + NO_COLOR env isolation, marker-split stdout parsing, round-trip assertion.

## Task Commits

1. **Task 1: Add two integration tests plus helpers to crates/tome/tests/cli.rs** — `14010e0` (test)

## Files Created/Modified

- `crates/tome/tests/cli.rs` — added 2 tests (`init_dry_run_no_input_empty_home`, `init_dry_run_no_input_seeded_home`), 2 helpers (`parse_generated_config`, `assert_config_roundtrips`), and 1 `use tome::config::{…}` import. No existing code reformatted.

## Decisions Made

None beyond the plan — followed Part A of the plan verbatim. Only minor non-substantive adjustment: rustfmt placed `unwrap_or_else(|| { panic!(…) })` closure bodies on their own lines (4-space continuation) rather than the plan's compact `.unwrap_or_else(|| panic!(…))` form. Functionally identical.

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

- `cargo test -p tome --test cli init_dry_run_no_input_empty_home init_dry_run_no_input_seeded_home` (plan's suggested verify command) errored because cargo treats the second test name as an unexpected argument unless separated by `--`. Used `cargo test -p tome --test cli -- init_dry_run_no_input_empty_home init_dry_run_no_input_seeded_home` instead. Not a test-code issue.

## User Setup Required

None — tests run headlessly in CI with no external config.

## Next Phase Readiness

- Phase 5 tasks complete: Plans 05-01 (no-input plumbing + assemble_config), 05-02 (wizard unit tests), 05-03 (this plan — init integration test), 05-04 (combo matrix) all landed.
- WHARD-04 (pure helpers), WHARD-05 (integration driver), WHARD-06 (combo matrix) all closed.
- No blockers for Phase 5 sign-off. Next: phase completion review + move to Phase 6 (browse polish / `tabled` display) or release.

## Verification

- `cargo test -p tome --test cli -- init_` → 6 passed (2 new + 4 pre-existing init-adjacent).
- `cargo fmt -- --check` → exit 0.
- `cargo clippy --all-targets -- -D warnings` → exit 0.
- `rg "fn init_dry_run_no_input_empty_home" crates/tome/tests/cli.rs` → 1 hit.
- `rg "fn init_dry_run_no_input_seeded_home" crates/tome/tests/cli.rs` → 1 hit.
- `rg "fn parse_generated_config" crates/tome/tests/cli.rs` → 1 hit.
- `rg "fn assert_config_roundtrips" crates/tome/tests/cli.rs` → 1 hit.
- `rg "use tome::config::" crates/tome/tests/cli.rs` → 1 hit.

## Self-Check: PASSED

- `crates/tome/tests/cli.rs` — FOUND (modified)
- Commit `14010e0` — FOUND
- All acceptance criteria from plan verified via rg + cargo test

---
*Phase: 05-wizard-test-coverage*
*Completed: 2026-04-20*
