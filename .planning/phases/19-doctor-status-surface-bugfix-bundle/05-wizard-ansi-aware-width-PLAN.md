---
phase: 19-doctor-status-surface-bugfix-bundle
plan: 05
type: execute
wave: 2
depends_on: [01]
files_modified:
  - crates/tome/src/wizard.rs
  - Cargo.toml
autonomous: true
requirements: [FIX-04]
requirements_addressed: [FIX-04]

must_haves:
  truths:
    - "Reproduce-first step verifies whether the bug still occurs despite the already-landed `tabled = { features = [\"ansi\"] }` fix in commit 0803afb (April 2026) — RESEARCH risk #1"
    - "If reproducible: strip-ansi-escapes 0.2 added as a regular (non-dev) dep + ANSI escapes stripped before tabled width calculation at wizard.rs:514-521"
    - "If not reproducible: D-FIX04-1 is SKIPPED (no redundant dep added) — #454 closes administratively with reference to commit 0803afb"
    - "Snapshot test (D-FIX04-2) ships regardless of outcome — pinning measure ensuring column alignment doesn't regress if someone removes `features = [\"ansi\"]` from Cargo.toml"
    - "Wizard summary table columns align in styled (ANSI bold) output mode"
  artifacts:
    - path: "Cargo.toml"
      provides: "strip-ansi-escapes = \"0.2\" workspace dep IF the bug reproduces (RESEARCH risk #1 gate)"
      contains: "strip-ansi-escapes"
    - path: "crates/tome/src/wizard.rs"
      provides: "ANSI-escape stripping before tabled width calc at wizard.rs:514-521 (IF reproduces) + a snapshot/golden test asserting column alignment under styled headers (always)"
      contains: "show_directory_summary"
  key_links:
    - from: "crates/tome/src/wizard.rs:514-521"
      to: "strip_ansi_escapes::strip_str"
      via: "Strip ANSI escapes from cell strings BEFORE passing to tabled::Table::from_iter"
      pattern: "strip_ansi_escapes::strip_str"
---

<objective>
Close GitHub #454 (wizard summary table ANSI width misalignment) with awareness that **the fix may already be in place**. RESEARCH risk #1 flagged that commit `0803afb` (April 2026, before v0.7.0) added `tabled = { features = ["ansi"] }` to address this exact bug — yet #454 remains OPEN. The planner-required reproduce-first step determines whether D-FIX04-1 (adding `strip-ansi-escapes` dep + strip-before-tabled call) is still needed or whether the issue should close administratively with a pinning snapshot test only.

Purpose: Either (a) ship the actual fix if the bug reproduces, or (b) avoid adding a redundant dependency and close #454 administratively. Snapshot test (D-FIX04-2) ships in either case as a regression guard.
Output: Reproduce-first gate decision + conditional Cargo.toml dep + conditional wizard.rs strip-call + a snapshot test that pins column alignment under styled (ANSI bold) headers.
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
@Cargo.toml

