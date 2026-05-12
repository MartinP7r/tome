---
gsd_state_version: 1.0
milestone: v0.11
milestone_name: TBD (run /gsd:new-milestone to define)
status: between_milestones
stopped_at: v0.10.0 shipped 2026-05-11 + milestone archived 2026-05-12
last_updated: "2026-05-12T22:55:00.000Z"
last_activity: 2026-05-12
progress:
  total_phases: 0
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated after v0.10)

**Core value:** Every AI coding tool on a developer's machine shares the same skill library without manual copying or per-tool configuration.
**Current focus:** Between milestones — v0.10 shipped, v0.11 to be scoped

## Current Position

Milestone: v0.11 (not yet defined — run /gsd:new-milestone)
Phase: —
Plan: —
Status: v0.10 archived; awaiting v0.11 scope
Last activity: 2026-05-12

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
- [Phase 14]: Plan 14-01: Schema lift adds previous_source: Option<DirectoryName> to SkillEntry + LockEntry with serde-default skip_serializing_if; lockfile::generate propagates from manifest. Three Owned->Unowned transition sites (cleanup Case 1, remove::execute dir flavour, apply_edit_decisions Fork branch) capture via .take(). #[allow(dead_code)] retained on SkillEntry::new_unowned (Rule 3 deviation) — production callers land in 14-04/14-05; tracked in phase deferred-items.md.
- [Phase 14-unowned-library-lifecycle]: Plan 14-03: Command::Remove restructured into nested clap subcommand (RemoveKind::Dir | RemoveKind::Skill) per D-API-2; Command::Reassign gains force: bool per D-A1. lib.rs::run preserves Dir flow byte-for-byte; Skill arm stubs to anyhow::bail awaiting 14-05; force shimmed via let _ = force; awaiting 14-04. All 10 tests/cli.rs sites migrated to remove dir <name>. 5 new clap-parse unit tests including BREAKING-rejection of bare tome remove <name>.
- [Phase 14-unowned-library-lifecycle]: Plan 14-07 (doctor unowned section): DoctorReport.unowned_skills field added in parallel to library_issues/directory_issues/config_issues; total_issues() body unchanged (D-D3 contract pinned by test); render_unowned_skills helper uses tabled NAME/LAST-KNOWN SOURCE/SYNCED columns mirroring 14-06 status renderer; manifest read failure degrades to empty Vec to avoid double-reporting (library_issues already surfaces corrupted-manifest errors); JSON shape stable (no skip_serializing_if). 4 new tests + 1 existing literal updated; 31/31 doctor tests pass.
- [Phase 14-unowned-library-lifecycle]: Plan 14-06 (status unowned section): pure formatter format_unowned_section returns Option<String> (None=empty/omit, Some=heading+table) so rendering is unit-testable without stdout capture; manifest read failure in gather() degrades to empty Unowned set (already surfaced via library_count.error); JSON shape stable with explicit empty array for empty unowned set (no skip_serializing_if).
- [Phase 14-unowned-library-lifecycle]: Plan 14-04: reassign::plan accepts Unowned input (D-API-1 stub deleted); D-A1 content-hash collision check (refuses different-content unless --force) + D-A2 target-only role rejection (refuses !is_discovery destinations); execute() clears previous_source on re-anchor (D-C1 closure). Fork's --force now also bypasses D-A1 (single shared semantic). Test fixture local-target switched from role=target to role=synced.
- [Phase 14-unowned-library-lifecycle]: Plan 14-05 (UNOWN-02): RemoveSkillFailureKind kept as separate enum from FailureKind (different failure modes); manifest mutation runs last in in-memory mutation sequence so retry still finds entry on panic; success banner enumerates only steps that actually cleaned something to reduce noise; partial-failure path retains in-memory state and skips all save() calls (I2/I3 retention parity with dir-flavour).
- [Phase 14-unowned-library-lifecycle]: Plan 14-08: D-API-1/-2 vocabulary merge documented across REQUIREMENTS.md/ROADMAP.md/PROJECT.md/CHANGELOG.md with strikethrough+supersession traceability; Phase14Fixture pattern lifts manifest pre-population for Unowned-state tests; 10 phase14_ integration tests anchor UNOWN-01..03 success criteria to the real binary via assert_cmd; 845 tests green
- [Phase 15-cli-hardening]: Plan 15-01: cmd_<name> helpers inlined in lib.rs (per CONTEXT.md Claude's Discretion); commands/ module deferred unless lib.rs grows further. Test split: 16 per-domain cli_*.rs files mirroring cmd_<name> structure; tests/common/mod.rs idiom with module-level #[allow(dead_code)]. Snapshot rename cli__*.snap -> cli_<domain>__*.snap to match insta per-test-crate convention.
- [Phase 15-cli-hardening]: Plan 15-02: config.rs (3122 LOC) split into config/{mod,types,overrides,validate}.rs with Config::save_checked locked to mod.rs (S3 lock for Plan 15-04 grep target); tilde helpers (expand_tilde, unexpand_tilde) live in paths.rs per CONTEXT.md Claude's Discretion; mod.rs re-exports expand_tilde so byte-identical public API preserved
- [Phase 15-cli-hardening]: Plan 15-02: paths::unexpand_tilde added (inverse of expand_tilde via shared dirs::home_dir()); Config::save_checked rewrites under-$HOME paths to ~/-shape on serialise (D-TILDE-1); MachinePrefs::save left untouched per D-TILDE-2 fence (3 verbatim regression tests pin contract); PORT-02 invariant preserved by construction (save_checked operates on \&self, never re-runs apply_machine_overrides)
- [Phase 15-cli-hardening]: Plan 15-03: ScanMode variant names use call-site semantics (Local / ManagedNoProvenance / ManagedWith) rather than the plan's recommended encoding-shape names (Bare / Provenanced / ProvenancedNullable). Per plan author guidance — variant names should reflect what the inner Some(None) actually means, not its old encoding.
- [Phase 15-cli-hardening]: Plan 15-03: HARD-06 scope kept to Lockfile top-level fields only (version, skills); LockEntry pub fields preserved per plan <interfaces> example. Internal get_mut sites in reconcile.rs preserved as direct pub(crate) field access — adding pub fn skills_mut would leak a mutable map handle externally, defeating the v1.0 GUI Tauri IPC goal.
- [Phase 15-cli-hardening]: Plan 15-03: HARD-07 LogLevel inlined in cli.rs per CONTEXT.md Claude's Discretion (it's a CLI-facing enum, not worth a separate log.rs module). Internal helpers continue to take 'verbose: bool' / 'quiet: bool' parameters — the plan only mandates removing the public boolean surface, so the dispatcher converts at the boundary, keeping the refactor contained to cli.rs + ~5 dispatch lines in lib.rs.
- [Phase 15-cli-hardening]: Bundle the cmd_migrate_library site with HARD-04: literal acceptance criteria require lib.rs to have NO process::exit(1); both lint and migrate-library now bubble typed errors through anyhow and main.rs downcasts
- [Phase 15-cli-hardening]: DiagnosticIssue kept as struct with optional kind field instead of converted to enum: backward-compat JSON shape preserved; POLISH-04 ALL-array applies at the kind level (DiagnosticIssueKind::ForeignSymlink)
- [Phase 15-cli-hardening]: Config::save and Config::save_checked promoted to atomic temp+rename via shared atomic_write_toml helper: pre-this-plan they used direct fs::write (not atomic); the plan called this out as fix-first scenario
- [Phase 15-cli-hardening]: Foreign-symlink detection uses 2x2 canonicalize+lexical prefix matrix instead of canonicalize-only: handles macOS symlinks-in-the-middle (/var → /private/var) and missing-leaf staleness without false-foreign-positives
- [Phase 15-cli-hardening]: Hostile-input rejection added in apply_machine_overrides (close to source) using PORT-04 wrapper convention (mention machine.toml). Covers .. traversal, NUL bytes, broken/looping symlinks, and duplicate target paths
- [Phase 15]: Plan 15-05: ratatui TestBackend + insta snapshots cover 13 canonical browse scenes (status dashboard, skill list default/empty/filtered/grouped, detail managed/local/unowned, help overlay, light theme, post-toggle); browse + machine modules widened to pub under feature 'test-support' (production stays pub(crate))
- [Phase 15]: Plan 15-05: HARD-21 D-BROWSE-2 vs D-BROWSE-3 enforced as TWO DISTINCT STRINGS — action-menu LABEL has NO skill name (verb + scope only); StatusMessage BODY DOES include skill name (verb + skill + scope). DetailAction::label takes (&row, &prefs) and returns String; fallback_label() covers prefs-less paths
- [Phase 15]: Plan 15-05: ToggleScope smart-routing (D-BROWSE-1) most-specific-list-wins (per-dir blocklist > per-dir allowlist > global) mirrors MachinePrefs::is_skill_allowed read-path; MACH-04 invariant preserved by construction (per-list mutators only ever touch one of disabled/enabled per directory)
- [Phase 15]: Plan 15-06 (closes Phase 15 / v0.10 beta cut): HARD-14 backup gpg-signing flake fix (per-test-fixture local commit.gpgsign=false + cli_backup GIT_CONFIG_GLOBAL isolation); HARD-15 wizard chrome routed to stderr (eprintln!), only the dry-run TOML body stays on stdout; HARD-16 provenance_from_link_result renamed to warn_if_unreadable_symlink (intent-first naming); HARD-18 cross-fs cleanup recovery hint as Phase 7 D-10 Conflict/Why/Suggestion via cross_fs_recovery_hint pure formatter; HARD-19 reassign PreReassignState read-once snapshot (closes plan/execute drift class — execute() consumes manifest_entry_at_plan rather than re-reading); HARD-20 manifest epoch-0 timestamp warning at Manifest::load via epoch_zero_warning pure formatter. 11 new tests (4 cross-fs hint + 3 reassign snapshot + 4 epoch warning); 955 total tests green.
- [Phase 16-cleanup-message-ux-docs]: Plan 16-01 / UX-01: Three-bucket cleanup output landed. Coordination shape = CleanupResult fields for Buckets A+B + sibling Vec<ExcludedSkill> for Bucket C (chosen over unified CleanupSummary because cross-module ownership). Bucket-distinct phrasing locked (Bucket A 'no longer in any source', Bucket B 'missing from configured source on disk', Bucket C 'now in exclude list'). Forbidden trigger phrase eliminated from cleanup.rs/lib.rs. D-UX01-4 stderr discipline honoured. Per-directory exclusion gap-fix (Rule 2) — cleanup_disabled_from_target now uses is_skill_allowed() so per-dir blocklists/allowlists also tear down stale symlinks.
- [Phase 16]: Plan 16-02 / UX-02: tome migrate-library confirm gate landed. dialoguer::Confirm::default(false) with --yes/-y bypass (Phase 14 D-B3); --no-input without --yes bails with Phase 7 D-10 Conflict/Why/Suggestion. MigrationEntry.byte_size: Option<u64> populated via walkdir+metadata().len() walk (follow_links(false) per D-UX02-4). render_plan rewritten as thin wrapper around render_plan_to(writer); summary line + tabled::Style::rounded() four-column SKILL/SOURCE/SIZE/STATUS table (D-UX02-3). Inline humanize_bytes helper chosen over humansize crate. run_migrate_library deleted; cmd_migrate_library composes plan/render_plan/prompt_confirmation/execute/render_result directly.
- [Phase 16]: Plan 16-03 / DOC-01: docs/src/architecture.md rewritten 60->251 lines for v0.10 library-canonical model. Sync Pipeline reorder lists Reconcile as step 1 (matches lib.rs::sync code). Modules list alphabetised + 4 new entries (marketplace.rs, reconcile.rs, migration_v010.rs, summary.rs). 4 new H2 sections inserted between Key Patterns and Testing (Library-canonical model / Lockfile-authoritative reconciliation / Marketplace adapter trait / Unowned lifecycle). D-API-1/-2 vocab merge honoured (tome adopt / tome forget appear only in supersession footnotes). 4 deviations auto-fixed (Reconcile pipeline step add, Excalidraw caption v0.10 staleness note, AutoInstall variant names corrected to Always/Ask/Never, MarketplaceAdapter trait uses &self per actual code).
- [Phase 16]: Plan 16-04 / DOC-02: CHANGELOG.md `[Unreleased]` rewritten 22->209 lines as v0.10 release notes draft. Migration walkthrough leads, three explicit BREAKING call-outs (library shape conversion, plugin-update propagation gone, `tome remove <name>` -> `tome remove dir <name>`). 22 HARD-cluster issue links + 5 older-bug links + #459 epic link present. Locked wordings honoured: 16-01 bucket names verbatim, 16-02 summary line verbatim, Phase-7-D-10 bail message paraphrased (CHANGELOG-appropriate). Phase 14 D-API-1/-2 supersession honoured (tome adopt / tome forget only in "Replaces the proposed" sentences). Phase 11 D-01 supersession honoured (no auto-on-first-sync). UX-01 trigger phrase absent. Process note: orchestrator inline execution after two prior gsd-executor agents stalled (stream-idle-timeout + watchdog 600s); plan content was verbatim and acceptance checks were rg-based, well-suited to inline.
- [Phase 16]: Plan 16-05 / DOC-03: docs/src/cross-machine-sync.md created (259 lines) with two walkthroughs (Machine A source-of-truth, Machine B fresh-machine bootstrap) + five reference sections (tome.lock semantics, auto_install_plugins consent Always|Ask|Never, directory_overrides, missing-claude error, v0.9 migration). Page wired into mdbook TOC between Configuration and Development Workflow; Command::Sync gains long_about referencing the page so 'tome sync --help' surfaces it; in-prose cross-link added from architecture.md Library-canonical model. Auto-install consent prompt rendering uses actual dialoguer::Select labels (not literal [Y/n/never] shorthand); missing-claude error reproduced verbatim from marketplace.rs. AutoInstall variant names corrected to Always|Ask|Never per actual code (CONTEXT.md DOC-03 D-DOC03-2 reference to Yes|Never|Prompt is wrong).

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

Last session: 2026-05-08T14:28:25.779Z
Stopped at: Completed Plan 16-05 (DOC-03 cross-machine-sync.md)
Resume file: None
