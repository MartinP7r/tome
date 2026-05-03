---
gsd_state_version: 1.0
milestone: v0.10
milestone_name: Library-canonical Model + Cross-Machine Plugin Reconciliation
status: executing
stopped_at: Completed 11-04-migrate-library-command-PLAN.md
last_updated: "2026-05-03T13:46:14.976Z"
last_activity: 2026-05-03
progress:
  total_phases: 7
  completed_phases: 0
  total_plans: 5
  completed_plans: 4
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-05-02)

**Core value:** Every AI coding tool on a developer's machine shares the same skill library without manual copying or per-tool configuration.
**Current focus:** Phase 11 — library-canonical-core

## Current Position

Phase: 11 (library-canonical-core) — EXECUTING
Plan: 5 of 5
Status: Ready to execute
Last activity: 2026-05-03

**v0.10 phase shape (Phases 11–17):**

| Phase | Goal | Reqs | Cut |
|------:|------|------|-----|
| 11 | Library-canonical core | LIB-01..05 (5) | — |
| 12 | Marketplace adapter | ADP-01..04 (4) | — |
| 13 | Lockfile-authoritative sync | RECON-01..05 (5) | **alpha** |
| 14 | Unowned-library lifecycle | UNOWN-01..03 (3) | — |
| 15 | CLI hardening | HARD-01..22 (22) | **beta** |
| 16 | Cleanup-message UX + docs | UX-01..02 + DOC-01..03 (5) | **rc** |
| 17 | Migration polish + UAT + release | REL-01..05 (5) | **v0.10 final** |

## Accumulated Context

### Decisions

Historical decisions are archived in:

- `.planning/PROJECT.md` — rolling Key Decisions table (v0.6 + v0.7 + v0.8 + v0.9 + v0.10 D-LIB-01..05)
- `.planning/research/v0.10-library-canonical-design.md` — v0.10 design exploration with 9 OQs resolved (rationale + alternatives + risk per question)
- `.planning/milestones/v0.9-ROADMAP.md` — per-phase decisions for v0.9
- `.planning/milestones/v0.8-ROADMAP.md` — per-phase decisions for v0.8
- `.planning/milestones/v0.7-ROADMAP.md` — per-phase decisions for v0.7
- `.planning/milestones/v0.6-ROADMAP.md` — per-phase decisions for v0.6
- `.planning/milestones/v1.0-{REQUIREMENTS,ROADMAP}.md` — Tauri GUI milestone (drafted, deferred to after v0.10 ships)
- [Phase 11]: LIB-03: SkillEntry/LockEntry source_name lifted to Option<DirectoryName>; SkillEntry::new signature unchanged via twin-constructor pattern (new + new_unowned); Manifest::skills_get_mut accessor lifted into Plan 11-01 to keep manifest.rs touches contained
- [Phase 11]: LIB-01/LIB-02: consolidate_managed rewritten as recursive copy; managed flag semantics shift to 'update channel'; consolidate_local mirrors hash-match flag flip for symmetry; v0.9-shape symlinks refused (not auto-converted) per D-02 boundary defense
- [Phase 11]: LIB-04 D-10 hybrid triggers: tome remove explicitly transitions owned manifest entries to Unowned (source_name = None) and preserves library content; cleanup phase adds the same safety-net transition for users who manually edit tome.toml. Already-Unowned entries are preserved by definition. RemoveResult.library_entries_transitioned_to_unowned replaces library_entries_removed; FailureKind shrunk to 2 variants (DistributionSymlink, GitCache).
- [Phase 11-library-canonical-core]: LIB-05: tome migrate-library is one-shot CLI command (D-01) with manifest-anchored detection (D-03), broken-symlink preservation (D-04), SAFE-01 failure aggregation (D-05), idempotent re-runs (D-06). Sync refuses on v0.9-shape libraries with Conflict/Why/Suggestion error pointing at the new command (D-02). Entire migration_v010 module + sync gate deletes cleanly in v0.11+.

### v0.10 design context (consume during planning)

- **Library-canonical model:** managed skills become real directory copies (not symlinks). Source removal preserves library content. New `Unowned` skill state with `tome adopt` / `tome forget` for explicit lifecycle.
- **Lockfile-authoritative sync:** `tome sync` reconciles installed plugins to lockfile state (Cargo.lock-shaped). Drift surfaces interactively unless auto-install consent set.
- **MarketplaceAdapter trait:** `ClaudeMarketplaceAdapter` (shells out to `claude plugin install/update/list --json`) + `GitAdapter` (wrap `git.rs`).
- **Hard upstream constraint:** Claude CLI doesn't accept `--version` flag — adapter goes to "latest" only. Lockfile records actual installed version + surfaces drift; true version pinning is upstream feature request.
- **Migration:** first-sync detects symlink library, prompts (with diff summary), persists consent in `machine.toml::migration_v010_acknowledged`. Idempotent.
- **Behavior change:** plugin updates no longer auto-propagate via symlink — they require `tome sync`. Document in release notes.
- **CLI hardening folded in:** 22 review-followup issues (#485–#503) + ~5 older bug backlog (#416, #430, #433, #447, #457). Bundled because they touch the same modules as the library-canonical work.

### Phase dependency graph

```
11 (Library-canonical core)
 ├── 12 (Marketplace adapter)
 │    └── 13 (Lockfile-authoritative sync) ── ALPHA CUT
 │         └── 15 (CLI hardening) ── BETA CUT
 │              └── 16 (Cleanup UX + docs) ── RC CUT
 │                   └── 17 (Migration + UAT + release) ── v0.10 FINAL
 └── 14 (Unowned-library lifecycle) ── feeds into 16
```

Phase 14 can land in parallel with Phase 13 once Phase 11 is complete (both depend only on the manifest unowned-state work).

### Pending Todos / Carry-over

- **Linux UAT (carry-over from v0.8):** 2 pending items in `.planning/phases/08-*/08-HUMAN-UAT.md` (clipboard runtime + xdg-open runtime tests). Pending Linux desktop hardware. Run `/gsd:verify-work 08` when on Linux. Carried over for the fourth consecutive milestone — folded into Phase 17 / REL-03 for explicit resolution at v0.10 ship time.
- **Pre-existing flake:** `backup::tests::push_and_pull_roundtrip` — passes in isolation, intermittent in full suite. Folded into Phase 15 / HARD-14 (issue #500).
- **In-flight PRs:** #484 (chore/v0.10-prep doc drift + safety fixes) and #504 (refactor/v0.10-phase-c type lifts — `source_name` → `DirectoryName`, `GitRef` enum). Both should land before Phase 11 planning starts (REL-01 in Phase 17, but blocking work earlier).

### Blockers/Concerns

- **Open Q remaining for v0.10 implementation:** Whether `claude plugin install` is non-interactive enough to safely shell out to (no prompts requiring user input mid-install). Mitigation: investigate during Phase 12 (adapter implementation); if interactive, wrap in a controlled shell or feature-request to upstream.
- **Type-surface stability for v1.0 GUI:** v0.10 changes `SkillEntry`, `LockEntry`, and adds `SkillOrigin::Unowned`. Settling these in v0.10 is the prerequisite for v1.0 to build on stable IPC types.

## Session Continuity

Last session: 2026-05-03T13:46:14.973Z
Stopped at: Completed 11-04-migrate-library-command-PLAN.md
Resume file: None
