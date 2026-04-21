---
phase: 06-display-polish-docs
plan: 02
subsystem: documentation
tags: [project-docs, changelog, traceability, whard-08, wiz-closure]

# Dependency graph
requires:
  - phase: 04-wizard-correctness
    provides: Config::validate() path-overlap checks, Config::save_checked TOML round-trip (hardens WIZ-03, WIZ-05)
  - phase: 05-wizard-test-coverage
    provides: Pure helper unit tests, --no-input integration test, 12-combo matrix (hardens WIZ-01, WIZ-02)
provides:
  - .planning/PROJECT.md '### Hardened in v0.7' subsection listing WIZ-01..05 with Phase 4+5 provenance
  - .planning/PROJECT.md footer dated 2026-04-21 referencing Phase 6 completion
  - CHANGELOG.md '### Changed — v0.7 Wizard Hardening' block under [Unreleased] citing WHARD-07 and WHARD-08
affects: [v0.7-release-notes, milestone-close, future-plan-phase-context-assembly]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Keep a Changelog: scope-annotated subsection under [Unreleased] (e.g., '### Changed — v0.7 Wizard Hardening') keeps milestone boundaries visible without cutting a release"
    - "PROJECT.md requirements taxonomy: dedicated '### Hardened in v0.X' subsection for items shipped in a prior milestone and hardened in the current one — distinct from '### Validated in v0.X' (net-new validation work)"

key-files:
  created:
    - .planning/phases/06-display-polish-docs/06-02-project-md-wiz-closure-SUMMARY.md
  modified:
    - .planning/PROJECT.md
    - CHANGELOG.md

key-decisions:
  - "Named the new PROJECT.md subsection '### Hardened in v0.7' (verbatim from the plan's suggested title). Chose this over alternatives like '### Wizard Hardening Closure' because it mirrors the existing '### Validated in v0.7' heading shape and reads consistently with the footer text."
  - "Placed the new subsection immediately AFTER '### Previously Validated (re-verified in v0.7 research)' and BEFORE '## Current Milestone' — keeps all Requirements subsections contiguous under '## Requirements'."
  - "Removed the entire '### Known Gaps (deferred from v0.6)' heading (not just the bullet) because the one WIZ-01–05 bullet was its sole content; leaving an empty heading would look like dead markup."
  - "Per WIZ-XX bullet: encoded the v0.7 provenance inline ('Shipped v0.6, hardened v0.7: ...') rather than relying only on the subsection intro paragraph. Redundant on paper, but each bullet is now self-contained for grep/traceability."
  - "Added a summary 'v0.7 hardening deliverables' line at the end of the subsection listing (a)-(f) items verbatim from CONTEXT.md §specifics, so the claim 'hardened in v0.7' is backed by concrete deliverables in-line."
  - "CHANGELOG layout: added '### Changed — v0.7 Wizard Hardening' as a new subsection at the top of the existing [Unreleased] block, directly above the pre-existing '### Breaking Changes — v0.6 Unified Directory Model'. This keeps v0.6 migration notes and v0.7 hardening notes both visible under [Unreleased] until `make release` cuts them."
  - "No Cargo.toml version bump, no release date on [Unreleased] (per user global memory: 'Don't bump Cargo.toml version; `make release` handles it')."

patterns-established:
  - "WIZ-XX / WHARD-XX cross-reference style: each 'Hardened in v0.X' bullet names the originating WIZ-XX label and the specific WHARD-XX deliverable that hardened it ('Phase 5 / WHARD-04')"

requirements-completed:
  - WHARD-08

# Metrics
duration: 2min
completed: 2026-04-21
---

# Phase 6 Plan 02: PROJECT.md WIZ Closure Summary

**Closed the v0.7 doc half of WHARD-08: PROJECT.md now explicitly marks WIZ-01–05 as shipped-in-v0.6 and hardened-in-v0.7 (Phases 4+5), stale "Known Gaps (deferred from v0.6)" subsection removed, footer dated 2026-04-21, CHANGELOG cites WHARD-07 + WHARD-08 under [Unreleased].**

## Performance

- **Duration:** 2 min
- **Started:** 2026-04-21T13:12:16Z
- **Completed:** 2026-04-21T13:14:05Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- `.planning/PROJECT.md` now has a dedicated `### Hardened in v0.7` subsection naming WIZ-01 through WIZ-05 with "Shipped v0.6, hardened v0.7 (Phases 4+5)" provenance on every bullet.
- Stale `### Known Gaps (deferred from v0.6)` subsection (which incorrectly described WIZ-01–05 as low-priority deferred work) removed entirely.
- `### Previously Validated (re-verified in v0.7 research)` preserved verbatim per D-10.
- Footer updated to `*Last updated: 2026-04-21 — Phase 6 (Display Polish & Docs) complete — wizard summary migrated to `tabled` (WHARD-07); WIZ-01–05 marked validated as hardened in v0.7 (WHARD-08)*`.
- CHANGELOG.md `[Unreleased]` now has a `### Changed — v0.7 Wizard Hardening` subsection citing WHARD-07 (tabled migration) and WHARD-08 (doc closure) in Keep-a-Changelog bullet style.

