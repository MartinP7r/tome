---
phase: 14-unowned-library-lifecycle
plan: 08
subsystem: docs+testing
tags: [changelog, requirements, traceability, integration-tests, assert_cmd, cli]

# Dependency graph
requires:
  - phase: 14-04
    provides: tome reassign accepts Unowned input + D-A1 collision check + D-A2 target-only rejection
  - phase: 14-05
    provides: tome remove skill subcommand + RemoveSkillFailureKind aggregation
  - phase: 14-06
    provides: status Unowned section (StatusReport.unowned + format_unowned_section)
  - phase: 14-07
    provides: doctor Unowned section (DoctorReport.unowned_skills + D-D3 contract)
provides:
  - REQUIREMENTS.md / ROADMAP.md / PROJECT.md vocabulary updates honoring D-API-1/-2 merge
  - CHANGELOG.md [Unreleased] v0.10 entry with BREAKING callout for `tome remove <name>` → `tome remove dir <name>`
  - 10 phase14_-prefixed end-to-end CLI integration tests anchoring UNOWN-01..03 success criteria to the real binary
  - Reusable Phase14Fixture builder pattern (manifest pre-population) for future Unowned-state CLI tests
affects: [phase-15-cli-hardening, phase-16-cleanup-ux-and-docs, phase-17-release]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Manifest pre-population fixture pattern — fabricate Unowned entries via direct .tome-manifest.json write rather than sync-then-orphan ceremony; complements TestEnvBuilder which is sync-driven"
    - "Phase14Fixture::cmd helper bundles --tome-home + --config + --machine + NO_COLOR=1 for end-to-end command invocation against pre-staged state"

key-files:
  created:
    - .planning/phases/14-unowned-library-lifecycle/14-08-docs-and-integration-tests-SUMMARY.md
  modified:
    - .planning/REQUIREMENTS.md
    - .planning/ROADMAP.md
    - .planning/PROJECT.md
    - CHANGELOG.md
    - crates/tome/tests/cli.rs

key-decisions:
  - "Used strikethrough + supersession note for UNOWN-01/02 in REQUIREMENTS.md rather than rewriting the bullets in-place — preserves traceability of the original spec wording for future readers tracing why the verb merge happened"
  - "Manifest fixtures use real on-disk skill directories (not just JSON) so reassign's content-hash path and remove-skill's library-removal path operate on actual filesystem state — only provenance is fabricated"
  - "Owned/Unowned mix supported by phase14_build_fixture so tests like phase14_status_text_omits_unowned_section_when_empty can stage 'no Unowned skills, only Owned ones' configurations"
  - "machine.toml fixtures pre-stage `disabled = [\"orphan-foo\"]` membership directly to verify D-B1 cleanup of all six required scopes (manifest, library dir, dist symlinks, lockfile, machine.toml disabled, per-directory memberships)"

patterns-established:
  - "Phase14Fixture pattern: fabricate Unowned manifest entries directly (skip sync-then-orphan ceremony) when testing post-Unowned-transition behaviour"
  - "Strikethrough + supersession-note traceability for vocabulary-merge scenarios (mirrors how Phase 13 D-01 handled the RECON-01 'version differs' wording supersession)"

requirements-completed: [UNOWN-01, UNOWN-02, UNOWN-03]

# Metrics
duration: 25min
completed: 2026-05-07
---

# Phase 14 Plan 08: Docs and Integration Tests Summary

**REQUIREMENTS.md / ROADMAP.md / PROJECT.md / CHANGELOG.md updated to reflect the D-API-1/-2 merge (`tome adopt` → `tome reassign`, `tome forget` → `tome remove skill`), v0.10 [Unreleased] entry calls out the BREAKING `tome remove <name>` → `tome remove dir <name>` restructure, and 10 phase14_-prefixed integration tests in tests/cli.rs anchor UNOWN-01..03 success criteria to the real `tome` binary via assert_cmd.**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-05-07T13:55:00Z (approx)
- **Completed:** 2026-05-07T14:20:00Z (approx)
- **Tasks:** 2
- **Files modified:** 5
- **Tests added:** 10 (all passing)
- **Total test count:** 845 (684 unit + 151 cli integration + 10 cli_sync_reconcile)

## Accomplishments

