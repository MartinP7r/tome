---
phase: 14-unowned-library-lifecycle
plan: 03
subsystem: cli
tags: [clap, subcommand, cli, refactor, breaking-change]

# Dependency graph
requires:
  - phase: 14-unowned-library-lifecycle
    provides: previous_source schema (14-01), SkillSummary type (14-02)
provides:
  - "Command::Remove restructured into nested clap subcommand with RemoveKind::Dir | RemoveKind::Skill"
  - "Command::Reassign carries new force: bool field for D-A1 collision-overwrite"
  - "lib.rs::run dispatch routes RemoveKind::Dir to existing flow byte-for-byte"
  - "lib.rs::run RemoveKind::Skill stub returning anyhow::bail (14-05 placeholder)"
  - "lib.rs::run Command::Reassign destructures force (14-04 placeholder via let _ = force;)"
  - "Integration tests in tests/cli.rs migrated from `tome remove <name>` to `tome remove dir <name>`"
  - "5 cli::tests unit tests covering new clap shapes + BREAKING rejection of bare `tome remove <name>`"
affects: [14-04-reassign-unowned-input, 14-05-remove-skill, 14-08-docs-and-integration-tests]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Nested clap subcommand pattern (mirrors Command::Backup → BackupCommand)"
    - "Placeholder dispatch arms via anyhow::bail + let _ = unused_field; for staged refactors"

key-files:
  created: []
  modified:
    - "crates/tome/src/cli.rs"
    - "crates/tome/src/lib.rs"
    - "crates/tome/tests/cli.rs"

key-decisions:
  - "Task 1 commit (cli.rs only) does not compile in isolation; first compile-clean state is at Task 2 commit. Acceptable in a planned refactor wave; project has no `every-commit-must-build` rule."
  - "Migrated all 10 tests/cli.rs `[remove, <name>]` invocations to `[remove, dir, <name>]` via single Edit replace_all (consistent indentation made the substitution exact)."
  - "Placeholder for force in Command::Reassign uses `let _ = force;` rather than `#[allow(unused_variables)]` — keeps the binding visible at the call-site so 14-04 can drop the shim with one line."
  - "Skill stub uses `let _ = (name, yes);` then `anyhow::bail!` — clippy-clean and cheap to delete in 14-05 when the real flow lands."

patterns-established:
  - "Nested clap subcommand for variant-typed CLI verbs: outer command holds `#[command(subcommand)] kind: KindEnum`, inner enum derives Subcommand. Identical to Backup → BackupCommand."

requirements-completed: [UNOWN-01, UNOWN-02]

# Metrics
duration: 5min
completed: 2026-05-07
---

# Phase 14 Plan 03: CLI Restructure Summary

**Replaced `tome remove <name>` with nested subcommand `tome remove dir|skill <name>` per D-API-2 and added `--force` to `tome reassign` per D-A1 — public CLI surface for plans 14-04 and 14-05 is now stable.**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-05-07T13:04:31Z
- **Completed:** 2026-05-07T13:09:20Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- `Command::Remove` is now a nested clap subcommand with `RemoveKind::Dir { name, force }` and `RemoveKind::Skill { name, yes }` variants. The outer variant uses `#[command(subcommand)]` per the BackupCommand pattern.
- `Command::Reassign` carries a new `force: bool` field. Help text and after_help examples updated.
- `lib.rs::run` dispatches `Command::Remove { kind } => match kind { ... }`. The `Dir` arm preserves the existing 100+ line plan/render/execute/save flow byte-for-byte; the `Skill` arm stubs with `anyhow::bail!("tome remove skill is not yet implemented — see Phase 14 plan 14-05")`.
- `Command::Reassign { skill, to, force }` destructures the new field; `let _ = force;` shim suppresses the unused warning until 14-04 wires it into `reassign::plan`.
- All 10 integration test sites in `tests/cli.rs` migrated from `["remove", "<name>"]` to `["remove", "dir", "<name>"]`.
- 5 new unit tests in `cli::tests` cover the new shapes and assert that bare `tome remove <name>` no longer parses (BREAKING per D-API-2; CHANGELOG entry deferred to plan 14-08).

## Task Commits

Each task was committed atomically:

1. **Task 1: Restructure `Command::Remove` and add `Reassign --force` in cli.rs** — `9155574` (feat)
2. **Task 2: Update `lib.rs::run` dispatch for the new shapes** — `960450d` (feat)

## Files Created/Modified

- `crates/tome/src/cli.rs` — `Command::Remove` becomes `{ kind: RemoveKind }`; new `RemoveKind` enum with `Dir { name, force }` and `Skill { name, yes }` variants; `Command::Reassign` gains `force: bool`; new `#[cfg(test)] mod tests` block with 5 clap-parse tests.
- `crates/tome/src/lib.rs` — `Command::Remove` dispatch now matches on `RemoveKind`; `Dir` arm preserves prior body verbatim; `Skill` arm stubs to `anyhow::bail`; `Command::Reassign` destructures `force` with placeholder shim.
- `crates/tome/tests/cli.rs` — 10 sites migrated from `["remove", "<name>"]` to `["remove", "dir", "<name>"]` via a single replace-all on the consistently-indented `"remove",\n` token.

