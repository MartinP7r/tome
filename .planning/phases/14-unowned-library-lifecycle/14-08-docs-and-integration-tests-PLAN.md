---
phase: 14-unowned-library-lifecycle
plan: 08
type: execute
wave: 4
depends_on:
  - 14-04
  - 14-05
  - 14-06
  - 14-07
files_modified:
  - .planning/REQUIREMENTS.md
  - .planning/ROADMAP.md
  - .planning/PROJECT.md
  - CHANGELOG.md
  - crates/tome/tests/cli.rs
autonomous: true
requirements:
  - UNOWN-01
  - UNOWN-02
  - UNOWN-03

must_haves:
  truths:
    - "REQUIREMENTS.md / ROADMAP.md / PROJECT.md text reflects the API merge: `tome adopt` is folded into `tome reassign`; `tome forget` is folded into `tome remove skill`; verbatim wording updated where appropriate; supersession notes added so traceability is preserved."
    - "CHANGELOG.md `[Unreleased]` section calls out the breaking change to `tome remove <name>` (now `tome remove dir <name>`) and the new `tome remove skill <name>` and `tome reassign <unowned> --to <dir>` flows."
    - "End-to-end integration tests in `tests/cli.rs` exercise UNOWN-01..03 against the real binary."
    - "`make ci` (fmt-check + clippy -D warnings + tests) passes on the final commit."
  artifacts:
    - path: ".planning/REQUIREMENTS.md"
      provides: "Updated UNOWN-01/02 wording with supersession notes pointing at D-API-1/-2"
    - path: ".planning/ROADMAP.md"
      provides: "Updated Phase 14 success criteria 1/2 wording"
    - path: ".planning/PROJECT.md"
      provides: "Updated Key Decisions table entry for the unowned lifecycle vocab"
    - path: "CHANGELOG.md"
      provides: "v0.10 [Unreleased] entries for UNOWN-01..03 + breaking-change callout"
    - path: "crates/tome/tests/cli.rs"
      provides: "≥6 new end-to-end integration tests covering UNOWN-01..03 success criteria"
      min_lines: 200
  key_links:
    - from: "REQUIREMENTS.md UNOWN-01/02"
      to: "14-CONTEXT.md D-API-1/-2"
      via: "supersession note"
    - from: "tests/cli.rs"
      to: "tome binary (assert_cmd)"
      via: "Command-driven E2E tests"
---

<objective>
Two parallel streams in this plan:

1. **Documentation traceability cleanup** (per CONTEXT.md "Deferred Ideas"
   bullet 5 + "canonical_refs" footnotes). UNOWN-01 and UNOWN-02 in
   `REQUIREMENTS.md` say `tome adopt` and `tome forget`, superseded by
   D-API-1 and D-API-2. ROADMAP success criteria 1 and 2 use the same
   vocabulary. PROJECT.md Key Decisions table line 142 mentions
   `tome adopt` / `tome forget`. CHANGELOG.md must call out the breaking
   change to `tome remove <name>` (now `tome remove dir <name>`).

2. **End-to-end integration tests** in `crates/tome/tests/cli.rs` exercising
   UNOWN-01..03 success criteria via `assert_cmd` — the real binary. Each
   ROADMAP success criterion gets at least one test:
   - SC-1: `tome reassign <unowned> --to <dir>` happy path; `tome reassign
     <unowned> --to <nonexistent>` fails fast.
   - SC-2: `tome remove skill <unowned>` happy path; `tome remove skill
     <owned>` refused with D-B2 hint; `tome remove skill <unowned> --yes`
     skips confirmation.
   - SC-3: `tome status` and `tome doctor` text + `--json` show Unowned
     section correctly; empty case omits cleanly; doctor's total_issues
     does not count unowned.

Final `make ci` must pass: fmt-check + clippy -D warnings + tests.

