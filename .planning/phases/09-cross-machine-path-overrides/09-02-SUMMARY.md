---
phase: 09-cross-machine-path-overrides
plan: 02
subsystem: config
tags: [machine-toml, directory-overrides, validation-surfacing, port-03, port-04, anyhow-error-classes]

# Dependency graph
requires:
  - phase: 09-cross-machine-path-overrides
    plan: 01
    provides: "Config::apply_machine_overrides, Config::load_with_overrides, Config::load_or_default_with_overrides, MachinePrefs.directory_overrides, DirectoryConfig.override_applied"
provides:
  - "config.rs: Config::warn_unknown_overrides(&self, prefs, warn: impl FnMut(String)) — typo guard helper, unit-testable via FnMut callback"
  - "config.rs: format_override_validation_error free function — wraps validate() Err with machine.toml-attribution message when (pre-override valid AND ≥1 override applied)"
  - "config.rs: load_with_overrides + load_or_default_with_overrides take machine_path: &Path (third arg) so wrapper messages can name the file to edit"
  - "tests/cli.rs: machine_override_unknown_target_warns_and_continues + machine_override_validation_failure_blames_machine_toml integration tests pin PORT-03 + PORT-04 end-to-end through the real CLI binary"
affects: [09-03-status-doctor-surfacing]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Discriminator pattern for blame attribution: only wrap an error in machine.toml-class messaging when (a) the same validate() succeeds against a pre-override clone of the config, AND (b) at least one override was actually applied. Otherwise the underlying tome.toml is what's broken — pass the raw error through unchanged."
    - "FnMut callback for testable warning emission: warn_unknown_overrides takes `mut warn: impl FnMut(String)` instead of writing to stderr directly. Unit tests capture into Vec<String>; production caller adapts via `|s| eprintln!(\"warning: {s}\")`. Mirrors the lib.rs::warn_unknown_disabled_directories shape but is unit-testable (the existing helper isn't)."
    - "Pre-override snapshot for diff messaging: load_with_overrides snapshots `BTreeMap<String, PathBuf>` of original paths BEFORE apply_machine_overrides runs. The wrapper can then show `was: <old>, in tome.toml` per overridden directory so users see what changed."
    - "Anyhow message-content conventions over typed errors: PORT-04 uses grep-able message content (`override-induced config error from machine.toml`, `directory_overrides`) rather than a typed enum variant. Matches existing tome conventions (Conflict/Why/hint message structure). v1.0 follow-up: migrate to a typed `OverrideValidationError` enum if a future caller needs programmatic detection."

key-files:
  created:
    - .planning/phases/09-cross-machine-path-overrides/09-02-SUMMARY.md
  modified:
    - crates/tome/src/config.rs
    - crates/tome/src/lib.rs
    - crates/tome/tests/cli.rs

key-decisions:
  - "Wrapper triggered ONLY when both (pre-override valid) AND (≥1 override applied). Three-case truth table: pre-invalid+no-override → raw, pre-valid+override-breaks-it → wrapped, pre-invalid+unrelated-override → raw (the override didn't cause the failure, blaming machine.toml would mislead). Tested explicitly by the matrix in load_with_overrides_pre_override_invalid_returns_raw_error / load_with_overrides_override_induces_invalid_returns_wrapped_error / load_with_overrides_override_unrelated_to_failure_returns_raw_error."
  - "warn_unknown_overrides signature uses FnMut(String) instead of writing to stderr directly so the caller (Config::load_with_overrides) decides emission, AND so unit tests can capture into a Vec without intercepting stderr. Sibling helper warn_unknown_disabled_directories in lib.rs uses the simpler eprintln-inline pattern; we deviated to make the warning string format unit-testable."
  - "load_with_overrides + load_or_default_with_overrides take machine_path: &Path as a third arg (between path/cli_path and prefs), not via reaching for machine::default_machine_path() at error-construction time. Explicit threading is clearer; lib.rs::run already has machine_path in scope."
  - "Anyhow message-content conventions instead of a typed `OverrideValidationError` variant for PORT-04. The 'distinct error class' is achieved by grep-able message content (`override-induced config error from machine.toml`) plus the structural pattern of header → diff → indented original → fix-hint. Tracked as a v1.0 follow-up to migrate to a typed enum if a future caller needs programmatic detection."
  - "Indented original validate() error using format!(\"{post_err:#}\").lines().map(|l| format!(\"  {l}\")) — the {:#} formatter exposes anyhow's chained context, so multi-line errors stay readable inside the wrapper."
  - "Negative assertion ('must NOT contain edit tome.toml') in PORT-04 integration test is the discriminator probe that verifies the wrapper's message correctly redirects user attention. Without this assertion, a wrapper that says BOTH machine.toml and tome.toml in the fix hint would still pass."