## Verification

- `cargo build -p tome` — clean
- `cargo test -p tome --lib cli::tests` — 5/5 pass (parse_remove_dir_with_force, parse_remove_skill_with_yes, parse_remove_skill_short_y, parse_reassign_force_flag_recognised, old_shape_remove_with_bare_name_fails)
- `cargo test -p tome --lib` — 651/651 pass
- `cargo test -p tome --test cli` — 141/141 pass (including the migrated `test_remove_local_directory` and `remove_preserves_git_lockfile_entries`)
- `cargo test -p tome --test cli_sync_reconcile` — 10/10 pass
- `cargo clippy --all-targets -p tome -- -D warnings` — clean
- `cargo fmt --check` — clean
- `tome remove --help` — shows `dir` and `skill` subcommands with descriptions
- `tome reassign --help` — shows new `--force` flag with description

## Decisions Made

- **Two-commit sequence preferred over a single squash.** Plan defines two tasks; commits map 1:1. Task 1 cli.rs commit alone would not compile (lib.rs still expects the old shape until Task 2 lands), but the project has no contract that every commit builds in isolation, and the wave-2 sequential constraint guarantees the broken intermediate state is never visible to other plans.
- **`let _ = force;` over `#[allow(unused_variables)]`.** Keeps the unused binding obvious at the call-site so plan 14-04 can drop the shim in a single-line patch when wiring `force` into `reassign::plan`. Same strategy on the `RemoveKind::Skill { name, yes }` arm via `let _ = (name, yes);`.
- **Help text mentions D-API-1, D-API-2, D-A1 by ID.** Slightly verbose for end-user help but useful as a forward pointer when future readers grep for those decision IDs. Project precedent: existing comments throughout the codebase reference Phase + decision IDs liberally.

## Deviations from Plan

None — plan executed exactly as written.

The one minor observation worth recording (not a deviation):
- `cargo fmt` collapsed the multi-line `anyhow::bail!(\n    "...message..."\n)` in the `Skill` stub arm into a single-line `anyhow::bail!("...")`. The plan's example showed the multi-line form; rustfmt's preference wins. No semantic change.

## Issues Encountered

- One transient flake observed during the very first `make ci` run: `remove_preserves_git_lockfile_entries` failed once with a "post-sync lockfile must contain a myrepo entry with git_commit_sha set" precondition error. The test passed cleanly on the immediate re-run and on every subsequent invocation (`cargo test -p tome --test cli`, `make ci` again). The test relies on a `file://` git clone of a fresh upstream repo, which is a known flake source under parallel test execution. Not on the existing flake-list (`backup::tests::push_and_pull_roundtrip`); worth noting if it recurs but no Phase 14 work involved this code path.

## Next Phase Readiness

- Plan 14-04 (`reassign --force`): the `force` flag arrives at the dispatch site as `force: bool`. Wiring is a one-line change — drop the `let _ = force;` shim, extend `reassign::plan`'s signature, thread the value through. The CLI shape and integration tests are stable.
- Plan 14-05 (`tome remove skill`): the `RemoveKind::Skill { name, yes }` arm is a single-stub site to replace. The arm currently `bail!`s; 14-05 will swap in `remove::skill_plan/render/execute` calls. The CLI shape and clap parsing are already verified by `cli::tests::parse_remove_skill_with_yes` and `parse_remove_skill_short_y`.
- Plan 14-08 (CHANGELOG): the BREAKING change to `tome remove <name>` shape needs a v0.10 CHANGELOG entry. Verified by `cli::tests::old_shape_remove_with_bare_name_fails`.

## Self-Check: PASSED

- `crates/tome/src/cli.rs` — exists with `pub enum RemoveKind`, `Dir` variant, `Skill` variant, `kind: RemoveKind,` field, `force: bool` on Reassign — verified by `grep`.
- `crates/tome/src/lib.rs` — exists with `Command::Remove { kind } => match kind`, `RemoveKind::Dir`, `RemoveKind::Skill`, `tome remove skill is not yet implemented`, `Command::Reassign { skill, to, force }` — verified by `grep`.
- `crates/tome/tests/cli.rs` — 10 `"remove", "dir",` token-pairs present (was 10 `"remove",` originally; replace-all confirmed).
- Commit `9155574` — present in `git log` (Task 1: cli.rs).
- Commit `960450d` — present in `git log` (Task 2: lib.rs + tests/cli.rs).

---
*Phase: 14-unowned-library-lifecycle*
*Completed: 2026-05-07*