Note for HARD-13 (Phase 15): tests land in `tests/cli.rs` today; Phase 15
will split them into per-domain `tests/cli_remove.rs` etc. Phase 14 just
adds tests to the existing monolith.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/phases/14-unowned-library-lifecycle/14-CONTEXT.md
@CHANGELOG.md

# Source-of-truth pattern files:
@crates/tome/tests/cli.rs

<interfaces>
<!-- ROADMAP.md Phase 14 success criteria (verbatim, lines 132-135): -->
1. `tome adopt <skill> <directory>` re-anchors an unowned skill...
2. `tome forget <skill>` deletes an unowned skill...
3. `tome status` and `tome doctor` text output include an `Unowned skills (N):`...

<!-- REQUIREMENTS.md UNOWN block (verbatim, lines 41-46): -->
- UNOWN-01: `tome adopt <skill> <directory>` re-anchors...
- UNOWN-02: `tome forget <skill>` explicitly deletes...
- UNOWN-03: `tome status` and `tome doctor` surface the unowned set...

<!-- assert_cmd pattern (existing tests/cli.rs uses this — verify with: -->
<!--   rg "assert_cmd::Command::cargo_bin" crates/tome/tests/cli.rs | head -3) -->
```rust
let mut cmd = Command::cargo_bin("tome").unwrap();
cmd.arg("status");
cmd.assert().success().stdout(predicate::str::contains("..."));
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Update planning docs and CHANGELOG.md to reflect the API merge</name>
  <read_first>
    - .planning/REQUIREMENTS.md (UNOWN-01..03 block lines 41-46; the Traceability table entry around line 140-142)
    - .planning/ROADMAP.md (Phase 14 block, particularly Success Criteria 1-3 around lines 132-135)
    - .planning/PROJECT.md (search for "tome adopt" or "tome forget" — `rg "tome adopt|tome forget" .planning/PROJECT.md` to locate)
    - CHANGELOG.md (look at the v0.9 entry for the format pattern; add to `[Unreleased]` or new `[0.10.0-alpha]` section)
    - .planning/phases/14-unowned-library-lifecycle/14-CONTEXT.md (D-API-1 / D-API-2 — the merge rationale)
  </read_first>
  <action>
    1. **Update `.planning/REQUIREMENTS.md` UNOWN block** (lines 41-46). Replace the existing block with:

    ```markdown
    ### Unowned-library lifecycle (UNOWN)

    Two new user-facing flows explicitly manage skills whose source has been
    removed. The verbs were folded into existing commands during Phase 14
    discussion (CONTEXT.md D-API-1 / D-API-2): `tome adopt` is delivered by
    extending `tome reassign` to accept Unowned input; `tome forget` is
    delivered as a new `tome remove skill <name>` subcommand. Behaviour is
    delivered in full; the verbs are different from the original wording.

    - [ ] **UNOWN-01**: ~~`tome adopt <skill> <directory>` re-anchors an
      unowned skill...~~ **(superseded by Phase 14 D-API-1)** Re-anchor an
      unowned skill via `tome reassign <skill> --to <directory>`. Updates
      manifest `source_name` from `None` to `Some(<directory>)` and copies
      skill content into the directory's path. Skill leaves the unowned set.
      `tome reassign foo --to nonexistent` fails fast naming the missing
      directory.
    - [ ] **UNOWN-02**: ~~`tome forget <skill>` explicitly deletes an
      unowned skill...~~ **(superseded by Phase 14 D-API-2)** Delete an
      unowned skill via `tome remove skill <name>`. Confirms via interactive
      prompt unless `--yes`. Removes manifest entry, library directory,
      downstream distribution symlinks, lockfile entry, and machine.toml
      memberships (D-B1). Owned skills refused with hint to use
      `tome remove dir` (D-B2).
    - [ ] **UNOWN-03**: `tome status` and `tome doctor` surface the unowned
      set: count + per-skill list with last-known source. JSON output
      includes `unowned: [SkillSummary]` (status) / `unowned_skills:
      [SkillSummary]` (doctor). Doctor's `total_issues()` is unaffected by
      the unowned set per D-D3.
    ```

    Also update the Traceability table entries for UNOWN-01..03 to mention the merge:

    Locate the rows in the Traceability table (around lines 140-142):

    ```
    | UNOWN-01 | Phase 14 | — | Pending |
    | UNOWN-02 | Phase 14 | — | Pending |
    | UNOWN-03 | Phase 14 | — | Pending |
    ```

    Add a footnote-style note immediately below the table, OR add a Status column note. Simplest is a paragraph below the table:

    ```markdown
    **Phase 14 vocabulary note:** UNOWN-01's "tome adopt" wording is
    superseded by Phase 14 D-API-1 (folded into `tome reassign`); UNOWN-02's
    "tome forget" wording is superseded by D-API-2 (subcommand on `tome
    remove`). Behaviour delivered in full; verbs are different. See
    `.planning/phases/14-unowned-library-lifecycle/14-CONTEXT.md` for
    rationale.
    ```

    2. **Update `.planning/ROADMAP.md` Phase 14 success criteria** (around lines 132-135). Replace the three Success Criteria entries with:

    ```markdown
    **Success Criteria** (what must be TRUE):
      1. `tome reassign <skill> --to <directory>` re-anchors an Unowned skill (per Phase 14 D-API-1, supersedes the literal `tome adopt` wording in UNOWN-01): manifest `source_name` updates from `None` to `Some(<directory>)`, the skill content is copied into the directory's path on disk, `previous_source` is cleared, and the skill leaves the unowned set on next discovery. `tome reassign foo --to nonexistent-dir` fails fast with a clear error naming the missing directory. `tome reassign foo --to <target-only-dir>` is rejected per D-A2. Different-content collisions at the target are refused without `--force` per D-A1.
      2. `tome remove skill <name>` deletes an Unowned skill (per Phase 14 D-API-2, supersedes the literal `tome forget` wording in UNOWN-02): manifest entry removed, library directory removed, downstream distribution symlinks removed, lockfile entry removed, machine.toml memberships removed. Interactive confirmation prompt unless `--yes` is passed. `tome remove skill <name>` on a still-owned skill fails fast per D-B2 with a message directing the user to `tome remove dir` first.
      3. `tome status` and `tome doctor` text output include an `Unowned skills (N):` section listing each unowned skill with its last-known source name (column LAST-KNOWN SOURCE renders `previous_source` per D-C1, falling back to `source_path` per D-C2); JSON output of both commands includes the new field (`unowned: [SkillSummary]` on `StatusReport`, `unowned_skills: [SkillSummary]` on `DoctorReport`). When the unowned set is empty, the section omits cleanly. Per D-D3, the unowned set does NOT contribute to `DoctorReport::total_issues` and does NOT affect `tome doctor` exit code.
    ```

    3. **Update `.planning/PROJECT.md`** for the Key Decisions vocabulary. Search the file for `tome adopt` / `tome forget` mentions. Two anticipated locations per CONTEXT.md `<canonical_refs>`:
       - Line 142 of PROJECT.md (or wherever it actually is — `rg "tome adopt|tome forget" .planning/PROJECT.md`)
       - The Decisions table entry mentioning "`tome adopt`/`forget` for unowned library entries (v0.10 / D-LIB-04)"

       For each match, replace the verb pair with:

       ```
       ~~`tome adopt`/`tome forget`~~ → `tome reassign` (Unowned input, D-API-1) and
       `tome remove skill` (D-API-2) for unowned library entries (v0.10 / D-LIB-04;
       merge per Phase 14 D-API-1/-2)
       ```

       Adapt the surrounding sentence so it still flows. Concretely: change "tome adopt <skill> <dir> and tome forget <skill>" to "tome reassign <skill> --to <dir> (Unowned input) and tome remove skill <skill>".

    4. **Update `CHANGELOG.md`.** Add entries to the `[Unreleased]` section (or create a `## [0.10.0-alpha] - <date>` section if the v0.10 work is being staged that way — check the file's current shape; v0.9.0 follows a date-stamped pattern). Use this content (verbatim, adapted to the file's existing heading style):

    ```markdown
    ### Added
    - `tome reassign <skill> --to <dir>` accepts Unowned skills (re-anchors per UNOWN-01 / D-API-1). Replaces the proposed `tome adopt` command — same mechanical work as Owned→Owned reassign, single verb (#TBD).
    - `tome remove skill <name>` deletes an Unowned skill: manifest entry, library directory, distribution symlinks, lockfile entry, and machine.toml memberships all cleaned (UNOWN-02 / D-API-2 / D-B1). Replaces the proposed `tome forget` command (#TBD).
    - `tome reassign --force` flag bypasses the new D-A1 different-content collision check (refuses to overwrite a target with different content unless explicit).
    - `tome reassign` rejects target-only directory roles (D-A2): a target-only dir cannot receive reassigned skills since nothing rediscovers them on next sync.
    - `tome status` and `tome doctor` show an `Unowned skills (N):` section with NAME / LAST-KNOWN SOURCE / SYNCED columns; JSON output includes a new `unowned` (status) / `unowned_skills` (doctor) array. Per D-D3, the unowned set is informational and does not contribute to `tome doctor` exit code (UNOWN-03).
    - `SkillEntry.previous_source` and `LockEntry.previous_source` schema fields capture the last directory that owned a skill before transition to Unowned (D-C1). Closes the Phase 13 D-13 lossy fork-in-place gap.

    ### Changed
    - **BREAKING:** `tome remove <name>` is now `tome remove dir <name>` (D-API-2). The new `tome remove skill <name>` subcommand handles unowned-skill deletion. Bare `tome remove <name>` no longer parses. Project policy "Backward compat: None" makes this acceptable; users running shell aliases or scripts must update them.
    - The literal stub error in `reassign.rs` pointing at "Phase 14" is deleted; Unowned input is now accepted directly.
    ```

    Replace `#TBD` with the actual GitHub issue numbers if they exist (search for "Phase 14" in `gh issue list`); otherwise leave the placeholder for the user to fill in at PR time, or remove the parenthetical.
  </action>
  <verify>
    <automated>grep -q "superseded by Phase 14 D-API-1" .planning/REQUIREMENTS.md && grep -q "superseded by Phase 14 D-API-2" .planning/REQUIREMENTS.md && grep -q "tome remove dir" CHANGELOG.md && grep -q "tome remove skill" CHANGELOG.md && grep -q "tome reassign --force" CHANGELOG.md</automated>
  </verify>
  <acceptance_criteria>
    - `grep -q "superseded by Phase 14 D-API-1" .planning/REQUIREMENTS.md` succeeds
    - `grep -q "superseded by Phase 14 D-API-2" .planning/REQUIREMENTS.md` succeeds
    - `grep -q "tome reassign <skill> --to <directory>" .planning/ROADMAP.md` succeeds (Phase 14 SC-1 verb update)
    - `grep -q "tome remove skill <name>" .planning/ROADMAP.md` succeeds (Phase 14 SC-2 verb update)
    - `grep -q "BREAKING" CHANGELOG.md` succeeds (the breaking-change callout for `tome remove <name>`)
    - `grep -q "tome remove dir" CHANGELOG.md` succeeds
    - `grep -q "tome remove skill" CHANGELOG.md` succeeds
    - `! grep -q "tome adopt" .planning/PROJECT.md` (the verb has been fully replaced; or if some references remain in historical context, they have a strikethrough/supersession note adjacent — verify by reading the file). Soft check: `grep -c "tome adopt" .planning/PROJECT.md` should be lower after this task than before.
  </acceptance_criteria>
  <done>
    Planning docs (REQUIREMENTS.md, ROADMAP.md, PROJECT.md) and CHANGELOG.md reflect the D-API-1/-2 merge with supersession notes preserving traceability. Breaking change to `tome remove <name>` is called out.
  </done>