patterns-established:
  - "Validation-blame discrimination: when a derived value (overridden config) fails validation, attribute blame to the derivation (machine.toml) only when the source (tome.toml) WOULD have validated. Generalizes beyond overrides — applies any time a load pipeline composes inputs."
  - "FnMut emission seams: helpers that emit user-visible diagnostics take a callback so unit tests can capture without filesystem/stderr interception."

requirements-completed: [PORT-03, PORT-04]

# Metrics
duration: ~10min
completed: 2026-04-28
---

# Phase 09 Plan 02: Validation Surfacing Summary

**Surfacing layer over Plan 09-01's `apply_machine_overrides`: stderr `warning:` for typo'd override targets (PORT-03) and a distinct `override-induced` error class that names `machine.toml` when an override breaks `validate()` (PORT-04).**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-04-28T13:58:20Z
- **Completed:** 2026-04-28T14:09:04Z
- **Tasks:** 3 (all atomic commits)
- **Files modified:** 3 (config.rs, lib.rs, tests/cli.rs)

## Accomplishments

- New helper `Config::warn_unknown_overrides(&self, prefs, warn: impl FnMut(String))` walks `prefs.directory_overrides` and emits a typo-warning string for any override target not present in `self.directories`. Caller-supplied callback decides emission strategy — keeps the helper pure and the warning format unit-testable.
- New free function `format_override_validation_error` builds a structured wrapper around a raw `Config::validate()` error: header `override-induced config error from machine.toml`, per-directory diff (`work: /lib (was: /work-original, in tome.toml)`), indented original validate() text using `{:#}` so anyhow chained context surfaces, and a closing `To fix: edit \`<machine_path>\` (NOT tome.toml)` hint.
- `Config::load_with_overrides` and `Config::load_or_default_with_overrides` updated to take `machine_path: &Path` as a third positional argument so the wrapper can name the file to edit. `lib.rs::run` updated to pass `&machine_path` through.
- `load_with_overrides` body restructured: `expand_tildes` → `warn_unknown_overrides` (PORT-03 emit via stderr) → snapshot pre-override paths → `apply_machine_overrides` → `validate`. On failure, reconstruct pre-override config, re-validate, and wrap iff (pre-override valid AND ≥1 override applied). Otherwise pass the raw error through (the underlying tome.toml is what's broken).
- 9 new unit tests + 2 integration tests pin the contract end-to-end. All 514 lib tests + 134 integration tests + typos pass under `make ci`.

## Task Commits

Each task was committed atomically on `gsd/phase-09-cross-machine-path-overrides` with `--no-verify` (parallel-wave coordination with the sibling 09-03 agent):

1. **Task 1: `Config::warn_unknown_overrides` helper + 5 unit tests** — `918ab53` (feat)
2. **Task 2: `format_override_validation_error` wrapper + signature update + 4 unit tests** — `3ef9e56` (feat)
3. **Task 3: 2 integration tests for PORT-03 + PORT-04 end-to-end** — `0ff4985` (test)

## New API Signatures

### `crates/tome/src/config.rs`

```rust
impl Config {
    /// Emit a warning for each `[directory_overrides.<name>]` entry whose
    /// `<name>` does not match any key in `self.directories`. Caller-supplied
    /// `warn` closure receives the formatted message body (without the
    /// `warning:` prefix).
    ///
    /// Used by `Config::load_with_overrides` to surface PORT-03 typo guards.
    /// Mirrors `lib.rs::warn_unknown_disabled_directories` shape; differs in
    /// taking a callback instead of writing to stderr directly so the warning
    /// format can be unit-tested without stderr capture.
    pub(crate) fn warn_unknown_overrides(
        &self,
        prefs: &crate::machine::MachinePrefs,
        mut warn: impl FnMut(String),
    );

    pub fn load_with_overrides(
        path: &Path,
        machine_path: &Path,                       // NEW (was: 2 args)
        prefs: &crate::machine::MachinePrefs,
    ) -> Result<Self>;

    pub fn load_or_default_with_overrides(
        cli_path: Option<&Path>,
        machine_path: &Path,                       // NEW (was: 2 args)
        prefs: &crate::machine::MachinePrefs,
    ) -> Result<Self>;
}

/// Wrap a `Config::validate()` error caused by `[directory_overrides.*]`
/// rewriting paths. Names `machine.toml` (NOT `tome.toml`) and shows the
/// pre-override vs post-override paths so the user sees what changed.
fn format_override_validation_error(
    post_err: &anyhow::Error,
    pre_override_paths: &BTreeMap<String, PathBuf>,
    config: &Config,
    machine_path: &Path,
) -> anyhow::Error;
```

### Wrapper message template (PORT-04)

```text
override-induced config error from machine.toml

The following directory paths come from `[directory_overrides.<name>]` overrides:
  - <name>: <new_path> (was: <old_path>, in tome.toml)

These overrides made an otherwise-valid `tome.toml` fail validation:

  <indented original validate() error, anyhow chained context preserved via {:#}>

To fix: edit `<machine_path>` (NOT tome.toml). Either remove the override(s) above or change them to paths that don't conflict.
```

### `crates/tome/src/lib.rs::run`

```rust
let machine_path = resolve_machine_path(cli.machine.as_deref())?;
let machine_prefs = machine::load(&machine_path)?;
let config = Config::load_or_default_with_overrides(
    effective_config.as_deref(),
    &machine_path,                       // NEW (third arg)
    &machine_prefs,
)?;
```

## Discriminator Logic (PORT-04)

The wrapper applies **iff** both:

1. The pre-override config validates successfully (i.e., the underlying `tome.toml` is fine), AND
2. At least one override was actually applied (i.e., the merge step changed at least one path).

Truth table:

| pre-override `validate()` | overrides applied | post-override `validate()` | result        |
| ------------------------- | ----------------- | -------------------------- | ------------- |
| ok                        | none              | ok                         | Ok(config)    |
| ok                        | none              | err                        | raw err       |
| err                       | none              | err                        | raw err       |
| err                       | one+              | err (same root cause)      | raw err       |
| ok                        | one+              | ok                         | Ok(config)    |
| ok                        | one+              | err                        | wrapped err   |

The third-row case ("override unrelated to failure") is exercised by `load_with_overrides_override_unrelated_to_failure_returns_raw_error`. Without the discriminator, a typo'd override that doesn't actually rewrite anything would cause every pre-existing tome.toml validation error to wear the machine.toml label — a regression we explicitly guard against.

## Tests Added

**`crates/tome/src/config.rs` test module — 9 new tests:**

PORT-03 (`warn_unknown_overrides`):
- `warn_unknown_overrides_no_overrides_emits_nothing`
- `warn_unknown_overrides_known_target_emits_nothing`
- `warn_unknown_overrides_unknown_target_emits_one_warning`
- `warn_unknown_overrides_multiple_unknowns_emit_one_each` (alphabetical via BTreeMap)
- `warn_unknown_overrides_does_not_mutate_config` (defense-in-depth `&self` runtime check)

PORT-04 (`load_with_overrides` wrapping):
- `load_with_overrides_pre_override_invalid_returns_raw_error`
- `load_with_overrides_override_induces_invalid_returns_wrapped_error`
- `load_with_overrides_override_unrelated_to_failure_returns_raw_error` (the discriminator)
- `load_with_overrides_path_appears_in_wrapper_message` (target name + new + old all present)

The pre-existing PORT-02 tests `load_with_overrides_runs_in_order_expand_apply_validate` and `load_with_overrides_validate_failure_propagates` and `override_applied_field_starts_false_after_load` were updated to pass the new `machine_path` arg and continue to pass — the I2 invariant test confirms wrapper changes did not regress the load order.

**`crates/tome/tests/cli.rs` — 2 new integration tests:**

- `machine_override_unknown_target_warns_and_continues` (PORT-03 end-to-end): asserts stderr contains `warning:` + `claud` + `machine.toml`, command succeeds.
- `machine_override_validation_failure_blames_machine_toml` (PORT-04 end-to-end): asserts stderr contains `machine.toml` + `library_dir overlaps` + (`override-induced` OR `directory_overrides`), AND does NOT contain "edit tome.toml" / "Edit tome.toml" (negative assertion = the discriminator probe).

## Files Created/Modified

**Modified:**
- `crates/tome/src/config.rs` — `warn_unknown_overrides` helper, signature change on `load_with_overrides` + `load_or_default_with_overrides`, body restructure (warn → snapshot → apply → validate → wrap-or-pass), `format_override_validation_error` free function, 9 new unit tests + 3 existing tests updated for new signature
- `crates/tome/src/lib.rs` — single call site update to pass `&machine_path` to `load_or_default_with_overrides`
- `crates/tome/tests/cli.rs` — 2 new integration tests for PORT-03 + PORT-04

**Created:**
- `.planning/phases/09-cross-machine-path-overrides/09-02-SUMMARY.md`

## Decisions Made

All decisions are recorded in the frontmatter `key-decisions` field. The most consequential ones:

1. **Discriminator gates the wrapper:** wrap iff (pre-override valid) AND (≥1 override applied). Three-case truth table tested explicitly. Without the discriminator, a typo'd override (which silently no-ops) would cause every pre-existing tome.toml validation error to be misattributed to machine.toml.
2. **FnMut callback for warn_unknown_overrides:** deviation from the sibling helper `warn_unknown_disabled_directories` (which uses inline `eprintln!`). Trade-off: one extra closure layer in the production caller, in exchange for unit-testable warning format. The sibling has zero unit tests on its warning format; we wanted ours pinned.
3. **Message-content conventions over typed errors for PORT-04:** matches existing tome anyhow conventions. v1.0 follow-up tracked: if a future caller needs programmatic detection of override-induced errors, migrate to a typed `OverrideValidationError` enum.
4. **Explicit `machine_path` parameter:** clearer than reaching for `machine::default_machine_path()` at error-construction time. `lib.rs::run` already has `machine_path` in scope, so the cost is one parameter.
5. **Indented original error using `{:#}` formatter:** preserves anyhow's chained context inside the wrapper. Multi-line `Conflict: / Why: / hint:` blocks stay readable.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Hygiene] `warn_unknown_overrides` triggered `dead_code` warning between Task 1 and Task 2**

- **Found during:** Task 1 final clippy gate
- **Issue:** After committing Task 1's helper but before Task 2 wires it into `load_with_overrides`, `cargo clippy --all-targets -- -D warnings` failed with `method warn_unknown_overrides is never used`. Identical situation to the Plan 09-01 `override_applied` field deviation.
- **Fix:** Added `#[allow(dead_code)] // Wired into load_with_overrides in Task 2 of this plan.` to the helper. Removed at the start of Task 2 once the helper was wired in. The Task 1 commit carries the allow; the Task 2 commit removes it as part of the body restructure.
- **Files modified:** `crates/tome/src/config.rs`
- **Verification:** Task 1 commit has clean clippy; Task 2 commit removes the attribute and clippy stays clean (helper now has a real caller).
- **Committed in:** `918ab53` (added) → `3ef9e56` (removed)

**2. [Rule 1 - Bug] rustfmt collapsed multi-line `let any_override_applied = ...` after Task 2 commit**

- **Found during:** Task 3 acceptance verification (final `cargo fmt -- --check`)
- **Issue:** Task 2 introduced a multi-line `let any_override_applied = config.directories.values().any(...)` because the line just barely exceeded rustfmt's threshold inside the new wrap-or-pass branch. After the build pulled in some indentation tweaks elsewhere, rustfmt re-evaluated and collapsed it to a one-liner. Identical pattern to deviation #3 in Plan 09-01-SUMMARY.md.
- **Fix:** Folded the formatting collapse into the Task 3 commit (matches the 09-01 precedent). No semantic change.
- **Files modified:** `crates/tome/src/config.rs` (formatting only)
- **Verification:** `cargo fmt -- --check` clean; full test suite still passes.
- **Committed in:** `0ff4985` (Task 3 commit, body documents the fold)

**3. [Out-of-scope artifact] Sibling agent (Plan 09-03) committed an in-progress test (`994f952`) on the same branch between my Task 1 and Task 2**

- **Found during:** Task 2 first clippy run (immediately after the helper signature change to `&machine_path`)
- **Issue:** `cargo clippy --all-targets -- -D warnings` reported errors in `crates/tome/src/doctor.rs` referencing fields (`DirectoryDiagnostic`, `override_applied`) that the sibling agent's RED commit hadn't yet provided GREEN production code for. These were NOT in scope for my plan.
- **Fix:** None on my side — verified my own work compiles via `cargo build --all-targets` (which passes), and confirmed my own tests pass via name-filtered `cargo test`. Per the parallel-wave contract documented in this plan's `<parallel_execution>` block, the orchestrator runs hooks once after both Wave 2 plans complete; sibling agent GREEN'd their RED tests in commit `fabbb6b` shortly after. Final `make ci` is clean.
- **Files affected on my side:** None.
- **Verification:** Final `make ci` after both wave-2 agents finished: 514 lib + 134 integration + typos all pass.

**4. [Pre-existing flake — out of scope] `remove_preserves_git_lockfile_entries` failed in the first `make ci` run**

- **Found during:** First post-task-3 full `make ci`
- **Issue:** The git-fixture-based test `remove_preserves_git_lockfile_entries` failed with a precondition assertion. Test passes in isolation (`cargo test remove_preserves_git_lockfile_entries`); fails intermittently in the full integration test suite. Matches the documented pre-existing flake pattern from PROJECT.md ("Pre-existing flaky test ... passes in isolation, intermittent in full suite") and from 09-01-SUMMARY.md ("Backup test flakiness in full lib suite").
- **Fix:** None — out of scope per deviation rules. Re-ran `make ci` and it passed cleanly.
- **Verification:** Re-run of `make ci` post-flake: clean, all tests pass.

---

**Total deviations:** 4 — all hygiene/parallel-coordination/pre-existing flake. Plan structure intact, scope unchanged. No architectural shifts.

## Issues Encountered

- **Sibling agent test flakiness in cli.rs target compilation** — between my Task 1 commit and the start of Task 2, the sibling agent (09-03) committed RED-style failing tests that broke `cargo test --lib` compilation when both agents' work-in-progress combined. Resolved automatically when the sibling GREEN'd their tests. Documented as deviation #3 above.
- **Pre-existing flake in `remove_preserves_git_lockfile_entries`** — see deviation #4. Not a regression introduced by this plan.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

**Plan 09-02 complete; ready for any future cross-machine work.** The artifacts this plan provides:

- `Config::warn_unknown_overrides` is callable from any context that has a `Config` and `MachinePrefs` reference; future wizard/doctor surfacing can reuse it without refactoring.
- `format_override_validation_error` could be promoted to `pub(crate)` if other load paths need similar wrapping; for now it's `fn`-private to the module since `load_with_overrides` is the only caller.
- The PORT-04 wrapper text is grep-able via `override-induced config error from machine.toml`, providing a stable contract for downstream tooling that wants to match these errors.

**Plan 09-03 (status/doctor surfacing, PORT-05)** is being executed in parallel by a sibling agent in this same wave. Their commits land on the same branch alongside mine; the orchestrator validates the final state at end-of-wave.

No blockers. No carry-over UAT items from this plan.

## Self-Check: PASSED

Verified by direct filesystem and git checks before writing this section:

- File `.planning/phases/09-cross-machine-path-overrides/09-02-SUMMARY.md`: FOUND (this file)
- Commit `918ab53` (Task 1, feat warn_unknown_overrides): FOUND in `git log --oneline`
- Commit `3ef9e56` (Task 2, feat format_override_validation_error): FOUND in `git log --oneline`
- Commit `0ff4985` (Task 3, test integration tests): FOUND in `git log --oneline`
- `rg -n "pub\(crate\) fn warn_unknown_overrides" crates/tome/src/config.rs`: 1 match (line 580)
- `rg -n "fn format_override_validation_error" crates/tome/src/config.rs`: 1 match (line 759)
- `rg -n "override-induced config error from machine.toml" crates/tome/src/config.rs`: 2 matches (line 754 doc-comment reference + line 791 the actual error string)
- `rg -nA4 "Config::load_or_default_with_overrides\(" crates/tome/src/lib.rs`: shows the call site on line 296 passing 3 args (`effective_config.as_deref()`, `&machine_path`, `&machine_prefs`)
- `rg -n "machine_override_unknown_target_warns_and_continues" crates/tome/tests/cli.rs`: 1 match (line 5274)
- `rg -n "machine_override_validation_failure_blames_machine_toml" crates/tome/tests/cli.rs`: 1 match (line 5323)
- `make ci`: clean (514 lib + 134 integration + typos)

---
*Phase: 09-cross-machine-path-overrides*
*Plan: 02*
*Completed: 2026-04-28*
