---
phase: 09-cross-machine-path-overrides
verified: 2026-04-26T00:00:00Z
status: passed
score: 5/5 must-haves verified (PORT-01 through PORT-05); 0 gaps
re_verification:
  is_re_verification: false
---

# Phase 9: Cross-Machine Path Overrides Verification Report

**Phase Goal:** A single `tome.toml` checked into dotfiles can be applied across machines with different filesystem layouts via per-machine `[directory_overrides.<name>]` blocks in `machine.toml`.

**Verified:** 2026-04-26
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (PORT-01 .. PORT-05)

| #     | Truth (requirement)                                                                                                                                  | Status     | Evidence                                                                                                                                                                                                                                                                                              |
| ----- | ---------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| PORT-01 | User can declare `[directory_overrides.<name>]` blocks in `machine.toml` to remap a directory's `path` without editing `tome.toml`                  | ✓ VERIFIED | `pub struct DirectoryOverride { pub path: PathBuf }` at `crates/tome/src/machine.rs:29` with `#[serde(deny_unknown_fields)]`; `pub(crate) directory_overrides: BTreeMap<DirectoryName, DirectoryOverride>` on `MachinePrefs` (line 71). Unit tests `directory_overrides_*` (5 tests) all pass. |
| PORT-02 | Per-machine overrides apply at config load time (after tilde expansion, before `Config::validate`); all downstream code sees merged result          | ✓ VERIFIED | `Config::apply_machine_overrides` at `config.rs:554`, `load_with_overrides` at `config.rs:615`, `load_or_default_with_overrides` at `config.rs:681`. `lib.rs:296` calls `Config::load_or_default_with_overrides` for the canonical post-Init load path. Order test `load_with_overrides_runs_in_order_expand_apply_validate` passes; integration test `machine_override_rewrites_directory_path_for_status` proves status sees merged path. |
| PORT-03 | Override targeting an unknown directory name produces stderr `warning:` line (typo guard) without aborting load                                       | ✓ VERIFIED | `Config::warn_unknown_overrides` at `config.rs:580`. Wired into `load_with_overrides` body. 5 unit tests `warn_unknown_overrides_*` all pass; integration test `machine_override_unknown_target_warns_and_continues` (cli.rs:5274) asserts stderr contains `warning:` + typo target + `machine.toml`, command succeeds. |
| PORT-04 | Validation failures triggered by an override surface as a distinct error class naming `machine.toml` (not `tome.toml`)                                | ✓ VERIFIED | `format_override_validation_error` at `config.rs:759`. Discriminator logic at `config.rs:664-666` (`pre_override_valid && any_override_applied`). Wrapper string `"override-induced config error from machine.toml"` at line 791. 4 unit tests cover the truth table; integration test `machine_override_validation_failure_blames_machine_toml` (cli.rs:5323) includes the negative-assertion discriminator probe. |
| PORT-05 | `tome status` and `tome doctor` indicate which directory entries are subject to a per-machine override (text + JSON)                                  | ✓ VERIFIED | `DirectoryStatus.override_applied: bool` at `status.rs:53`, `format_dir_path_column` at `status.rs:126` (renders ` (override)`), wired in render loop at line 200. `DirectoryDiagnostic` struct at `doctor.rs:37` with `override_applied: bool` field replaces the old tuple. `format_dir_diagnostic_header` + `render_issues_for_directory` apply the same annotation. `#[allow(dead_code)]` on `DirectoryConfig.override_applied` REMOVED (config.rs:234 — field is now consumed). 5 status + 6 doctor unit tests pass; integration test `machine_override_appears_in_status_and_doctor` (cli.rs:5096) exercises full chain. |

**Score:** 5/5 truths verified

### Required Artifacts (Plan 09-01: PORT-01, PORT-02)

