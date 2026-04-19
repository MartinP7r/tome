---
phase: 04-wizard-correctness
plan: 02
subsystem: validation

tags: [config, path-overlap, circular-symlinks, lexical-compare, tilde-expand, tdd]

requires:
  - phase: 04-wizard-correctness
    provides: "Plan 04-01 — D-10 Conflict+Why+Suggestion error template used by all new overlap bail!s; DirectoryRole::description() supplies the plain-english parenthetical (D-11)"
provides:
  - "Path-overlap detection in Config::validate() covering all three library-vs-distribution relations (D-04)"
  - "Private path_contains() helper with trailing-separator normalization (D-06)"
  - "Tilde-expansion-before-compare parity with Config::load() (D-07)"
affects: [04-03-wizard-save-hardening]

tech-stack:
  added: []
  patterns:
    - "Lexical path overlap check (no canonicalize) for I/O-free validate() — D-02"
    - "Trailing-separator-normalized prefix comparison via to_string_lossy().trim_end_matches('/')"

key-files:
  created: []
  modified:
    - crates/tome/src/config.rs

key-decisions:
  - "path_contains helper kept private to the config module — no new public API"
  - "Overlap block placed at the end of validate(), after the per-directory role/type loop, before the terminal Ok(())"
  - "Case A uses both PathBuf equality AND trailing-slash-normalized string comparison, so equality fires even when trailing slashes differ"
  - "Source-role nesting intentionally NOT flagged — Source dirs don't participate in distribution (D-05)"

patterns-established:
  - "D-10 Conflict+Why+Suggestion template extends to path-relation errors (not just role/type)"
  - "Private helpers at module level (below expand_tilde) avoid re-validating invariants the caller already enforced"

requirements-completed: [WHARD-02, WHARD-03]

duration: 5min
completed: 2026-04-19
---

# Phase 04 Plan 02: Library Overlap Validation Summary

**Config::validate() now rejects every path relation where library_dir overlaps a distribution directory — equality, nesting either direction — using lexical, tilde-aware, trailing-separator-normalized comparison**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-04-19T05:43:47Z
- **Completed:** 2026-04-19T05:48:31Z
- **Tasks:** 1 (TDD: RED + GREEN)
- **Files modified:** 1

## Accomplishments

- `Config::validate()` now rejects all three library-vs-distribution relations (D-04):
  - **Case A** — `library_dir == distribution_path` (tolerates a trailing slash on either side)
  - **Case B** — `library_dir` lives inside a distribution directory (the WHARD-03 circular-symlink case)
  - **Case C** — a distribution directory lives inside `library_dir`
- New private helper `path_contains(ancestor, descendant)` does trailing-separator-normalized prefix matching, so `/tmp/foo` does NOT match `/tmp/foobar` (D-06)
- Both sides of every overlap compare run through `expand_tilde()` first, matching the order `Config::load()` uses (D-07)
- No `Path::canonicalize()` — validate() stays I/O-free (D-02)
- Scope stays strictly library-vs-distribution; Source-role nesting and distro-to-distro overlap are deliberately allowed (D-05)
- Error bodies follow Plan 04-01's D-10 Conflict+Why+Suggestion template; every role mention routes through `DirectoryRole::description()` (D-11)
- Seven new unit tests in `config::tests` cover Cases A/B/C, the trailing-separator variant, the sibling-path negative, Source-role nesting, and tilde-prefixed equality
- `cargo fmt --check` (my lines), `cargo clippy --all-targets -- -D warnings`, and `cargo test -p tome` (105 + lib tests) all green

## Task Commits

TDD flow produced two commits for the single task:

1. **Task 1 (RED): Failing tests for library/distribution overlap** — `0a0a815` (test)
2. **Task 1 (GREEN): Overlap block + path_contains helper** — `ed32dad` (feat)

## Files Created/Modified

