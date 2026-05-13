---
phase: 19-doctor-status-surface-bugfix-bundle
plan: 06
type: execute
wave: 2
depends_on: [01]
files_modified:
  - crates/tome/tests/cli_init.rs
autonomous: true
requirements: [FIX-05]
requirements_addressed: [FIX-05]

must_haves:
  truths:
    - "Wizard library-default derivation is ALREADY implemented at wizard.rs:637 (`<tome_home>/skills` derived from the resolved tome_home — RESEARCH confirmed)"
    - "The MISSING piece per RESEARCH is the pinning integration test — no code change to wizard.rs needed for the implementation; this plan is test-only"
    - "Integration test drives `tome init --dry-run --no-input` with a custom TOME_HOME and asserts the proposed library default is `<TOME_HOME>/skills`, NOT the hardcoded `~/.tome/skills` fallback"
    - "Test also asserts that NO fallback to `~/.tome/skills` occurs when TOME_HOME is set (D-FIX05-2 no-fallback contract)"
  artifacts:
    - path: "crates/tome/tests/cli_init.rs"
      provides: "Integration test `wizard_library_default_follows_custom_tome_home` driving a custom TOME_HOME in --no-input mode"
      contains: "wizard_library_default_follows_custom_tome_home"
  key_links:
    - from: "crates/tome/tests/cli_init.rs::wizard_library_default_follows_custom_tome_home"
      to: "wizard.rs:637 library-default derivation (already in place)"
      via: "TOME_HOME env var + `init --dry-run --no-input` invocation; assertion on combined stdout/stderr output"
      pattern: "TOME_HOME"
---

<objective>
Close GitHub #453 + #456 (wizard library-default not following tome_home) by adding the pinning integration test that RESEARCH.md flagged as the actual missing piece. The implementation at `wizard.rs:637` already derives `<resolved_tome_home>/skills` correctly — what's missing is the regression test pinning this behavior.

Purpose: Close two GitHub issues with a single test addition. No production code change needed. The test prevents future regressions where someone might re-introduce a hardcoded `~/.tome/skills` fallback.
Output: One new integration test in `crates/tome/tests/cli_init.rs` (or new file) driving a custom TOME_HOME through wizard `--no-input` mode.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/phases/19-doctor-status-surface-bugfix-bundle/19-CONTEXT.md
@.planning/phases/19-doctor-status-surface-bugfix-bundle/19-RESEARCH.md
@crates/tome/src/wizard.rs

<interfaces>
<!-- Existing implementation that the test pins (RESEARCH-verified). -->

**`crates/tome/src/wizard.rs:637`** — already derives library default from tome_home:
```rust
let default = crate::paths::collapse_home_path(&tome_home.join("skills"));
```
The function signature at `wizard.rs:632` takes `tome_home: &Path`. The call chain (from `lib.rs::run_wizard` or similar) threads the resolved `tome_home` value, which already accounts for `TOME_HOME` env var override + tilde expansion.

**`crates/tome/src/wizard.rs:430-460`** — the wizard's existing path-creation flow (handles missing-directory case orthogonally; D-FIX05-2's "no fallback" only matters at the derivation site, not here).

