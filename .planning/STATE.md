---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: tome Desktop (Tauri GUI)
status: completed
stopped_at: Phase 25 context gathered
last_updated: "2026-05-25T09:55:59.050Z"
last_activity: 2026-05-23 — Milestone v1.0 ratified, drafts promoted
progress:
  total_phases: 26
  completed_phases: 19
  total_plans: 79
  completed_plans: 79
  percent: 73
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-05-23 with v1.0 Current Milestone section)

**Core value:** Every AI coding tool on a developer's machine shares the same skill library without manual copying or per-tool configuration. v1.0 makes that library *visible* — directories, skills, sync state, and conflicts are observed and managed from a desktop app rather than a terminal.
**Current focus:** Phase 25 — Rust core extraction + Tauri integration spike (not started)

## Current Position

Phase: Not started (defining requirements complete; phase 25 awaits `/gsd:plan-phase 25`)
Plan: —
Status: Defining requirements complete; ratified via `/gsd-new-milestone`
Last activity: 2026-05-23 — Milestone v1.0 ratified, drafts promoted

**v1.0 phase shape (Phases 25–31):**

| Phase | Goal | Reqs | Cut |
|------:|------|------|-----|
| 25 | Rust core extraction + Tauri integration spike | CORE-01..05 (5) | — |
| 26 | Read-only views | VIEW-01..06 (6) + NF-01..03, NF-05 | **alpha** |
| 27 | Sync + triage UI | SYNC-01..05 (5) | — |
| 28 | Configuration UI | CFG-01..05 (5) + NF-04 | **beta** |
| 29 | Mutating operations UI | OPS-01..04 (4) + NF-04 | — |
| 30 | Backup UI | BAK-01..04 (4) + NF-04 | **rc** |
| 31 | Distribution: sign, notarize, auto-update, DMG, first-run UX | DIST-01..05 (5) | **v1.0** |

**Last 6 milestones (recap):**

| Milestone | Phases | Shipped |
|-----------|--------|---------|
| v0.10 Library-canonical Model | 11–17 | 2026-05-11 |
| v0.11 Polish + Observability | 18–19 | 2026-05-14 (+ v0.11.1 2026-05-15) |
| v0.12 Pre-v1.0 Review Polish | — (no-phase bundle) | 2026-05-17 (+ v0.12.1 same day) |
| v0.13 `tome add` UX | — | 2026-05-19 |
| v0.14 Type+role UX + claim-orphan | 20–21 | 2026-05-20 |
| v0.15 Generic managed source directory | 22 | 2026-05-20 |
| v0.16 Doctor diagnostics expansion | 23–24 | 2026-05-20 |

## Accumulated Context

### Decisions

Historical decisions are archived in:

- `.planning/PROJECT.md` — rolling Key Decisions table (v0.6 + v0.7 + v0.8 + v0.9 + v0.10 D-LIB-01..05)
- `.planning/research/v0.10-library-canonical-design.md` — v0.10 design exploration with 9 OQs resolved (rationale + alternatives + risk per question)
- `.planning/milestones/v0.10-ROADMAP.md` — per-phase decisions for v0.10
- `.planning/milestones/v0.9-ROADMAP.md` — per-phase decisions for v0.9
- `.planning/milestones/v0.8-ROADMAP.md` — per-phase decisions for v0.8
- `.planning/milestones/v0.7-ROADMAP.md` — per-phase decisions for v0.7
- `.planning/milestones/v0.6-ROADMAP.md` — per-phase decisions for v0.6
- `.planning/milestones/v1.0-{REQUIREMENTS,ROADMAP}.md` — v1.0 source drafts (ratified into top-level REQUIREMENTS.md + ROADMAP.md on 2026-05-23)
- `.planning/milestones/v0.11-REQUIREMENTS.md` — v0.11 archived requirements (REQUIREMENTS.md overwritten with v1.0 content on ratification)
- [Phase 18..24]: decisions logged historically here; see archived `.planning/STATE.md` history in git for full list

### v1.0 design context (consume during phase planning)

