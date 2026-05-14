---
phase: 19-doctor-status-surface-bugfix-bundle
plan: 07
type: execute
wave: 3
depends_on: [01, 02, 03, 04, 05, 06]
files_modified:
  - CHANGELOG.md
  - .planning/REQUIREMENTS.md
autonomous: false
requirements: [OBS-06, OBS-07, FIX-01, FIX-02, FIX-03, FIX-04, FIX-05, FIX-06]
requirements_addressed: [OBS-06, OBS-07, FIX-01, FIX-02, FIX-03, FIX-04, FIX-05, FIX-06]

must_haves:
  truths:
    - "CHANGELOG.md `[Unreleased]` section has Phase 19 entries: Added (OBS-06 doctor categories, OBS-07 last-sync + skill-count), Fixed (FIX-01..06 with closing issue refs)"
    - "REQUIREMENTS.md Traceability table marks OBS-06..07 + FIX-01..06 as Done"
    - "make ci passes (fmt-check + clippy -D warnings + tests) on the local machine"
    - "Total test count is ≥1000 (was 994 pre-phase per RESEARCH; projected 1007-1011 post-phase)"
    - "Human verification checkpoint confirms the v0.11 surface meets ROADMAP success criteria 1-4"
  artifacts:
    - path: "CHANGELOG.md"
      provides: "Updated [Unreleased] block with Added + Fixed subsections covering all 8 Phase 19 requirements"
      contains: "OBS-06"
    - path: ".planning/REQUIREMENTS.md"
      provides: "Traceability table updated — OBS-06..07 + FIX-01..06 status: Pending → Done"
      contains: "Done"
  key_links:
    - from: "CHANGELOG.md [Unreleased] block"
      to: "Phase 19 deliverables"
      via: "Added (OBS-06, OBS-07) + Fixed (FIX-01..06) subsections with GitHub issue cross-refs"
      pattern: "OBS-06|OBS-07|FIX-0[1-6]"
    - from: ".planning/REQUIREMENTS.md Traceability table"
      to: "Phase 19 closure"
      via: "Status column flip from Pending to Done for 8 requirements"
      pattern: "Done"
---

<objective>
Final wave of Phase 19. After Plans 01-06 land their substantive changes, this plan: (a) updates CHANGELOG.md `[Unreleased]` section with Phase 19 entries, (b) flips REQUIREMENTS.md Traceability table OBS-06..07 + FIX-01..06 from Pending to Done, (c) runs `make ci` to verify the full quality gate, (d) verifies test count crossed 1000, and (e) presents a human-verification checkpoint mapping back to ROADMAP success criteria 1-4.

Purpose: Close Phase 19 cleanly. The CHANGELOG update is the user-visible signal of completed work; the REQUIREMENTS update is the traceability anchor for v0.11 ship; the CI check is the quality gate; the human checkpoint is the goal-backward verification.
Output: Updated CHANGELOG.md + REQUIREMENTS.md, green `make ci`, test count >= 1000, human-confirmed match against ROADMAP success criteria.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/STATE.md
@.planning/phases/19-doctor-status-surface-bugfix-bundle/19-CONTEXT.md
@.planning/phases/19-doctor-status-surface-bugfix-bundle/19-RESEARCH.md
@CHANGELOG.md
@.planning/phases/19-doctor-status-surface-bugfix-bundle/19-01-SUMMARY.md
@.planning/phases/19-doctor-status-surface-bugfix-bundle/19-02-SUMMARY.md
@.planning/phases/19-doctor-status-surface-bugfix-bundle/19-03-SUMMARY.md
@.planning/phases/19-doctor-status-surface-bugfix-bundle/19-04-SUMMARY.md
@.planning/phases/19-doctor-status-surface-bugfix-bundle/19-05-SUMMARY.md
@.planning/phases/19-doctor-status-surface-bugfix-bundle/19-06-SUMMARY.md

<interfaces>
Existing CHANGELOG.md `[Unreleased]` shape (Phase 18 already added entries
for OBS-01..05). The Phase 19 work APPENDS to the existing subsections,
it does NOT create a new `[Unreleased]` block.

