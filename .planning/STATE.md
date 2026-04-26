---
gsd_state_version: 1.0
milestone: v0.8
milestone_name: Wizard UX & Safety Hardening
status: executing
stopped_at: Completed 08.1-01-hotfix-01-lockfile-regen-resolved-paths-PLAN.md
last_updated: "2026-04-26T12:33:55.438Z"
last_activity: 2026-04-26
progress:
  total_phases: 2
  completed_phases: 2
  total_plans: 7
  completed_plans: 7
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-04-23)

**Core value:** Every AI coding tool on a developer's machine shares the same skill library without manual copying or per-tool configuration.
**Current focus:** Phase 08.1 — v0-8-1-hotfix-lockfile-regen-and-save-chain

## Current Position

Milestone: v0.8 — Wizard UX & Safety Hardening
Phase: 08.1 (v0-8-1-hotfix-lockfile-regen-and-save-chain) — EXECUTING
Plan: 2 of 3
Status: Ready to execute
Last activity: 2026-04-26

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
- [Phase 07-wizard-ux-greenfield-brownfield-legacy]: WUX-02: brownfield decision 4-way dispatch (UseExisting/Edit/Reinit/Cancel); --no-input + invalid config = Cancel (no silent advance); backup_brownfield_config uses copy-not-rename so Cancel-after-backup is safe; prefill union in configure_directories preserves custom dirs through edit (Pitfall 2 fix)
- [Phase 08-safety-refactors-partial-failure-visibility-cross-platform]: SAFE-01: RemoveResult.failures Vec<RemoveFailure> with typed FailureKind enum; Command::Remove surfaces grouped ⚠ K operations failed stderr block and returns Err on partial cleanup failures — exit ≠ 0 (closes #413)
- [Phase 08-safety-refactors-partial-failure-visibility-cross-platform]: SAFE-01: Integration test uses chmod 0o500 (not 0o000 per plan) — 0o000 causes plan() to bail before execute() partial-failure loop runs; 0o500 lets read_dir enumerate but blocks remove_file unlink
- [Phase 08-safety-refactors-partial-failure-visibility-cross-platform]: SAFE-02: arboard (default-features = false) replaces sh -c | pbcopy; cfg!(target_os = "macos") dispatches open/xdg-open; App.status_message renders ✓/⚠ in status bar until next keypress (closes #414)
- [Phase 08-safety-refactors-partial-failure-visibility-cross-platform]: SAFE-02: glyph-prefix dispatch (starts_with('⚠') → theme.alert; else → theme.accent) reuses existing theme fields; no theme.warning added. Test tolerates both ✓/⚠ prefixes — no trait ClipboardBackend (D-17/D-19 held)
- [Phase 08-safety-refactors-partial-failure-visibility-cross-platform]: SAFE-03: relocate::plan() now surfaces read_link failures via eprintln warning mirroring PR #448's format verbatim; regression test uses chmod 0o000 + documents Unix platform caveat that is_symlink and read_link share the same parent-search permission (closes #449)
- [Phase 08.1-v0-8-1-hotfix-lockfile-regen-and-save-chain]: HOTFIX-01: introduced offline lockfile::resolved_paths_from_lockfile_cache helper (option-(b) lockfile-as-cache) — destructive commands recover git-skill provenance from previous lockfile + on-disk repo cache without network calls
- [Phase 08.1-v0-8-1-hotfix-lockfile-regen-and-save-chain]: HOTFIX-01: integration test asserts on git_commit_sha (not source_name) — bug wipes provenance via lockfile::generate's None-fallback, source_name comes from manifest and survives unrelated removes; uses real local file:// git repo so sync's normal clone/update flow seeds the lockfile offline
- [Phase 08.1-v0-8-1-hotfix-lockfile-regen-and-save-chain]: Note: HOTFIX-01/02/03 are referenced in plan frontmatter and ROADMAP.md but were never added to REQUIREMENTS.md —  is a no-op for these. Track via the phase ROADMAP/SUMMARY artifacts and #461 instead.

### Roadmap Evolution

- Phase 8.1 inserted after Phase 8 (2026-04-26): v0.8.1 hotfix — lockfile regen + save chain (URGENT). Source: post-merge re-review of PR #460 surfaced 3 correctness/UX findings ([#461](https://github.com/MartinP7r/tome/issues/461)). H1 is a silent-drop regression (git skills omitted from regenerated lockfile in Remove/Reassign/Fork), H2 is the I2/I3 retention guarantee being voided by post-execute save failures, H3 is wording. Worth a patch release before v0.9.

### Pending Todos

- **First:** merge PR #455 + ship v0.7.1 via `make release VERSION=0.7.1`
- **Then:** ship v0.7.2 patch with #456 + #457 (small scope, could bundle with v0.8 Phase 7 or ship separately)
- **Then:** `/gsd:plan-phase 7` to decompose the first v0.8 phase (Wizard UX)
- **Then:** `/gsd:plan-phase 8` for the safety refactors

### Blockers/Concerns

- `make release VERSION=0.7.1` is user-triggered (not gsd automation) — can happen in parallel with v0.8 phase planning
- Cross-machine portability (#458) intentionally punted to v0.9 — users needing it before v0.9 can use the manual workaround in epic #459

## Session Continuity

Last session: 2026-04-26T12:33:29.701Z
Stopped at: Completed 08.1-01-hotfix-01-lockfile-regen-resolved-paths-PLAN.md
Resume file: None