- Phase 14 vocabulary merge is now documented across all four planning surfaces (REQUIREMENTS.md, ROADMAP.md, PROJECT.md, CHANGELOG.md). Future readers tracing UNOWN-01/02 see both the original wording (strikethrough) and the supersession note pointing at D-API-1/-2 in CONTEXT.md.
- BREAKING change to `tome remove <name>` shape is called out in CHANGELOG.md `[Unreleased]` so v0.10 release notes inherit it cleanly.
- 10 end-to-end integration tests covering all three success criteria of Phase 14:
  - 3 reassign tests (UNOWN-01: happy path + D-A2 target-only rejection + D-A1 collision with --force bypass)
  - 3 remove-skill tests (UNOWN-02: full cleanup of all six D-B1 scopes + D-B2 owned guard with hint + D-B3 confirmation gate)
  - 4 status/doctor tests (UNOWN-03: status text section + status JSON shape + doctor D-D3 informational + empty-set stable rendering)
- Reusable Phase14Fixture builder pattern lifts the "stage Unowned entries without sync ceremony" mechanic into a shared helper. HARD-13 (Phase 15) will absorb this into `tests/cli_remove.rs` etc.

## Task Commits

1. **Task 1: Update planning docs and CHANGELOG.md** — `570f261` (docs)
2. **Task 2: Add UNOWN-01..03 integration tests in tests/cli.rs** — `0878e50` (test)

**Plan metadata commit:** added below as part of `/gsd:execute-phase` final-commit step.

## Files Created/Modified

- `.planning/REQUIREMENTS.md` — UNOWN-01/02 wording superseded with D-API-1/-2 notes; Phase 14 vocabulary note added below the Traceability table; UNOWN-01..03 status flipped Pending → Validated.
- `.planning/ROADMAP.md` — Phase 14 plan-list bullet updated to reflect the merged API; success criteria 1/2/3 expanded with the delivered semantics (`tome reassign --to`, `tome remove skill`, `previous_source` rendering, D-D3 informational).
- `.planning/PROJECT.md` — Active milestone bullet for unowned-library lifecycle now references the merged commands; Decisions table entry strikes through `tome adopt`/`tome forget` and points at the merge rationale; status flipped to "Validated".
- `CHANGELOG.md` — `[Unreleased]` filled with v0.10 Phase 14 surface (Added: reassign Unowned input, remove skill, --force, target-only rejection, status/doctor unowned sections, previous_source schema; Changed: BREAKING `tome remove <name>` shape).
- `crates/tome/tests/cli.rs` — +540 lines, 10 new tests + Phase14Fixture builder + helper functions (phase14_manifest_entry, phase14_write_library_skill).

## Decisions Made

- **Strikethrough + supersession note over in-place rewrite for REQUIREMENTS.md.** The original UNOWN-01/02 wording is preserved as `~~strikethrough~~` followed by the merged-API description. Mirrors how Phase 13 D-01 handled the RECON-01 "version differs" wording supersession in ROADMAP.md (visible precedent in the codebase). Trade-off: bullets are slightly longer, but `git blame` for someone tracing "why does the wording differ from CONTEXT.md" lands on this commit with both shapes in view.
- **Manifest fixtures use real on-disk skill directories.** `phase14_write_library_skill` writes a real `library_dir/<skill>/SKILL.md` so commands that touch filesystem state (reassign content-hash, remove-skill library-dir-deletion) operate on actual data. Only the `source_name`/`previous_source`/`source_path` provenance is fabricated. Trade-off: heavier fixtures than pure-JSON manifest stubs, but the realism prevents test gaps where a code path reads from disk and gets None.
- **Phase14Fixture as a separate struct from TestEnv.** TestEnvBuilder is sync-driven (skills get added by writing source dirs and running `tome sync`). Phase14Fixture is manifest-driven (skills get added by writing the manifest directly). Different mental models; co-locating them in one builder would have made the API confusing. HARD-13 (Phase 15) is where the broader `tests/cli.rs` split happens; this lays groundwork for the per-domain helpers.
- **`cmd()` helper sets `--config` + `--tome-home` + `--machine` + `NO_COLOR=1` together.** All Phase 14 fixtures use a non-default tome.toml location and an explicit machine.toml, so threading those flags through every test invocation would have been noisy. Centralising in `Phase14Fixture::cmd()` keeps the test bodies focused on the behaviour they're asserting.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed clippy `doc_lazy_continuation` warning on Phase14Fixture doc comment**

- **Found during:** Task 2 (after `cargo clippy --all-targets -- -D warnings`)
- **Issue:** Doc comment on `struct Phase14Fixture` wrapped a sentence with a leading `+` continuation — clippy 1.95 flagged this as a lazy-continuation lint and refused the build under `-D warnings`.
- **Fix:** Reflowed the doc comment to use prose ("…library_dir / and the on-disk locations…") rather than the `+`-continuation shape.
- **Files modified:** crates/tome/tests/cli.rs (line 6181)
- **Verification:** `cargo clippy -p tome --all-targets -- -D warnings` exits 0 after fix.
- **Committed in:** 0878e50 (Task 2 commit, applied before commit)

