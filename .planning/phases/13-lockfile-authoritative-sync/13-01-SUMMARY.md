---
phase: 13-lockfile-authoritative-sync
plan: 01
subsystem: cli
tags: [serde, clap, machine-toml, sync-options, schema]

# Dependency graph
requires:
  - phase: 12-marketplace-adapter
    provides: MarketplaceAdapter trait + ClaudeMarketplaceAdapter (consumed by Plan 13-04 reconcile loop, not this plan)
provides:
  - AutoInstall enum (3-state: Always | Ask | Never) with lowercase serde rename_all
  - MachinePrefs.auto_install_plugins: Option<AutoInstall> field with skip_serializing_if = "Option::is_none"
  - --no-install CLI flag on tome sync (RECON-02 D-09)
  - SyncOptions.no_install: bool field plumbed end-to-end (CLI → run() dispatch → sync() body)
affects: [13-02, 13-03, 13-04, 13-05]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "3-state consent enum with None-as-unset distinguished from Some(Ask) (RECON-02 D-07)"
    - "skip_serializing_if = \"Option::is_none\" for backward-compatible TOML evolution"
    - "Underscore-prefix unused variable in destructure pending downstream consumer plan"

key-files:
  created: []
  modified:
    - crates/tome/src/machine.rs
    - crates/tome/src/cli.rs
    - crates/tome/src/lib.rs

key-decisions:
  - "AutoInstall variants serialize as lowercase string values via #[serde(rename_all = \"lowercase\")] — matches CONTEXT.md D-08 convention"
  - "auto_install_plugins is Option<_> so absent field (None) signals 'first-time prompt' distinguished from Some(Ask) which signals 'user picked n last time, ask again'"
  - "--no-install is single-run scope only (CONTEXT.md D-09) — doesn't touch persisted machine.toml setting; mirrors Cargo --frozen/--locked"
  - "no_install destructure binds as _no_install in sync() body to avoid -D warnings until Plan 13-04 wires the consumer"
  - "Init post-wizard sync site populated with no_install: false (init never wants to skip install of fresh plugins)"

patterns-established:
  - "Schema-only plan: types and CLI surface land in Plan 13-01; logic that consumes them lands in subsequent plans (13-04 reconcile.rs)"
  - "Test cluster anchor pattern: new auto_install_* tests appended after directory_overrides_* cluster, mirroring earlier PORT-01 test layout"

requirements-completed: [RECON-02]

# Metrics
duration: 6m
completed: 2026-05-05
---

# Phase 13 Plan 01: Schema scaffolding for auto-install + --no-install Summary

**AutoInstall 3-state enum + auto_install_plugins field on MachinePrefs + --no-install CLI flag plumbed through SyncOptions, backward-compatible with existing machine.toml files**

## Performance

- **Duration:** ~6 min
- **Started:** 2026-05-05T20:57:21Z
- **Completed:** 2026-05-05T21:03:32Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- `AutoInstall` enum with `Always | Ask | Never` variants serializing as lowercase strings
- `MachinePrefs.auto_install_plugins: Option<AutoInstall>` field — `None` means "first-time prompt"; absent on disk via `skip_serializing_if`
- 6 new schema tests covering default, round-trip (all 3 variants), omit-on-empty, lowercase serde, backward compat (existing machine.toml without field), unknown-value rejection
- `tome sync --no-install` CLI flag parsed by clap and shown in `--help`
- `SyncOptions::no_install: bool` plumbed end-to-end from `Command::Sync` through `run()` dispatch into `sync()` body
- Init post-wizard sync site updated with `no_install: false`
- All 38 `machine::tests` pass (32 prior + 6 new); 605 lib tests pass; clippy clean with `-D warnings`

## Task Commits

Each task was committed atomically:

1. **Task 1: Add AutoInstall enum and auto_install_plugins field to machine.rs** — `bedd0e2` (feat) — TDD: tests + implementation in single commit since both are tightly coupled to the same schema change
2. **Task 2: Add --no-install flag to Command::Sync and plumb through SyncOptions** — `1832324` (feat)

## Files Created/Modified

- `crates/tome/src/machine.rs` — added `AutoInstall` enum (10 lines), `auto_install_plugins` field on `MachinePrefs` (5 lines), 6 new tests (~120 lines).
- `crates/tome/src/cli.rs` — added `no_install: bool` arm to `Command::Sync` with doc comment + updated `after_help` example.
- `crates/tome/src/lib.rs` — added `no_install` field to `SyncOptions` struct (1 line); destructured `no_install` from `Command::Sync` and passed into `SyncOptions { ... }` literal in dispatch (3 lines added); populated `no_install: false` at the post-init sync site (1 line); destructured as `no_install: _no_install` in `sync()` body (1 line).

