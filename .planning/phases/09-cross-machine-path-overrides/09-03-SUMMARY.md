---
phase: 09-cross-machine-path-overrides
plan: 03
subsystem: status-doctor-surfacing
tags: [machine-toml, directory-overrides, status, doctor, port-05, dotfiles-portability]

# Dependency graph
requires:
  - phase: 09-cross-machine-path-overrides
    plan: 01
    provides: "DirectoryConfig.override_applied: bool flag set by Config::apply_machine_overrides during canonical config load"
provides:
  - "status.rs: DirectoryStatus.override_applied: bool field (text + JSON), format_dir_path_column helper appending ` (override)` annotation"
  - "doctor.rs: DirectoryDiagnostic struct with name/issues/override_applied, replaces (String, Vec<DiagnosticIssue>) tuple in DoctorReport.directory_issues, format_dir_diagnostic_header helper, render_issues_for_directory takes override_applied"
  - "tests/cli.rs: end-to-end integration test machine_override_appears_in_status_and_doctor pinning the full PORT-05 contract (sync + status text + status JSON + doctor JSON)"
  - "config.rs: removed #[allow(dead_code)] from DirectoryConfig.override_applied — now consumed by status::gather and doctor::check"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Render-helper extraction for unit-testable annotation: `format_dir_path_column` (status) and `format_dir_diagnostic_header` (doctor) compose the path/name + `(override)` suffix as a returnable String, so the annotation logic is unit-tested without capturing stdout. Mirrors a pattern already used in status.rs for the `count` column."
    - "JSON schema break on `tome doctor --json`: `directory_issues` items changed from `[name, issues]` tuples to `{ name, issues, override_applied }` objects. Acceptable because (a) consumers are humans grepping JSON or the future tome-desktop GUI, and (b) the wrapped shape is what the GUI would want anyway. To be flagged in v0.9 CHANGELOG."
    - "`(override)` annotation styled `console::style(...).cyan()` for visual consistency with status.rs's existing path/count cyan styling. Matches doctor.rs's existing `style(...).dim()` and `style(...).cyan()` use."

key-files:
  created:
    - .planning/phases/09-cross-machine-path-overrides/09-03-SUMMARY.md
  modified:
    - crates/tome/src/status.rs
    - crates/tome/src/doctor.rs
    - crates/tome/src/relocate.rs
    - crates/tome/src/config.rs
    - crates/tome/tests/cli.rs

key-decisions:
  - "Render annotation uses `(override)` (lowercase, parens) rather than `[override]`, `*` suffix, or a separate column. Lightest visual treatment that's still unambiguous; one bit of state doesn't justify a 6th column on an already 5-col-wide table."
  - "`format_dir_path_column` and `format_dir_diagnostic_header` extracted as pure-string helpers so tests assert on substring matches without ANSI-strip dance or stdout capture. Existing test patterns in status.rs/doctor.rs do not capture stdout — this keeps that convention."
  - "Replaced the `Vec<(String, Vec<DiagnosticIssue>)>` tuple in `DoctorReport.directory_issues` with `Vec<DirectoryDiagnostic>` (option a from the plan). Cleaner shape, JSON-schema-future-proof for the GUI, and only one consumer outside doctor.rs (relocate::verify) — minor migration cost, large clarity win."
  - "Removed `#[allow(dead_code)]` from `DirectoryConfig.override_applied`. Plan 09-01 set it as a placeholder because Plan 09-03 (this plan) is the consumer; the field is now live-used by `status::gather` and `doctor::check`."

patterns-established:
  - "Render-helper-for-flag-annotation: when adding a `bool` flag that contributes a small text decoration (e.g., `(override)`), extract a pure-string helper `format_FOO_column(input, flag) -> String` and unit-test that helper directly. Keeps the printing function thin and the annotation logic regression-tested."

requirements-completed: [PORT-05]

# Metrics
duration: ~10min
completed: 2026-04-28
---

# Phase 09 Plan 03: Status and Doctor Surfacing Summary

**`tome status` and `tome doctor` now surface `[directory_overrides.<name>]` activations as an `(override)` annotation in text mode and an `override_applied: bool` field in JSON output, closing PORT-05 and completing Phase 9.**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-04-28T13:58:02Z
- **Completed:** 2026-04-28T14:08:53Z (approx)
- **Tasks:** 3 (all autonomous, all TDD)
- **Files modified:** 5 (status.rs, doctor.rs, relocate.rs, config.rs, tests/cli.rs)