| Artifact                                                                                                       | Status      | Details                                                                                                                                                                                                            |
| -------------------------------------------------------------------------------------------------------------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `pub struct DirectoryOverride { path: PathBuf }` in `machine.rs`                                              | ✓ VERIFIED  | Found at `machine.rs:29`. Has `#[serde(deny_unknown_fields)]` per key-decision.                                                                                                                                   |
| `pub(crate) directory_overrides: BTreeMap<DirectoryName, DirectoryOverride>` on `MachinePrefs` w/ `#[serde(default)]` | ✓ VERIFIED  | Found at `machine.rs:71` with `#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]`. Note: declared `pub(crate)` (visible inside crate) — appropriate scope; sufficient for downstream consumers in `config.rs`. |
| `pub(crate) fn apply_machine_overrides(&mut self, prefs: &MachinePrefs)` in `config.rs`                       | ✓ VERIFIED  | Found at `config.rs:554`. `pub(crate)` scope — sufficient (not called from outside crate).                                                                                                                        |
| `pub fn load_with_overrides(...)` in `config.rs` chaining tilde-expand → apply → validate                     | ✓ VERIFIED  | Found at `config.rs:615`. Body order verified by unit test `load_with_overrides_runs_in_order_expand_apply_validate`.                                                                                              |
| `lib.rs` Sync/Status/Doctor/Init handlers use `load_with_overrides` (≥3 matches expected)                     | ✓ VERIFIED  | `rg "load_with_overrides|load_or_default_with_overrides" crates/tome/src/lib.rs` returns 5 matches (lines 267, 291, 296, 723, 920). The canonical post-Init `run()` load path (line 296) covers Sync/Status/Doctor/lockfile uniformly via shared `Config` value. Init keeps plain `load_or_default` for malformed-config probe per documented decision. |
| `pub override_applied: bool` field on `DirectoryConfig` with `#[serde(skip)]`                                  | ✓ VERIFIED  | Found at `config.rs:234`. `#[serde(skip)]` annotation present. `#[allow(dead_code)]` was REMOVED in Plan 09-03 (verified — no `allow(dead_code)` in config.rs). Field now live-consumed by status + doctor.       |

### Required Artifacts (Plan 09-02: PORT-03, PORT-04)

| Artifact                                                                                                                                            | Status     | Details                                                                                                                                                                                                                                                |
| --------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Config::warn_unknown_overrides` helper iterating `prefs.directory_overrides`, emitting `eprintln!("warning: ...")` for typos without aborting     | ✓ VERIFIED | `config.rs:580`. Uses `FnMut(String)` callback (testable seam); `lib.rs` adapter emits via `eprintln!`. Wired into `load_with_overrides` body.                                                                                                          |
| Wrapper `format_override_validation_error` names `machine.toml` only when (pre-override valid AND ≥1 override applied)                              | ✓ VERIFIED | `config.rs:759`. Discriminator at `config.rs:664-666`. Three-row truth-table tests pass: pre-invalid-no-override → raw, pre-invalid-unrelated → raw, pre-valid-override-breaks-it → wrapped. Integration negative-assertion probe present.             |
| Integration tests in `tests/cli.rs` covering typo warning + machine.toml error wrapper                                                              | ✓ VERIFIED | `machine_override_unknown_target_warns_and_continues` (cli.rs:5274), `machine_override_validation_failure_blames_machine_toml` (cli.rs:5323). Both pass.                                                                                              |

### Required Artifacts (Plan 09-03: PORT-05)

| Artifact                                                                                              | Status     | Details                                                                                                                                                                                                                                                                |
| ----------------------------------------------------------------------------------------------------- | ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `DirectoryStatus.override_applied: bool` (status.rs)                                                   | ✓ VERIFIED | `status.rs:53`. Populated from `dir_config.override_applied` at line 100.                                                                                                                                                                                              |
| `DirectoryDiagnostic` struct in doctor.rs with `override_applied`                                     | ✓ VERIFIED | `doctor.rs:37` (`pub struct DirectoryDiagnostic { name, issues, override_applied }`). Replaces old `Vec<(String, Vec<DiagnosticIssue>)>` tuple shape (verified gone via `rg`).                                                                                       |
| Text rendering: `(override)` annotation on overridden directories in `tome status` AND `tome doctor` | ✓ VERIFIED | `format_dir_path_column` (status.rs:126), `format_dir_diagnostic_header` (doctor.rs:409), styled `.cyan()`. Wired in render loops.                                                                                                                                    |
| JSON rendering: `override_applied` boolean field per-directory in both `tome status --json` and `tome doctor --json` | ✓ VERIFIED | Both structs derive `serde::Serialize`; field is `pub`, so it appears in JSON. Verified end-to-end by `machine_override_appears_in_status_and_doctor` integration test which asserts JSON contents directly.                                                            |
| `#[allow(dead_code)]` on `DirectoryConfig.override_applied` is GONE                                  | ✓ VERIFIED | `rg -n "allow\(dead_code\)" crates/tome/src/config.rs` returns 0 matches. Field is now live-consumed.                                                                                                                                                                  |
| End-to-end integration test `tome.toml + machine.toml override → tome sync → tome status (text + JSON) → tome doctor (text + JSON)` | ✓ VERIFIED | `machine_override_appears_in_status_and_doctor` at cli.rs:5096. Exercises sync + status text + status JSON + doctor JSON. Asserts `override_applied: true` for `work` dir, `override_applied: false` for the other dir, and shape conformance of every doctor diagnostic entry. |

