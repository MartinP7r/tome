---
phase: 06-display-polish-docs
plan: 02
type: execute
wave: 1
depends_on: []
files_modified:
  - .planning/PROJECT.md
  - CHANGELOG.md
autonomous: true
requirements:
  - WHARD-08
must_haves:
  truths:
    - "`.planning/PROJECT.md` contains a dedicated subsection titled `### Hardened in v0.7` (or Claude-discretion equivalent like `### v0.7 Wizard Closure (WIZ-01–05)`) that lists WIZ-01, WIZ-02, WIZ-03, WIZ-04, WIZ-05 as individual bullets (D-08)"
    - "Each WIZ-XX bullet carries the `Shipped v0.6, hardened v0.7 (Phases 4+5)` provenance note (D-08)"
    - "The outdated `### Known Gaps (deferred from v0.6)` subsection WIZ-01–05 bullet is removed from `.planning/PROJECT.md` (D-09)"
    - "The `### Previously Validated (re-verified in v0.7 research)` subsection is left untouched (D-10)"
    - "The `*Last updated:* …` footer at the bottom of `.planning/PROJECT.md` is updated to note Phase 6 completion and WIZ-01–05 closure, dated 2026-04-21 (D-11)"
    - "CHANGELOG.md `## [Unreleased]` (or v0.7) section references WHARD-07 tabled migration and WHARD-08 doc closure (Claude's discretion, executed)"
    - "The markdown remains valid (no broken headings, no orphaned bullets, no stray backticks)"
  artifacts:
    - path: ".planning/PROJECT.md"
      provides: "v0.7 wizard hardening closure section naming WIZ-01 through WIZ-05 as validated"
      contains: "WIZ-01"
    - path: "CHANGELOG.md"
      provides: "user-facing release-notes entry for v0.7 display polish + doc cleanup"
      contains: "WHARD-07"
  key_links:
    - from: ".planning/PROJECT.md §Requirements"
      to: ".planning/REQUIREMENTS.md WHARD-08 entry"
      via: "explicit WIZ-01..WIZ-05 mapping in the new Hardened in v0.7 subsection"
      pattern: "WIZ-0[1-5]"
    - from: ".planning/PROJECT.md footer"
      to: "Phase 6 completion"
      via: "dated last-updated line mentioning WIZ-01–05 closure"
      pattern: "2026-04-21.*Phase 6"
---

