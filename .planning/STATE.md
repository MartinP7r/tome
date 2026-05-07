---
gsd_state_version: 1.0
milestone: v0.10
milestone_name: Library-canonical Model + Cross-Machine Plugin Reconciliation
status: executing
stopped_at: Completed 14-02-skill-summary-type-PLAN.md
last_updated: "2026-05-07T12:49:56.134Z"
last_activity: 2026-05-07
progress:
  total_phases: 7
  completed_phases: 3
  total_plans: 22
  completed_plans: 15
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-05-02)

**Core value:** Every AI coding tool on a developer's machine shares the same skill library without manual copying or per-tool configuration.
**Current focus:** Phase 14 — unowned-library-lifecycle

## Current Position

Phase: 14 (unowned-library-lifecycle) — EXECUTING
Plan: 2 of 8
Status: Ready to execute
Last activity: 2026-05-07

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
- [Phase 11]: Plan 11-05: Re-exported manifest::hash_directory at crate root for integration test reuse — single canonical hashing implementation, no parallel SHA-256 helper. Five end-to-end CLI tests anchor LIB-01/LIB-04/LIB-05 success criteria via assert_cmd binary invocations against a synthetic v0.9 library fixture mirroring CONTEXT.md <specifics>. Resolved deferred-items.md entry for symlink_chain_managed_skill (cli.rs:1775).
- [Phase 12]: Plan 12-01 (Marketplace adapter trait scaffolding): trait + InstalledPlugin + MockMarketplaceAdapter shipped per D-08/D-10. Module placed in strict-alphabetical position (between manifest and migration_v010), not literally between library and lint as the plan said — plan's intent was alphabetical sibling order. #[allow(dead_code)] applied to trait + struct + pre-existing SkillEntry::new_unowned (Rule 3 deviation, documented in deferred-items.md) so cargo clippy --all-targets -- -D warnings exits 0; attrs drop when Plans 12-03/12-04 / Phase 14 add real consumers.
- [Phase 12]: Plan 12-02 (Marketplace failure types + renderer): InstallFailure struct + InstallOp/InstallFailureKind enums + ALL fixed-size [_;4] array + POLISH-04 sentinel + format_install_failures pure formatter + render_install_failures eprint! wrapper, all in marketplace.rs. Renderer split (pure-formatter returning String + thin eprint! wrapper) for testability — replaces lib.rs's inline rendering pattern. #[allow(dead_code)] applied per Rule 3 (drops in Plan 12-04 / Phase 13 when consumers land).
- [Phase 12]: Plan 12-03 (GitAdapter): thin shim over crate::git per D-05; for_directory uses path.to_str().ok_or_else(...)? (mirrors remove.rs:241-244, NOT to_string_lossy); available() trusts local-clone existence per RESEARCH Q #5; #[allow(dead_code)] dropped from MarketplaceAdapter trait (GitAdapter is the first impl) but kept on InstalledPlugin and added to GitAdapter struct/impl block (Rule 3) until Phase 13's D-11 dispatcher lands. 9 unit tests anchor empty-cache and post-install paths of every trait method. D-05a regression contract honored: cargo test -p tome --test cli passes 141 tests byte-for-byte same as baseline; git.rs and tests/cli.rs unchanged.
- [Phase 12]: Plan 12-04 (ClaudeMarketplaceAdapter): D-01 subprocess invocation with stdin = Stdio::null() and verbatim stderr capture; D-02 zero-extra-subprocess available() via cached errors[] substring match; D-04 RefCell<Option<Vec<InstalledPlugin>>> cache auto-invalidates on Ok install/update with public refresh(); D-09 default scope (no --scope flag); twin-constructor pattern (new probes claude --version + new_for_test bypasses for unit tests); pure parser + heuristic classifier as pub(crate) siblings testable without claude on PATH. clippy::if_same_then_else fix collapses two NotFound arms into a single OR with inline mapping comments. ADP-02 satisfied; Phase 12 complete (all 4 ADP requirements wired).
- [Phase 13]: [Phase 13]: Plan 13-02 (marketplace test-support feature gate): MockMarketplaceAdapter + fixture_plugin lifted from #[cfg(test)] pub(super) into pub mod testing gated by cfg(any(test, feature = "test-support")); marketplace module widened from pub(crate) to pub at lib.rs:42; production-symbol scan proves zero leakage; +1 visibility-probe test (41→42 marketplace tests). Per OQ-2 option 2 (feature-gated, not plain pub mod testing) — keeps mock out of v1.0 GUI Tauri IPC surface.
- [Phase 13-lockfile-authoritative-sync]: Plan 13-01: AutoInstall enum + Option<AutoInstall> field on MachinePrefs (D-07) + --no-install CLI flag plumbed through SyncOptions (D-09); schema-only — Plan 13-04 wires consumers
- [Phase 13-lockfile-authoritative-sync]: Plan 13-03 (reconcile module): pub fn reconcile_lockfile + ReconcileClass (4 variants) + ReconcileReport + 7 internal helpers + 25 unit tests in crates/tome/src/reconcile.rs (1620 LOC). Implements RECON-01..05 + Pitfalls 2/4/5 + OQ-3/4. D-22 partial-failure invariant verified by partial-failure test. Plan 13-04 wires the consumer (replaces install.rs reconcile_managed_plugins call site).
- [Phase 13-lockfile-authoritative-sync]: Plan 13-04 (call-site wiring + install.rs deletion): lib.rs::sync invokes reconcile::reconcile_lockfile through ClaudeMarketplaceAdapter (D-11/D-18); legacy install.rs (312 LOC) deleted; D-13 fork in-place flip applied at the manifest call site via apply_edit_decisions; sync exits non-zero via anyhow::bail when reconcile install_failures non-empty (RESEARCH OQ-6); revert decision parked behind a warning (D-16 safety guarantee preserved, dedicated revert path is a Phase 14 follow-up). RECON-01..05 fully wired.
- [Phase 13]: Plan 13-05 (CLI sync reconcile integration tests): 10 end-to-end integration tests in tests/cli_sync_reconcile.rs covering RECON-01..05 non-interactive flow paths via assert_cmd; D-20 verbatim error contract is now CI-asserted; dev-dep self-reference (tome = { path = ".", features = ["test-support"] }) keeps marketplace::testing reachable for future binary-level mock injection. Two plan-spec bugs auto-fixed (Rule 1): role naming (distribution → target/managed) + missing library_dir in fixtures.
- [Phase 14]: SkillSummary lives in dedicated summary.rs; previous_source is Option<String> (display projection); JSON shape stable with explicit null for None

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

Last session: 2026-05-07T12:49:56.129Z
Stopped at: Completed 14-02-skill-summary-type-PLAN.md
Resume file: None
