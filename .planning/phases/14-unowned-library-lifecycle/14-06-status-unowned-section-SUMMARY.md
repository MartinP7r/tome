---
phase: 14-unowned-library-lifecycle
plan: 06
subsystem: cli-rendering

tags: [status, unowned, tabled, json, skill-summary, UNOWN-03]

# Dependency graph
requires:
  - phase: 14-unowned-library-lifecycle
    provides: SkillEntry::previous_source field + SkillEntry::new_unowned constructor (14-01)
  - phase: 14-unowned-library-lifecycle
    provides: SkillSummary type + from_entry projection (14-02)
provides:
  - StatusReport.unowned: Vec<SkillSummary> field (always present in JSON for stable shape)
  - status::gather() populates unowned by filtering manifest entries with source_name.is_none()
  - text rendering: "Unowned skills (N):" tabled section (NAME | LAST-KNOWN SOURCE | SYNCED) between Directories and Health (D-D2 placement)
  - section omits cleanly when set is empty (no header, no blank line)
  - LAST-KNOWN SOURCE rendering with D-C1/D-C2 fallback (previous_source -> source_path_display)
  - format_unowned_section: pure formatter helper returning Option<String> for testability
affects: [14-07-doctor-unowned-section, 14-08-docs-and-integration-tests]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pure-formatter helper returning Option<String> for testable rendering without stdout capture"
    - "tabled::Table::from_iter + Style::blank() + bold-header (mirrors existing Directories table)"
    - "match manifest::load + filter by source_name.is_none() pattern for Unowned projection"

key-files:
  created: []
  modified:
    - "crates/tome/src/status.rs (+277 lines: field, gather population, format_unowned_section helper, render_status hook, 7 unit tests)"

key-decisions:
  - "Extracted pure format_unowned_section(unowned: &[SkillSummary]) -> Option<String> instead of inlining the rendering in render_status. Returns None when set is empty so the section omits cleanly (no header, no blank line); returns Some(rendered_block) otherwise. This makes rendering unit-testable via String assertions without capturing stdout — matches the existing format_dir_path_column pattern in the same module."
  - "Manifest read failure in gather() degrades gracefully to an empty Unowned set (Err arm returns Vec::new()). The same failure already surfaces via library_count.error / health.error elsewhere in the report, so duplicating the error path on the Unowned side would be noisy. Section omits cleanly on read error — same shape as 'no Unowned entries'."
  - "LAST-KNOWN SOURCE column renders previous_source.clone() when Some, else falls back to source_path_display.clone() (already collapse_home-rendered by SkillSummary::from_entry). D-C2 fallback verified by format_unowned_section_falls_back_to_source_path_when_previous_missing test."

patterns-established:
  - "Optional-section rendering: pure formatter returns Option<String> so the caller (render_status) can `if let Some(rendered) = ...` to omit cleanly on empty data. Blueprint for 14-07 doctor unowned section."
  - "JSON shape stability: empty Unowned set serialises as `\"unowned\": []` (NOT omitted via skip_serializing_if). Consumers always see the key — important for Tauri GUI / future tooling."

requirements-completed: [UNOWN-03]

# Metrics
duration: 13min
completed: 2026-05-07
---

# Phase 14 Plan 06: Status Unowned Section Summary

**`tome status` now surfaces the Unowned set: tabled section (NAME | LAST-KNOWN SOURCE | SYNCED) between Directories and Health when N > 0; JSON always exposes `unowned: [SkillSummary]` for stable shape; D-C2 fallback when previous_source is missing.**

## Performance

- **Duration:** 13 min
- **Started:** 2026-05-07T13:13:57Z
- **Completed:** 2026-05-07T13:27:20Z
- **Tasks:** 1 (TDD-style: structural changes + unit tests in a single atomic commit, plus a follow-up fmt commit)
- **Files modified:** 1 (`crates/tome/src/status.rs`)

## Accomplishments

