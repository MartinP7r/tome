---
phase: 04-wizard-correctness
plan: 01
subsystem: validation

tags: [config, validation, error-messages, anyhow, tdd]

requires:
  - phase: 01-unified-directory-foundation
    provides: DirectoryRole::description() and DirectoryType Display
provides:
  - D-10 Conflict+Why+Suggestion error template applied to four Config::validate() bail! sites
  - D-11 plain-english role parentheticals in every role-naming error
  - New validate_rejects_subdir_with_non_git_type test covering previously-untested case
affects: [04-02-library-overlap-validation, 04-03-wizard-save-hardening]

tech-stack:
  added: []
  patterns:
    - "D-10 Conflict+Why+Suggestion error template for Config::validate()"
    - "DirectoryRole::description() is the canonical source for role names in errors (never use Display)"

key-files:
  created: []
  modified:
    - crates/tome/src/config.rs

key-decisions:
  - "Followed D-10 template verbatim from plan; no phrasing drift"
  - "library_dir-is-a-file check left untouched (D-12 scope)"
  - "Used raw Display for subdir/git-fields type mention (no role involved) — clarity over symmetry"

patterns-established:
  - "Multi-line bail! with \\n\\\\-continuation for Conflict/Why/hint lines"
  - "`hint:` prefix lowercase, on its own trailing line, per Phase 1 convention"

requirements-completed: [WHARD-01]

duration: 3min
completed: 2026-04-19
---

# Phase 04 Plan 01: Validate Error Template Summary

**All four Config::validate() bail! bodies rewritten to the D-10 Conflict+Why+Suggestion template with DirectoryRole::description() used for every role mention**

## Performance

- **Duration:** 3 min
- **Started:** 2026-04-19T05:38:33Z
- **Completed:** 2026-04-19T05:41:33Z
- **Tasks:** 1 (TDD: RED + GREEN)
- **Files modified:** 1

## Accomplishments

- Four bail! sites in Config::validate() now follow Conflict + Why + Suggestion template
- Every role mention routes through DirectoryRole::description() (D-11 parenthetical)
- Three existing validate_rejects_* tests updated to assert on new substrings
- New validate_rejects_subdir_with_non_git_type test added (previously untested case)
- Regression-safe: validate_rejects_library_dir_that_is_a_file untouched, still passes
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test -p tome` all green

## Task Commits

TDD flow produced two commits for the single task:

1. **Task 1 (RED): Failing tests for new D-10 substrings** — `e230a18` (test)
2. **Task 1 (GREEN): Apply D-10 template to validate() bail! calls** — `c0c23a5` (feat)

## Files Created/Modified

- `crates/tome/src/config.rs` — Rewrote four `bail!` bodies in `Config::validate()` (sites 1-4); updated three existing tests; added one new test `validate_rejects_subdir_with_non_git_type`

## Decisions Made

None beyond plan — wording followed the plan's authoritative bodies verbatim. The only judgment call was scope containment: Site 1 in the plan (library_dir-is-a-file) was explicitly excluded and was left exactly as-is per D-12.

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

- `make ci` fails on the `typos` target because the `typos` binary is not installed locally. This is an unrelated tooling gap — the plan's verification gates (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p tome`) all pass. Not a regression from this plan.

## Verification Results

- `rg "Conflict: role is" crates/tome/src/config.rs` → 2 hits (sites 1 and 2) ✓
- `rg "Conflict: branch/tag/rev" crates/tome/src/config.rs` → 1 hit (site 3) ✓
- `rg "Conflict: subdir is set" crates/tome/src/config.rs` → 1 hit (site 4) ✓
- `rg "Conflict:" crates/tome/src/config.rs` → 4 hits (one per re-templated bail!) ✓
- `rg "DirectoryRole::Managed\.description\(\)"` → 2 hits (site 1 + test trust) ✓
- `rg "DirectoryRole::Target\.description\(\)"` → 2 hits ✓
- `rg "DirectoryRole::Source\.description\(\)"` → 3 hits (sites 1 and 2 both suggest Source) ✓
- `rg "Synced \(skills discovered here AND distributed here\)"` → 2 hits (description() def + test) ✓
- `rg "validate_rejects_subdir_with_non_git_type"` → 1 hit (new test added) ✓
- All four targeted tests pass; regression test `validate_rejects_library_dir_that_is_a_file` also passes
- `cargo clippy --all-targets -- -D warnings` exits 0
- `cargo test -p tome` → 105 integration + lib tests pass, 0 failures

## Next Phase Readiness

Plan 04-02 (library-overlap-validation) can now append its new overlap checks to `Config::validate()` using the same D-10 template — the template's shape is established and proven green. Plan 04-03 (wizard-save-hardening) can call `Config::validate()` knowing the error surface is consistent.

## Self-Check: PASSED

- File `crates/tome/src/config.rs`: FOUND
- Commit `e230a18`: FOUND
- Commit `c0c23a5`: FOUND

---
*Phase: 04-wizard-correctness*
*Completed: 2026-04-19*