- `crates/tome/src/config.rs` — Added private `path_contains()` helper below `expand_tilde()`; appended a path-overlap block to `Config::validate()` immediately before the terminal `Ok(())`; added seven new unit tests at the bottom of the `#[cfg(test)] mod tests` block

## Decisions Made

None beyond plan. The plan's authoritative error bodies and test assertions were followed verbatim. The only judgement calls were minor and already anticipated by the plan:

- `path_contains` stays private (module-local) — not part of the public API, per plan constraint
- Case A uses both structural `PathBuf` equality AND trailing-slash-normalized string comparison so that `PathBuf::from("/tmp/lib/")` and `PathBuf::from("/tmp/lib")` are treated as equal without calling `starts_with`

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

- **Parallel-execution write contention with 04-03** (documented in the executor prompt). Between my RED `test` commit and my GREEN `feat` commit, the 04-03 agent wrote to the same file (`config.rs`), adding `save_checked_*` tests that reference a not-yet-existing method. My GREEN commit (`ed32dad`) therefore pulled in those foreign test additions alongside my own implementation lines. 04-03 shortly after committed the `save_checked` method (`c94d81c`), and the build turned green again.
  - **Impact on this plan:** none in the long run. My 7 target tests and the existing 6 pre-existing validate_* tests all pass. `cargo clippy --all-targets -- -D warnings` exits 0.
  - **Leftover cost:** temporary transient compile failure window between commits `ed32dad` and `c94d81c`. A single-agent sequential run would not have hit this.
  - **What I did NOT touch:** the `save_checked_*` test bodies and the `save_checked` method — those are 04-03's responsibility.
  - A separate `cargo fmt --check` nit still flags two lines inside `save_checked()` and `wizard.rs` — both owned by 04-03, out of scope here (Rule: "Only auto-fix issues DIRECTLY caused by the current task's changes").
- `make ci` carries the same pre-existing typos-binary-not-installed quirk noted in 04-01's summary — still a local tooling gap, not a code issue.

## Verification Results

- `rg "fn path_contains" crates/tome/src/config.rs` → 1 hit ✓
- `rg "library_dir overlaps distribution directory" crates/tome/src/config.rs` → 1 hit ✓
- `rg "library_dir is inside distribution directory" crates/tome/src/config.rs` → 1 hit ✓
- `rg "distribution directory .* is inside library_dir" crates/tome/src/config.rs` → 1 hit ✓
- `rg "circular symlink risk" crates/tome/src/config.rs` → 1 hit ✓
- `rg "self.distribution_dirs\(\)" crates/tome/src/config.rs` → 1 hit inside validate() at line 398 ✓
- `rg "canonicalize" crates/tome/src/config.rs` → 0 hits (D-02 honored) ✓
- `rg "Synced \(skills discovered here AND distributed here\)" crates/tome/src/config.rs` → 5 hits (description() def + 4 test assertions) ✓
- All 7 target tests pass individually: `validate_rejects_library_equals_distribution`, `validate_rejects_library_inside_synced_dir`, `validate_rejects_target_inside_library`, `validate_accepts_sibling_paths_not_false_positive`, `validate_rejects_equality_despite_trailing_separator`, `validate_accepts_source_role_inside_library`, `validate_rejects_tilde_equal_paths` ✓
- `cargo clippy --all-targets -- -D warnings` exits 0 ✓
- `cargo test -p tome` → 105 integration + 47 config lib tests pass, 0 failures ✓

## Next Phase Readiness

Plan 04-03 (wizard-save-hardening) can rely on `Config::validate()` to reject every library-vs-distribution overlap before any on-disk write, so the wizard's save path inherits the check automatically once it routes through `save_checked()` (which 04-03 has just landed). WHARD-02 and WHARD-03 are both satisfied by the overlap block.

## Self-Check: PASSED

- File `crates/tome/src/config.rs`: FOUND
- Commit `0a0a815`: FOUND (test)
- Commit `ed32dad`: FOUND (feat)

---
*Phase: 04-wizard-correctness*
*Completed: 2026-04-19*