- **No CLI regression.** `crates/tome` ships unchanged via cargo-dist. The CLI must work the same end-to-end after every v1.0 phase. Integration tests in `crates/tome/tests/cli*.rs` are the regression suite.
- **Library-canonical types are the contract.** v0.10's library-canonical model + v0.11–v0.16 polish stabilized `SkillEntry`, `LockEntry`, `RemovePlan`, `StatusReport`, `DirectoryRole`, etc. v1.0 wraps these as Tauri commands; do not change their shape mid-milestone without explicit decision.
- **CORE-01 is foundational.** Decomposing `lib.rs::run` into CLI presenter + structured-type domain calls is Phase 25's central task. The frontend framework spike (D-GUI-04) happens *after* the domain functions exist — picking the framework against fake data would burn the decision.
- **`specta` + `tauri-specta` for bindings** (D-GUI-03). No hand-rolled TS types. CI should fail if generated `bindings.ts` is out of date.
- **macOS only v1.0** (D-GUI-06). Linux deferred to v2. The two pending Linux UAT items from v0.8 stay deferred; they don't block v1.0 ship.
- **#542 absorption.** Owned/Unowned enum migration (deferred from v0.12 review) is folded into Phase 25 CORE-01 work — the structured types the GUI needs are exactly where the enum belongs. Plan must explicitly call this out.
- **Backward compat:** none (per project policy). New `tome-desktop` crate, new `TomeError` enum with stable codes, new event channel — all additive at the CLI layer, no compat shim for old library shape.

### Phase dependency graph (v1.0)

```
25 (Rust core extraction + Tauri integration spike)
 ├── 26 (Read-only views) ── alpha cut
 │    └── 27 (Sync + triage UI)
 │         └── 28 (Configuration UI) ── beta cut
 │              └── 29 (Mutating operations UI)
 │                   └── 30 (Backup UI) ── rc cut
 │                        └── 31 (Distribution) ── v1.0 ship
 └── (NF-01..05 verified at cut boundaries — alpha + beta + rc + final)
```

Phases 26–31 form a strict linear chain; each depends on the previous. NF gates (perf, a11y, HIG, safety, concurrency) are verified at the indicated cuts, not as their own phase.

### Pending Todos / Carry-over

- **Linux UAT (carry-over from v0.8):** 2 pending items in `.planning/phases/08-*/08-HUMAN-UAT.md` (clipboard runtime + xdg-open runtime tests). Pending Linux desktop hardware. Carried over for the sixth+ consecutive milestone — formally deferred to **v2 (post-v1.0)** when Linux GUI build hardware lands.
- **#542 Owned/Unowned enum migration** — deferred from v0.12 whole-codebase review; absorbed into Phase 25 CORE-01 scope.
- **#548 role-transition cleanup gap** — surfaced during v0.13 dogfooding (when a directory's role transitions synced→source, ~171 stale tome symlinks linger). Standalone follow-up; not v1.0-blocking but should land before the alpha cut so dogfooding sessions don't repeat the manual cleanup.
- **Phase 22 deferrals** — two in-flight items: (a) `is_foreign_symlink` managed-source recognition (managed-source paths flag as foreign currently); (b) detect-and-warn for upstream's own distribution fighting tome's. Both have v0.15 dogfooding rationale; absorb into Phase 27 (Sync + triage UI) where foreign-symlink discipline matters most.

### Blockers/Concerns

- **Tauri 2 cross-platform code-signing on CI** (Phase 31) — first time the project will need a Developer ID for non-CLI artifacts. cargo-dist's existing macOS signing flow may need extension or replacement. Plan should enumerate the certificate, notarization, and `tauri-plugin-updater` signing requirements before Phase 31 starts.
- **Frontend framework decision is load-bearing** (D-GUI-04). All UI phases (26–31) depend on it. Phase 25's spike must produce a defensible pick (React / Solid / Svelte) and lock it in writing. Mid-milestone framework swap is not acceptable.
- **`crates/tome-desktop` as a workspace member** — adds Tauri + webview deps to the workspace. Verify cargo-dist's CLI artifact build does not start pulling Tauri deps unintentionally. Workspace-level feature flags or per-crate build matrices may be needed.
- **CLI snapshot tests** — the v0.10–v0.16 hardening pass landed many `insta` snapshots of CLI output. Decomposing `lib.rs::run` into presenter + domain calls must preserve these snapshots byte-for-byte unless the change is explicitly intended.

## Session Continuity

Last session: 2026-05-25T09:55:59.037Z
Stopped at: Phase 25 context gathered
Resume file: .planning/phases/25-rust-core-extraction-tauri-integration-spike/25-CONTEXT.md