### Key Link Verification

| From                                  | To                                              | Via                                                              | Status     | Details                                                                                                                          |
| ------------------------------------- | ----------------------------------------------- | ---------------------------------------------------------------- | ---------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `lib.rs::run`                         | `Config::load_or_default_with_overrides`        | direct call w/ `&machine_path` and `&machine_prefs`              | ✓ WIRED    | `lib.rs:296-300`. Result feeds the rest of the pipeline (Sync, Status, Doctor, lockfile::generate) through the shared `Config`. |
| `Config::load_with_overrides`         | `apply_machine_overrides` + `warn_unknown_overrides` + `validate` | sequential body order                                            | ✓ WIRED    | I2 invariant test: `load_with_overrides_runs_in_order_expand_apply_validate`.                                                    |
| `apply_machine_overrides`             | `DirectoryConfig.override_applied`              | `dir.override_applied = true` mutation                           | ✓ WIRED    | `config.rs:561`.                                                                                                                  |
| `DirectoryConfig.override_applied`    | `DirectoryStatus.override_applied`              | `gather()` populates from config                                 | ✓ WIRED    | `status.rs:100`.                                                                                                                  |
| `DirectoryConfig.override_applied`    | `DirectoryDiagnostic.override_applied`          | `check()` populates from config                                  | ✓ WIRED    | `doctor.rs:91`.                                                                                                                  |
| `DirectoryStatus.override_applied`    | rendered `(override)` annotation in text + JSON | `format_dir_path_column` + `serde::Serialize`                    | ✓ WIRED    | `status.rs:200`; JSON via derive.                                                                                                |
| `DirectoryDiagnostic.override_applied`| rendered `(override)` annotation in text + JSON | `format_dir_diagnostic_header` + `render_issues_for_directory`   | ✓ WIRED    | `doctor.rs:140, 409, 417`; JSON via derive.                                                                                      |

### Data-Flow Trace (Level 4)

| Artifact                                | Data Variable                                  | Source                                                                | Produces Real Data | Status     |
| --------------------------------------- | ---------------------------------------------- | --------------------------------------------------------------------- | ------------------ | ---------- |
| `DirectoryStatus.override_applied`      | `dir_config.override_applied`                  | mutated by `apply_machine_overrides` from real `MachinePrefs` on disk | Yes                | ✓ FLOWING  |
| `DirectoryDiagnostic.override_applied`  | `dir_config.override_applied`                  | same upstream                                                         | Yes                | ✓ FLOWING  |
| `DirectoryConfig.path` (overridden)     | `prefs.directory_overrides[name].path`         | parsed from `~/.config/tome/machine.toml`                             | Yes                | ✓ FLOWING  |
| Wrapper error message diff lines        | `pre_override_paths` snapshot vs current paths | `BTreeMap<String, PathBuf>` snapshot before apply                     | Yes                | ✓ FLOWING  |

End-to-end data flow proven by `machine_override_appears_in_status_and_doctor` integration test which writes a real `machine.toml`, runs the real binary, and asserts override information appears in JSON output.

### Behavioral Spot-Checks

