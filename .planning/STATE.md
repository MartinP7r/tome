---
gsd_state_version: 1.0
milestone: v0.8
milestone_name: Wizard UX & Safety Hardening
status: defining_requirements
stopped_at: Milestone v0.8 started — requirements defined, roadmap pending
last_updated: "2026-04-23T00:00:00Z"
last_activity: 2026-04-23
progress:
  total_phases: 0
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-04-23)

**Core value:** Every AI coding tool on a developer's machine shares the same skill library without manual copying or per-tool configuration.
**Current focus:** v0.8 Wizard UX & Safety Hardening — see epic [#459](https://github.com/MartinP7r/tome/issues/459)

## Current Position

Milestone: v0.8 — Wizard UX & Safety Hardening
Phase: Not started (defining requirements)
Plan: —
Status: Defining requirements
Last activity: 2026-04-23 — Milestone v0.8 started

Progress: [░░░░░░░░░░] 0% (requirements defined, roadmap pending)

## Performance Metrics

- Requirements defined: 8 (v1 — 5 WUX + 3 SAFE)
- Scope anchor: GitHub issue #459 (epic)
- Prerequisites (not in v0.8): v0.7.1 (PR #455) + v0.7.2 (#456, #457) — both patch releases

## Accumulated Context

### Decisions

Historical decisions are archived in:
- `.planning/PROJECT.md` — rolling Key Decisions table (v0.6 + v0.7)
- `.planning/milestones/v0.7-ROADMAP.md` — per-phase decisions for v0.7
- `.planning/milestones/v0.6-ROADMAP.md` — per-phase decisions for v0.6

v0.8-specific decisions (from epic #459):

- **D-1 (v0.8 scope):** machine.toml path overrides are NOT in v0.8 — deferred to v0.9 because it's a bigger design requiring new schema fields and override-apply timing in the load pipeline.
- **D-2 (v0.8 scope):** `tome_home` prompt writes XDG config (not `TOME_HOME` env-var injection into shell rc) — XDG file is shell-agnostic and propagates to cron/editor/subshells.
- **D-3 (v0.8 scope):** Wizard brownfield flow default = "use existing" — safest for the dotfiles-sync workflow the reporter described.
- **D-4 (v0.8 scope):** Legacy `~/.config/tome/config.toml` detection = warn + offer delete, NOT silent auto-delete — file may contain user-valued data worth manual review.

### Pending Todos

- **First:** merge PR #455 + ship v0.7.1 via `make release VERSION=0.7.1`
- **Then:** ship v0.7.2 patch with #456 + #457 (small scope, could bundle with v0.8 Phase 1 or ship separately)
- **Then:** `/gsd:plan-phase 7` to decompose the first v0.8 phase (wizard UX)

### Blockers/Concerns

- `make release VERSION=0.7.1` is user-triggered (not gsd automation) — can happen in parallel with v0.8 phase planning
- Cross-machine portability (#458) intentionally punted to v0.9 — users needing it before v0.9 can use the manual workaround in epic #459

## Session Continuity

Last session: 2026-04-23 — `/gsd:new-milestone v0.8`
Stopped at: Milestone v0.8 defined (PROJECT.md + STATE.md + REQUIREMENTS.md); roadmap pending roadmapper spawn
Resume file: None (next step is `/gsd:plan-phase 7` after roadmap lands)