</task>

<task type="auto">
  <name>Task 2: Add end-to-end integration tests for UNOWN-01..03 in tests/cli.rs</name>
  <read_first>
    - crates/tome/tests/cli.rs (skim — particularly the existing test scaffolding helpers, fixture builders, and any `tome remove` / `tome reassign` / `tome status` / `tome doctor` tests; use `rg "remove|reassign|status|doctor" crates/tome/tests/cli.rs | head -50`)
    - crates/tome/src/lib.rs (the dispatch arms — to confirm exit codes and stderr/stdout patterns the tests must assert against)
    - .planning/phases/14-unowned-library-lifecycle/14-CONTEXT.md (the test list under "Tests to write" — integration section)
  </read_first>
  <action>
    1. **Identify existing test patterns** in tests/cli.rs. Use `rg "Command::cargo_bin" crates/tome/tests/cli.rs | head -5` to find the assert_cmd invocation style. Use `rg "fn write_skill_in|fn build_test_fixture|fn make_test_setup" crates/tome/tests/cli.rs` to locate fixture builders.

    2. **Add a new test module section** at the bottom of `tests/cli.rs` (or in a dedicated `mod phase14 { ... }` block at the end of the file). Use the existing helpers where possible. The tests should be concrete commands run via `assert_cmd::Command::cargo_bin("tome")`.

    3. **Add these tests** (write them to match whatever scaffolding helpers exist; the bodies are templates):

    ```rust
    // ============================================================
    // Phase 14 — UNOWN-01..03 integration tests
    // (HARD-13 in Phase 15 will split these into per-domain files)
    // ============================================================

    /// UNOWN-01 / D-API-1: `tome reassign <unowned> --to <dir>` succeeds.
    #[test]
    fn phase14_reassign_unowned_input_succeeds() {
        // Build a fixture: tome.toml with one Synced dir "local-skills"; a
        // library containing one Unowned skill "orphan-foo"; manifest entry
        // for orphan-foo with source_name = None.
        // [Use existing fixture helpers; if none, write a minimal inline
        // setup mirroring `make_test_setup` from remove::tests.]
        //
        // Run: tome --tome-home <tmp> reassign orphan-foo --to local-skills
        //
        // Assert: exit 0; manifest source_name = Some("local-skills");
        // <local-skills>/orphan-foo/SKILL.md exists.
        // ...
    }

    /// UNOWN-01 / D-A2: target-only role rejected.
    #[test]
    fn phase14_reassign_into_target_only_role_rejected() {
        // Fixture: tome.toml with one Target-role dir "claude-target"; a
        // library containing skill "foo".
        //
        // Run: tome --tome-home <tmp> reassign foo --to claude-target
        //
        // Assert: exit non-zero; stderr contains "target-only".
        // ...
    }

    /// UNOWN-01 / D-A1: different-content collision refused without --force; succeeds with --force.
    #[test]
    fn phase14_reassign_force_bypasses_different_content_collision() {
        // Fixture: library has skill "foo" with content X; target dir has
        // existing "foo/" with content Y (different).
        //
        // Run without --force: stderr contains "with different content"; exit non-zero.
        // Run with --force: exit 0; target now has content X.
        // ...
    }

    /// UNOWN-02 / D-API-2 / D-B1: tome remove skill happy path.
    #[test]
    fn phase14_remove_skill_full_cleanup() {
        // Fixture: an Unowned skill "orphan-foo" in library; a distribution
        // symlink in some Target dir; a lockfile entry; a machine.toml
        // disabled-set membership.
        //
        // Run: tome --tome-home <tmp> remove skill orphan-foo --yes
        //
        // Assert: exit 0; library/orphan-foo gone; dist symlink gone;
        // lockfile entry gone; machine.toml disabled-set membership gone;
        // manifest entry gone.
        // ...
    }

    /// UNOWN-02 / D-B2: tome remove skill on Owned refused.
    #[test]
    fn phase14_remove_skill_refuses_owned() {
        // Fixture: an Owned skill "kept" with source_name = Some("active-dir").
        //
        // Run: tome --tome-home <tmp> remove skill kept --yes
        //
        // Assert: exit non-zero; stderr contains "is owned by directory";
        // stderr contains "tome remove dir"; manifest entry preserved.
        // ...
    }

    /// UNOWN-02 / D-B3: --yes skips confirmation in non-interactive mode.
    /// (The fact that the previous test --yes succeeds is partial evidence;
    /// add an explicit no-flag-no-input refusal test.)
    #[test]
    fn phase14_remove_skill_no_input_without_yes_bails() {
        // Run: tome --no-input --tome-home <tmp> remove skill orphan-foo
        // (no --yes flag)
        //
        // Assert: exit non-zero; stderr contains "requires confirmation".
        // ...
    }

    /// UNOWN-03: tome status text includes Unowned section.
    #[test]
    fn phase14_status_text_shows_unowned_section() {
        // Fixture: one Unowned skill "orphan" with previous_source = "removed-dir".
        //
        // Run: tome --tome-home <tmp> status
        //
        // Assert: stdout contains "Unowned skills (1)"; contains "orphan";
        // contains "removed-dir".
        // ...
    }

    /// UNOWN-03: tome status --json includes unowned array.
    #[test]
    fn phase14_status_json_includes_unowned_field() {
        // Run: tome --tome-home <tmp> status --json
        //
        // Assert: stdout parses as JSON; "unowned" is an array of length 1
        // with the expected shape.
        // ...
    }

    /// UNOWN-03 / D-D3: tome doctor unowned does not affect total_issues
    /// or exit code.
    #[test]
    fn phase14_doctor_informational_unowned_does_not_affect_exit_code() {
        // Fixture: 2 Unowned skills, no library/directory/config issues.
        //
        // Run: tome --tome-home <tmp> doctor
        //
        // Assert: exit 0 (no actionable issues per D-D3); stdout contains
        // "Unowned skills (2)"; stdout contains "No issues found." (or
        // whichever string the existing doctor renders for issues==0).
        // ...
    }

    /// UNOWN-03: tome status text omits Unowned section cleanly when empty.
    #[test]
    fn phase14_status_text_omits_unowned_section_when_empty() {
        // Fixture: only Owned skills (or no skills at all).
        //
        // Run: tome --tome-home <tmp> status
        //
        // Assert: stdout does NOT contain "Unowned skills" header.
        // ...
    }
    ```

    4. **Implement each test body** using whatever fixture-builder pattern exists in tests/cli.rs. If the existing helpers don't cover the manifest-with-Unowned-entries case, build a minimal inline fixture: write `tome.toml` to disk, write `.tome-manifest.json` directly with a known shape (use `serde_json::to_string_pretty` against `Manifest`), and run the binary.

    Concrete fixture pattern (adapt to existing helpers):

    ```rust
    use assert_cmd::Command;
    use tempfile::TempDir;

    fn write_fixture_with_unowned_skill(tmp: &TempDir, skill_name: &str, previous: Option<&str>) {
        let library = tmp.path().join("library");
        std::fs::create_dir_all(library.join(skill_name)).unwrap();
        std::fs::write(
            library.join(skill_name).join("SKILL.md"),
            "---\nname: x\n---\nbody",
        )
        .unwrap();

        // Write tome.toml with an empty directories table (or one Target dir).
        let toml = format!("library_dir = \"{}\"\n", library.display());
        std::fs::write(tmp.path().join("tome.toml"), toml).unwrap();

        // Write manifest with an Unowned entry.
        let prev = previous
            .map(|s| format!("\"previous_source\": \"{}\",", s))
            .unwrap_or_default();
        let manifest_json = format!(
            r#"{{"skills": {{"{}":{{"source_path":"/tmp/old/{}",{}"content_hash":"{}","synced_at":"2024-01-01T00:00:00Z","managed":false}}}}}}"#,
            skill_name,
            skill_name,
            prev,
            "a".repeat(64),
        );
        std::fs::write(tmp.path().join(".tome-manifest.json"), manifest_json).unwrap();
    }
    ```

    5. **Run the tests** locally: `cargo test -p tome --test cli phase14_`. If any test relies on shaping that the binary doesn't yet emit (e.g. exact error message wording), check 14-04 / 14-05 actions and confirm the verbatim error text matches.

    6. **Final acceptance gate:** `make ci` must pass.
  </action>
  <verify>
    <automated>cargo test -p tome --test cli phase14_</automated>
  </verify>
  <acceptance_criteria>
    - `grep -q "phase14_reassign_unowned_input_succeeds" crates/tome/tests/cli.rs` succeeds
    - `grep -q "phase14_reassign_into_target_only_role_rejected" crates/tome/tests/cli.rs` succeeds
    - `grep -q "phase14_reassign_force_bypasses_different_content_collision" crates/tome/tests/cli.rs` succeeds
    - `grep -q "phase14_remove_skill_full_cleanup" crates/tome/tests/cli.rs` succeeds
    - `grep -q "phase14_remove_skill_refuses_owned" crates/tome/tests/cli.rs` succeeds
    - `grep -q "phase14_status_text_shows_unowned_section" crates/tome/tests/cli.rs` succeeds
    - `grep -q "phase14_status_json_includes_unowned_field" crates/tome/tests/cli.rs` succeeds
    - `grep -q "phase14_doctor_informational_unowned_does_not_affect_exit_code" crates/tome/tests/cli.rs` succeeds
    - `cargo test -p tome --test cli phase14_` exits 0 (all phase14_-prefixed tests pass)
    - `cargo test -p tome` exits 0 (FULL test suite passes — no regression)
    - `cargo clippy --all-targets -p tome -- -D warnings` exits 0
    - `cargo fmt -p tome -- --check` exits 0
    - `make ci` exits 0 (final gate — fmt-check + clippy + tests on both ubuntu-latest and macos-latest equivalent)
  </acceptance_criteria>
  <done>
    8+ end-to-end integration tests covering UNOWN-01..03 via assert_cmd. All tests pass on the final commit. `make ci` green.
  </done>
</task>

</tasks>

<verification>
- All planning-doc edits land: `grep -q "superseded by Phase 14" .planning/REQUIREMENTS.md` succeeds
- CHANGELOG.md has BREAKING callout for `tome remove <name>`
- 8+ phase14_ integration tests pass
- `make ci` exits 0
- No regressions in pre-existing tests
</verification>

<success_criteria>
- Phase 14 traceability is clean: REQUIREMENTS.md / ROADMAP.md / PROJECT.md / CHANGELOG.md all reflect the D-API-1/-2 merge with supersession notes pointing back to CONTEXT.md.
- BREAKING change to `tome remove <name>` (now `tome remove dir <name>`) called out in CHANGELOG.
- 8+ phase14_-prefixed integration tests in tests/cli.rs cover UNOWN-01 (reassign Unowned + D-A1 force + D-A2 role rejection), UNOWN-02 (remove skill happy path + D-B2 owned guard + D-B3 confirmation), UNOWN-03 (status + doctor text + JSON + empty-omits + D-D3 informational).
- `make ci` passes on the final commit.
</success_criteria>

<output>
After completion, create `.planning/phases/14-unowned-library-lifecycle/14-08-SUMMARY.md`
</output>