**Existing CLI test harness pattern** (from `crates/tome/tests/cli.rs`):
```rust
use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn some_test() {
    let tmp = TempDir::new().unwrap();
    let output = Command::cargo_bin("tome").unwrap()
        .env("TOME_HOME", tmp.path())
        .args(["init", "--dry-run", "--no-input"])
        .output().unwrap();
    // ...
}
```
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Add wizard_library_default_follows_custom_tome_home integration test</name>
  <files>crates/tome/tests/cli_init.rs</files>
  <read_first>
    - crates/tome/src/wizard.rs lines 625-680 (`configure_library` function — confirm `<tome_home>/skills` derivation is still at line 637 or thereabouts)
    - crates/tome/tests/cli.rs (existing CLI test harness style — `Command::cargo_bin`, `TempDir`, `--no-input` invocation, env-var pattern)
    - .planning/phases/19-doctor-status-surface-bugfix-bundle/19-CONTEXT.md (D-FIX05-1, D-FIX05-2)
    - .planning/phases/19-doctor-status-surface-bugfix-bundle/19-RESEARCH.md "FIX-05 (wizard library default — closes #453 + #456)" section (lines 610-655) — exact test shape + caveats
    - Check whether `crates/tome/tests/cli_init.rs` already exists with `fd cli_init crates/tome/tests`. If not, create it.
  </read_first>
  <behavior>
    - Test 1 (D-FIX05-1): With `TOME_HOME=<tmpdir>/custom-tome`, run `tome init --dry-run --no-input`. Combined stdout+stderr contains `<tmpdir>/custom-tome/skills` (the derived library default).
    - Test 2 (D-FIX05-2): Same invocation. Combined stdout+stderr does NOT contain the hardcoded `~/.tome/skills` (no-fallback contract).
  </behavior>
  <action>
    1. **Check whether `crates/tome/tests/cli_init.rs` exists:**
       ```bash
       fd cli_init crates/tome/tests
       ```
       If it does: append the new test to the existing file.
       If it does not: create the file with the standard test-file header pattern matching `crates/tome/tests/cli.rs` (top 5 lines — imports + `use assert_cmd::Command;` etc.).

    2. **Add the integration test** (test shape from RESEARCH.md FIX-05 specifics, lines 627-648):

       ```rust
       //! Wizard integration tests — pin the library-default derivation
       //! against TOME_HOME (FIX-05, closes #453 + #456). The implementation
       //! at wizard.rs:637 already derives `<resolved_tome_home>/skills`;
       //! these tests prevent future regressions where someone might
       //! re-introduce a hardcoded `~/.tome/skills` fallback.

       use assert_cmd::Command;
       use std::fs;
       use tempfile::TempDir;

       #[test]
       fn wizard_library_default_follows_custom_tome_home() {
           // CAVEAT (per RESEARCH): `tome init --no-input` against a TOME_HOME
           // with existing config short-circuits to the "Existing config detected"
           // branch. The wizard's library-default proposal only appears in the
           // greenfield path — so the fixture MUST NOT seed any config.
           let tmp = TempDir::new().unwrap();
           let custom_tome_home = tmp.path().join("custom-tome");
           fs::create_dir_all(&custom_tome_home).unwrap();

           let output = Command::cargo_bin("tome").unwrap()
               .env("TOME_HOME", &custom_tome_home)
               .args(["init", "--dry-run", "--no-input"])
               .output()
               .unwrap();

           let stderr = String::from_utf8_lossy(&output.stderr);
           let stdout = String::from_utf8_lossy(&output.stdout);
           let combined = format!("{stderr}{stdout}");

           let expected = format!("{}/skills", custom_tome_home.display());
           assert!(
               combined.contains(&expected),
               "library default must follow TOME_HOME (expected substring: '{expected}').\n\
                Full output:\n{combined}"
           );
       }

       #[test]
       fn wizard_library_default_does_not_fall_back_to_home_tome_skills() {
           // D-FIX05-2 no-fallback contract: even if `<TOME_HOME>/skills` doesn't
           // exist on disk, the wizard does NOT fall back to `~/.tome/skills`.
           let tmp = TempDir::new().unwrap();
           let custom_tome_home = tmp.path().join("custom-tome");
           fs::create_dir_all(&custom_tome_home).unwrap();

           let output = Command::cargo_bin("tome").unwrap()
               .env("TOME_HOME", &custom_tome_home)
               .args(["init", "--dry-run", "--no-input"])
               .output()
               .unwrap();

           let stderr = String::from_utf8_lossy(&output.stderr);
           let stdout = String::from_utf8_lossy(&output.stdout);
           let combined = format!("{stderr}{stdout}");

           // The literal hardcoded path "~/.tome/skills" must NOT appear in
           // the proposed default (it would indicate a regression to the
           // pre-FIX-05 behavior). Allow it elsewhere only if it appears as
           // an unrelated help text reference — verify by checking against
           // the explicit fallback pattern.
           // Note: collapse_home_path may emit "~/<rest>" if TOME_HOME happens
           // to be under $HOME. The TempDir is NOT under $HOME on macOS/Linux
           // CI (it's under /tmp or /var/folders), so the literal "~/.tome/skills"
           // string would only appear from a hardcoded fallback.
           assert!(
               !combined.contains("~/.tome/skills"),
               "wizard must not fall back to ~/.tome/skills when TOME_HOME is set.\n\
                Full output:\n{combined}"
           );
       }
       ```

    3. **Verify the test invocation actually exercises the greenfield wizard path:**
       Per RESEARCH caveat: "If `tome init` in `--no-input` mode against a TOME_HOME with existing config short-circuits to the 'Existing config detected' branch... the test fixture must NOT seed any config — pure greenfield."

       The test as written creates only an empty `<tmpdir>/custom-tome/` directory — NO `tome.toml` is seeded. This should land in the greenfield wizard branch. If the test fails because the wizard short-circuits OR the output doesn't contain the expected substring, the executor should:
       - Run the same invocation manually to inspect the actual output (`cargo run -p tome -- init --dry-run --no-input` with TOME_HOME set to a fresh TempDir)
       - Adjust the test's expected substring to match the actual greenfield-wizard output format (e.g. the wizard may display the path as `~/.tome/skills` collapsed OR as the absolute path — adjust the assertion accordingly)
       - The CORE assertion is: the proposed library path resolves under `<custom_tome_home>`, not under `~/.tome/`

    4. **Do NOT modify `wizard.rs`** — RESEARCH confirmed the implementation at `:637` is already correct. This plan is test-only.

    5. **If the test FAILS** even after greenfield-path adjustments, it would mean RESEARCH's finding ("library-default derivation is already implemented") was wrong. In that case, escalate: re-read `wizard.rs:625-680`, find where the fallback is, fix it to use `tome_home.join("skills")`, and add a note in `19-06-SUMMARY.md` documenting the deviation from RESEARCH.

    6. **Smoke test the assertion is sensitive enough**: temporarily mutate `wizard.rs:637` to hardcode `~/.tome/skills`, re-run the test, confirm it FAILS, then revert. This proves the test is actually pinning the behavior. (Optional but recommended — adds confidence the regression guard works.)
  </action>
  <verify>
    <automated>cargo test -p tome --test cli_init wizard_library_default</automated>
  </verify>
  <acceptance_criteria>
    - `crates/tome/tests/cli_init.rs` exists
    - `rg "wizard_library_default_follows_custom_tome_home" crates/tome/tests/cli_init.rs` returns 1 match
    - `rg "wizard_library_default_does_not_fall_back_to_home_tome_skills" crates/tome/tests/cli_init.rs` returns 1 match
    - `cargo test -p tome --test cli_init wizard_library_default` exits 0 (2 new tests pass)
    - `cargo clippy --all-targets -- -D warnings` exits 0
    - `crates/tome/src/wizard.rs` is NOT modified (this plan is test-only)
  </acceptance_criteria>
  <done>Two integration tests pin the wizard library-default-follows-tome_home behavior; both pass; clippy clean; wizard.rs unchanged.</done>
</task>

</tasks>

<verification>
- `cargo test -p tome --test cli_init wizard_library_default` — 2 tests pass
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo fmt -- --check` — clean
- Sensitivity check (optional): temporarily replace `wizard.rs:637` with a hardcoded fallback, confirm test fails, revert
</verification>

<success_criteria>
- FIX-05: Wizard library default derivation is pinned by two integration tests (positive: follows TOME_HOME; negative: no fallback to ~/.tome/skills)
- Closes GitHub #453 + #456 with a single fix (test-only — implementation was already correct)
- No production code change in `wizard.rs` (RESEARCH-confirmed implementation already correct at :637)
</success_criteria>

<output>
After completion, create `.planning/phases/19-doctor-status-surface-bugfix-bundle/19-06-SUMMARY.md` documenting:
- Confirmation that wizard.rs was NOT modified (RESEARCH finding upheld)
- Final test assertion strings used (in case greenfield-path output format required adjustment from RESEARCH's example)
- Sensitivity-check result (did temporarily mutating wizard.rs:637 make the test fail? if yes, regression guard verified)
- Any caveats about CI behavior (e.g. tilde-expansion in TempDir paths)
</output>