**2. [Rule 1 - Bug] Added `#[allow(dead_code)]` to Phase14Fixture struct**

- **Found during:** Task 2 (initial `cargo build -p tome --tests`)
- **Issue:** `struct Phase14Fixture { tmp: TempDir, ... }` triggered a dead-code warning on the `tmp` field (never read but must exist to keep TempDir alive for the test body).
- **Fix:** Added `#[allow(dead_code)]` to the struct (matches the existing TestEnvBuilder/TestEnv pattern at lines 82, 92, 103, 304 of tests/cli.rs).
- **Files modified:** crates/tome/tests/cli.rs
- **Verification:** `cargo build -p tome --tests` clean.
- **Committed in:** 0878e50 (Task 2 commit, applied before commit)

**3. [Rule 3 - Blocking] cargo fmt reflowed two long-argument call sites in test bodies**

- **Found during:** Task 2 (after `cargo fmt --check`)
- **Issue:** Two call sites of `phase14_build_fixture(&[...], &[...], &[...])` wrapped onto multi-line shapes in my initial draft; rustfmt preferred a single-line shape since the args fit within line length.
- **Fix:** Ran `cargo fmt -p tome` to apply the rustfmt-preferred shape. No semantic change.
- **Files modified:** crates/tome/tests/cli.rs (two call sites)
- **Verification:** `cargo fmt --check` clean.
- **Committed in:** 0878e50 (Task 2 commit, applied before commit)

---

**Total deviations:** 3 auto-fixed (2 Rule 1 lints, 1 Rule 3 formatting)
**Impact on plan:** No scope creep. All three deviations are mechanical compliance with the project's `-D warnings` + fmt-check policy enforced in CI. The plan's intent (8+ tests, all green) is delivered exactly as specified; the test count is 10 (the plan listed 9 explicit scenarios, 8+ acceptance threshold — both satisfied).

## Issues Encountered

- **`make ci` requires the `typos` system binary which is not on PATH locally.** Resolution: ran `make fmt-check && make lint && make test` (all green), then `cargo install typos-cli` and `~/.cargo/bin/typos` against the changed files (no typos). CI on GitHub Actions will run `typos` from its provisioned PATH. This isn't a Phase 14 issue — it's a local-dev shell PATH gap independent of this plan's work.
- **No other issues.**

## User Setup Required

None — all changes are documentation and tests, no external services touched.

## Next Phase Readiness

Phase 14 is now functionally complete. All 8 plans (14-01 through 14-08) have shipped:

- 14-01: previous_source schema (manifest + lockfile)
- 14-02: SkillSummary type
- 14-03: tome remove dir/skill nested clap subcommand + tome reassign --force
- 14-04: tome reassign accepts Unowned input (UNOWN-01)
- 14-05: tome remove skill (UNOWN-02)
- 14-06: status Unowned section
- 14-07: doctor Unowned section (UNOWN-03 — both halves)
- 14-08: docs vocabulary merge + integration tests (this plan)

After this plan, the `/gsd:execute-phase` orchestrator's verifier pass will run against the phase. Successful verification advances STATE.md to Phase 15 (CLI hardening — beta cut). HARD-13 in Phase 15 will absorb the Phase14Fixture pattern when splitting tests/cli.rs into per-domain files (`cli_remove.rs`, `cli_reassign.rs`, `cli_status.rs`).

**No blockers.** All quality gates green: 845 tests pass, `cargo fmt --check` clean, `cargo clippy --all-targets -- -D warnings` clean, `typos` clean.

## Self-Check

All claimed artifacts verified:

- File: `.planning/phases/14-unowned-library-lifecycle/14-08-docs-and-integration-tests-SUMMARY.md` — FOUND (this file).
- File: `.planning/REQUIREMENTS.md` — modified, contains "superseded by Phase 14 D-API-1" and "superseded by Phase 14 D-API-2".
- File: `.planning/ROADMAP.md` — modified, Phase 14 success criteria 1/2 use `tome reassign <skill> --to <directory>` and `tome remove skill <name>` wording; plan 14-08 marked complete.
- File: `.planning/PROJECT.md` — modified, line 142 + Decisions table both reflect the merge.
- File: `CHANGELOG.md` — modified, contains "BREAKING", "tome remove dir", "tome remove skill", "tome reassign --force".
- File: `crates/tome/tests/cli.rs` — modified, +540 lines, all 10 phase14_-prefixed tests pass.
- Commit `570f261` — FOUND in `git log --oneline`.
- Commit `0878e50` — FOUND in `git log --oneline`.

## Self-Check: PASSED

---
*Phase: 14-unowned-library-lifecycle*
*Completed: 2026-05-07*
