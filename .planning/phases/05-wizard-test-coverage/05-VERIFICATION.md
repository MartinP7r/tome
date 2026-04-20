---
phase: 05-wizard-test-coverage
verified: 2026-04-20T09:35:00Z
status: passed
score: 10/10 must-haves verified
---

# Phase 5: Wizard Test Coverage Verification Report

**Phase Goal:** Close Phase 4's wizard testing debt by covering the pure
(non-interactive) helpers and the headless `tome init --no-input` path with
automated tests, plus an exhaustive (DirectoryType × DirectoryRole) matrix guard.

**Verified:** 2026-04-20T09:35:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `tome init --no-input --dry-run` exits 0 and prints `Generated config:` marker | VERIFIED | Live binary run with empty TempDir HOME exited 0; `Generated config:` header emitted at wizard.rs:329. Integration test `init_dry_run_no_input_empty_home` also exercises this path and passed. |
| 2 | `tome init --no-input` (without `--dry-run`) saves config via save_checked | VERIFIED | wizard.rs:331 `else if no_input \|\| Confirm::new()…` short-circuits the `Save configuration?` prompt when `no_input` is true, falling into the `config.save_checked(&config_path)` branch at wizard.rs:339-341. |
| 3 | Under `--no-input`, the wizard uses D-01 defaults exactly | VERIFIED | configure_directories includes all (wizard.rs:444); configure_library returns `~/.tome/skills` default (wizard.rs:496-498); configure_exclusions returns empty (wizard.rs:531-533); role-edit loop is `while !no_input` (wizard.rs:186); custom-dir loop is `while !no_input` (wizard.rs:237); git-init gated by `!no_input` (wizard.rs:348). |
| 4 | `wizard::assemble_config(directories, library_dir, exclude) -> Config` exists as `pub(crate)` | VERIFIED | wizard.rs:376 — `pub(crate) fn assemble_config(directories: BTreeMap<DirectoryName, DirectoryConfig>, library_dir: PathBuf, exclude: std::collections::BTreeSet<crate::discover::SkillName>) -> Config`. Called once at wizard.rs:302. |
| 5 | `Config::directories()`, `Config::library_dir()`, `Config::exclude()` exist as `pub fn` read-only accessors; field visibility remains `pub(crate)` | VERIFIED | Accessors at config.rs:330/335/340. Fields still `pub(crate)` at config.rs:255/259/263. |
| 6 | `cargo test -p tome --lib wizard::tests` exercises helpers end-to-end | VERIFIED | 17 tests pass (6 pre-existing + 11 new): `known_directories_default_role_matches_type`, `known_directories_default_role_is_in_valid_roles`, `find_known_directories_in_discovers_every_registry_entry/multiple_entries/mixed_dir_and_file`, `assemble_config_*` (6 variants). |
| 7 | `cargo test -p tome --test cli init_` includes tests that drive `tome init --no-input` and parse via Config accessors | VERIFIED | `init_dry_run_no_input_empty_home`, `init_dry_run_no_input_seeded_home`, and `init_with_no_input_and_dry_run_succeeds` all pass. Helpers `parse_generated_config` and `assert_config_roundtrips` present. |
| 8 | `cargo test -p tome --lib config::tests::combo_matrix_` iterates all 12 combos, derives pass/fail at runtime from `valid_roles().contains(&role)` | VERIFIED | `combo_matrix_all_type_role_pairs` at config.rs:1717 loops `ALL_TYPES_FOR_MATRIX × ALL_ROLES_FOR_MATRIX` (3×4=12) with `dir_type.valid_roles().contains(role)` as the decision rule (config.rs:1727). Exhaustiveness assert at config.rs:1799-1805. Both tests pass. |
| 9 | Invalid-combo error messages contain role.description() substring + `hint:` line | VERIFIED | `combo_matrix_invalid_error_mentions_role_description` (config.rs:1809) asserts both `msg.contains(role.description())` (line 1834) and `msg.contains("hint:")` (line 1841) for every invalid combo. Test passes. |
| 10 | Full CI gate passes | VERIFIED | `cargo fmt -- --check` exit 0; `cargo clippy --all-targets -- -D warnings` exit 0; `cargo test -p tome` passes with 417 unit + 107 integration tests, 0 failed. |

