---
gsd_state_version: 1.0
milestone: v0.10
milestone_name: Library-canonical Model + Cross-Machine Plugin Reconciliation
status: ready-to-plan
stopped_at: ROADMAP.md created (7 phases, 49 reqs mapped, 100% coverage); next is /gsd:plan-phase 11
last_updated: "2026-05-02T00:00:00.000Z"
last_activity: 2026-05-02
progress:
  total_phases: 7
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-05-02)

**Core value:** Every AI coding tool on a developer's machine shares the same skill library without manual copying or per-tool configuration.
**Current focus:** v0.10 — Library-canonical Model + Cross-Machine Plugin Reconciliation. Reshape the library to be a single source of truth (managed-as-copy), make the lockfile authoritative for cross-machine reproducibility, ship marketplace adapters for plugin install/update on sync, and bundle the v0.9-review CLI hardening backlog.

## Current Position

Phase: 11 — Library-canonical core
Plan: — (none yet; awaiting `/gsd:plan-phase 11`)
Status: Ready to plan Phase 11
Last activity: 2026-05-02 — ROADMAP.md created (7 phases, 49 reqs, 100% coverage)

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

Last session: 2026-05-02T00:00:00.000Z
Stopped at: ROADMAP.md created — 7 phases (11–17), 49 v0.10 requirements mapped 1:1, 100% coverage validated. REQUIREMENTS.md traceability table filled. Next: `/gsd:plan-phase 11` to decompose Library-canonical core into executable plans.
Resume file: `.planning/ROADMAP.md` (Phase 11 details + success criteria) + `.planning/research/v0.10-library-canonical-design.md` (design source-of-truth).
