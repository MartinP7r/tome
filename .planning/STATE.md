---
gsd_state_version: 1.0
milestone: v0.8
milestone_name: Wizard UX & Safety Hardening
status: executing
stopped_at: Completed 07-03-wux-01-05-tome-home-prompt-PLAN.md
last_updated: "2026-04-23T12:31:16.166Z"
last_activity: 2026-04-23
progress:
  total_phases: 2
  completed_phases: 0
  total_plans: 4
  completed_plans: 3
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-04-23)

**Core value:** Every AI coding tool on a developer's machine shares the same skill library without manual copying or per-tool configuration.
**Current focus:** Phase 07 — wizard-ux-greenfield-brownfield-legacy

## Current Position

Milestone: v0.8 — Wizard UX & Safety Hardening
Phase: 07 (wizard-ux-greenfield-brownfield-legacy) — EXECUTING
Plan: 4 of 4
Status: Ready to execute
Last activity: 2026-04-23

Progress: [░░░░░░░░░░] 0% (roadmap created, plans pending)

## Performance Metrics

- Requirements defined: 8 (v1 — 5 WUX + 3 SAFE)
- Requirements mapped to phases: 8/8 ✓
- Phases: 2 (Phase 7 WUX, Phase 8 SAFE)
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
- [Phase 07-wizard-ux-greenfield-brownfield-legacy]: WUX-04: additive resolve_tome_home_with_source — kept existing resolve_tome_home for non-init call sites; only Command::Init consumes the tagged variant
- [Phase 07-wizard-ux-greenfield-brownfield-legacy]: WUX-03: parse TOML (not substring-match) for legacy-schema detection; graceful no-op on malformed files; interactive default is move-aside (non-destructive backup); --no-input default is leave with stderr note
- [Phase 07-wizard-ux-greenfield-brownfield-legacy]: WUX-01/05: Step 0 gated on matches!(source, TomeHomeSource::Default) && !no_input; custom tome_home persisted to XDG via merge-preserve write; configure_library default derives from <tome_home>/skills; fixed wizard.rs:310 latent bug by using resolve_config_dir(tome_home)

### Pending Todos

- **First:** merge PR #455 + ship v0.7.1 via `make release VERSION=0.7.1`
- **Then:** ship v0.7.2 patch with #456 + #457 (small scope, could bundle with v0.8 Phase 7 or ship separately)
- **Then:** `/gsd:plan-phase 7` to decompose the first v0.8 phase (Wizard UX)
- **Then:** `/gsd:plan-phase 8` for the safety refactors

### Blockers/Concerns

- `make release VERSION=0.7.1` is user-triggered (not gsd automation) — can happen in parallel with v0.8 phase planning
- Cross-machine portability (#458) intentionally punted to v0.9 — users needing it before v0.9 can use the manual workaround in epic #459

## Session Continuity

Last session: 2026-04-23T12:31:16.163Z
Stopped at: Completed 07-03-wux-01-05-tome-home-prompt-PLAN.md
Resume file: None