**Score:** 10/10 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/tome/src/wizard.rs` | `pub fn run(dry_run: bool, no_input: bool) -> Result<Config>` | VERIFIED | wizard.rs:132. Present with correct signature. |
| `crates/tome/src/wizard.rs` | `pub(crate) fn assemble_config` + tests | VERIFIED | wizard.rs:376 (helper); wizard.rs:792-969 (6 `assemble_config_*` tests). |
| `crates/tome/src/wizard.rs` | Registry invariant + `find_known_directories_in_*` extensions | VERIFIED | 2 invariant tests + 3 new find_known_* tests at wizard.rs:669-790. |
| `crates/tome/src/lib.rs` | Bail removed; wizard::run called with both flags; regression guard | VERIFIED | lib.rs:171 (call site); bail string absent (only appears in the split-`concat!` sentinel of the regression test at lib.rs:1672). Test `init_with_no_input_does_not_bail_from_lib_run` passes. |
| `crates/tome/src/cli.rs` | Init after_help mentions `--dry-run` and `--no-input` | VERIFIED | cli.rs:78 contains `tome init --dry-run`, `tome init --no-input`, and `tome init --dry-run --no-input`. |
| `crates/tome/src/config.rs` | Three pub `Config` accessors; fields still `pub(crate)` | VERIFIED | Accessors at config.rs:330/335/340; fields at 255/259/263 still `pub(crate)`. |
| `crates/tome/src/config.rs` | Combo matrix + helper + constants | VERIFIED | `ALL_TYPES_FOR_MATRIX` (1669), `ALL_ROLES_FOR_MATRIX` (1674), `build_single_entry_config` (1689), `combo_matrix_all_type_role_pairs` (1717), `combo_matrix_invalid_error_mentions_role_description` (1809). |
| `crates/tome/tests/cli.rs` | Two new init tests + helpers | VERIFIED | `parse_generated_config` (3735), `assert_config_roundtrips` (3745), `init_dry_run_no_input_empty_home` (3757), `init_dry_run_no_input_seeded_home` (3812). `use tome::config::{…}` at 3727. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| lib.rs Init branch | wizard::run | `wizard::run(cli.dry_run, cli.no_input)` | WIRED | Exact pattern at lib.rs:171. |
| wizard::run | wizard::assemble_config | direct call at end of interactive flow | WIRED | `let config = assemble_config(directories, library_dir, exclude);` at wizard.rs:302. |
| tests/cli.rs (external crate) | Config accessors | `config.directories()`, `config.library_dir()`, `config.exclude()` | WIRED | Both init integration tests use accessors — 10+ accessor calls across the two tests, exactly 0 bare-field accesses (external crate cannot reach `pub(crate)` fields). |
| combo_matrix tests | DirectoryType::valid_roles | `dir_type.valid_roles().contains(role)` | WIRED | config.rs:1727 (main matrix) and config.rs:1821 (invalid-specific). Runtime derivation per D-08. |
| combo_matrix_all_type_role_pairs | Config::save_checked | positive-path save + reload | WIRED | config.rs:1737 `config.save_checked(&path)` for valid combos, followed by `Config::load(&path)` at 1750 and type/role equality at 1760-1770. |
| combo_matrix_invalid_error_mentions_role_description | DirectoryRole::description | substring assertion on error text | WIRED | config.rs:1834 `msg.contains(role.description())`. |

### Data-Flow Trace (Level 4)

Not applicable — this phase delivers test coverage and plumbing, no new rendered-data components. The integration tests themselves verify data flow: `init_dry_run_no_input_seeded_home` traces HOME seed → wizard discovery → assemble_config → expand_tildes → TOML emission → reparse → Config, asserting the expected directory entries survive each transformation.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `tome init --no-input --dry-run` on empty HOME | `HOME=$TMP TOME_HOME=$TMP/.tome NO_COLOR=1 tome --no-input --dry-run init` | exit 0; `Generated config:` emitted; valid TOML with empty `[directories]` | PASS |
| `tome init --no-input --dry-run` on seeded HOME | Same + `mkdir -p $TMP/.claude/plugins $TMP/.claude/skills` | exit 0; TOML contains `[directories.claude-plugins]` (type=claude-plugins, role=managed) and `[directories.claude-skills]` (role=synced) | PASS |
| Wizard unit tests pass | `cargo test -p tome --lib wizard::tests` | 17 passed (6 original + 11 new), 0 failed | PASS |
| Combo matrix tests pass | `cargo test -p tome --lib config::tests::combo_matrix` | 2 passed, 0 failed | PASS |
| Init integration tests pass | `cargo test -p tome --test cli -- init_` | 6 passed (3 init-adjacent pre-existing + 3 new/updated), 0 failed | PASS |
| Full test suite | `cargo test -p tome` | 417 unit + 107 integration tests passed, 0 failed | PASS |
| Clippy clean | `cargo clippy --all-targets -- -D warnings` | exit 0 | PASS |
| Fmt clean | `cargo fmt -- --check` | exit 0 | PASS |
| Regression guard | `cargo test -p tome --lib tests::init_with_no_input_does_not_bail_from_lib_run` | 1 passed | PASS |
| Phase 4 regression: validate_rejects_* | `cargo test -p tome --lib config::tests::validate_rejects` | 10 passed | PASS |
| Phase 4 regression: save_checked_* | `cargo test -p tome --lib config::tests::save_checked` | 4 passed (including `save_checked_rejects_role_type_conflict` and `save_checked_writes_valid_config_and_reloads_unchanged`) | PASS |

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|-------------|----------------|-------------|--------|----------|
| WHARD-04 | 05-01, 05-02 | Pure helpers have unit test coverage (`find_known_directories_in`, registry, `DirectoryType::default_role`, config assembly) | SATISFIED | 11 new tests in `wizard::tests`; registry-type invariant + registry-valid_roles invariant tests ensure future drift fails CI; `assemble_config` exercised across 6 shapes. |
| WHARD-05 | 05-01, 05-03 | Integration test running `tome init --dry-run --no-input` that validates + round-trips | SATISFIED | `init_dry_run_no_input_empty_home` and `init_dry_run_no_input_seeded_home` both run `tome init --dry-run --no-input`, parse `Generated config:`, call `Config::validate()`, and call `assert_config_roundtrips`. Both pass. |
| WHARD-06 | 05-04 | Every (DirectoryType, DirectoryRole) combo has a test — valid combos save successfully, invalid combos are rejected | SATISFIED | 12-combo cross-product test with runtime-derived expectations via `valid_roles().contains()`. Invalid-combo sibling test asserts role description + hint substrings. Note: REQUIREMENTS.md WHARD-06 row still shows unchecked — the docs are stale relative to implementation (cosmetic docs lag, not an implementation gap). |

No orphaned requirements — ROADMAP.md and REQUIREMENTS.md map exactly WHARD-04/05/06 to Phase 5, and all three are claimed (and delivered) by the four plans.

### Anti-Patterns Found

None. Scanned `wizard.rs`, `lib.rs`, `cli.rs`, `config.rs`, `tests/cli.rs` for TODO/FIXME/XXX/HACK/PLACEHOLDER — 0 hits in all modified files.

### In-Scope Deviation (Documented)

Plan 05-04 originally scoped as "No production code changes," but the plan author added a 24-line catch-all to `Config::validate()` at config.rs:389-412 to close a pre-existing inconsistency between `DirectoryType::valid_roles()` and `Config::validate()`. The pre-04 validator only rejected specific invalid combos (`Managed` + non-ClaudePlugins, `Target` + Git, plus git-only-field misuse) and would silently pass `ClaudePlugins + Synced/Source/Target` and `Git + Synced`. The catch-all:

- Uses the Phase 4 D-10 Conflict+Why+Suggestion template (role description + accepted-roles list + hint).
- Runs AFTER the two specific-case rejections, so their tailored hints still fire for their common cases.
- Closes the gap WHARD-06 itself requires ("any change to `valid_roles()` or `Config::validate()` that regresses any combo fails CI" presumes the two agree).

The deviation is documented in `05-04-combo-matrix-test-SUMMARY.md` `## Deviations` and is in-scope for the phase goal.