## Task Commits

Each task was committed atomically (both with `--no-verify` per parallel-execution contract — Plan 06-01 was running concurrently):

1. **Task 1: Rewrite PROJECT.md Requirements + Known Gaps + footer** — `c7aa180` (docs)
2. **Task 2: Add CHANGELOG.md entry for WHARD-07 / WHARD-08** — `426e15c` (docs)

**Plan metadata commit:** will follow this SUMMARY.md write (final commit captures SUMMARY + STATE + ROADMAP + REQUIREMENTS).

## Files Created/Modified

- `.planning/PROJECT.md` — Added `### Hardened in v0.7` subsection (WIZ-01..05 with Phase 4+5 provenance + hardening-deliverables recap). Removed `### Known Gaps (deferred from v0.6)` subsection. Updated footer date/content to Phase 6 completion.
- `CHANGELOG.md` — Added `### Changed — v0.7 Wizard Hardening` subsection under `[Unreleased]` with WHARD-07 (tabled migration) and WHARD-08 (doc closure) bullets.

## Decisions Made

See `key-decisions` in frontmatter — seven decisions captured, all following plan + CONTEXT.md D-08..D-11 verbatim. Highlights:

- Subsection title: `### Hardened in v0.7` (verbatim from plan's suggested title, aligns with existing `### Validated in v0.7`).
- Per-bullet provenance phrasing standardized: "Shipped v0.6, hardened v0.7: ..." + "(Phase N / WHARD-XX)" suffix.
- CHANGELOG subsection scoped to v0.7 (`### Changed — v0.7 Wizard Hardening`) to avoid mixing with the already-present v0.6 migration notes in the same [Unreleased] block.

## Deviations from Plan

None — plan executed exactly as written. All three edits (A/B/C) in Task 1 used the verbatim prose from the plan's `<action>` block. Task 2 used the verbatim bullets from the plan. Acceptance criteria all passed on first pass.

## Issues Encountered

None.

## Verification Evidence

Plan-level verification commands from `<verification>` block:

```
$ grep -E '(WIZ-0[1-5]|Hardened in v0.7|Last updated: 2026-04-21)' .planning/PROJECT.md
### Hardened in v0.7
The wizard-surface work below shipped in v0.6 (as WIZ-01–05) ...
- ✓ **WIZ-01** ...
- ✓ **WIZ-02** ...
- ✓ **WIZ-03** ...
- ✓ **WIZ-04** ...
- ✓ **WIZ-05** ...
*Last updated: 2026-04-21 — Phase 6 (Display Polish & Docs) complete ...*

$ grep -E '(WHARD-07|WHARD-08)' CHANGELOG.md
- Migrated `tome init` directory summary table to `tabled` ... (WHARD-07)
- Marked WIZ-01 through WIZ-05 as validated / hardened in `PROJECT.md` ... (WHARD-08)
```

Both commands returned matches. Task 1 `grep -c` sweep: `### Hardened in v0.7` = 1, WIZ-01..05 each = 1+, `### Known Gaps (deferred from v0.6)` = 0, `Low priority since` = 0, `### Previously Validated (re-verified in v0.7 research)` = 1, `Last updated: 2026-04-21` = 1, `Shipped v0.6, hardened v0.7` = 6 (subsection intro + 5 bullets). Task 2 `grep -c`: `WHARD-07` = 1, `WHARD-08` = 1, no new `## [0.7.x]` release heading added.

File length sanity: PROJECT.md 123 lines (was 115 pre-edit — +13 net lines, within the plan's ±20 tolerance).

## User Setup Required

None.

## Next Phase Readiness

Phase 6 is the last phase of the v0.7 milestone. With Plan 06-01 (WHARD-07: tabled migration) and this plan (WHARD-08: doc closure) both landed, the v0.7 Wizard Hardening milestone is complete. Ready for:

1. Final `make ci` sanity pass (after Plan 06-01's wave commits).
2. Verifier pass on Phase 6 as a whole.
3. `make release VERSION=0.7.0` when the user cuts the release.

## Self-Check: PASSED

All claims verified:

- `c7aa180` exists in `git log --oneline`: **FOUND**
- `426e15c` exists in `git log --oneline`: **FOUND**
- `.planning/PROJECT.md` exists with new `### Hardened in v0.7` subsection: **FOUND**
- `CHANGELOG.md` exists with WHARD-07 / WHARD-08 bullets under `[Unreleased]`: **FOUND**
- `.planning/phases/06-display-polish-docs/06-02-project-md-wiz-closure-SUMMARY.md` (this file): will exist after Write completes.

---
*Phase: 06-display-polish-docs*
*Completed: 2026-04-21*