## Accomplishments

- `DirectoryStatus.override_applied: bool` field added; `gather()` populates it from `DirectoryConfig.override_applied`. `format_dir_path_column(path, override_applied)` helper appends styled ` (override)` to the PATH column when the flag is set. JSON output includes `override_applied` per directory automatically via `serde::Serialize`.
- `DirectoryDiagnostic { name, issues, override_applied }` struct replaces the `(String, Vec<DiagnosticIssue>)` tuple in `DoctorReport.directory_issues`. `check()` populates the flag from `DirectoryConfig.override_applied`. `render_issues_for_directory(name, issues, override_applied)` and `format_dir_diagnostic_header` apply the same `(override)` styling pattern as status.
- One end-to-end integration test (`machine_override_appears_in_status_and_doctor`) pins the full PORT-05 contract on an actually-overridden directory (`role = "synced"` so it's both a discovery + distribution dir): sync respects override → status text shows `(override)` exactly once → status JSON has correct booleans for both dirs → doctor JSON's `work` entry exists with `override_applied: true` → every doctor `directory_issues` entry uses the new struct shape.
- 5 new status::tests, 6 new doctor::tests, 1 new integration test. All passing alongside the existing 19 status::tests and 21 doctor::tests, and the Wave 2 sibling Plan 09-02's tests for PORT-03/04.
- `#[allow(dead_code)]` removed from `DirectoryConfig.override_applied` (Plan 09-01 placeholder). Field is now live-used.
- `relocate::verify` migrated from tuple-destructuring to struct-field access for the new `DirectoryDiagnostic` shape.

## Task Commits

Each task was committed atomically on `gsd/phase-09-cross-machine-path-overrides`. RED → GREEN TDD cycle for Tasks 1 and 2; Task 3 was a single integration-test commit because Tasks 1–2 already implemented the surfacing.

| # | Task                                                              | Commit    | Type      |
| - | ----------------------------------------------------------------- | --------- | --------- |
| 1 | Failing tests for DirectoryStatus.override_applied                | `c94affa` | test RED  |
| 2 | DirectoryStatus + format_dir_path_column + render integration     | `2d7d580` | feat      |
| 3 | Failing tests for DirectoryDiagnostic.override_applied            | `994f952` | test RED  |
| 4 | DirectoryDiagnostic struct + format_dir_diagnostic_header + render| `fabbb6b` | feat      |
| 5 | End-to-end PORT-05 surfacing integration test                     | `5c3971f` | test      |
| 6 | rustfmt fix for the integration-test assert                       | `a4282ff` | style     |

## New API Signatures

### `crates/tome/src/status.rs`

```rust
#[derive(serde::Serialize)]
pub struct DirectoryStatus {
    pub name: String,
    pub directory_type: String,
    pub role: String,
    pub path: String,
    pub skill_count: CountOrError,
    pub warnings: Vec<String>,
    pub override_applied: bool,   // <-- NEW
}

fn format_dir_path_column(path: &str, override_applied: bool) -> String;
```

### `crates/tome/src/doctor.rs`

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct DirectoryDiagnostic {              // <-- NEW
    pub name: String,
    pub issues: Vec<DiagnosticIssue>,
    pub override_applied: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct DoctorReport {
    pub configured: bool,
    pub library_issues: Vec<DiagnosticIssue>,
    pub directory_issues: Vec<DirectoryDiagnostic>,   // <-- was Vec<(String, Vec<DiagnosticIssue>)>
    pub config_issues: Vec<DiagnosticIssue>,
}

fn format_dir_diagnostic_header(name: &str, override_applied: bool) -> String;
fn render_issues_for_directory(name: &str, issues: &[DiagnosticIssue], override_applied: bool);
//                                                                     ^^^^^^^^^^^^^^^^^^^^^^^^ NEW
```

### `crates/tome/src/config.rs`

```rust
pub struct DirectoryConfig {
    // ... existing fields ...
    #[serde(skip)]
    pub(crate) override_applied: bool,   // <-- #[allow(dead_code)] REMOVED — now consumed
}
```

## Tests Added

**`crates/tome/src/status.rs` tests module — 5 tests:**
- `gather_with_no_overrides_sets_flag_false`
- `gather_with_override_applied_sets_flag_true`
- `render_status_appends_override_marker_to_path`
- `render_status_no_override_omits_marker`
- `status_json_includes_override_applied_field`

**`crates/tome/src/doctor.rs` tests module — 6 tests:**
- `check_with_no_overrides_sets_flags_false`
- `check_with_override_applied_sets_flag_true`
- `render_issues_for_directory_appends_override_marker_when_set`
- `render_issues_for_directory_omits_marker_when_unset`
- `doctor_json_includes_override_applied_per_directory`
- `total_issues_unchanged_by_directory_diagnostic_shape`

**`crates/tome/tests/cli.rs` — 1 integration test:**
- `machine_override_appears_in_status_and_doctor` — exercises the full chain: `tome sync` → `tome status` text → `tome status --json` → `tome doctor --json`. Asserts override marker appears exactly once in status text, both `override_applied: true` (work) and `override_applied: false` (other) in status JSON, presence of `work` in doctor JSON `directory_issues` with `override_applied: true`, and shape-conformance of every doctor diagnostic entry.

## Files Created/Modified

**Modified:**
- `crates/tome/src/status.rs` — `override_applied` field + `format_dir_path_column` helper + table-loop wiring + 5 unit tests
- `crates/tome/src/doctor.rs` — `DirectoryDiagnostic` struct + `format_dir_diagnostic_header` helper + `render_issues_for_directory` signature update + `check()` + `render_repair_plan_auto` migration to struct-field access + 6 unit tests
- `crates/tome/src/relocate.rs` — `verify()` rendering migrated from tuple-destructuring `(name, issues)` to struct-field access `d.name` / `d.issues`
- `crates/tome/src/config.rs` — `#[allow(dead_code)]` removed from `DirectoryConfig.override_applied`, doc comment updated to reflect that the field is now live-used by status + doctor
- `crates/tome/tests/cli.rs` — `machine_override_appears_in_status_and_doctor` integration test

**Created:**
- `.planning/phases/09-cross-machine-path-overrides/09-03-SUMMARY.md`

## Decisions Made

1. **`(override)` annotation form** — chose lowercase parenthesized over `[override]`, `*`, or a 6th column. Lightest visual treatment that remains unambiguous.
2. **Render-helper extraction (`format_dir_path_column`, `format_dir_diagnostic_header`)** — pure-string helpers enable substring-based unit testing without stdout capture. The existing test conventions in status.rs/doctor.rs do not capture stdout; this approach keeps that convention.
3. **`Vec<DirectoryDiagnostic>` over parallel `Vec<String> override_applied_directory_names`** — option (a) from the plan. Cleaner shape, single source of truth per entry, future-proof for the tome-desktop GUI. JSON-schema break is acceptable in v0.9.
4. **Removed `#[allow(dead_code)]` from `override_applied`** — Plan 09-01 set it as a placeholder noting "Wired by Plan 09-03". Plan 09-03 is now the consumer; the field is live-used by `status::gather` and `doctor::check`.
5. **`.cyan()` styling for `(override)`** — matches status.rs's existing `.cyan()` use for paths and library counts. Avoided `.bold()` (would dominate a row visually) and `.dim()` (used in doctor.rs for skip/secondary text — wrong semantic).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `relocate.rs::verify` consumed the old tuple shape; not listed in Task 2's enumerated consumers**

- **Found during:** Task 2 GREEN — first compile of doctor.rs after switching to `DirectoryDiagnostic`.
- **Issue:** Task 2's `<read_first>` listed `doctor.rs` lines 116–127 (the in-file `diagnose()` render loop) and lines 335–365 (`render_repair_plan_auto`) as the consumers of `report.directory_issues`. There was a third consumer outside doctor.rs: `relocate::verify` (line 292) which destructured the tuple as `for (name, issues) in &report.directory_issues`.
- **Fix:** Migrated the loop to struct-field access (`for d in &report.directory_issues { for issue in &d.issues { ... d.name ... } }`). Pure rename, no semantic change.
- **Files modified:** `crates/tome/src/relocate.rs`
- **Verification:** `cargo build -p tome --all-targets` clean, `cargo test -p tome --lib` passes.
- **Committed in:** `fabbb6b` (Task 2 GREEN, same task)

**2. [Rule 1 - Hygiene] rustfmt wrapped a long `assert!()` in the integration test**

- **Found during:** Final `make ci` after Task 3.
- **Issue:** `cargo fmt --check` complained about the single-line `assert!(real_path.is_dir(), "real_path must exist for sync to succeed");` — over rustfmt's threshold.
- **Fix:** Split across three lines per rustfmt's preferred style.
- **Files modified:** `crates/tome/tests/cli.rs` (formatting only, no semantic change)
- **Verification:** `cargo fmt -- --check` clean.
- **Committed in:** `a4282ff` (separate `style(09-03)` commit so the diff is reviewable)

---

**Total deviations:** 2 auto-fixed (1 missed-consumer, 1 fmt-correction). Plan structure intact, scope unchanged. No architectural shifts.

## Issues Encountered

- **Pre-existing flake `backup::tests::diff_shows_changes`** — Failed during `make ci test` step with "agent refused operation? signing failed". Same flavor as the documented `backup::tests::push_and_pull_roundtrip` flake (PROJECT.md). Not related to this plan's changes; passes in isolation. Out of scope per deviation rules.
- **Pre-existing flake `remove_preserves_git_lockfile_entries`** — Integration test occasionally fails with "post-sync lockfile must contain a myrepo entry with git_commit_sha set". Passes in isolation. Not related to this plan; out of scope.
- **Wave 2 parallel-execution interleave** — Plan 09-02 was modifying `crates/tome/src/config.rs` and `crates/tome/tests/cli.rs` while this plan ran. Used file-level diff inspection (`git diff --stat`) and partial-patch staging (`git apply --cached <hunk>`) to keep my commits focused only on PORT-05 work. No semantic conflicts; both plans converged cleanly with the test suite passing for both sets of work.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

**Phase 9 is complete with this plan.** All five PORT requirements (PORT-01 through PORT-05) are now shipped across plans 09-01, 09-02, 09-03:

| Plan  | Requirements                  | Status   |
| ----- | ----------------------------- | -------- |
| 09-01 | PORT-01, PORT-02              | Shipped  |
| 09-02 | PORT-03, PORT-04 (Wave 2)     | Shipped  |
| 09-03 | PORT-05 (Wave 2)              | **Shipped (this plan)** |

The `[directory_overrides.<name>]` end-to-end story is closed: machine.toml schema → canonical load pipeline → typo guard (PORT-03) → distinct error class on validation (PORT-04) → status + doctor surfacing (PORT-05).

**v0.9 CHANGELOG note required:** `tome doctor --json` schema break — `directory_issues` items are now objects with `name`/`issues`/`override_applied` instead of `[name, issues]` tuples.

No blockers. No carry-over UAT items from this plan.

## Self-Check: PASSED

Verified by direct filesystem and git checks:

- File `.planning/phases/09-cross-machine-path-overrides/09-03-SUMMARY.md`: FOUND (this file)
- Commit `c94affa` (Task 1 RED): FOUND in `git log --oneline`
- Commit `2d7d580` (Task 1 GREEN): FOUND in `git log --oneline`
- Commit `994f952` (Task 2 RED): FOUND in `git log --oneline`
- Commit `fabbb6b` (Task 2 GREEN): FOUND in `git log --oneline`
- Commit `5c3971f` (Task 3 integration test): FOUND in `git log --oneline`
- Commit `a4282ff` (Task 3 fmt fix): FOUND in `git log --oneline`
- `rg -n "pub override_applied: bool" crates/tome/src/status.rs`: 1 match (line 53)
- `rg -n "fn format_dir_path_column" crates/tome/src/status.rs`: 1 match
- `rg -n "override_applied: dir_config.override_applied" crates/tome/src/status.rs`: 1 match
- `rg -n "pub struct DirectoryDiagnostic" crates/tome/src/doctor.rs`: 1 match (line 37)
- `rg -n "pub override_applied: bool" crates/tome/src/doctor.rs`: 1 match (line 44)
- `rg -n "Vec<DirectoryDiagnostic>" crates/tome/src/doctor.rs`: 1 match
- `rg -n "Vec<\(String, Vec<DiagnosticIssue>\)>" crates/tome/src/doctor.rs`: 0 matches (old tuple shape gone)
- `rg -n "fn format_dir_diagnostic_header" crates/tome/src/doctor.rs`: 1 match
- `rg -B2 "override_applied" crates/tome/src/config.rs | head` confirms `#[allow(dead_code)]` is gone from the field
- `cargo test -p tome --lib status::tests`: 25 passed
- `cargo test -p tome --lib doctor::tests`: 27 passed
- `cargo test -p tome --test cli machine_override`: 4 passed (smoke from 09-01 + 2 from 09-02 + 1 from 09-03)
- `cargo fmt -- --check`: clean
- `cargo clippy -p tome --all-targets -- -D warnings`: clean

---
*Phase: 09-cross-machine-path-overrides*
*Plan: 03*
*Completed: 2026-04-28*
