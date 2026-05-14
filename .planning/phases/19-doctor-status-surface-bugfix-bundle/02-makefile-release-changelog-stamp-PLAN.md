---
phase: 19-doctor-status-surface-bugfix-bundle
plan: 02
type: execute
wave: 1
depends_on: []
files_modified:
  - Makefile
  - crates/tome/tests/cli_make_release.rs
autonomous: true
requirements: [FIX-06]
requirements_addressed: [FIX-06]

must_haves:
  truths:
    - "make release VERSION=X.Y.Z substitutes '## [Unreleased]' → '## [X.Y.Z] - YYYY-MM-DD' in CHANGELOG.md"
    - "The substitution is idempotent — if CHANGELOG.md has no '[Unreleased]' section, sed is a silent no-op and the release proceeds"
    - "The CHANGELOG.md edit is staged alongside Cargo.toml + Cargo.lock in the version-bump commit"
    - "Date format is YYYY-MM-DD via `date -u +%Y-%m-%d` (UTC, BSD/GNU portable)"
  artifacts:
    - path: "Makefile"
      provides: "Inline sed line in the release recipe that stamps CHANGELOG.md, plus an idempotency-explainer comment"
      contains: "sed -i '' \"s/^## \\[Unreleased\\]"
    - path: "crates/tome/tests/cli_make_release.rs"
      provides: "Rust integration test that runs the sed command against a fixture CHANGELOG and asserts substitution + idempotency"
      contains: "make_release_sed_replaces_unreleased_section"
  key_links:
    - from: "Makefile release recipe"
      to: "CHANGELOG.md"
      via: "sed -i '' substitution executed between cargo check and branch creation"
      pattern: "sed.*Unreleased.*CHANGELOG\\.md"
    - from: "Makefile release recipe"
      to: "git add"
      via: "CHANGELOG.md added to the same commit as Cargo.toml + Cargo.lock"
      pattern: "git add Cargo\\.toml Cargo\\.lock CHANGELOG\\.md"
---

<objective>
Close GitHub #533: `make release VERSION=X.Y.Z` should automatically stamp the release date in CHANGELOG.md, replacing `## [Unreleased]` with `## [X.Y.Z] - YYYY-MM-DD`. Inline sed in the existing Makefile recipe — style matches the existing Cargo.toml version-bump sed line. Idempotent (no-op if `[Unreleased]` is missing).

Purpose: Eliminate manual CHANGELOG date-stamping during release cuts. Lands in Wave 1 so the v0.11 release cut (sequenced AFTER Phase 19) finds an updated Makefile.
Output: One added `sed -i ''` line in Makefile + CHANGELOG.md added to the staged-files list + a Makefile comment documenting idempotency + a regression test.
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
@Makefile
@CHANGELOG.md

<interfaces>
<!-- Existing Makefile release recipe shape (lines 14-32 per RESEARCH).
     The sed insertion point is between `cargo check` and the branch-creation step. -->

Existing release recipe excerpt (Makefile:14-32):
```makefile
release:
    @if [ -z "$$VERSION" ]; then echo "Usage: make release VERSION=X.Y.Z" >&2; exit 1; fi
    @SEMVER="$$VERSION"; \
    TAG="v$$SEMVER"; \
    echo "Cutting release $$TAG..."; \
    sed -i '' "s/^version = .*/version = \"$$SEMVER\"/" Cargo.toml; \
    cargo check --quiet; \
    BRANCH="chore/release-$$TAG"; \
    git checkout -b "$$BRANCH"; \
    git add Cargo.toml Cargo.lock; \
    git commit -m "chore(release): $$TAG"; \
    git push -u origin "$$BRANCH"; \
    gh pr create --title "chore(release): $$TAG" --body "Release $$TAG" --draft; \
    gh pr merge --squash --auto; \
    git tag "$$TAG"; \
    git push origin "$$TAG"
```