<interfaces>
<!-- Existing wizard code path that may or may not need modification (RESEARCH risk #1). -->

**`crates/tome/src/wizard.rs:499-539`** — `show_directory_summary` function. Builds a `tabled::Table` for the wizard's directory-summary display. Lines 514-521 are the row-construction loop where `console::style(...).bold()` may be applied to header cells.

**`Cargo.toml:32`** (RESEARCH-verified): `tabled = { version = "0.20", features = ["ansi"] }`. The `ansi` feature enables `ansi-str`/`ansitok` which provides ANSI-aware width calculation INSIDE tabled. Per RESEARCH ("tabled crate on docs.rs"): the feature exists specifically to handle the case D-FIX04-1 is meant to solve.

**Commit `0803afb`** (Thu Apr 23 13:36:28 2026): `fix(wizard): align tabled summary header with body in interactive TTY` — added the `ansi` feature. Yet GitHub #454 is `OPEN` per `gh issue view 454`.

**`strip-ansi-escapes` 0.2.1 API** (verified — RESEARCH secondary sources):
```rust
strip_ansi_escapes::strip_str(data: &str) -> String  // use this
strip_ansi_escapes::strip(data: &[u8]) -> Vec<u8>    // byte form
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Reproduce-first — verify whether bug #454 still occurs despite already-landed `tabled[ansi]` fix</name>
  <files>(no files modified — investigation only)</files>
  <read_first>
    - crates/tome/src/wizard.rs (full file — focus on `show_directory_summary` at :499-539, especially the row-construction loop at :514-521)
    - Cargo.toml (full — confirm `tabled = { version = "0.20", features = ["ansi"] }` is present at line ~32)
    - .planning/phases/19-doctor-status-surface-bugfix-bundle/19-CONTEXT.md (D-FIX04-1, D-FIX04-2)
    - .planning/phases/19-doctor-status-surface-bugfix-bundle/19-RESEARCH.md "FIX-04 (wizard summary ANSI width — closes #454)" section (lines 565-609) — especially the ANOMALY flag and the recommendation
  </read_first>
  <action>
    **Step 1 — Confirm pre-existing fix is still present:**
    Run `rg 'features = \["ansi"\]' Cargo.toml`. Confirm the `tabled` line has `features = ["ansi"]`. If this returns 0 matches, something regressed; surface immediately and do NOT proceed.

    **Step 2 — Reproduce the bug:**
    Run `tome init` in a fresh greenfield TempDir (no existing config) via `script(1)` to force-TTY mode (so `console::style().bold()` actually emits ANSI escapes — non-TTY mode strips them automatically):

    Two reproduction paths to try (pick whichever works on the executor's machine):

    **Path A — `script` (BSD/macOS or `script -q` on Linux):**
    ```bash
    set TMPDIR (mktemp -d)
    set TOME_HOME $TMPDIR/.tome
    script -q /dev/null env TOME_HOME=$TOME_HOME FORCE_COLOR=1 cargo run -p tome -- init 2>&1 | head -100
    ```

    **Path B — Direct invocation with `CLICOLOR_FORCE=1`** (some `console` versions honor this):
    ```bash
    set TMPDIR (mktemp -d)
    set TOME_HOME $TMPDIR/.tome
    CLICOLOR_FORCE=1 cargo run -p tome -- init 2>&1 | head -100
    ```

    **Step 3 — Inspect the output for column misalignment:**
    Look for the summary table (typically rendered as a `tabled::Table` with header + body rows). Check whether the `│` (vertical bar) characters in the header row appear at the SAME column positions as in body rows. If they DO align: the bug does NOT reproduce. If they DO NOT align: the bug reproduces.

    **Step 4 — Decision:**

    If bug DOES reproduce → proceed to Task 2A (apply D-FIX04-1 strip-ansi-escapes fix).
    If bug DOES NOT reproduce → proceed to Task 2B (skip D-FIX04-1; ship snapshot test only; flag #454 for administrative close).

    **Document the outcome** in a comment block inside this task's verification (so downstream verification knows which path was taken) AND in the eventual `19-05-SUMMARY.md`:
    - Reproduction method used (Path A or B)
    - Whether the bug reproduced (yes/no)
    - Terminal type (`tty -s; echo $TERM`)
    - Sample table output (a few lines showing alignment or misalignment)

    **Cleanup:** Remove the TempDir from Step 2 (`rm -rf $TMPDIR`).
  </action>
  <verify>
    <automated>rg 'features = \["ansi"\]' Cargo.toml</automated>
  </verify>
  <acceptance_criteria>
    - `rg 'features = \["ansi"\]' Cargo.toml` returns at least 1 match (the already-landed fix is still present)
    - Executor has documented the reproduction outcome in this plan's running notes / staging area for `19-05-SUMMARY.md` (yes/no + sample output)
    - No files modified in this task (investigation only)
  </acceptance_criteria>
  <done>Reproduction outcome documented; executor knows whether to take Task 2A or Task 2B path.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Apply fix per Task 1 outcome — either D-FIX04-1 (strip-ansi-escapes) OR administrative close path. Snapshot test (D-FIX04-2) ships in either case.</name>
  <files>crates/tome/src/wizard.rs, Cargo.toml</files>
  <read_first>
    - crates/tome/src/wizard.rs lines 499-540 (show_directory_summary — exact row-construction loop)
    - Cargo.toml (workspace deps section — for adding strip-ansi-escapes if applicable)
    - .planning/phases/19-doctor-status-surface-bugfix-bundle/19-RESEARCH.md "FIX-04" "Regression test (D-FIX04-2)" subsection (lines 590-608) — exact snapshot test shape
    - Task 1's notes documenting the reproduction outcome
  </read_first>
  <behavior>
    - Test (D-FIX04-2 snapshot): Render `show_directory_summary` (or extract its table-construction helper) with ANSI-bold header cells; assert column-divider (`│`) alignment between header row and body rows.
    - The snapshot test passes regardless of whether `strip-ansi-escapes` is added — because tabled's `ansi` feature already provides ANSI-aware width.
    - If `strip-ansi-escapes` is added (Path 2A): an additional unit test asserts `strip_str` is called on cells before tabled processes them.
  </behavior>
  <action>
    **Path 2A — IF Task 1 confirmed the bug reproduces:**

    1. **Add `strip-ansi-escapes` to Cargo.toml** as a regular workspace dep (NOT dev-dep — runtime path):
       In `Cargo.toml` `[workspace.dependencies]` section, add:
       ```toml
       strip-ansi-escapes = "0.2"
       ```
       In `crates/tome/Cargo.toml` `[dependencies]` section, add:
       ```toml
       strip-ansi-escapes = { workspace = true }
       ```
       (Verify the workspace pattern matches existing deps — e.g. `tabled = { workspace = true }` is the template.)

    2. **Modify `show_directory_summary` in `crates/tome/src/wizard.rs`** at the row-construction site (RESEARCH-verified `:514-521`, executor confirms by content):
       Before each cell string is passed to `tabled::Table::from_iter` (or equivalent table-builder API), strip ANSI escapes:
       ```rust
       use strip_ansi_escapes::strip_str;
       // ...
       // Before:
       let row = [name_styled.to_string(), type_str.clone(), role_str.clone(), path_styled.to_string()];
       // After:
       let row = [
           strip_str(&name_styled.to_string()),
           strip_str(&type_str),
           strip_str(&role_str),
           strip_str(&path_styled.to_string()),
       ];
       ```
       IMPORTANT: this strips ANSI from the cells FED TO TABLED for width calc — the visible output to the terminal must still include the styled (ANSI-decorated) versions if styling is desired. If tabled's API requires a single string-per-cell, the stripped version is what tabled uses for width AND for display — which means the bold styling is lost. Two possible approaches:

       (a) **Use stripped strings for display** — simplest, sacrifices the bold styling. Acceptable per RESEARCH because the goal is alignment, and the user already has bold rendering from other display elements.
       (b) **Use tabled's `ansi` feature** — but that's already enabled. If the bug still reproduces with `ansi` enabled, tabled's ANSI-aware path may have an edge case. In that case, applying `strip_str` to the displayed text loses styling but fixes alignment.

       Pick (a) — drop the bold styling from the wizard summary table cells. The header row's bold styling can stay (single row, no alignment issue at the row level).

    3. **Document the choice in a comment** above the `strip_str` calls:
       ```rust
       // FIX-04 (#454): strip ANSI escapes from cells before tabled width calc.
       // tabled[ansi] feature was added in commit 0803afb but #454 persisted
       // in some terminal configurations; explicit strip is a belt-and-braces fix.
       ```

    **Path 2B — IF Task 1 confirmed the bug does NOT reproduce:**

    1. **Skip the Cargo.toml change** — do NOT add `strip-ansi-escapes` (would be a redundant dep per RESEARCH guidance: "If the bug does NOT reproduce, the issue is administrative... Do NOT add `strip-ansi-escapes` unnecessarily — it's redundant with `tabled[ansi]`.").

    2. **Skip the wizard.rs strip_str call** — leave `show_directory_summary` as-is.

    3. **Add a comment in `wizard.rs` above `show_directory_summary`** documenting the resolution:
       ```rust
       // FIX-04 (#454) reference: this function uses tabled's `ansi` feature
       // (Cargo.toml line ~32) for ANSI-aware width calculation. Commit
       // 0803afb established this in April 2026. Phase 19 verified during
       // reproduction that the bug no longer manifests; snapshot test
       // `show_directory_summary_aligns_header_with_body_under_ansi` (below)
       // pins the alignment behavior as a regression guard.
       ```

    **BOTH PATHS — Add the D-FIX04-2 snapshot test** in `crates/tome/src/wizard.rs` `#[cfg(test)] mod tests` block:

    ```rust
    #[test]
    fn show_directory_summary_aligns_header_with_body_under_ansi() {
        use console::set_colors_enabled;
        use std::collections::BTreeMap;

        // Force-enable ANSI colors so styled headers actually emit escape codes
        set_colors_enabled(true);

        // Build a minimal directories map with realistic content widths
        let mut dirs: BTreeMap<DirectoryName, DirectoryConfig> = BTreeMap::new();
        dirs.insert(
            DirectoryName::new("claude-skills".to_string()).unwrap(),
            DirectoryConfig {
                kind: DirectoryType::Directory,
                role: DirectoryRole::DiscoveryAndDistribution,
                path: PathBuf::from("~/.claude/skills"),
                // ... fill remaining required fields with sensible defaults
            },
        );
        dirs.insert(
            DirectoryName::new("codex-skills".to_string()).unwrap(),
            DirectoryConfig {
                kind: DirectoryType::Directory,
                role: DirectoryRole::Discovery,
                path: PathBuf::from("~/.codex/skills"),
                // ...
            },
        );

        // Capture the rendered table by invoking the table-construction helper
        // that show_directory_summary uses (extract into a testable helper
        // if not already separated — return the rendered String).
        let rendered = render_directory_summary_table(&dirs);

        // Assert alignment: every `│` divider in the header row appears at
        // the same column index as in body rows.
        let lines: Vec<&str> = rendered.lines().collect();
        let header_pipes: Vec<usize> = lines[0].match_indices('│').map(|(i, _)| i).collect();
        for body_line in &lines[1..] {
            let body_pipes: Vec<usize> = body_line.match_indices('│').map(|(i, _)| i).collect();
            assert_eq!(
                header_pipes, body_pipes,
                "column dividers must align between header and body. Header={header_pipes:?} Body={body_pipes:?}\nRendered:\n{rendered}"
            );
        }
    }
    ```

    NOTE: the test depends on `render_directory_summary_table` being a callable helper that returns a String. If `show_directory_summary` only prints to stdout, EXTRACT the table-rendering logic into a `pub(crate) fn render_directory_summary_table(dirs: &BTreeMap<DirectoryName, DirectoryConfig>) -> String` helper, and have `show_directory_summary` call `println!("{}", render_directory_summary_table(dirs))`. This is a minor refactor but enables testability.

    If the executor finds `show_directory_summary` already returns a `String` (or has an internal helper), use that directly.

    **Bonus**: this snapshot test also catches a regression if someone removes `features = ["ansi"]` from `Cargo.toml` — because removing it would re-introduce the misalignment.
  </action>
  <verify>
    <automated>cargo test -p tome --lib wizard::tests::show_directory_summary_aligns_header_with_body_under_ansi</automated>
  </verify>
  <acceptance_criteria>
    - One of these patterns must hold (per Task 1 outcome):
      (Path 2A) `rg "strip-ansi-escapes" Cargo.toml` returns at least 1 match AND `rg "strip_ansi_escapes::strip_str" crates/tome/src/wizard.rs` returns at least 1 match
      (Path 2B) `rg "strip-ansi-escapes" Cargo.toml` returns 0 matches AND `rg "FIX-04 \(#454\) reference" crates/tome/src/wizard.rs` returns 1 match
    - `rg "show_directory_summary_aligns_header_with_body_under_ansi" crates/tome/src/wizard.rs` returns 1 match (the snapshot test ships in EITHER path)
    - `cargo test -p tome --lib wizard::tests::show_directory_summary_aligns_header_with_body_under_ansi` exits 0
    - `cargo clippy --all-targets -- -D warnings` exits 0
    - If Path 2B: GitHub #454 needs an administrative close — record this as a follow-up action in the summary (the close itself happens in Plan 07 alongside CHANGELOG updates)
  </acceptance_criteria>
  <done>Fix path chosen per Task 1 outcome; snapshot test ships regardless; column alignment is pinned as a regression guard; #454 either fixed or queued for administrative close.</done>
</task>

</tasks>

<verification>
- `cargo test -p tome --lib wizard::tests::show_directory_summary_aligns_header_with_body_under_ansi` — passes
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo fmt -- --check` — clean
- Manual smoke (Path 2A only): re-run the Task 1 reproduction; verify columns now align
- Manual smoke (Path 2B): re-run the Task 1 reproduction; verify columns ALREADY align (no change needed beyond the snapshot test)
</verification>

<success_criteria>
- FIX-04: Wizard summary table columns align under ANSI-bold styled headers; snapshot test pins the alignment as a regression guard
- RESEARCH risk #1 honored: reproduce-first step taken; redundant `strip-ansi-escapes` dep avoided if `tabled[ansi]` is sufficient
- If administrative close path: #454 close action is queued for Plan 07's CHANGELOG update step
</success_criteria>

<output>
After completion, create `.planning/phases/19-doctor-status-surface-bugfix-bundle/19-05-SUMMARY.md` documenting:
- Task 1 reproduction outcome (yes/no + terminal type + sample output)
- Which path (2A or 2B) was taken
- If 2A: the exact strip_str insertion site + whether bold styling was sacrificed for alignment
- If 2B: confirmation that #454 should close administratively with reference to commit 0803afb
- Snapshot test pin shape (the assertion mechanism used to compare column positions)
</output>