## Decisions Made

- **`#[serde(rename_all = "lowercase")]` for AutoInstall** — emits `"always" | "ask" | "never"` matching CONTEXT.md D-08 prompt option labels. Default rust-cased PascalCase serialization (`"Always"`) would have looked unidiomatic in TOML.
- **`Option<AutoInstall>` (not `AutoInstall` with a `FirstTimePrompt` variant)** — keeps the on-disk surface minimal: an unset machine has no `auto_install_plugins` line at all (skip_serializing_if). A 4th `FirstTime` variant would have polluted serialization.
- **Underscore-prefix the unused variable** — Plan said "rename to `_no_install` if clippy complains; revert in Plan 13-04". Rustc emitted `unused_variables` warning (which `-D warnings` treats as fatal), so renamed.
- **Populated init-site with `no_install: false`** — the post-wizard sync runs in a freshly-initialized environment where install behavior should follow the persisted (or first-time-prompted) consent. Not a CLI knob, so `false` is correct.

## Deviations from Plan

None — plan executed exactly as written, including the planned `_no_install` rename when the unused-variable warning fired (the plan anticipated this contingency).

One operational note: parallel agents (Plan 13-02 + Plan 13-01) ran simultaneously, and an intermediate revert by another agent's tooling required re-applying the Task 2 cli.rs and lib.rs edits once. No code-content deviation — same edits, retried after the parallel-agent revert. Final commit `1832324` reflects the intended Task 2 state.

## Issues Encountered

- **Parallel-agent contention on cli.rs and lib.rs**: While iterating on Task 2, a parallel agent (executing Plan 13-02) reverted my in-progress edits to `cli.rs` and `lib.rs`. I detected this via `cargo build` failing with `missing field 'no_install'` and a manual `rg "no_install"` returning empty matches. Re-applied the edits, rebuilt, ran clippy, and committed cleanly. No rework on Task 1 (machine.rs) since it was already committed (`bedd0e2`) before the parallel revert hit.
- **Pre-existing flake**: `browse::app::tests::copy_path_retry_helper_returns_within_bound` failed once during the first full lib test run, then passed in isolation. Known timing-sensitive test; folded into Phase 15 / HARD-14 (issue #500) per STATE.md.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

Plan 13-02 (test-support feature gate) was committed in parallel as `b47fd58`. Plan 13-04 (reconcile loop) can now consume:
- `AutoInstall` enum + `MachinePrefs::auto_install_plugins` field for the 3-state prompt + persistence
- `SyncOptions::no_install` boolean to gate the apply loop

When Plan 13-04 wires the consumer, the destructure should be renamed back from `no_install: _no_install` to `no_install` (the plan explicitly anticipates this).

## Verification Output

```
$ cargo test -p tome machine::tests::auto_install
test machine::tests::auto_install_default_is_none ... ok
test machine::tests::auto_install_unset_omitted_on_save ... ok
test machine::tests::auto_install_lowercase_serde ... ok
test machine::tests::auto_install_unknown_value_rejected ... ok
test machine::tests::auto_install_existing_machine_toml_without_field_parses ... ok
test machine::tests::auto_install_round_trip ... ok
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 599 filtered out

$ cargo test -p tome machine::tests
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 568 filtered out

$ cargo clippy -p tome --all-targets -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s)

$ ./target/debug/tome sync --help | grep -A1 -- "--no-install"
      --no-install
          Skip auto-install/update of missing or drifted managed plugins this run.

$ ./target/debug/tome sync --no-install --dry-run --no-input
[dry-run] No changes will be made
... (downstream library-shape error from running on real library — flag PARSED clean)
```

---
*Phase: 13-lockfile-authoritative-sync*
*Completed: 2026-05-05*

## Self-Check: PASSED

- SUMMARY.md exists at `.planning/phases/13-lockfile-authoritative-sync/13-01-SUMMARY.md`
- Task 1 commit `bedd0e2` exists in git log
- Task 2 commit `1832324` exists in git log
- `pub enum AutoInstall` present in `crates/tome/src/machine.rs:29`
- `auto_install_plugins: Option<AutoInstall>` present in `crates/tome/src/machine.rs:94`
- `no_install: bool` present in `crates/tome/src/cli.rs:102`
- `no_install: bool` present in `crates/tome/src/lib.rs:772`
- All 6 new `machine::tests::auto_install_*` tests pass
- `cargo clippy -p tome --all-targets -- -D warnings` exits 0
- `tome sync --help` displays `--no-install` flag
