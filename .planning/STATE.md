---
gsd_state_version: 1.0
milestone: v0.10
milestone_name: Library-canonical Model + Cross-Machine Plugin Reconciliation
status: defining-requirements
stopped_at: PROJECT.md + STATE.md updated for v0.10; requirements + roadmap pending
last_updated: "2026-05-02T00:00:00.000Z"
last_activity: 2026-05-02
progress:
  total_phases: 0
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

Phase: Not started (defining requirements)
Plan: —
Status: Defining requirements
Last activity: 2026-05-02 — Milestone v0.10 started

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

### v0.10 design context (consume during requirements + roadmap)

- **Library-canonical model:** managed skills become real directory copies (not symlinks). Source removal preserves library content. New `Unowned` skill state with `tome adopt` / `tome forget` for explicit lifecycle.
- **Lockfile-authoritative sync:** `tome sync` reconciles installed plugins to lockfile state (Cargo.lock-shaped). Drift surfaces interactively unless auto-install consent set.
- **MarketplaceAdapter trait:** `ClaudeMarketplaceAdapter` (shells out to `claude plugin install/update/list --json`) + `GitAdapter` (wrap `git.rs`).
- **Hard upstream constraint:** Claude CLI doesn't accept `--version` flag — adapter goes to "latest" only. Lockfile records actual installed version + surfaces drift; true version pinning is upstream feature request.
- **Migration:** first-sync detects symlink library, prompts (with diff summary), persists consent in `machine.toml::migration_v010_acknowledged`. Idempotent.
- **Behavior change:** plugin updates no longer auto-propagate via symlink — they require `tome sync`. Document in release notes.
- **CLI hardening folded in:** 19 review-followup issues (#485–#503) + ~10 older bug backlog (#416, #430, #433, #447, #454, #456, #457, etc.). Bundled because they touch the same modules as the library-canonical work.

### Pending Todos / Carry-over

- **Linux UAT (carry-over from v0.8):** 2 pending items in `.planning/phases/08-*/08-HUMAN-UAT.md` (clipboard runtime + xdg-open runtime tests). Pending Linux desktop hardware. Run `/gsd:verify-work 08` when on Linux. Carried over for the fourth consecutive milestone.
- **Pre-existing flake:** `backup::tests::push_and_pull_roundtrip` — passes in isolation, intermittent in full suite. Folded into v0.10 hardening bundle (issue #500).
- **In-flight PRs:** #484 (chore/v0.10-prep doc drift + safety fixes) and #504 (refactor/v0.10-phase-c type lifts — `source_name` → `DirectoryName`, `GitRef` enum). Both are independent of v0.10 design and should land before phase planning starts.

### Blockers/Concerns

- **Open Q remaining for v0.10 implementation:** Whether `claude plugin install` is non-interactive enough to safely shell out to (no prompts requiring user input mid-install). Mitigation: investigate during Phase 12 (adapter implementation); if interactive, wrap in a controlled shell or feature-request to upstream.
- **Type-surface stability for v1.0 GUI:** v0.10 changes `SkillEntry`, `LockEntry`, and adds `SkillOrigin::Unowned`. Settling these in v0.10 is the prerequisite for v1.0 to build on stable IPC types.

## Session Continuity

Last session: 2026-05-02T00:00:00.000Z
Stopped at: PROJECT.md + STATE.md updated for v0.10. Next: define REQUIREMENTS.md (Step 9 of new-milestone workflow), then spawn gsd-roadmapper to produce ROADMAP.md (Step 10).
Resume file: `.planning/research/v0.10-library-canonical-design.md` is the design source-of-truth feeding requirements + roadmap.
