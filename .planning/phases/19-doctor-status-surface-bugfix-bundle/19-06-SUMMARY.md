---
phase: 19-doctor-status-surface-bugfix-bundle
plan: 06
subsystem: wizard
tags: [wizard, regression-test, library-default, tome-home, fix-05]

# Dependency graph
requires:
  - phase: 07-wizard-ux-greenfield-brownfield-legacy
    provides: WUX-01 wizard tome_home prompt + resolved tome_home propagation
  - phase: 11-library-canonical-core
    provides: library is single source of truth (library_dir matters more post-v0.10)
provides:
  - "Two integration tests in cli_init.rs pinning the wizard library-default derivation against TOME_HOME (positive + no-fallback contract)"
  - "Sensitivity-verified regression guard for FIX-05: closes #453 + #456 with test-only changes"
affects: [tome init, FIX-05 acceptance, future wizard refactors]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "TOME_HOME-env-var-driven integration test for wizard --dry-run --no-input"
    - "parse_generated_config + library_dir assertion (mirrors init_derived_library_default_under_custom_tome_home which covers the --tome-home flag source)"
    - "Negative assert_ne!(library_dir, <HOME>/.tome/skills) for no-fallback contract (D-FIX05-2)"

key-files:
  created: []
  modified:
    - "crates/tome/tests/cli_init.rs - +84 LOC; two new tests (wizard_library_default_follows_custom_tome_home, wizard_library_default_does_not_fall_back_to_home_tome_skills)"

key-decisions:
  - "Tests use TOME_HOME env var (not --tome-home flag) — the --tome-home flag case is already pinned by init_derived_library_default_under_custom_tome_home above. The env-var case is the documented FIX-05 acceptance shape (D-FIX05-1 example: `tome_home = ~/dev/coding-agent-files/.tome` → library default = `~/dev/coding-agent-files/.tome/skills`)."
  - "Assert against parse_generated_config(stdout).library_dir() — not against raw stderr substring matching. RESEARCH's example used combined-output substring match, but the existing pattern in cli_init.rs (init_derived_library_default_under_custom_tome_home) uses parsed-config assertion. Parsed-config is more robust to chrome-text changes and matches the file's idiomatic style."
  - "No fallback test: assert_ne!(library_dir, tmp/.tome/skills) — where tmp is both HOME and the TempDir parent. The fallback path would only resolve to that location, so a precise negative assertion catches it without false positives from unrelated help text."
  - "wizard.rs NOT modified — RESEARCH finding upheld (implementation at :637 was already correct). Plan is test-only."

patterns-established:
  - "Wizard library-default regression coverage now spans both source-of-truth signals: --tome-home flag (existing init_derived_library_default_under_custom_tome_home test) AND TOME_HOME env (new wizard_library_default_follows_custom_tome_home test). Future regressions in either resolution path fail-fast."

requirements-completed: [FIX-05]

# Metrics
duration: 10min
completed: 2026-05-13
---

# Phase 19 Plan 06: Wizard library default — TOME_HOME pinning test (FIX-05) Summary

Closes GitHub #453 + #456 (wizard library-default does not follow tome_home) by adding the regression test that RESEARCH.md flagged as the actual missing piece. The implementation at `wizard.rs:637` already derives `<resolved_tome_home>/skills` correctly; what was missing was a regression test guarding against someone re-introducing a hardcoded `~/.tome/skills` fallback.

## Tasks Executed

### Task 1: Add wizard_library_default_follows_custom_tome_home integration test (commit a5cc0a0)

Two new integration tests appended to `crates/tome/tests/cli_init.rs`, grouped immediately after the existing `init_derived_library_default_under_custom_tome_home` test under a `// === FIX-05: ... ===` section header:

1. **`wizard_library_default_follows_custom_tome_home`** — D-FIX05-1 positive contract. Runs `tome init --dry-run --no-input` with `HOME=<tmp>` and `TOME_HOME=<tmp>/custom-tome`. Parses the emitted TOML body via the file's existing `parse_generated_config` helper and asserts `config.library_dir() == <tmp>/custom-tome/skills`. Greenfield path (no config seeded), so the wizard's library-default selection is exercised — not the brownfield "Existing config detected" branch.

2. **`wizard_library_default_does_not_fall_back_to_home_tome_skills`** — D-FIX05-2 no-fallback contract. Same fixture, asserts `config.library_dir() != <tmp>/.tome/skills` (the HOME-anchored fallback that would manifest if someone hardcoded `~/.tome/skills` in `configure_library`).

Both assertions operate on the parsed TOML body (stdout), not on chrome substring matching, matching the idiomatic pattern of the file's other library-default tests.