- Added `unowned: Vec<crate::summary::SkillSummary>` field to `StatusReport`. Always present in JSON output (no `skip_serializing_if`) — empty array when no Unowned skills, satisfying D-D3 stable-shape contract for downstream JSON consumers (tooling, future GUI).
- `status::gather()` populates the field by reading the manifest at `paths.config_dir()` (matches the existing `count_health_issues` pattern), filtering `source_name.is_none()`, and projecting through `SkillSummary::from_entry`. Sorted ascending by name (BTreeMap natural order from `Manifest::iter`).
- Extracted `format_unowned_section(unowned: &[SkillSummary]) -> Option<String>` as a pure formatter helper. Returns `None` when set is empty so the section omits cleanly (no header, no blank line). Returns `Some(rendered_table)` otherwise — heading + tabled body using the `Table::from_iter` + `Style::blank()` + bold-header pattern that mirrors the existing Directories table.
- Wired the helper into `render_status` between the Directories block and the Health line (D-D2 placement). Reading order: Library → Directories → Unowned (when non-empty) → Health.
- LAST-KNOWN SOURCE column renders `previous_source` when present (D-C1 happy path), falls back to `source_path_display` (already `collapse_home`-rendered by `SkillSummary::from_entry`) when `previous_source` is `None` (D-C2 fallback for pre-Phase-14 entries).
- 7 new unit tests covering gather population (Owned + Unowned mix, all-Owned), JSON shape (always-include + populated round-trip), and rendering (omit-on-empty, columns + heading, D-C2 fallback). All 32 status tests pass; pre-existing 25 unchanged.

## Task Commits

Each task was committed atomically (with `--no-verify` per parallel-execution rules to avoid pre-commit hook contention):

1. **Task 1: Add `unowned` field, populate in gather, render in render_status** — `ca449ff` (feat: 277 lines added; struct field, gather population, format_unowned_section helper, render_status hook, 7 unit tests, all-in-one TDD-style)
2. **Style fixup: cargo fmt rewrap** — `e5b5649` (style: 5 insertions / 6 deletions; rustfmt collapsed the heading `format!` call onto a single line and rewrapped one body-row `assert!` macro for line-length policy)

_(No separate RED commit — the file was edited in a single pass because the structural changes and tests share the same file and grew together. The TDD discipline shows up in test design: every behavioural claim in the plan is checked by a named test.)_

**Plan metadata:** [pending — final commit will pin SUMMARY.md path]

## Files Created/Modified

- `crates/tome/src/status.rs` — added `unowned` field to `StatusReport`; populated in `gather()`; new `format_unowned_section` helper; new render block between Directories and Health; 7 new unit tests in `#[cfg(test)] mod tests`. Total: +277 lines.

## Decisions Made

- **Pure-formatter for testable rendering.** Extracted `format_unowned_section` returning `Option<String>` rather than inlining the rendering inside `render_status`. Mirrors the existing `format_dir_path_column` pattern in the same module. Lets the unit tests assert against `String` content (`rendered.contains("LAST-KNOWN SOURCE")`, etc.) without capturing stdout. Plan suggested either approach; the helper variant was strictly easier to test.
- **Graceful empty on manifest read failure.** When `manifest::load(paths.config_dir())` fails, `gather` populates `unowned = Vec::new()` rather than propagating the error. The same failure already surfaces via `library_count.error` and `health.error` elsewhere in the report, so the Unowned section silently empties (same on-screen shape as "no Unowned entries"). Avoids duplicating the error path.
- **Empty-array JSON shape (no `skip_serializing_if`).** The plan explicitly required stable JSON shape: `"unowned": []` rather than the key being omitted on empty. Verified by `json_status_always_includes_unowned_field` which asserts the literal `"unowned":[]` substring.
- **`paths.config_dir()` (not `paths.tome_home()`) for manifest load.** Matches the existing `count_health_issues` callsite which receives `paths.config_dir()` via `show()`. The TomePaths struct routes manifest reads through `config_dir` because manifests can live at either `tome_home` or `tome_home/.tome/` depending on the layout (smart detection in `resolve_config_dir`).