### Regression Surface Verified

All Phase 4 regression targets listed in `<regression_surface>` still pass:

| Test | Result |
|------|--------|
| `config::tests::validate_rejects_managed_with_directory_type` | PASS (covered by `validate_rejects` run — 10 passed) |
| `config::tests::validate_rejects_target_with_git_type` | PASS (covered by `validate_rejects` run) |
| `config::tests::save_checked_rejects_role_type_conflict` | PASS |
| `config::tests::save_checked_writes_valid_config_and_reloads_unchanged` | PASS |
| `wizard::tests` (6 original) | PASS — all 6 preserved alongside 11 new tests |
| `init_with_no_input_and_dry_run_succeeds` (replaced `init_with_no_input_fails`) | PASS — asserts exit 0 + `Generated config:` marker with isolated HOME |

### Commit Audit

All 5 commits referenced in SUMMARY frontmatters exist in the branch:
- `79bd7d3` feat(05-01): add read-only Config accessors for external crates
- `ff42faf` feat(05-01): plumb --no-input through wizard, extract assemble_config
- `34f1a3e` test(05-02): add pure wizard helper unit tests (WHARD-04)
- `14010e0` test(05-03): add init --dry-run --no-input integration tests
- `e9e6e0a` test(05-04): add cross-product type/role matrix tests