## Deviations from Plan

Three minor deviations from the plan's RESEARCH-quoted test sketch — none affect the acceptance criteria:

1. **Assertion shape:** Plan/RESEARCH proposed `combined.contains("<custom_tome_home>/skills")` on `stderr+stdout`. I used `parse_generated_config(&stdout).library_dir() == <expected>` instead. The file's existing similar test (`init_derived_library_default_under_custom_tome_home`) uses the parsed-config assertion, and that pattern is more robust against wizard chrome-text changes. Acceptance criteria 4 (`cargo test` exits 0) and the FIX-05 success criterion (wizard library default pinned) are satisfied either way.

2. **No-fallback assertion:** Plan/RESEARCH proposed `!combined.contains("~/.tome/skills")`. I used `assert_ne!(library_dir, <HOME>/.tome/skills)` (where HOME is the TempDir). The original would have a false-positive risk if any unrelated chrome text mentioned `~/.tome/skills`; the precise negative on the parsed library_dir avoids that and matches the file's idiomatic style.

3. **Test file location:** Plan said "append to `crates/tome/tests/cli_init.rs`" if it exists. It exists. Tests were appended after `init_derived_library_default_under_custom_tome_home` (related test) rather than at the very end of the file, keeping the library-default tests grouped.

## Sensitivity Check Result

Per the plan's optional smoke test (step 6 of the action plan): I temporarily replaced `crates/tome/src/wizard.rs:637`'s `crate::paths::collapse_home_path(&tome_home.join("skills"))` with `PathBuf::from("~/.tome/skills")` (the exact regression we're guarding against) and re-ran `cargo test -p tome --test cli_init wizard_library_default`. **Both new tests FAILED** as expected, with the no-fallback test producing the diagnostic:

```
assertion `left != right` failed: wizard must not fall back to <HOME>/.tome/skills when TOME_HOME is set; got library_dir="/var/folders/.../T/.tmprK3RJ1/.tome/skills"
  left:  "/var/folders/.../T/.tmprK3RJ1/.tome/skills"
  right: "/var/folders/.../T/.tmprK3RJ1/.tome/skills"
```

I then reverted the mutation, re-ran the suite — both tests pass green. `git diff crates/tome/src/wizard.rs` is empty post-revert. The regression guard is verified sensitive to the exact bug pattern it's meant to catch.

## Caveats / CI Notes

- **TempDir is NOT under `$HOME` in CI.** macOS `TempDir::new()` returns `/var/folders/...`; Linux returns `/tmp/...`. Both are outside the test's `HOME=<tmp>` env, so `collapse_home_path` does NOT abbreviate the test's TempDir path to `~/`. The library_dir assertions compare absolute paths, which is correct on both platforms.

- **`HOME=<tmp>` interaction.** The test sets `HOME=<tmp>` (the TempDir root) and `TOME_HOME=<tmp>/custom-tome`. Because `custom-tome` IS under `HOME=<tmp>`, the wizard's `collapse_home_path(tome_home.join("skills"))` will emit `~/custom-tome/skills` in the TOML body. After `parse_generated_config` calls `Config::load`'s tilde expansion against the test-process's `HOME` (note: NOT the child process's HOME — `dirs::home_dir()` doesn't honor env per-process), the resulting absolute path is what we assert against. This matches the existing `init_derived_library_default_under_custom_tome_home` test's working pattern at line 599-635.

- **Pure greenfield contract.** Both new tests create only an empty `<tmp>/custom-tome/` directory — no `tome.toml` is seeded. This lands the wizard in the greenfield code path (NOT the "Existing config detected" brownfield branch), where `configure_library` is actually invoked.

## Verification

```bash
cargo test -p tome --test cli_init wizard_library_default
# running 2 tests
# test wizard_library_default_does_not_fall_back_to_home_tome_skills ... ok
# test wizard_library_default_follows_custom_tome_home ... ok
# test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 18 filtered out

cargo test -p tome --test cli_init
# test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

cargo clippy --all-targets -- -D warnings
# Finished `dev` profile [unoptimized + debuginfo] target(s) in 15.97s (no warnings)

cargo fmt -- --check
# clean (no output)
```

## Commits

- `a5cc0a0` — test(19-06): pin wizard library default to TOME_HOME (FIX-05, #453 + #456)

## Self-Check: PASSED

- crates/tome/tests/cli_init.rs: FOUND
- Commit a5cc0a0: FOUND in git log
- wizard.rs: NOT modified (confirmed via `git diff crates/tome/src/wizard.rs` — empty)
- 2 new tests pass
- Full cli_init suite (20 tests) passes
- clippy clean with -D warnings
- fmt clean