<objective>
Close the documentation half of the v0.7 milestone (WHARD-08) by updating `.planning/PROJECT.md` so WIZ-01 through WIZ-05 are explicitly marked validated with a "Shipped v0.6, hardened v0.7 (Phases 4+5)" provenance note, and by removing the stale "Known Gaps (deferred from v0.6)" WIZ-01–05 bullet. Apply decisions D-08, D-09, D-10, D-11 from CONTEXT.md. Additionally, add a CHANGELOG entry covering both WHARD-07 and WHARD-08 (Claude's discretion per CONTEXT.md — executed here since the release pass expects it).

Purpose: The current PROJECT.md contradicts reality. Line 70 still describes WIZ-01–05 as "Low priority since `tome init` is a one-time operation" — but Phases 4+5 just hardened precisely these items with validation, overlap detection, and test coverage. This plan corrects the record.

Output: A `.planning/PROJECT.md` whose Requirements section accurately reflects v0.7 wizard hardening closure, a refreshed "Last updated" footer, and a CHANGELOG entry marking the unreleased v0.7 scope.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/phases/06-display-polish-docs/06-CONTEXT.md

<interfaces>
<!-- Current state of .planning/PROJECT.md relevant blocks (executor will edit these). -->

Requirements section structure today (`.planning/PROJECT.md:11-56`):
- `## Requirements`
  - `### Validated` (shipped-historical bullets, ending at line 34)
  - `### Active` (lines 36-39) — contains "Migrate `show_directory_summary()` from manual println to `tabled`" bullet that is being closed by plan 06-01 AND a "Expand `KNOWN_DIRECTORIES`" bullet that stays (v2 WREG-01/02/03)
  - `### Validated in v0.7` (lines 41-47) — WHARD-01..06 bullets, keep unchanged
  - `### Previously Validated (re-verified in v0.7 research)` (lines 49-55) — per D-10, DO NOT TOUCH

Known Gaps block to remove (`.planning/PROJECT.md:68-70`):
```
### Known Gaps (deferred from v0.6)

- WIZ-01 through WIZ-05: Wizard rewrite with merged `KNOWN_DIRECTORIES` registry. The old wizard code still works but uses the legacy source/target mental model. Low priority since `tome init` is a one-time operation.
```

Footer today (`.planning/PROJECT.md:114-115`):
```
---
*Last updated: 2026-04-20 — Phase 5 (Wizard Test Coverage) complete — pure helpers and `tome init --no-input` now have unit + integration test coverage; combo matrix locks in `valid_roles()` ↔ `validate()` agreement*
```

WIZ-XX deliverable scope (from CONTEXT.md §specifics for D-08 one-liners):
- WIZ-01: Merged `KNOWN_DIRECTORIES` registry (shipped v0.6 silently; now formally validated)
- WIZ-02: Auto-discovery with role auto-assignment at wizard time
- WIZ-03: Custom directory addition with role selection
- WIZ-04: Summary table before confirmation
- WIZ-05: Removal of the legacy `find_source_target_overlaps()` dead code / split source-target mental model

v0.7 hardening that backs the "hardened" claim (from CONTEXT.md specifics bullet):
  (a) `Config::validate()` path-overlap checks (Cases A/B/C) — Phase 4
  (b) `Config::save_checked` with TOML round-trip — Phase 4
  (c) `--no-input` plumbing — Phase 5
  (d) unit + integration test coverage for pure wizard helpers — Phase 5
  (e) 12-combo `(DirectoryType, DirectoryRole)` cross-product validation test — Phase 5

CHANGELOG.md location: `/Users/martin/dev/opensource/tome/CHANGELOG.md` (repo root). Executor must Read this file before editing to see the exact shape of existing `## [Unreleased]` or v0.7 section headings.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="false">
  <name>Task 1: Rewrite PROJECT.md Requirements + Known Gaps + footer for WIZ-01–05 closure</name>
  <files>.planning/PROJECT.md</files>
  <read_first>
    - /Users/martin/dev/opensource/tome/.planning/PROJECT.md (file under edit — see current lines 11-56 Requirements section, lines 68-70 Known Gaps, lines 114-115 footer)
    - /Users/martin/dev/opensource/tome/.planning/phases/06-display-polish-docs/06-CONTEXT.md (decisions D-08, D-09, D-10, D-11 verbatim)
    - /Users/martin/dev/opensource/tome/.planning/REQUIREMENTS.md (canonical WHARD-01..08 definitions for cross-reference)
    - /Users/martin/dev/opensource/tome/.planning/ROADMAP.md (Phase 4 + Phase 5 completion rows — confirms "hardened in v0.7 (Phases 4+5)" provenance)
  </read_first>
  <action>
    Make three targeted edits to `/Users/martin/dev/opensource/tome/.planning/PROJECT.md`.

    **Edit A: Insert a new `### Hardened in v0.7` subsection under `## Requirements`.**

    Insert this new subsection immediately AFTER the existing `### Previously Validated (re-verified in v0.7 research)` subsection (which currently ends around line 55 with the bullet `- ✓ Removed `find_source_target_overlaps()` dead code`) and BEFORE the `## Current Milestone: v0.7 Wizard Hardening` heading (around line 57). The new block:

    ```markdown
    ### Hardened in v0.7

    The wizard-surface work below shipped in v0.6 (as WIZ-01–05) but lacked validation, circular-path detection, and test coverage. v0.7 closed those gaps. All items are now shipped AND hardened — Shipped v0.6, hardened v0.7 (Phases 4+5).

    - ✓ **WIZ-01** — Merged `KNOWN_DIRECTORIES` registry replacing the split `KNOWN_SOURCES` / `KNOWN_TARGETS` arrays. Shipped v0.6, hardened v0.7: formal unit-test coverage for registry invariants and `find_known_directories_in` (Phase 5 / WHARD-04).
    - ✓ **WIZ-02** — Auto-discovery with role auto-assignment (ClaudePlugins→Managed, Directory→Synced, Git→Source) at wizard time. Shipped v0.6, hardened v0.7: `(DirectoryType, DirectoryRole)` combo-matrix test locks in `valid_roles()` ↔ `Config::validate()` agreement across all 12 combos (Phase 5 / WHARD-06).
    - ✓ **WIZ-03** — Custom directory addition with role selection during `tome init`. Shipped v0.6, hardened v0.7: invalid type/role combos are now rejected by `Config::validate()` before `save()` instead of being silently written (Phase 4 / WHARD-01).
    - ✓ **WIZ-04** — Summary table before confirmation. Shipped v0.6, hardened v0.7: migrated to `tabled` with `Style::rounded()` and terminal-width-aware truncation (Phase 6 / WHARD-07).
    - ✓ **WIZ-05** — Removal of the legacy source/target split mental model, including dead-code cleanup of `find_source_target_overlaps()`. Shipped v0.6, hardened v0.7: replaced with `Config::validate()` Cases A/B/C path-overlap detection and `Config::save_checked` TOML round-trip (Phase 4 / WHARD-02/03).

    *v0.7 hardening deliverables:* (a) `Config::validate()` path-overlap checks (Phase 4), (b) `Config::save_checked` with TOML round-trip (Phase 4), (c) `--no-input` plumbing (Phase 5), (d) unit + integration test coverage for pure wizard helpers (Phase 5), (e) 12-combo validation matrix (Phase 5), (f) `tabled` summary migration (Phase 6).
    ```

    **Edit B: Remove the stale Known Gaps bullet (D-09).**

    Delete the entire `### Known Gaps (deferred from v0.6)` subsection (currently lines 68-70 of PROJECT.md) — both the `### Known Gaps (deferred from v0.6)` heading AND the single WIZ-01 through WIZ-05 bullet beneath it. The WIZ-01–05 closure is now accurately represented in the new `### Hardened in v0.7` subsection; the "Known Gaps" heading has no other content, so the whole subsection goes away. Leave `### Out of Scope` (the next subsection) untouched.

    **Edit C: Update the footer (D-11).**

    Replace the final footer line (currently line 115):
    ```
    *Last updated: 2026-04-20 — Phase 5 (Wizard Test Coverage) complete — pure helpers and `tome init --no-input` now have unit + integration test coverage; combo matrix locks in `valid_roles()` ↔ `validate()` agreement*
    ```
    with:
    ```
    *Last updated: 2026-04-21 — Phase 6 (Display Polish &amp; Docs) complete — wizard summary migrated to `tabled` (WHARD-07); WIZ-01–05 marked validated as hardened in v0.7 (WHARD-08)*
    ```
    Keep the `---` separator on the line above the footer. Do NOT append a second footer line; overwrite in place.

    **Do NOT touch (per D-10 and out-of-scope):**
    - `### Validated` (shipped-history bullets) — unchanged.
    - `### Active` — unchanged. The "Migrate `show_directory_summary()` …" bullet is technically being shipped by Plan 06-01, but the Active section is a living checklist and Plan 06-01's summary handles that checkbox. This plan is scoped to WIZ-01–05 + footer.
    - `### Validated in v0.7` (WHARD-01..06) — unchanged.
    - `### Previously Validated (re-verified in v0.7 research)` — explicitly preserved per D-10.
    - `## Current Milestone`, `## Constraints`, `## Key Decisions`, `## Evolution` — unchanged.
  </action>
  <verify>
    <automated>grep -q '### Hardened in v0.7' .planning/PROJECT.md &amp;&amp; grep -q 'WIZ-01' .planning/PROJECT.md &amp;&amp; grep -q 'WIZ-05' .planning/PROJECT.md &amp;&amp; ! grep -q '### Known Gaps (deferred from v0.6)' .planning/PROJECT.md &amp;&amp; grep -q 'Last updated: 2026-04-21' .planning/PROJECT.md</automated>
  </verify>
  <acceptance_criteria>
    - `grep '### Hardened in v0.7' .planning/PROJECT.md` returns exactly one match.
    - `grep 'WIZ-01' .planning/PROJECT.md` returns at least one match.
    - `grep 'WIZ-02' .planning/PROJECT.md` returns at least one match.
    - `grep 'WIZ-03' .planning/PROJECT.md` returns at least one match.
    - `grep 'WIZ-04' .planning/PROJECT.md` returns at least one match.
    - `grep 'WIZ-05' .planning/PROJECT.md` returns at least one match.
    - All five WIZ-XX bullets appear WITHIN the `### Hardened in v0.7` subsection (verified by reading the file — the hardened section contains bullets starting with `- ✓ **WIZ-01**`, `- ✓ **WIZ-02**`, etc.).
    - `grep '### Known Gaps (deferred from v0.6)' .planning/PROJECT.md` returns zero matches.
    - `grep 'Low priority since .tome init. is a one-time operation' .planning/PROJECT.md` returns zero matches (the stale bullet text is gone).
    - `grep '### Previously Validated (re-verified in v0.7 research)' .planning/PROJECT.md` still returns a match (D-10 compliance).
    - `grep 'Last updated: 2026-04-21' .planning/PROJECT.md` returns a match.
    - `grep 'Phase 6' .planning/PROJECT.md` returns at least one match (present in footer per D-11).
    - `grep 'Shipped v0.6, hardened v0.7' .planning/PROJECT.md` returns at least one match.
    - Running `wc -l .planning/PROJECT.md` — file length is within ±20 lines of the pre-edit length (~115 lines) given one block added and one subsection removed. (Sanity check against accidental truncation; not a hard fail.)
  </acceptance_criteria>
  <done>`.planning/PROJECT.md` explicitly validates WIZ-01–05 as hardened in v0.7 under a new dedicated subsection, the stale "Known Gaps (deferred from v0.6)" entry is gone, and the footer reflects Phase 6 completion on 2026-04-21.</done>
</task>

<task type="auto" tdd="false">
  <name>Task 2: Add CHANGELOG.md entry for WHARD-07 tabled migration and WHARD-08 doc closure</name>
  <files>CHANGELOG.md</files>
  <read_first>
    - /Users/martin/dev/opensource/tome/CHANGELOG.md (file under edit — Read in full to see current shape of `## [Unreleased]` or v0.7 section heading and existing bullet style)
    - /Users/martin/dev/opensource/tome/.planning/phases/06-display-polish-docs/06-CONTEXT.md (the "CHANGELOG.md if touched" bullet in §specifics provides the exact prose template)
  </read_first>
  <action>
    Add CHANGELOG entries for WHARD-07 and WHARD-08 under the current unreleased / v0.7 section.

    **Step 1 — Inspect.** Read `/Users/martin/dev/opensource/tome/CHANGELOG.md` and identify the section header for unreleased/v0.7 changes. This could be `## [Unreleased]`, `## [0.7.0] - unreleased`, `## [0.7.0]`, or similar. Note the existing bullet style (hyphen, asterisk, markdown heading conventions). Match it exactly.

    **Step 2 — Append two bullets** to the most appropriate subsection within the unreleased/v0.7 block (commonly `### Changed` or `### Added` — pick whichever matches the existing file's conventions; if there is no subsection, insert them directly under the section header). Bullet content:

    ```markdown
    - Migrated `tome init` directory summary table to `tabled` with `Style::rounded()` borders and terminal-width-aware truncation via `Width::truncate(..).priority(PriorityMax::right())`. Long paths (including git-repo clones under `~/.tome/repos/<sha>/`) now render cleanly on narrow terminals without breaking column alignment. (WHARD-07)
    - Marked WIZ-01 through WIZ-05 as validated / hardened in `PROJECT.md`; removed the stale "Known Gaps (deferred from v0.6)" entry. Phases 4 + 5 of v0.7 already closed the correctness gaps (validation, overlap detection, test coverage); this update reflects that in the project docs. (WHARD-08)
    ```

    **Step 3 — No version-bump, no date edit.** Do NOT change `Cargo.toml` version numbers. Do NOT add a release date to the section header. Per the user's global memory note "Don't bump Cargo.toml version; `make release` handles it", this plan only adds entries to the existing unreleased section.

    **Step 4 — If CHANGELOG.md does not yet have an unreleased/v0.7 section heading**, add one in the canonical Keep-a-Changelog style immediately above the most recent released section:
    ```markdown
    ## [Unreleased]

    ### Changed

    - [bullet 1]
    - [bullet 2]
    ```
    But PREFER extending an existing section if one exists — do not create a duplicate.

    Note: If CHANGELOG.md is managed by cargo-dist / conventional-commit tooling and is fully auto-generated with a warning comment at the top, skip this task and note in the task SUMMARY that the CHANGELOG is auto-generated. CONTEXT.md flags this edit as Claude's-discretion precisely because the CHANGELOG shape is not locked by the milestone.
  </action>
  <verify>
    <automated>grep -q 'WHARD-07' CHANGELOG.md &amp;&amp; grep -q 'WHARD-08' CHANGELOG.md</automated>
  </verify>
  <acceptance_criteria>
    - `grep 'WHARD-07' CHANGELOG.md` returns at least one match (unless task was skipped because file is auto-generated — in which case the SUMMARY documents why).
    - `grep 'WHARD-08' CHANGELOG.md` returns at least one match (same exception).
    - `grep 'tabled' CHANGELOG.md` returns a match containing the WHARD-07 bullet (verifies bullet prose, not just reference-tag presence).
    - `grep 'PROJECT.md' CHANGELOG.md` returns a match containing the WHARD-08 bullet.
    - No new `## [X.Y.Z] - YYYY-MM-DD` release heading is added — only the existing unreleased block is modified.
    - Markdown remains valid (no orphaned `##` headings, no broken list items — spot-check by reading the edited section).
  </acceptance_criteria>
  <done>CHANGELOG.md references WHARD-07 (tabled migration) and WHARD-08 (doc closure) in the unreleased v0.7 section, matching the existing file's prose/bullet style. No version bump, no release date, no duplicate sections.</done>
</task>

</tasks>

<verification>
After both tasks complete:
```
grep -E '(WIZ-0[1-5]|Hardened in v0.7|Last updated: 2026-04-21)' .planning/PROJECT.md
grep -E '(WHARD-07|WHARD-08)' CHANGELOG.md
```
Both commands should return matches. Additionally, a manual read of the edited `### Hardened in v0.7` subsection should flow naturally and link the v0.6 shipping event to the v0.7 hardening work without contradicting other sections of PROJECT.md.

No Rust code is touched by this plan, so `cargo test` / `cargo clippy` are not gating here — but running `make ci` is harmless and a good final sanity pass after BOTH plans in this phase land.
</verification>

<success_criteria>
- WHARD-08 requirement met: WIZ-01 through WIZ-05 are explicitly marked validated in `.planning/PROJECT.md` with a "Shipped v0.6, hardened v0.7 (Phases 4+5)" provenance note (D-08).
- The stale `### Known Gaps (deferred from v0.6)` WIZ-01–05 entry is removed (D-09).
- `### Previously Validated (re-verified in v0.7 research)` is unchanged (D-10).
- Footer reflects Phase 6 completion dated 2026-04-21 (D-11).
- CHANGELOG.md references both WHARD-07 and WHARD-08 in its unreleased section.
</success_criteria>

<output>
After completion, create `.planning/phases/06-display-polish-docs/06-02-project-md-wiz-closure-SUMMARY.md` using the standard summary template. Highlights to capture:
- Exact headings inserted and removed in `.planning/PROJECT.md`
- Confirmation that D-10 was honored (Previously Validated subsection untouched)
- New footer text verbatim
- CHANGELOG section/bullets added (or note why skipped if auto-generated)
- Any discretion choices made (exact subsection title, one-liner wording per WIZ-XX)
</output>
