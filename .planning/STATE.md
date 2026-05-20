---
gsd_state_version: 1.0
milestone: v0.16
milestone_name: Doctor diagnostics expansion
status: shipped
stopped_at: v0.16.0 released 2026-05-20 (Phases 23+24 — PR #555 added two `tome doctor` diagnostics: broken-frontmatter cascade (backlog 999.1, Library Warning, no auto-fix) and real-dir-in-target repair (backlog 999.3, auto-fixable when target content matches library byte-for-byte; no-repair Warning when diverging). New `RepairKind::ConsolidateTargetRealDirToSymlink` variant. Single-day double-ship after v0.15.0 same morning. v0.14.0 + v0.13.0 + v0.12.x in the preceding three days. Backlog parking lot now empty of v0.12-dogfooding items (999.1–999.5 all promoted + shipped). Open follow-ups #542 (v1.0 Phase 10 deferred), #548 (role-transition cleanup), plus two new in-flight from Phase 22 deferral: is_foreign_symlink managed-source recognition + detect-and-warn for upstream's own distribution fighting tome's. Next milestone v1.0 Tauri GUI drafted — pending /gsd-new-milestone ratification.
last_updated: "2026-05-20T00:00:00.000Z"
last_activity: 2026-05-20
progress:
  total_phases: 9
  completed_phases: 8
  total_plans: 43
  completed_plans: 43
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated after v0.10; v0.11 milestone now active)

**Core value:** Every AI coding tool on a developer's machine shares the same skill library without manual copying or per-tool configuration.
**Current focus:** Phase 19 — doctor-status-surface-bugfix-bundle

## Current Position

Milestone: v0.11 Polish + Observability — SHIPPED 2026-05-14
Phase: 19 (doctor-status-surface-bugfix-bundle) — COMPLETED 2026-05-13, verified retrospectively 2026-05-15
Plan: 7/7 complete (verified 8/8 must-haves passed)
Status: v0.11.0 released; Cargo.toml = 0.11.0; CHANGELOG.md = [0.11.0] - 2026-05-14
Last activity: 2026-05-15

**v0.11 phase shape (Phases 18–19):**

| Phase | Goal | Reqs | Cut |
|------:|------|------|-----|
| 18 | Observability foundation + sync diagnostics | OBS-01..05 (5) | — |
| 19 | Doctor/status surface + bugfix bundle | OBS-06..07 + FIX-01..06 (8) | **v0.11 final** |