Existing structure (Phase 18):
```
## [Unreleased]

### Added
- OBS-01..05 (Phase 18) ...

### Fixed
- ...
```

Phase 19 additions:
- Added subsection: append OBS-06 (doctor categorization) + OBS-07 (status last-sync + skill-count)
- Fixed subsection: append FIX-01 (#530), FIX-02 (#511 + HARD-14), FIX-03 (#532), FIX-04 (#454), FIX-05 (#453 + #456), FIX-06 (#533)

`.planning/REQUIREMENTS.md` Traceability table (lines 59-73 per RESEARCH):
Each Phase 19 row currently has `| Pending |` in the Status column; flip to `| Done |`.
The checkbox list at the top of the file (lines 14-32) flips `- [ ]` to `- [x]` for each Phase 19 requirement (matches Phase 18's already-done pattern for OBS-01..05).
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Update CHANGELOG.md `[Unreleased]` block with Phase 19 entries</name>
  <files>CHANGELOG.md</files>
  <read_first>
    - CHANGELOG.md (full file — confirm current `[Unreleased]` structure from Phase 18)
    - .planning/phases/19-doctor-status-surface-bugfix-bundle/19-01-SUMMARY.md through 19-06-SUMMARY.md (the actual delivered scope of each plan — Phase 19 entries reference real ships, not aspirational plans)
    - .planning/phases/19-doctor-status-surface-bugfix-bundle/19-CONTEXT.md (per CONTEXT specifics: "CHANGELOG `[Unreleased]` rule of thumb — Phase 18 wrote v0.11 work under `[Unreleased]`. After Phase 19 lands, the v0.11 release cut renames it to `[0.11.0] - <date>`")
  </read_first>
  <action>
    Append Phase 19 entries to the existing `[Unreleased]` block in `CHANGELOG.md`. Do NOT create a new `[Unreleased]` block — Phase 18 already created one.

    Under `### Added` (append after existing OBS-01..05 entries — preserve Phase 18 entries verbatim):

    ```markdown
    - **OBS-06** — `tome doctor` issue categorization. Each `DiagnosticIssue` carries an `IssueCategory` (Library / Directory / Config / Foreign-symlink) derived at construction from the DoctorReport field + DiagnosticIssueKind (ForeignSymlink promotes). Text summary line includes per-category breakdown (e.g. `(3 auto-fixable: Library 2, Foreign-symlink 1)`). JSON shape gains a `category` field per issue + `summary.by_category` + `summary.auto_fixable_by_category` maps. Built on the POLISH-04 enum-exhaustiveness pattern.
    - **OBS-07** — `tome status` richer surface. Top-line `Last sync: <RFC-3339 timestamp>` (or `never` when manifest doesn't exist). Per-directory SKILLS column in the Directories table (existing `(override)` annotation from PORT-05 preserved). JSON shape gains top-level `last_sync: Option<String>` field. Manifest schema additive lift — pre-v0.11 manifests deserialize cleanly with `last_synced_at: None`. Stamp fires after distribute + cleanup succeed (D-LSYNC-3), before lockfile.save.
    ```

    Under `### Fixed` (append after existing Phase 18 entries — preserve Phase 18 entries verbatim):

    ```markdown
    - **FIX-01 / #530** — `tome doctor` no longer prints "N auto-fixable issues" followed by "(no auto-repair available)". The dispatcher now uses typed `RepairKind` discrimination (`RemoveStaleManifestEntry`, `RemoveBrokenLibrarySymlink`, `RemoveStaleTargetSymlink`) instead of message substring matching. When `auto_fixable_count == 0`, the global "Apply N auto-fixable repairs? [Y/n]" prompt is skipped entirely. Adding a `RepairKind` variant without a dispatcher handler now fails to compile (POLISH-04 sentinel + exhaustive match).
    - **FIX-02 / #511 + HARD-14** — Browse copy-path timing flake. Upper bound on `copy_path_retry_helper_returns_within_bound` relaxed from 600ms to 2000ms, with a multi-line `FLAKE-FIX` comment naming `arboard` clipboard contention under `--test-threads=N` as the root cause and the rejected clock-injection alternative. `backup::tests::push_and_pull_roundtrip` addressed per its actual root-cause class (see Phase 19-04 SUMMARY for the outcome chosen).
    - **FIX-03 / #532** — `tome doctor` no longer reports `N managed symlink(s) tracked in git` warnings on v0.10-shape libraries. The stale check was deleted wholesale (v0.10 made managed skills real directory copies, so the original concern no longer applies): the `check_library` emit block, the `tracked_managed_symlinks` helper, and the interactive git-tracked render path were all removed. Regression test pins the clean-library behavior.
    - **FIX-04 / #454** — Wizard summary table column alignment under ANSI-bold styled headers. Phase 19 added a snapshot test pinning alignment; see Phase 19-05 SUMMARY for whether the underlying fix required `strip-ansi-escapes` or the existing `tabled[ansi]` feature (commit `0803afb`, April 2026) was sufficient on reproduction.
    - **FIX-05 / #453 + #456** — Wizard library default follows `tome_home`. Implementation at `wizard.rs:637` was already correct — Phase 19 adds the missing pinning integration tests (positive: `<TOME_HOME>/skills`; negative: no fallback to `~/.tome/skills`).
    - **FIX-06 / #533** — `make release VERSION=X.Y.Z` now automatically replaces `## [Unreleased]` with `## [X.Y.Z] - YYYY-MM-DD` in `CHANGELOG.md` during the version-bump commit. Idempotent (silent no-op if no `[Unreleased]` section present). Inline `sed -i ''` in the Makefile recipe, style-matched with the existing Cargo.toml version-bump sed line.
    ```

    DO NOT rename `[Unreleased]` to `[0.11.0] - <date>` — that step happens at the v0.11 release cut, which is sequenced AFTER Phase 19 (per CONTEXT.md specifics). FIX-06's automation will do the rename automatically when `make release VERSION=0.11.0` runs.

    Per Keep-a-Changelog convention (verified at CHANGELOG.md:106 `## [0.10.0] - 2026-05-11`), subsections are in the order: Added, Changed, Deprecated, Removed, Fixed, Security. Phase 19's Added entries go in the Added subsection (preserving Phase 18 entries above them); Fixed entries go in the Fixed subsection.
  </action>
  <verify>
    <automated>rg "OBS-06" CHANGELOG.md && rg "OBS-07" CHANGELOG.md && rg "FIX-0[1-6]" CHANGELOG.md && rg "#530" CHANGELOG.md && rg "#511" CHANGELOG.md && rg "#532" CHANGELOG.md && rg "#454" CHANGELOG.md && rg "#453" CHANGELOG.md && rg "#456" CHANGELOG.md && rg "#533" CHANGELOG.md</automated>
  </verify>
  <acceptance_criteria>
    - `rg "OBS-06" CHANGELOG.md` returns at least 1 match
    - `rg "OBS-07" CHANGELOG.md` returns at least 1 match
    - `rg "FIX-01" CHANGELOG.md` returns at least 1 match (same for FIX-02 through FIX-06)
    - `rg "#530|#511|#532|#454|#453|#456|#533" CHANGELOG.md` returns at least 7 matches (one for each closed issue)
    - `rg "^## \[Unreleased\]" CHANGELOG.md` returns 1 match (the [Unreleased] block is NOT renamed to [0.11.0])
    - The Added subsection comes BEFORE the Fixed subsection (Keep-a-Changelog convention)
    - Phase 18 entries are PRESERVED verbatim above Phase 19 entries (manual visual check; or grep for existing OBS-01..05 entries)
  </acceptance_criteria>
  <done>CHANGELOG.md [Unreleased] block has Phase 19 entries under Added (OBS-06, OBS-07) and Fixed (FIX-01..06) with closing issue refs; Phase 18 entries preserved; [Unreleased] header NOT renamed.</done>
</task>

<task type="auto">
  <name>Task 2: Flip REQUIREMENTS.md Traceability table OBS-06..07 + FIX-01..06 from Pending to Done</name>
  <files>.planning/REQUIREMENTS.md</files>
  <read_first>
    - .planning/REQUIREMENTS.md (full file — focus on the Traceability table at lines 59-73 and the checkbox list at lines 14-32)
  </read_first>
  <action>
    In `.planning/REQUIREMENTS.md`, locate the Traceability table (lines 59-73). For each Phase 19 row, change the Status column from `Pending` to `Done`:

    Rows to update:
    - `OBS-06` Phase 19 row: Pending -> Done
    - `OBS-07` Phase 19 row: Pending -> Done
    - `FIX-01` Phase 19 row (refs #530): Pending -> Done
    - `FIX-02` Phase 19 row (refs #511): Pending -> Done
    - `FIX-03` Phase 19 row (refs #532): Pending -> Done
    - `FIX-04` Phase 19 row (refs #454): Pending -> Done
    - `FIX-05` Phase 19 row (refs #453 + #456): Pending -> Done
    - `FIX-06` Phase 19 row (refs #533): Pending -> Done

    Preserve the existing URL formatting in the GitHub Issue column (per CHANGELOG conventions — the `[#NNN](https://...)` markdown link form). The ONLY change per row is `Pending` -> `Done`.

    Also update the checkbox list at the top of the file (lines 14-32) — change `- [ ] **OBS-06**:` to `- [x] **OBS-06**:` etc. for all 8 Phase 19 requirements. This matches the OBS-01..05 pattern already in place from Phase 18 (verified at REQUIREMENTS.md lines 15-19).
  </action>
  <verify>
    <automated>rg "OBS-06 .* Phase 19 .* Done" .planning/REQUIREMENTS.md && rg "FIX-06 .* Phase 19 .* Done" .planning/REQUIREMENTS.md && rg "- \[x\] \*\*OBS-06\*\*" .planning/REQUIREMENTS.md && rg "- \[x\] \*\*FIX-06\*\*" .planning/REQUIREMENTS.md</automated>
  </verify>
  <acceptance_criteria>
    - `rg "OBS-06 \\| Phase 19 \\| .* \\| Done" .planning/REQUIREMENTS.md` returns 1 match (Traceability table row updated)
    - `rg "FIX-06 \\| Phase 19 \\| .* \\| Done" .planning/REQUIREMENTS.md` returns 1 match
    - `rg "- \\[x\\] \\*\\*OBS-06\\*\\*" .planning/REQUIREMENTS.md` returns 1 match (checkbox flipped)
    - `rg "- \\[x\\] \\*\\*OBS-07\\*\\*" .planning/REQUIREMENTS.md` returns 1 match
    - `rg "- \\[x\\] \\*\\*FIX-01\\*\\*" .planning/REQUIREMENTS.md` returns 1 match
    - `rg "- \\[x\\] \\*\\*FIX-06\\*\\*" .planning/REQUIREMENTS.md` returns 1 match
    - `rg "Phase 19 \\| .* \\| Pending" .planning/REQUIREMENTS.md` returns 0 matches (no Phase 19 row left as Pending)
  </acceptance_criteria>
  <done>All 8 Phase 19 requirements are marked Done in the Traceability table AND the checkbox list at the top.</done>
</task>

<task type="auto">
  <name>Task 3: Run `make ci` quality gate + verify test count >= 1000</name>
  <files>(no files modified — verification only)</files>
  <read_first>
    - Makefile (confirm `make ci` target shape: fmt-check + lint + test)
    - .planning/phases/19-doctor-status-surface-bugfix-bundle/19-RESEARCH.md "Test-Count Growth Audit" section (lines 805-836) — projected 1007-1011 post-phase
  </read_first>
  <action>
    1. Run the full CI pipeline locally:
       ```bash
       make ci
       ```
       This runs `fmt-check + lint + test` per the Makefile. Must exit 0.

    2. Count the tests:
       ```bash
       rg -c "^\s*#\[test\]" --type=rust crates/tome/src crates/tome/tests | awk -F: '{sum+=$2} END {print sum}'
       ```
       Must report a number >= 1000.

    3. If `make ci` fails:
       - For fmt failures: run `make fmt` and re-run.
       - For clippy failures: address each warning. Do NOT silence with `#[allow(...)]` without justification.
       - For test failures: address the specific test failure. If the failure is in one of the FIX-02-touched tests, document the failure mode in `19-04-SUMMARY.md`.

    4. If test count is below 1000:
       - List the tests added per RESEARCH.md projection (lines 813-831). Cross-reference with what was actually shipped per plans 01-06.
       - If genuinely below 1000 but goals 1-3 are met, document the shortfall in `19-07-SUMMARY.md` and surface as a phase-completion note. Do NOT add scope-creep tests just to hit the number — per RESEARCH ("opportunistic only, not scope-creep").
  </action>
  <verify>
    <automated>make ci && test (rg -c "^\\s*#\\[test\\]" --type=rust crates/tome/src crates/tome/tests | awk -F: '{sum+=$2} END {print sum}') -ge 1000</automated>
  </verify>
  <acceptance_criteria>
    - `make ci` exits 0 (fmt-check clean, clippy `-D warnings` clean, all tests pass)
    - `rg -c "^\\s*#\\[test\\]" --type=rust crates/tome/src crates/tome/tests | awk -F: '{sum+=$2} END {print sum}'` outputs a number >= 1000
    - No `#[ignore]` annotations were added during Phase 19 work (grep for new ignores via `git diff --stat | rg ignore`)
  </acceptance_criteria>
  <done>make ci is green; test count >= 1000; no test scope-creep.</done>
</task>

<task type="checkpoint:human-verify" gate="blocking">
  <name>Task 4: Human-verify ROADMAP success criteria 1-4 (goal-backward checkpoint)</name>
  <files>(no files modified — human verification only)</files>
  <action>
    Present the v0.11 surface to the user and walk through ROADMAP success criteria 1-4. The user inspects the output of each `tome` invocation listed in `<how-to-verify>` and confirms each criterion holds. The executor agent presents the verification steps below; the user executes them manually (or the agent runs each command and presents output for human approval); the agent waits for the resume signal before continuing.

    Phase 19 substantive changes shipped via Plans 01-06:
    - Plan 01: OBS-06 + FIX-01 + FIX-03 (doctor.rs categorization + RepairKind + tracked-in-git delete)
    - Plan 02: FIX-06 (Makefile sed line for CHANGELOG date-stamp)
    - Plan 03: OBS-07 (last_sync + skill_count column)
    - Plan 04: FIX-02 (browse + backup flakes addressed)
    - Plan 05: FIX-04 (wizard ANSI width — fix or pinning test per reproduction outcome)
    - Plan 06: FIX-05 (wizard library-default pinning tests)
    - Plan 07 Tasks 1-3: CHANGELOG + REQUIREMENTS updated; `make ci` green; test count >= 1000
  </action>
  <verify>
    <automated>make ci && cargo test -p tome --test cli_doctor --test cli_status --test cli_init --test cli_make_release</automated>
  </verify>
  <done>User has explicitly typed "approved" indicating all four ROADMAP success criteria pass; any issues are documented in 19-07-SUMMARY.md.</done>
  <how-to-verify>
    Walk through ROADMAP success criteria 1-4 manually. The user inspects the v0.11 surface and confirms each criterion holds.

    **Success Criterion 1 — `tome doctor` text + JSON shape with categorization:**
    1. Run `cargo run -p tome -- doctor` against a real repo with at least one issue. Verify the summary line shows per-category counts.
    2. Run `cargo run -p tome -- doctor --json | jq '.library_issues[0].category, .summary'`. Verify `category` is a snake_case string per issue AND `summary` has `by_category` + `auto_fixable_by_category` maps.
    3. Run `cargo run -p tome -- doctor` on a repo where `auto_fixable_count == 0`. Verify the global "Apply N auto-fixable repairs?" prompt is NOT shown AND the literal text "(no auto-repair available)" does NOT appear.

    **Success Criterion 2 — `tome status` text + JSON shape:**
    1. Run `cargo run -p tome -- status` on a fresh TempDir (no manifest). Verify text contains `Last sync: never`.
    2. Run `cargo run -p tome -- sync && cargo run -p tome -- status`. Verify text contains `Last sync: <RFC-3339 timestamp>`.
    3. Run `cargo run -p tome -- status` against a configured repo. Verify the Directories table has a SKILLS column with `✓ N` cells; the existing `(override)` annotation is preserved on overridden rows.
    4. Run `cargo run -p tome -- status --json | jq '.last_sync, .directories[].skill_count'`. Verify `last_sync` is `null` or a string; `skill_count` is present per directory.
    5. Verify the Unowned skills section still appears when applicable (UNOWN-03 preserved).

    **Success Criterion 3 — Five bugfixes land cleanly with one regression test each:**
    1. #511: `cargo test -p tome --lib browse::app::tests::copy_path_retry_helper_returns_within_bound` — passes locally; bound is 2000ms; comment names #511 + HARD-14 + arboard.
    2. #532: `cargo test -p tome --test cli_doctor doctor_clean_v010_library_emits_no_tracked_in_git_warning` — passes; grep `tracked in git` in `doctor.rs` returns 0.
    3. #454: `cargo test -p tome --lib wizard::tests::show_directory_summary_aligns_header_with_body_under_ansi` — passes.
    4. #453 + #456: `cargo test -p tome --test cli_init wizard_library_default_follows_custom_tome_home` — passes.
    5. #533: `cargo test -p tome --test cli_make_release` — 3 tests pass.

    **Success Criterion 4 — CI green + clippy clean + test count >= 1000:**
    1. `make ci` exits 0 (Task 3 of this plan already confirmed locally).
    2. Verify the CI workflow on GitHub Actions is green for the pushed branch (this requires `git push` first; see the session-completion workflow in CLAUDE.md).
    3. Test count >= 1000 (Task 3 already confirmed).

    If ANY criterion fails, document the failure in `19-07-SUMMARY.md` and surface as a phase-completion blocker. Do NOT proceed to ship signal until resolved.
  </how-to-verify>
  <resume-signal>Type "approved" if all 4 success criteria pass, or describe issues (which criterion failed + observed vs expected).</resume-signal>
</task>

</tasks>

<verification>
- `make ci` exits 0
- Test count >= 1000 (per `rg -c "^\\s*#\\[test\\]"` aggregate)
- CHANGELOG.md `[Unreleased]` has Phase 19 entries for all 8 requirements
- REQUIREMENTS.md Traceability table has all 8 Phase 19 rows marked Done
- Human checkpoint approved (Task 4)
</verification>

<success_criteria>
All four ROADMAP success criteria (1-4) for Phase 19 are confirmed by the human-verification checkpoint:
1. `tome doctor` categorization + auto-fixable contradiction gone (#530 closed)
2. `tome status` last-sync line + per-directory skill counts
3. Five bugfixes ship with regression tests (#511, #532, #454, #453+#456, #533)
4. CI green + clippy clean + test count >= 1000

Phase 19 is complete; v0.11 is ready for release cut via `make release VERSION=0.11.0` (which now auto-stamps the CHANGELOG date per FIX-06).
</success_criteria>

<output>
After completion, create `.planning/phases/19-doctor-status-surface-bugfix-bundle/19-07-SUMMARY.md` documenting:
- Final test count (must be >= 1000)
- CI pipeline outcome (fmt-check + clippy + tests — green status confirmed)
- Human checkpoint outcome (approved / which criterion needed iteration)
- Any administrative actions queued (e.g. close #454 if Plan 05 took the Path 2B administrative-close route)
- Carry-overs identified during Phase 19 verification (none expected, but record any)
- Confirmation that `[Unreleased]` is NOT renamed (deferred to v0.11 release cut via `make release`)
</output>