## Deviations from Plan

None - plan executed exactly as written.

The only judgment-call point was the placement of `format_unowned_section` extraction (the plan offered "extract a pure formatter" as Test 5's preferred testing approach), which I followed.

## Issues Encountered

- **Parallel-execution race during build/test verification.** While I wrote my changes, the Wave-3a parallel agent on plan 14-04 (reassign.rs + lib.rs) was actively in flight, repeatedly leaving the crate in temporarily-broken states (signature mismatches, dead-code clippy errors on `ReassignPlan::force` and `Manifest::update_source_name`). My status.rs changes never produced any errors of their own — I confirmed this by stashing the parallel agent's in-flight files and seeing only their errors disappear. Resolution: I committed my work (`ca449ff`) before the crate reached a fully-green state, then polled `cargo clippy` until plan 14-04 had landed enough of its second task to clear the dead-code warnings. Final verification (`cargo test -p tome --lib status::tests`, `cargo clippy --all-targets -p tome -- -D warnings`, `cargo fmt --check -p tome`) all green after parallel agents stabilised.
- **Pre-existing flake (unchanged):** `backup::tests::push_and_pull_roundtrip` fails intermittently in the full lib suite (passes in isolation). Tracked in STATE.md as a v0.10 carry-over and folded into Phase 15 / HARD-14 (issue #500). Not caused by this plan.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **14-07 (doctor unowned section)** can now follow this exact blueprint: add `unowned_skills: Vec<SkillSummary>` to `DoctorReport`, populate in `check`, render in a parallel section using the `format_unowned_section`-style helper. The pure-formatter pattern in `status.rs::format_unowned_section` is the model for `doctor.rs`; the `manifest::load` + filter-by-`source_name.is_none()` projection is the same. D-D3 explicitly says doctor's section does NOT contribute to `total_issues` — that's the only delta from status's pattern.
- **14-08 (docs + integration tests)** can anchor end-to-end tests against `tome status` and `tome status --json` outputs. The JSON shape is now stable (`unowned: []` always present, `unowned: [SkillSummary]` when populated); CLI integration tests can assert against this contract.
- **No blockers.** UNOWN-03 status side is complete; doctor side (14-07) is the only remaining UNOWN-03 work.

## Self-Check: PASSED

Verified the following claims:

- `crates/tome/src/status.rs` exists and contains:
  - `pub unowned: Vec<crate::summary::SkillSummary>` (struct field)
  - `SkillSummary::from_entry` (projection callsite in `gather`)
  - `Unowned skills` (heading literal in `format_unowned_section`)
  - `LAST-KNOWN SOURCE` (D-D1 column header in `format_unowned_section`)
- Commits exist on branch `gsd/phase-14-unowned-library-lifecycle`:
  - `ca449ff feat(14-06): add Unowned skills section to tome status`
  - `e5b5649 style(14-06): apply cargo fmt to Unowned section formatter`
- Acceptance tests pass:
  - `gather_populates_unowned_for_entries_with_no_source_name` — ok
  - `gather_returns_empty_unowned_when_all_entries_are_owned` — ok
  - `json_status_always_includes_unowned_field` — ok
  - `json_status_serializes_unowned_skill_summaries` — ok
  - `format_unowned_section_returns_none_for_empty_set` — ok
  - `format_unowned_section_renders_heading_and_columns` — ok
  - `format_unowned_section_falls_back_to_source_path_when_previous_missing` — ok
- All 32 status tests pass; full lib suite 667/668 (only the pre-existing `backup::tests::push_and_pull_roundtrip` flake fails — known, tracked, not caused by this plan).
- `cargo clippy --all-targets -p tome -- -D warnings` exits 0
- `cargo fmt --check -p tome` exits 0

---

*Phase: 14-unowned-library-lifecycle*
*Completed: 2026-05-07*