Existing CHANGELOG.md header pattern (verified from CHANGELOG.md):
- `## [Unreleased]` line at the top of the current in-progress section
- Past sections use the form `## [X.Y.Z] - YYYY-MM-DD` (e.g. `## [0.10.0] - 2026-05-11`)
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Write the FIX-06 regression test FIRST (RED) — sed substitution + idempotency</name>
  <files>crates/tome/tests/cli_make_release.rs</files>
  <read_first>
    - Makefile (full file — current release recipe shape)
    - CHANGELOG.md (top 30 lines — current `[Unreleased]` block shape)
    - .planning/phases/19-doctor-status-surface-bugfix-bundle/19-RESEARCH.md "FIX-06 (`make release` CHANGELOG date-stamp — closes #533)" section (lines 656-733)
    - crates/tome/tests/cli.rs (top 30 lines — for test harness style: use of TempDir, std::process::Command, std::fs::write)
  </read_first>
  <behavior>
    - Test 1: Sed against a fixture CHANGELOG containing `## [Unreleased]` substitutes the line to `## [0.99.0] - <today's UTC date>`.
    - Test 2: Re-running the sed against the post-substitution content is idempotent (no further changes — the second invocation does NOT find `## [Unreleased]` to replace, so the file is byte-identical).
    - Test 3: Sed against a CHANGELOG without `## [Unreleased]` is a silent no-op (exit 0, file unchanged).
  </behavior>
  <action>
    Create `crates/tome/tests/cli_make_release.rs` with a Rust integration test that exercises the same `sed` invocation the Makefile will use. The test does NOT shell out to `make` — it shells out to `sed` directly with the same command string the Makefile recipe will contain. This avoids requiring a checked-out git repo + branch setup inside the test.

    ```rust
    //! Regression test for FIX-06 (#533): `make release` stamps the release
    //! date in CHANGELOG.md by replacing `## [Unreleased]` with
    //! `## [X.Y.Z] - YYYY-MM-DD`. This test exercises the exact `sed` command
    //! the Makefile recipe runs.

    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    /// Returns today's UTC date as `YYYY-MM-DD` via `date -u +%Y-%m-%d` —
    /// the same command the Makefile recipe uses. Portable across BSD `date`
    /// (macOS) and GNU `date` (Linux).
    fn today_utc() -> String {
        let out = Command::new("date")
            .args(["-u", "+%Y-%m-%d"])
            .output()
            .expect("invoke `date`");
        String::from_utf8(out.stdout)
            .expect("date output is UTF-8")
            .trim()
            .to_string()
    }

    #[test]
    fn make_release_sed_replaces_unreleased_section() {
        let tmp = TempDir::new().unwrap();
        let changelog = tmp.path().join("CHANGELOG.md");
        fs::write(
            &changelog,
            "## [Unreleased]\n\n### Added\n- foo\n\n## [0.10.0] - 2026-05-11\n",
        )
        .unwrap();

        let date = today_utc();
        let sed_expr = format!("s/^## \\[Unreleased\\]/## [0.99.0] - {date}/");
        let status = Command::new("sed")
            .args(["-i", "", &sed_expr, changelog.to_str().unwrap()])
            .status()
            .unwrap();
        assert!(status.success(), "sed exited non-zero");

        let content = fs::read_to_string(&changelog).unwrap();
        let expected_line = format!("## [0.99.0] - {date}");
        assert!(
            content.contains(&expected_line),
            "sed did not replace [Unreleased]; got:\n{content}"
        );
        assert!(
            !content.contains("## [Unreleased]"),
            "[Unreleased] line still present after sed; got:\n{content}"
        );
    }

    #[test]
    fn make_release_sed_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let changelog = tmp.path().join("CHANGELOG.md");
        fs::write(
            &changelog,
            "## [Unreleased]\n\n### Added\n- foo\n",
        )
        .unwrap();

        let date = today_utc();
        let sed_expr = format!("s/^## \\[Unreleased\\]/## [0.99.0] - {date}/");
        Command::new("sed")
            .args(["-i", "", &sed_expr, changelog.to_str().unwrap()])
            .status()
            .unwrap();
        let first_pass = fs::read_to_string(&changelog).unwrap();

        // Second pass with a DIFFERENT version — must NOT find [Unreleased]
        // to replace (it's already gone), so the file is byte-identical.
        let sed_expr2 = format!("s/^## \\[Unreleased\\]/## [1.0.0] - {date}/");
        let status2 = Command::new("sed")
            .args(["-i", "", &sed_expr2, changelog.to_str().unwrap()])
            .status()
            .unwrap();
        assert!(status2.success(), "second sed exited non-zero");
        let second_pass = fs::read_to_string(&changelog).unwrap();
        assert_eq!(
            first_pass, second_pass,
            "sed must be idempotent (no [Unreleased] left after first run)"
        );
    }

    #[test]
    fn make_release_sed_silent_noop_when_no_unreleased_section() {
        let tmp = TempDir::new().unwrap();
        let changelog = tmp.path().join("CHANGELOG.md");
        let original = "## [0.10.0] - 2026-05-11\n\n### Added\n- foo\n";
        fs::write(&changelog, original).unwrap();

        let date = today_utc();
        let sed_expr = format!("s/^## \\[Unreleased\\]/## [0.99.0] - {date}/");
        let status = Command::new("sed")
            .args(["-i", "", &sed_expr, changelog.to_str().unwrap()])
            .status()
            .unwrap();
        assert!(status.success(), "sed exited non-zero on no-match input");

        let content = fs::read_to_string(&changelog).unwrap();
        assert_eq!(content, original, "file must be unchanged when no [Unreleased] is present");
    }
    ```

    Run the test once after creating it. **Expected outcome at this point: tests PASS already** (because the `sed` command is platform-side, not Makefile-side — there's no Makefile change required to make this test green). This is intentional: this test is the contract the Makefile change in Task 2 must continue to honor. The test is a regression guard ensuring the `sed` pattern works as documented in CONTEXT.md.

    NOTE for the executor: the Linux CI uses GNU sed which accepts `sed -i ''` (treating `''` as the suffix for `-i`, which is empty — both BSD and GNU sed accept this form, though it's idiomatic to BSD). Confirmed in RESEARCH.md ("`date -u +%Y-%m-%d` is portable across macOS BSD `date` and GNU `date`"). If a CI flake surfaces on Linux, fall back to `sed -i.bak` and add the `.bak` to a `.gitignore` entry — but expectation is the BSD form works.
  </action>
  <verify>
    <automated>cargo test -p tome --test cli_make_release</automated>
  </verify>
  <acceptance_criteria>
    - `crates/tome/tests/cli_make_release.rs` exists
    - `rg "make_release_sed_replaces_unreleased_section" crates/tome/tests/cli_make_release.rs` returns 1 match
    - `rg "make_release_sed_is_idempotent" crates/tome/tests/cli_make_release.rs` returns 1 match
    - `rg "make_release_sed_silent_noop_when_no_unreleased_section" crates/tome/tests/cli_make_release.rs` returns 1 match
    - `cargo test -p tome --test cli_make_release` exits 0 (3 tests pass)
    - `cargo clippy --all-targets -- -D warnings` exits 0
  </acceptance_criteria>
  <done>Three regression tests exist for the sed substitution + idempotency contract; all pass; test file is clippy-clean.</done>
</task>

<task type="auto">
  <name>Task 2: Add the sed line + comment to the Makefile release recipe; update git add to include CHANGELOG.md</name>
  <files>Makefile</files>
  <read_first>
    - Makefile (full file — verify current `release:` recipe shape and the `git add Cargo.toml Cargo.lock` line)
    - .planning/phases/19-doctor-status-surface-bugfix-bundle/19-RESEARCH.md "FIX-06" section, especially lines 660-682 (exact sed line + insertion point) and lines 671-682 (comment + `git add` update)
  </read_first>
  <action>
    Modify `Makefile`. The release recipe currently has this shape (RESEARCH-verified lines 14-32; executor reads to confirm exact line numbers may have drifted):

    ```makefile
    sed -i '' "s/^version = .*/version = \"$$SEMVER\"/" Cargo.toml; \
    cargo check --quiet; \
    BRANCH="chore/release-$$TAG"; \
    ...
    git add Cargo.toml Cargo.lock; \
    ```

    1. **Insert a new sed line** AFTER the existing `sed -i '' "s/^version = ..."` line and AFTER `cargo check --quiet;` — placed between `cargo check --quiet;` and `BRANCH="chore/release-$$TAG";` per D-FIX06-1. Use a tab (Makefile recipe indent) + continuation backslash style matching the surrounding lines:

       ```makefile
       # Stamp the release date in CHANGELOG.md by replacing [Unreleased] with [VERSION] - DATE.
       # Idempotent: if CHANGELOG.md lacks an [Unreleased] section, sed is a no-op and the
       # release proceeds without the changelog edit. Style matches the Cargo.toml version-bump
       # sed line above.
       sed -i '' "s/^## \[Unreleased\]/## [$$SEMVER] - $$(date -u +%Y-%m-%d)/" CHANGELOG.md; \
       ```

       IMPORTANT: Makefile escaping rules — `$$` for shell `$`, `\[` for literal `[`. The exact line is:
       ```
       sed -i '' "s/^## \[Unreleased\]/## [$$SEMVER] - $$(date -u +%Y-%m-%d)/" CHANGELOG.md; \
       ```

       Place the multi-line comment (the `# Stamp...` block) ABOVE the sed line. Comments inside a recipe-continuation block need leading whitespace + `#`; the recipe's existing comment style (if any) is the template. If no recipe-internal comment style is established, place the explanatory comment ABOVE the entire `release:` recipe block instead, and keep only a single short `# FIX-06: stamp CHANGELOG date` line above the sed for inline context.

    2. **Update the `git add` line** to include `CHANGELOG.md`:

       Before: `git add Cargo.toml Cargo.lock; \`
       After:  `git add Cargo.toml Cargo.lock CHANGELOG.md; \`

       This ensures the CHANGELOG.md edit ships in the same commit as the version bump (D-FIX06-1 contract: "The CHANGELOG.md edit is staged and committed alongside the version bump").

    3. **Do NOT change** any other line in the recipe. The `git commit -m` message, branch name, PR creation, tag push are all unchanged.

    4. **Verify the recipe still parses** by running `make -n release VERSION=0.0.0` (dry-run mode prints the recipe without executing). The output should show the new sed line.

    NOTE on date `-u`: per RESEARCH.md "Date format `YYYY-MM-DD`: Matches existing CHANGELOG entries — verify `CHANGELOG.md:106` (`## [0.10.0] - 2026-05-11`). `date -u +%Y-%m-%d` is portable across macOS BSD `date` and GNU `date`. ✓."
  </action>
  <verify>
    <automated>rg "## \\[Unreleased\\]" Makefile && rg "CHANGELOG\\.md" Makefile && make -n release VERSION=0.0.0 2>/dev/null | rg "CHANGELOG\\.md"</automated>
  </verify>
  <acceptance_criteria>
    - `rg 'sed -i.*\[Unreleased\].*CHANGELOG\.md' Makefile` returns 1 match
    - `rg 'date -u \+%Y-%m-%d' Makefile` returns 1 match
    - `rg 'git add Cargo\.toml Cargo\.lock CHANGELOG\.md' Makefile` returns 1 match (CHANGELOG.md added to the staged-files list)
    - `make -n release VERSION=0.0.0` includes the new sed line in its dry-run output (verifies recipe syntax + line ordering)
    - `cargo test -p tome --test cli_make_release` still exits 0 (Task 1 tests remain green)
  </acceptance_criteria>
  <done>Makefile recipe has the new sed line between cargo check and branch creation; git add includes CHANGELOG.md; dry-run shows the new line; Task 1 regression tests still pass.</done>
</task>

</tasks>

<verification>
- `cargo test -p tome --test cli_make_release` — 3 tests pass
- `make -n release VERSION=0.0.0 2>/dev/null | rg "sed.*Unreleased.*CHANGELOG"` returns the new line
- `rg "git add Cargo\\.toml Cargo\\.lock CHANGELOG\\.md" Makefile` — 1 match
- Optional smoke: create a fixture CHANGELOG.md in a temp dir with `[Unreleased]` + run the sed command verbatim from the Makefile recipe + verify the substitution
</verification>

<success_criteria>
- FIX-06: `make release VERSION=X.Y.Z` substitutes `## [Unreleased]` → `## [X.Y.Z] - YYYY-MM-DD` in CHANGELOG.md and stages the file alongside Cargo.toml + Cargo.lock
- Idempotency: a release without an `[Unreleased]` section proceeds without error (sed is a silent no-op)
- Test count increases by 3 (the three regression tests in `cli_make_release.rs`)
</success_criteria>

<output>
After completion, create `.planning/phases/19-doctor-status-surface-bugfix-bundle/19-02-SUMMARY.md` documenting:
- Final Makefile diff (the inserted sed line + git add update)
- Any platform-portability adjustments made (e.g. if BSD `sed -i ''` form had issues and was changed)
- Confirmation that `make -n release` dry-run shows the new line
</output>