**v0.10 phase shape (Phases 11–17, SHIPPED 2026-05-11):**

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
- `.planning/milestones/v0.10-ROADMAP.md` — per-phase decisions for v0.10
- `.planning/milestones/v0.9-ROADMAP.md` — per-phase decisions for v0.9
- `.planning/milestones/v0.8-ROADMAP.md` — per-phase decisions for v0.8
- `.planning/milestones/v0.7-ROADMAP.md` — per-phase decisions for v0.7
- `.planning/milestones/v0.6-ROADMAP.md` — per-phase decisions for v0.6
- `.planning/milestones/v1.0-{REQUIREMENTS,ROADMAP}.md` — Tauri GUI milestone (drafted, deferred to after v0.11 ships)
- [Phase 18-observability-foundation-sync-diagnostics]: tracing substrate landed via tracing_init::install(LogLevel); TOME_LOG env > LogLevel directive precedence (D-ENV-1); reconcile.rs warnings routed through tracing::warn! as locked proof module (D-SUB-2)
- [Phase 18]: Plan 18-02: 5 step spans + top-level sync span via lexical info_span!(...).entered() blocks; ChangeCause enum with four locked vocabulary strings; OBS-05 reconcile classification line in render_sync_report; DirectoryNowAllowed false-positive accepted, PreviouslyFailed deferred to v0.12 schema bump.
- [Phase 18]: Plan 18-03: OBS-03 span emission pinned by sync_verbose_emits_step_spans_on_stderr regression test asserting on the 3 always-firing step span names + time.busy auto-field; CHANGELOG [Unreleased] documents OBS-01..05, D-ENV-1 trade-off, and PreviouslyFailed + DirectoryNowAllowed deferrals.
- [Phase 19]: Plan 19-01: RepairKind has 3 variants (RemoveStaleManifestEntry, RemoveBrokenLibrarySymlink, RemoveStaleTargetSymlink) — one per real auto-repair handler. Orphan directories stay interactive-only (repair_kind: None).
- [Phase 19]: Plan 19-01: Per-category DiagnosticIssue constructors (library/library_repairable/directory/directory_repairable/directory_foreign_symlink/config) replace untyped/typed shims; D-CAT-1 ForeignSymlink promotion happens at construction time. Dispatcher matches exhaustively on Option<RepairKind> — substring matching anti-pattern eliminated.
- [Phase 19]: Plan 19-01 / FIX-03 (#532): 'managed symlink(s) tracked in git' check, render+Confirm flow, and tracked_managed_symlinks helper deleted wholesale — v0.10's library-canonical model made the check incapable of firing on clean libraries. D-FIX03-2 integration test pins the absence of the warning.
- [Phase 19]: Plan 19-02 / FIX-06 (#533): Makefile `release` recipe now stamps CHANGELOG release date via inline `sed -i ''` between `cargo check` and branch creation; CHANGELOG.md added to the version-bump `git add`; 3 regression tests in `crates/tome/tests/cli_make_release.rs` pin sed substitution + idempotency + silent-noop. Inline shell comments inside `\`-continuation recipe blocks rejected (Make joins lines before shell parsing — `#` would comment out trailing commands); all docs live above the `release:` target.
- [Phase 19]: OBS-07: stamp_last_synced_at() placed at lib.rs:1789, inside the !dry_run guard, immediately before manifest::save — D-LSYNC-3 honored. JSON last_sync emits literal null for fresh manifests (no skip_serializing_if), matching the stable-shape pattern used by unowned: [].
- [Phase 19]: Plan 19-04: Outcome C selected for backup test flake — defensive FLAKE-WATCH comment shipped (50/50 isolated + 10/10 module + 5/5 lib-suite runs all pass locally on M1 macOS); browse-test bound relaxed 600ms→2000ms with arboard-rooted FLAKE-FIX comment (100/100 stability)
- [Phase 19]: FIX-04 (#454) closes administratively: reproduce-first proved tabled[ansi] fix from 0803afb is sufficient; no strip-ansi-escapes dep added; snapshot test render_directory_summary_table pins header/body alignment as regression guard
- [Phase 19]: Plan 19-06 / FIX-05 (#453 + #456): Two regression tests in `crates/tome/tests/cli_init.rs` pin the wizard library-default derivation against `TOME_HOME` env (positive D-FIX05-1: library_dir == `<TOME_HOME>/skills`; negative D-FIX05-2: no fallback to `<HOME>/.tome/skills`). `wizard.rs:637` already derived `<resolved_tome_home>/skills` correctly per RESEARCH; this plan is test-only. Sensitivity verified — replacing :637 with hardcoded `~/.tome/skills` makes both tests fail.

### v0.11 design context (consume during planning)

- **Scope discipline:** observability is "instrument existing output" — not "redesign output." Wizard prompts, TUI browse output, and user-facing summary tables (`tome status`/`list`/`doctor` tables, `tome sync` final summary) stay on direct stdout. Only the *log-like* output (sync progress, cleanup actions, diagnostic warnings) gets routed through `tracing`.
- **`tracing` is the default** structured-logging crate (spans, structured fields, async-aware; aligns with what v1.0 Tauri IPC will need). `log` is the cheaper fallback if adoption cost shows up during Phase 18 planning.
- **HARD-07 substrate:** Phase 15's `LogLevel` enum already exists and wraps `(verbose, quiet)` at the CLI boundary. OBS-02 extends this enum to also produce a `tracing_subscriber::EnvFilter`; no new public flags.
- **OBS-06 + FIX-01 fold together:** the doctor categorization work (Library / Directory / Config / Foreign-symlink) is the same code change that splits "auto-fixable" from "all issues" — one implementation closes [#530](https://github.com/MartinP7r/tome/issues/530) and delivers OBS-06.
- **Phase 19 internal parallelism:** the 5 FIX items (FIX-02..06) are independent of each other and of the OBS-06/07 surface work. They can ship in any order or in parallel waves once Phase 18's logging substrate is in place.
- **Linux UAT carry-over (v0.8):** still pending. Formally deferred to v1.0 (Tauri build forces Linux access). Sixth consecutive milestone without Linux hardware; written rationale in `08-HUMAN-UAT.md` frontmatter.
- **Backward compat:** none. Flag/env-var behavior changes (e.g., `TOME_LOG` semantics, default log level) will be release-noted but not gated on a migration shim.

### Phase dependency graph (v0.11)

```
18 (Observability foundation + sync diagnostics)
 └── 19 (Doctor/status surface + bugfix bundle) ── v0.11 FINAL
```

Phase 19 depends on Phase 18 for the logging substrate (doctor/status warnings route through `tracing` consistently). The FIX bundle inside Phase 19 is internally parallelizable.

### Pending Todos / Carry-over

- **Linux UAT (carry-over from v0.8):** 2 pending items in `.planning/phases/08-*/08-HUMAN-UAT.md` (clipboard runtime + xdg-open runtime tests). Pending Linux desktop hardware. Carried over for the sixth consecutive milestone — formally deferred to **v1.0** where the Tauri build target forces Linux access.
- **Pre-existing flake (RESOLVED via Plan 19-04, Outcome C):** `backup::tests::push_and_pull_roundtrip` — historically intermittent. HARD-14 in Phase 15 addressed signing; Phase 19 Plan 04 / FIX-02 paired this with the browse copy-path flake per D-FLAKE-2 and concluded with Outcome C (flake NOT reproducible locally on M1 macOS; 50/50 isolated + 10/10 module + 5/5 lib-suite runs all clean). Defensive FLAKE-WATCH comment shipped at `backup.rs::push_and_pull_roundtrip` documenting flake history + next-mitigation retry-wrapper pattern for future-phase pickup if it recurs in CI. The browse flake ([#511](https://github.com/MartinP7r/tome/issues/511)) bound was relaxed 600ms→2000ms with the canonical FLAKE-FIX comment (arboard contention root cause).

### Blockers/Concerns

- **`tracing` adoption cost (open question for Phase 18 planning):** the codebase currently has ~50+ `eprintln!`/`println!` call sites in sync/reconcile/consolidate/distribute/cleanup. Phase 18 planning should sample the call sites and decide whether the full migration is one plan or split into a substrate plan + a migration plan. If cost looks high, `log` (simpler, no spans) is the documented fallback.
- **Output discipline boundary:** OBS-01 explicitly excludes wizard prompts, TUI browse output, and summary tables from the migration. Plan 18 must enumerate which modules ARE in scope (sync, reconcile, consolidate, distribute, cleanup, doctor diagnostics) versus which are NOT (wizard, browse, status/list/doctor table renderers, lint frontmatter output) to avoid drift during execution.

## Session Continuity

Last session: 2026-05-13T08:00:00.000Z
Stopped at: Completed Wave 2 (19-03 + 19-04 + 19-05 + 19-06)
Resume file: None