| Behavior                                                          | Command                                                                  | Result                          | Status |
| ----------------------------------------------------------------- | ------------------------------------------------------------------------ | ------------------------------- | ------ |
| `apply_machine_overrides` unit tests                              | `cargo test -p tome --lib config::tests::apply_machine_overrides`        | 5 passed                        | ✓ PASS |
| `load_with_overrides` unit tests (incl. order + wrapper logic)    | `cargo test -p tome --lib config::tests::load_with_overrides`            | 6 passed                        | ✓ PASS |
| `warn_unknown_overrides` unit tests                               | `cargo test -p tome --lib config::tests::warn_unknown_overrides`         | 5 passed                        | ✓ PASS |
| `status::tests::*` (incl. PORT-05 surfacing)                      | `cargo test -p tome --lib status::tests::`                               | 25 passed                       | ✓ PASS |
| `doctor::tests::*` (incl. PORT-05 surfacing)                      | `cargo test -p tome --lib doctor::tests::`                               | 27 passed                       | ✓ PASS |
| Integration tests `machine_override_*`                            | `cargo test -p tome --test cli machine_override`                         | 4 passed                        | ✓ PASS |
| Full CI quality gates                                             | `make ci`                                                                | fmt-check + clippy `-D warnings` + 514 lib + 134 integration + typos all green | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description                                                                                                                                          | Status      | Evidence                                                                                                                                                                       |
| ----------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| PORT-01     | 09-01       | Declare `[directory_overrides.<name>]` in machine.toml without editing tome.toml                                                                       | ✓ SATISFIED | DirectoryOverride struct + `directory_overrides` field; round-trip test, parses-from-toml test pass                                                                            |
| PORT-02     | 09-01       | Overrides apply post-tilde-expand, pre-validate; downstream sees merged result                                                                        | ✓ SATISFIED | `load_with_overrides` order invariant tested; `lib.rs::run` uses `load_or_default_with_overrides` for canonical path; integration smoke test `machine_override_rewrites_directory_path_for_status` proves status::gather sees merged path |
| PORT-03     | 09-02       | Unknown override target produces stderr warning without aborting                                                                                       | ✓ SATISFIED | `warn_unknown_overrides` + integration test `machine_override_unknown_target_warns_and_continues`                                                                              |
| PORT-04     | 09-02       | Override-induced validation failures surface as distinct error class naming machine.toml                                                              | ✓ SATISFIED | `format_override_validation_error` + discriminator gates + integration test `machine_override_validation_failure_blames_machine_toml` (with negative-assertion probe)            |
| PORT-05     | 09-03       | `tome status` and `tome doctor` show which directories are overridden (text + JSON)                                                                  | ✓ SATISFIED | `DirectoryStatus.override_applied` + `DirectoryDiagnostic.override_applied` + render helpers + integration test `machine_override_appears_in_status_and_doctor`                  |

Each PORT ID is claimed by exactly one plan (verified via `requirements:` field in plan frontmatter). REQUIREMENTS.md marks all five as `[x] Complete` and the traceability table lists each at "Phase 9 | Complete". No orphaned requirements.

### Anti-Patterns Found

| File                                                              | Line  | Pattern                                  | Severity | Impact                                                                                                                                                              |
| ----------------------------------------------------------------- | ----- | ---------------------------------------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| (none)                                                            | -     | -                                        | -        | `#[allow(dead_code)]` on `DirectoryConfig.override_applied` was REMOVED in Plan 09-03 — verified via `rg`. No TODO/FIXME/placeholder in any modified Phase 9 files. |

### Human Verification Required

None — Phase 9 is end-to-end testable via integration tests. All observable behaviors (text output, JSON output, stderr warnings, error messages, command success/failure) are pinned by automated tests.

### Gaps Summary

No gaps. All 5 requirements (PORT-01 through PORT-05) are satisfied by code, unit tests, and end-to-end integration tests. `make ci` passes cleanly: fmt-check + clippy `-D warnings` + 514 lib tests + 134 integration tests + typos all green.

**Notes (non-gap, informational):**
- `.planning/ROADMAP.md` line 51 still shows Phase 9 as `[ ]` (unchecked) and line 106 shows `2/3 | In Progress`. This is roadmap-tracking state, not a phase-goal verification gap — the actual code/tests are complete and REQUIREMENTS.md correctly marks all 5 PORT IDs as Complete. The orchestrator's post-verification commit cycle should update ROADMAP.md to mark the phase as shipped (3/3) and STATE.md's `stopped_at` field accordingly.
- Plan 09-03 documented a JSON schema break for `tome doctor --json` (`directory_issues` items: tuples → objects). Per the plan's note this is to be flagged in the v0.9 CHANGELOG when the release is cut — out of scope for Phase 9 verification but worth tracking as a release-note item.

---

## Verification Complete

**Status:** passed
**Score:** 5/5 must-haves verified (PORT-01 through PORT-05)
**Report:** `.planning/phases/09-cross-machine-path-overrides/09-VERIFICATION.md`

All must-haves verified. Phase goal achieved. Phase 9 (Cross-Machine Path Overrides) is end-to-end complete: machine.toml schema → canonical load pipeline (expand → warn → apply → validate) → typo guard (PORT-03) → distinct error class on validation (PORT-04) → status + doctor surfacing (PORT-05). Ready to proceed to next phase.

---

_Verified: 2026-04-26_
_Verifier: Claude (gsd-verifier)_