Plus `dd5e96b` docs(05): finalize Wave 1 SUMMARYs and roadmap (05-01, 05-04).

### Human Verification Required

None. Every phase truth is programmatically verifiable and was verified:
- No visual/UX claims in this phase (display polish is Phase 6's scope).
- No real-time or external service behaviors.
- Plumbing, tests, and accessor surfaces are fully covered by automated checks.

### Gaps Summary

No gaps. Phase 5 meets every success criterion listed in ROADMAP.md and closes every requirement listed in REQUIREMENTS.md for this phase:

1. `cargo test` exercises unit tests for `find_known_directories_in`, `KNOWN_DIRECTORIES` registry lookup, `DirectoryType::default_role`, and pure config-assembly helpers — **satisfied** via 11 new `wizard::tests`.
2. Integration test runs `tome init --dry-run --no-input` and asserts the generated config passes validation and round-trips through TOML unchanged — **satisfied** via `init_dry_run_no_input_empty_home` + `init_dry_run_no_input_seeded_home`.
3. Every `(DirectoryType, DirectoryRole)` combination the wizard can produce has a test — **satisfied** via the 12-combo matrix pair with runtime-derived expectations.
4. CI (ubuntu + macos) passes with the new tests as non-optional gates — **satisfied** — all new tests are in the default `cargo test` target and run on every CI invocation.

The documented in-scope deviation to `Config::validate()` (Plan 05-04's catch-all) is a net correctness improvement that keeps the validator in lockstep with `valid_roles()` — a precondition for WHARD-06 to be meaningful.

One minor documentation lag: REQUIREMENTS.md traceability table still shows WHARD-06 as `Pending`, while the 05-04 SUMMARY marks it complete and code/tests confirm closure. This is a docs-side update for the Phase 6 tidy pass, not a gap in the implementation.

---

*Verified: 2026-04-20T09:35:00Z*
*Verifier: Claude (gsd-verifier)*
