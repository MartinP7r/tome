# Roadmap: tome

## Milestones

- ✅ **v0.6 Unified Directory Model** — Phases 1-3 (shipped 2026-04-16) — [archive](milestones/v0.6-ROADMAP.md)
- ✅ **v0.7 Wizard Hardening** — Phases 4-6 (shipped 2026-04-22) — [archive](milestones/v0.7-ROADMAP.md)
- ✅ **v0.8 Wizard UX & Safety Hardening** — Phases 7-8 + 8.1 hotfix (shipped 2026-04-27) — [archive](milestones/v0.8-ROADMAP.md)
- ✅ **v0.9 Cross-Machine Config Portability & Polish** — Phases 9-10 (shipped 2026-04-29) — [archive](milestones/v0.9-ROADMAP.md)
- ✅ **v0.10 Library-canonical Model + Cross-Machine Plugin Reconciliation** — Phases 11-17 (shipped 2026-05-11) — closes epic [#459](https://github.com/MartinP7r/tome/issues/459) — [archive](milestones/v0.10-ROADMAP.md)
- 🚧 **v0.11 Polish + Observability** — Phases 18-19 (in progress, started 2026-05-12)
- 📋 **v1.0 tome Desktop (Tauri GUI)** — drafted, deferred to after v0.11 ships — see [milestones/v1.0-REQUIREMENTS.md](milestones/v1.0-REQUIREMENTS.md) and [milestones/v1.0-ROADMAP.md](milestones/v1.0-ROADMAP.md)

## Phases

<details>
<summary>✅ v0.6 Unified Directory Model (Phases 1-3) — SHIPPED 2026-04-16</summary>

- [x] Phase 1: Unified Directory Foundation (3/5 plans) — config type system, pipeline rewrite, state schema
- [x] Phase 2: Git Sources & Selection (4/4 plans) — git clone/update, per-dir filtering, tome remove
- [x] Phase 3: Import, Reassignment & Browse Polish (2/2 plans) — tome add/reassign/fork, browse TUI polish

**Known gaps:** WIZ-01 through WIZ-05 (wizard rewrite) deferred — closed as "hardened" in v0.7.

</details>

<details>
<summary>✅ v0.7 Wizard Hardening (Phases 4-6) — SHIPPED 2026-04-22</summary>

- [x] Phase 4: Wizard Correctness (3/3 plans) — `Config::validate()` Conflict+Why+Suggestion errors, library↔distribution overlap detection (Cases A/B/C), `Config::save_checked` expand→validate→round-trip→write pipeline (WHARD-01/02/03)
- [x] Phase 5: Wizard Test Coverage (4/4 plans) — `--no-input` plumbing + `assemble_config` helper extraction, pure-helper unit tests, `tome init --dry-run --no-input` integration tests, 12-combo `(DirectoryType, DirectoryRole)` matrix (WHARD-04/05/06)
- [x] Phase 6: Display Polish & Docs (2/2 plans) — wizard summary migrated to `tabled::Table` with `Style::rounded()` + `PriorityMax::right()` truncation, PROJECT.md "Hardened in v0.7" subsection, CHANGELOG WHARD-07/08 entries (WHARD-07/08)

**Closed WIZ-01..05:** v0.6's known wizard gaps are now shipped AND hardened.

</details>

<details>
<summary>✅ v0.8 Wizard UX & Safety Hardening (Phases 7-8 + 8.1) — SHIPPED 2026-04-27</summary>

- [x] Phase 7: Wizard UX — Greenfield / Brownfield / Legacy (4/4 plans) — `tome init` handles new machines, existing configs, and pre-v0.6 cruft without surprises; resolved `tome_home` surfaced up-front and optionally persisted via XDG config (WUX-01/02/03/04/05)
- [x] Phase 8: Safety Refactors — Partial-Failure Visibility & Cross-Platform (3/3 plans) — `tome remove` aggregates partial-cleanup failures with non-zero exit, `tome browse` works on Linux via `xdg-open` + `arboard`, silent `read_link().ok()` drops replaced with stderr warnings (SAFE-01/02/03)
- [x] Phase 8.1: v0.8.1 hotfix — lockfile regen + save chain (3/3 plans) — `resolved_paths_from_lockfile_cache` helper restores git-skill provenance after Remove/Reassign/Fork (H1), `Command::Remove` save chain reordered to surface partial-failure ⚠ block before save errors (H2), failure-summary wording reworded (H3)

**Released as:** v0.8.0 (2026-04-26) + v0.8.1 hotfix (2026-04-27)
**Carry-over:** 2 Linux-runtime UAT items in `08-HUMAN-UAT.md` (clipboard / xdg-open) — accepted as carry-over pending Linux desktop hardware

</details>

<details>
<summary>✅ v0.9 Cross-Machine Config Portability & Polish (Phases 9-10) — SHIPPED 2026-04-29</summary>

- [x] Phase 9: Cross-Machine Path Overrides (3/3 plans) — `[directory_overrides.<name>]` schema in `machine.toml`, override-apply timing in load pipeline, typo warning, distinct `machine.toml` error class, `(override)` annotation in `tome status`/`tome doctor` text+JSON (PORT-01..05)
- [x] Phase 10: Phase 8 Review Tail (3/3 plans) — `StatusMessage` enum redesign, `status_message_from_open_result` helper, "Opening: ..." pre-block UX, `ClipboardOccupied` retry, `FailureKind::ALL` compile-enforcement, `RemoveFailure::new` invariant, `arboard` patch-pin, deferred regen-warnings, banner-absence + retry e2e tests, dead `source_path` removal (POLISH-01..06 + TEST-01..05)

**Released as:** v0.9.0 (2026-04-29). Includes the bare-slug `tome add` improvement (PR #471) bundled in.

</details>

<details>
<summary>✅ v0.10 Library-canonical Model + Cross-Machine Plugin Reconciliation (Phases 11-17) — SHIPPED 2026-05-11</summary>

**Cuts:** Phase 13 = alpha · Phase 15 = beta · Phase 16 = rc · Phase 17 = v0.10 final

- [x] Phase 11: Library-canonical core (5/5 plans, LIB-01..05) — completed 2026-05-03
- [x] Phase 12: Marketplace adapter (4/4 plans, ADP-01..04) — completed 2026-05-05
- [x] Phase 13: Lockfile-authoritative sync (5/5 plans, RECON-01..05) — completed 2026-05-05 — **alpha cut**
- [x] Phase 14: Unowned-library lifecycle (8/8 plans, UNOWN-01..03) — completed 2026-05-07
- [x] Phase 15: CLI hardening (6/6 plans, HARD-01..22) — completed 2026-05-08 — **beta cut**
- [x] Phase 16: Cleanup-message UX + docs (5/5 plans, UX-01..02 + DOC-01..03) — completed 2026-05-08 — **rc cut**
- [x] Phase 17: Migration polish + UAT + release (5/5 plans, REL-01..05) — completed 2026-05-12 — **v0.10 final** (cargo-dist tag 578f787, GitHub Release 11 assets)

Full archive: [milestones/v0.10-ROADMAP.md](milestones/v0.10-ROADMAP.md). Closes epic [#459](https://github.com/MartinP7r/tome/issues/459).

</details>

### 🚧 v0.11 Polish + Observability (In Progress)

**Milestone Goal:** Ship the v0.10-surfaced bug bundle and adopt structured logging across the codebase so `tome sync`/`doctor`/`status` give clearer signal — laying groundwork for the v1.0 GUI's IPC + log-capture needs. Scope discipline: instrument existing output, not redesign it.

**Phase Numbering:** Continues from Phase 17 (v0.10). Phase 18 is the first new phase.

- [x] **Phase 18: Observability foundation + sync diagnostics** — Adopt `tracing` + `tracing-subscriber`; wire `--verbose`/`--quiet`/`TOME_LOG` to subscriber filter; per-pipeline-step spans with elapsed-ms; change-cause attribution in `info!`; reconcile classification breakdown in `tome sync` summary (OBS-01..05) (completed 2026-05-12)
- [ ] **Phase 19: Doctor/status surface + bugfix bundle** — Richer `tome doctor` (per-category counts + JSON `category` field; folds in #530 auto-fixable contradiction fix); richer `tome status` (per-directory counts, last-sync timestamp, JSON parity); plus the v0.11 bugfix backlog: #511 browse copy-path timing flake, #532 stale managed-symlinks-in-git check, #454 wizard summary ANSI width, #453+#456 library-default follows `tome_home`, #533 `make release` CHANGELOG date stamp (OBS-06..07 + FIX-01..06)

## Phase Details

### Phase 11: Library-canonical core
**Goal**: The library becomes the single source of truth — managed and local skills are stored uniformly as real directories. Source removal no longer deletes content. Existing symlink-based libraries migrate cleanly on first sync.
**Depends on**: Nothing new (foundation for the rest of v0.10; builds on shipped v0.9 manifest/sync infrastructure)
**Requirements**: LIB-01, LIB-02, LIB-03, LIB-04, LIB-05
**Success Criteria** (what must be TRUE):
  1. After `tome sync` completes, the library on disk contains zero symlinks for managed skills — `library_dir/<skill>/` is a real directory copy of source content for every entry, verified via `find <library_dir> -type l | wc -l == 0`.
  2. Removing a `[directories.*]` entry from `tome.toml` and running `tome sync` preserves all library content originally sourced from that directory; an integration test removes a directory entry and asserts every previously-discovered skill remains on disk with content_hash unchanged.
  3. `Manifest` deserialization accepts both old (`source_name: DirectoryName`) and new (`source_name: Option<DirectoryName>`) shapes via `#[serde(default)]`; entries with `source_name: None` are correctly classified as `Unowned`.
  4. On a machine with an existing v0.9-shape library (mix of symlinks + real dirs), `tome sync` refuses with a Conflict / Why / Suggestion error pointing at `tome migrate-library`. Running `tome migrate-library` (or `tome migrate-library --dry-run`) detects v0.9-shape entries via `is_symlink() && manifest.contains_key(name) && manifest[name].managed == true` (D-03), converts symlinks to real-dir copies, and exits non-zero on any failure with a SAFE-01 grouped summary. Re-runs are idempotent — successful conversions are skipped, broken-source entries are preserved in place (D-04), and post-migration `tome sync` proceeds normally. **No `machine.toml::migration_v010_acknowledged` flag is persisted** — running the command IS the consent. Decision rationale: CONTEXT.md D-01.
  5. The cleanup phase no longer auto-deletes orphan library entries; orphans surface in `tome status` and `tome doctor` output as the new unowned set (Phase 14 wires the surfacing; this phase ensures cleanup leaves them in place).
**Plans**: 5 plans
- [x] 11-01-PLAN.md — Manifest + lockfile schema lift (`source_name: Option<DirectoryName>`, `new_unowned` constructor; LIB-03)
- [x] 11-02-PLAN.md — `consolidate_managed` rewrite (symlink → real-dir copy; LIB-01, LIB-02)
- [x] 11-03-PLAN.md — Source-removal → Unowned transition (cleanup Case 1/2 split + `tome remove` explicit trigger; LIB-04)
- [x] 11-04-PLAN.md — `tome migrate-library` CLI command + sync v0.9-shape refuse-with-hint (LIB-05)
- [x] 11-05-PLAN.md — Integration tests for migrate-library, sync refuse-with-hint, source-removal preservation (LIB-01/04/05 binary-level anchoring)

### Phase 12: Marketplace adapter
**Goal**: A pluggable `MarketplaceAdapter` trait isolates marketplace-specific install/update logic. v0.10 ships two production adapters (Claude marketplace, git) plus partial-failure aggregation matching the v0.8 SAFE-01 pattern.
**Depends on**: Phase 11 (manifest changes for unowned/managed semantics must be in place before adapter hooks discovery)
**Requirements**: ADP-01, ADP-02, ADP-03, ADP-04
**Success Criteria** (what must be TRUE):
  1. `MarketplaceAdapter` trait exists in `crates/tome/src/marketplace.rs` with `id()`, `current_version()`, `install()`, `update()`, `list_installed()`, `available()` methods, all returning `anyhow::Result`. A `MockMarketplaceAdapter` test double exercises the trait shape in unit tests.
  2. `ClaudeMarketplaceAdapter::install` shells out to `claude plugin install <plugin>@<marketplace>` and returns success on exit-0; `ClaudeMarketplaceAdapter::list_installed` parses `claude plugin list --json` and returns a `Vec<InstalledPlugin>` with id, version, install path. Missing `claude` on PATH produces a clear actionable error message naming the binary.
  3. `GitAdapter` implements `MarketplaceAdapter` by delegating to `crates/tome/src/git.rs::clone_repo` / `update_repo`; behavior for existing git directories is byte-for-byte unchanged from v0.9 (regression-tested via existing git-source integration tests).
  4. When `tome sync` invokes an adapter and one or more `install`/`update` calls fail, failures aggregate into a `Vec<InstallFailure>`, render as a grouped summary (`⚠ N install operations failed`) in stderr, and `tome sync` exits non-zero. Library distribution still completes for skills whose adapter calls succeeded.
**Plans**: 4 plans (Wave 1: 12-01, 12-02 — foundation; Wave 2: 12-03, 12-04 — adapters)
- [x] 12-01-PLAN.md — `MarketplaceAdapter` trait + `InstalledPlugin` + `MockMarketplaceAdapter` + module declaration (ADP-01)
- [x] 12-02-PLAN.md — `InstallFailure` + `InstallOp` + `InstallFailureKind` + ALL/sentinel + `render_install_failures` (ADP-04)
- [x] 12-03-PLAN.md — `GitAdapter` wrap of `crate::git` helpers (ADP-03)
- [x] 12-04-PLAN.md — `ClaudeMarketplaceAdapter` with cache, parser, heuristic classifier, smoke tests (ADP-02)

### Phase 13: Lockfile-authoritative sync
**Goal**: `tome.lock` becomes the authoritative state for what's installed on every machine. `tome sync` reconciles drift via marketplace adapters, surfaces drift interactively, and never silently overwrites user content.
**Depends on**: Phase 12 (adapter trait must exist for sync to call it)
**Requirements**: RECON-01, RECON-02, RECON-03, RECON-04, RECON-05
**Success Criteria** (what must be TRUE):
  1. After `tome sync` runs, every managed skill in `tome.lock` is classified as **Match** (version + content_hash agree), **Drift** (version differs from lockfile or older), or **Vanished** (`adapter.available()` returns false), and a per-class summary appears in stdout (`✓ 12 match · ⚠ 2 drift · ⚠ 1 vanished`).
  2. On a fresh machine with no `auto_install_plugins` setting in `machine.toml`, the first `tome sync` that detects drift prompts "Auto-install missing plugins on every sync? [Y/n/never]" exactly once; subsequent syncs honor the persisted choice; passing `--no-install` overrides the persisted choice for the current invocation.
  3. When auto-install consent is set and drift is detected, `tome sync` renders a per-skill diff (`plugin X: 5.0.5 → 5.0.7`), invokes `adapter.install`/`adapter.update`, re-discovers the skill, and verifies the resulting library `content_hash` matches the freshly-recorded lockfile entry. When auto-install is off, the same drift surfaces as a warning block without filesystem modification.
  4. A skill whose `adapter.available()` returns false produces a stderr warning (`plugin X vanished from marketplace Y; using preserved library copy`) and `tome sync` continues; downstream distribution still symlinks the preserved library copy into target tool directories. An integration test simulates a vanished plugin via a mock adapter and asserts the symlink is created.
  5. When `managed: true` and `content_hash(library/<skill>) != lockfile.content_hash`, `tome sync` prompts the user with three choices (fork / revert / skip); default is fork; in `--no-input` mode, the default is **skip with warning** (never silently overwrites edited content). Each choice is wired to existing semantics: fork uses `tome fork` machinery, revert overwrites from the marketplace copy, skip emits a single-line warning.

**Note (planner traceability flag — D-01 supersedes RECON-01 wording):** The drift basis is **content_hash mismatch**, not version. RECON-01's literal "version differs from lockfile or older" phrasing in `REQUIREMENTS.md` is superseded by Phase 13 D-01 (Phase 11 D-08 inheritance). The version string is display-only in the diff output (`plugin X: 5.0.5 → 5.0.7`); it never causes drift on its own. Cleanup commit to update REQUIREMENTS.md may follow in a future hardening phase.

**Plans**: 5 plans (Wave 1: 13-01, 13-02 — schema + mock lift; Wave 2: 13-03 — reconcile module; Wave 3: 13-04 — sync integration + install.rs deletion; Wave 4: 13-05 — integration tests)
- [x] 13-01-PLAN.md — `AutoInstall` enum + `MachinePrefs.auto_install_plugins` field + `--no-install` CLI flag + `SyncOptions` plumbing (RECON-02 schema)
- [x] 13-02-PLAN.md — Lift `MockMarketplaceAdapter` into feature-gated `pub mod testing` + `[features] test-support = []` in `Cargo.toml` (RECON-01 test surface)
- [x] 13-03-PLAN.md — New `reconcile.rs` module: `ReconcileClass`/`ReconcileReport`/`ReconcileOpts`, classify_lockfile, detect_edited, apply_drift_and_missing, resolve_consent, prompt_consent, handle_edited, format_summary + 25 unit tests (RECON-01..05 core)
- [x] 13-04-PLAN.md — Wire `reconcile::reconcile_lockfile` into `lib.rs::sync` (replaces `reconcile_managed_plugins` at line 978); add `build_claude_adapter` dispatcher; add `apply_edit_decisions` for D-13 fork-in-place flip; delete `crates/tome/src/install.rs` per D-17; bail on partial install failure per OQ-6 (RECON-01..05 wiring)
- [x] 13-05-PLAN.md — Integration tests in `crates/tome/tests/cli_sync_reconcile.rs` exercising `--no-input` flow paths (`--no-install`, D-20 missing-claude error, machine.toml round-trip for `auto_install_plugins`, vanished-distribution preservation proxy) — interactive prompts covered by Plan 13-03 unit tests per RESEARCH Pitfall 6

### Phase 14: Unowned-library lifecycle
**Goal**: Two new commands explicitly manage skills whose source has been removed. The unowned set is a first-class concept surfaced in status/doctor.
**Depends on**: Phase 11 (unowned state must exist in manifest before commands can manipulate it)
**Requirements**: UNOWN-01, UNOWN-02, UNOWN-03
**Success Criteria** (what must be TRUE):
  1. `tome reassign <skill> --to <directory>` re-anchors an Unowned skill (per Phase 14 D-API-1, supersedes the literal `tome adopt` wording in UNOWN-01): manifest `source_name` updates from `None` to `Some(<directory>)`, the skill content is copied into the directory's path on disk, `previous_source` is cleared, and the skill leaves the unowned set on next discovery. `tome reassign foo --to nonexistent-dir` fails fast with a clear error naming the missing directory. `tome reassign foo --to <target-only-dir>` is rejected per D-A2. Different-content collisions at the target are refused without `--force` per D-A1.
  2. `tome remove skill <name>` deletes an Unowned skill (per Phase 14 D-API-2, supersedes the literal `tome forget` wording in UNOWN-02): manifest entry removed, library directory removed, downstream distribution symlinks removed, lockfile entry removed, machine.toml memberships removed. Interactive confirmation prompt unless `--yes` is passed. `tome remove skill <name>` on a still-owned skill fails fast per D-B2 with a message directing the user to `tome remove dir` first.
  3. `tome status` and `tome doctor` text output include an `Unowned skills (N):` section listing each unowned skill with its last-known source name (column LAST-KNOWN SOURCE renders `previous_source` per D-C1, falling back to `source_path` per D-C2); JSON output of both commands includes the new field (`unowned: [SkillSummary]` on `StatusReport`, `unowned_skills: [SkillSummary]` on `DoctorReport`). When the unowned set is empty, the section omits cleanly. Per D-D3, the unowned set does NOT contribute to `DoctorReport::total_issues` and does NOT affect `tome doctor` exit code.
**Plans**: 8 plans
- [x] 14-01-previous-source-schema-PLAN.md — Add `previous_source` field to SkillEntry/LockEntry + capture at all 3 Owned→Unowned transition sites (closes Phase 13 D-13 lossy-fork-in-place gap)
- [x] 14-02-skill-summary-type-PLAN.md — Shared `SkillSummary` type in new `summary.rs` module (consumed by 14-06 status + 14-07 doctor)
- [x] 14-03-cli-restructure-PLAN.md — `Remove { kind: RemoveKind::Dir | Skill }` clap split + `Reassign --force` flag + lib.rs dispatch (BREAKING: `tome remove <name>` → `tome remove dir <name>`)
- [x] 14-04-reassign-unowned-input-PLAN.md — `tome reassign` accepts Unowned input (UNOWN-01 / D-API-1) + D-A1 content-hash collision check + D-A2 target-only role rejection + D-C1 clear-on-re-anchor
- [x] 14-05-remove-skill-PLAN.md — `tome remove skill <name>` plan/render/execute triple + RemoveSkillFailureKind (4 variants, ALL array, compile-time guard) + D-B1 full cleanup (manifest+library+dist+lockfile+machine.toml) + D-B2 owned guard + D-B3 confirmation default-no
- [x] 14-06-status-unowned-section-PLAN.md — `StatusReport.unowned: Vec<SkillSummary>` field + text Unowned-skills section + JSON shape (UNOWN-03 status side)
- [x] 14-07-doctor-unowned-section-PLAN.md — `DoctorReport.unowned_skills` field + parallel informational section + D-D3 (does NOT contribute to total_issues; exit code unaffected)
- [x] 14-08-docs-and-integration-tests-PLAN.md — REQUIREMENTS.md/ROADMAP.md/PROJECT.md vocabulary update for D-API-1/-2 merge + CHANGELOG.md BREAKING callout + 8+ end-to-end integration tests in tests/cli.rs

### Phase 15: CLI hardening
**Goal**: Bundle of v0.9-review followups (#485-#503) plus older bug backlog (#416, #430, #433, #447, #457) lands as a single hardening pass. Most touch the same modules as the library-canonical work, so doing them together is more efficient than serializing.
**Depends on**: Phase 13 (sync architecture must be settled before refactoring `lib.rs::run` and `config.rs`; lockfile shape stable before tightening visibility)
**Requirements**: HARD-01, HARD-02, HARD-03, HARD-04, HARD-05, HARD-06, HARD-07, HARD-08, HARD-09, HARD-10, HARD-11, HARD-12, HARD-13, HARD-14, HARD-15, HARD-16, HARD-17, HARD-18, HARD-19, HARD-20, HARD-21, HARD-22
**Success Criteria** (what must be TRUE):
  1. The "architecture" cluster lands cleanly: `skill::parse` returns `anyhow::Result`, `lib.rs::run()` decomposes into per-subcommand `cmd_<name>` helpers (no single match arm exceeds ~30 lines), `config.rs` splits into `config/{mod,types,overrides,validate}.rs`, `process::exit(1)` in lint flow replaced with downcastable `LintFailed` error, `scan_for_skills` adopts a `ScanMode` enum, `Lockfile` fields are `pub(crate)` with mirroring accessors, and `(verbose, quiet)` flags collapse into a `LogLevel` enum. All 22 closed GitHub issues link to the merging PRs.
  2. The "safety + tests" cluster lands cleanly: atomic-save preservation regression test exists for manifest+lockfile+machine.toml; `distribute` refuses to clobber pre-existing symlinks pointing outside the current library; hostile-input tests cover `..` traversal, symlink loops, and same-target overrides for `[directory_overrides]`; `tome remove <git-dir>` and `tome remove <claude-plugins-dir>` end-to-end integration tests pass; `browse/ui.rs` has ratatui `TestBackend` + `insta` snapshot coverage for status dashboard, skill list, detail pane, help overlay; `tests/cli.rs` (was 5580 LOC) splits into per-domain `cli_*.rs` files with shared `common/` helpers; `backup::tests::push_and_pull_roundtrip` flake fixed via local-config-disabled git signing.
  3. The "polish + older bugs" cluster lands cleanly: `wizard.rs` diagnostic prints converted to `eprintln!`; `relocate.rs::provenance_from_link_result` renamed to `warn_if_unreadable_symlink`; `TryFrom<String>` impls for `SkillName`/`DirectoryName`; `tome relocate` cross-fs cleanup recovery hint surfaces; `tome reassign` plan/execute reads filesystem state once (no plan/execute drift); manifest epoch-0 timestamp surfaces a warning instead of silent garbage; browse UI Disable/Enable actions wired up (no `#[allow(dead_code)]`); `Config::save_checked` preserves tilde-shaped paths instead of expanding to absolute (#457 dotfiles regression closed).
  4. CI green on all three platforms (ubuntu-latest, macos-latest) for the entire HARD bundle; clippy-D-warnings clean; test count grows by at least the snapshot + integration additions (target: ≥720 tests at end of Phase 15, was 662 at v0.9.0).
**Plans**: 6 plans (Wave 1: 15-01, 15-02, 15-03 — independent module surfaces; Wave 2: 15-04, 15-05, 15-06 — depend on Wave 1 landings)
- [x] 15-01-cli-decomposition-PLAN.md — lib.rs::run() decomposition into cmd_<name> helpers + tests/cli.rs split into per-domain files (HARD-02, HARD-13)
- [x] 15-02-config-module-PLAN.md — config.rs split into config/{mod,types,overrides,validate}.rs + paths::unexpand_tilde + tilde-preserving Config::save_checked (HARD-03, HARD-22)
- [x] 15-03-type-system-tightening-PLAN.md — skill::parse anyhow + ScanMode enum + Lockfile pub(crate) + LogLevel enum + TryFrom<String> for SkillName/DirectoryName (HARD-01, HARD-05, HARD-06, HARD-07, HARD-17)
- [x] 15-04-safety-guards-and-integration-tests-PLAN.md — LintFailed error + atomic-save regression + distribute foreign-symlink refuse + directory_overrides hostile-input tests + tome remove dir e2e tests (HARD-04, HARD-08, HARD-09, HARD-10, HARD-11)
- [x] 15-05-browse-ui-PLAN.md — ratatui TestBackend + insta snapshots + DetailAction Disable/Enable wiring per D-BROWSE-1/-2/-3 (HARD-12, HARD-21)
- [x] 15-06-polish-and-older-bugs-PLAN.md — backup test flake fix + wizard eprintln! + relocate rename + cross-fs hint + reassign read-once + manifest epoch-0 warning (HARD-14, HARD-15, HARD-16, HARD-18, HARD-19, HARD-20)

### Phase 16: Cleanup-message UX + docs
**Goal**: Rewrite the cleanup message that originally triggered this milestone discussion into three actionable buckets. Document the library-canonical model + cross-machine workflow + behavior change in user-facing docs.
**Depends on**: Phase 13 (sync semantics must be final), Phase 14 (unowned lifecycle must exist for cleanup messaging to reference it), Phase 15 (config/lib refactors landed so doc anchors are stable)
**Requirements**: UX-01, UX-02, DOC-01, DOC-02, DOC-03
**Success Criteria** (what must be TRUE):
  1. `tome sync` cleanup output partitions stale-candidate skills into three buckets: **removed-from-config** (source dir was removed from `tome.toml`), **missing-from-disk** (source dir still configured but file vanished), **now-in-exclude-list** (skill was disabled). Each bucket renders with a per-bucket header, count, and per-entry actionable hint (`re-add directory <name>`, `restore from backup`, `remove from exclude list`). The original "no longer configured" wording no longer appears.
  2. The first-sync v0.10 migration prompt (LIB-05) renders a summary table before any conversion runs: number of symlinks → real directories, approximate additional disk usage, list of affected skills (truncated if >N). User confirms or aborts; aborted migrations leave the library state byte-for-byte unchanged (verified by integration test).
  3. `docs/src/architecture.md` updated for v0.10: managed-as-copy section explaining the model shift, lockfile-authoritative reconciliation flow diagram, marketplace adapter trait surface, unowned lifecycle overview. Old "library is a consolidated cache" framing removed.
  4. `CHANGELOG.md` v0.10 release notes call out the two behavior changes explicitly: (a) plugin updates no longer auto-propagate via symlink — `tome sync` required to reach Claude Code skills, (b) first-sync converts symlink library to real copies (one-time prompt). Migration step documented at the top of the v0.10 section.
  5. New page `docs/src/cross-machine-sync.md` exists and documents the full library-as-dotfiles workflow: committing the library to git, `tome.lock` semantics on Machine B, `auto_install_plugins` consent flow, expected new-machine bootstrap behavior. Linked from `docs/src/SUMMARY.md` and from the `tome sync --help` long description.
**Plans**: 5 plans (Wave 1: 16-01 + 16-02 — code; Wave 2: 16-03, 16-04, 16-05 — docs)
- [x] 16-01-cleanup-three-bucket-PLAN.md — `cleanup.rs` + `lib.rs::sync` three-bucket partition + per-skill inline hints + stderr discipline (UX-01)
- [x] 16-02-migrate-confirm-summary-PLAN.md — `migration_v010.rs` + `cli.rs` + `lib.rs::cmd_migrate_library` confirm gate + `tabled` summary table + `byte_size` walk + `--yes` flag (UX-02)
- [x] 16-03-architecture-doc-PLAN.md — `docs/src/architecture.md` rewrites + 4 new sections (Library-canonical model, Lockfile-authoritative reconciliation, Marketplace adapter trait, Unowned lifecycle) (DOC-01)
- [x] 16-04-changelog-PLAN.md — `CHANGELOG.md` v0.10 release notes draft with three breaking-change call-outs + migration step paragraph (DOC-02)
- [x] 16-05-cross-machine-doc-PLAN.md — new `docs/src/cross-machine-sync.md` (Machine A/B walkthroughs + reference sections) + `SUMMARY.md` TOC entry + `Command::Sync long_about` reference (DOC-03)

### Phase 17: Migration polish + UAT + release
**Goal**: Ship v0.10. In-flight PRs landed, issue tracker cleaned up, real-library migration smoke-tested, cargo-dist release published.
**Depends on**: Phase 16 (docs must be ready for release notes; rc cut tagged before final)
**Requirements**: REL-01, REL-02, REL-03, REL-04, REL-05
**Success Criteria** (what must be TRUE):
  1. PRs #484 (chore/v0.10-prep doc drift + safety fixes) and #504 (refactor/v0.10-phase-c type lifts) are merged to main; `git log --grep "Closes #484\|Closes #504"` shows the merge commits.
  2. Issue triage pass complete: shipped issues (#392, #365, #396, #459, #463) closed with resolution comments; review-followup ↔ older-refactor duplicates de-duped (#419/#488, #423/#489, #427/#491, #432/#485, #441/#487, #428+#429/#486) with one canonical issue closed and the other linked. Final v0.10 milestone on GitHub has zero open non-deferred issues.
  3. Linux UAT carry-over from v0.8 (clipboard runtime, xdg-open runtime) is either verified on Linux hardware (UAT marked complete in `08-HUMAN-UAT.md`) or explicitly deferred to v1.0 with a written rationale in the v0.10 release notes carry-over section.
  4. Migration smoke-test executed on the user's real `~/dev/coding-agent-files` library: 62 known symlinks convert to real directories cleanly, distribution targets re-symlink to the new library copies, no skill content lost (verified by pre/post content_hash comparison), no `tome doctor` warnings introduced. Smoke-test transcript captured in the phase artifacts.
  5. cargo-dist release published: v0.10.0 tag exists on origin/main, GitHub Release contains macOS (signed/notarized) + Linux artifacts, Homebrew formula updated, `tome --version` on installed binary reports `0.10.0`. CHANGELOG.md `[Unreleased]` section moved under `## [0.10.0]` with the release date.
**Plans**: 5 plans (completed 2026-05-12 — all REL-01..05 shipped; see [milestones/v0.10-ROADMAP.md](milestones/v0.10-ROADMAP.md) for archive)

### Phase 18: Observability foundation + sync diagnostics
**Goal**: Adopt `tracing` + `tracing-subscriber` as the structured-logging substrate, then use it to give `tome sync` clearer signal — per-step spans with elapsed-ms, change-cause attribution, and a reconcile classification breakdown in the final summary. Scope discipline: instrument the *log-like* output (sync progress, cleanup actions, diagnostic warnings); leave wizard prompts, TUI browse output, and user-facing summary tables on direct stdout untouched.
**Depends on**: Nothing new (foundation for v0.11; builds on shipped v0.10 reconcile + sync pipeline). Phase 15 / HARD-07 already collapsed `(verbose, quiet)` into the `LogLevel` enum that this phase wraps into a subscriber configuration.
**Requirements**: OBS-01, OBS-02, OBS-03, OBS-04, OBS-05
**Success Criteria** (what must be TRUE):
  1. `tracing` + `tracing-subscriber` are wired in `main.rs` / `lib.rs::run` as the application logging substrate; internal `eprintln!`/`println!` chatter in sync, reconcile, consolidate, distribute, and cleanup paths is replaced with `tracing::{info,warn,debug}!` calls. Wizard prompts (`dialoguer`), TUI browse output, and user-facing summary tables (`tome status`/`list`/`doctor` tables, `tome sync` final summary block) still emit on direct stdout — `cargo run -- status` and `cargo run -- init --dry-run --no-input` produce byte-identical stdout to v0.10.0 for the table portions.
  2. The default log level is `info`; `--verbose` raises it to `debug` and `--quiet` lowers it to `warn`. `TOME_LOG=tome::sync=debug,tome::reconcile=info` (or any other `EnvFilter`-shaped value) overrides the flag-derived level. The `LogLevel` enum from HARD-07 is the single source of truth that maps to `tracing_subscriber::EnvFilter`; users who only set the flags see the same behavior as v0.10.
  3. `tome sync --verbose` emits one span per pipeline step (`discover`, `reconcile`, `consolidate`, `distribute`, `cleanup`) with an `elapsed_ms` field on span close. The same spans are reachable in `info`-level output via `TOME_LOG=tome::sync=debug`. Spans nest correctly under a top-level `sync` span so a single end-to-end run produces a hierarchical trace.
  4. When `consolidate` or `distribute` re-emits a skill, the log line names the cause at `info!` level — one of `hash changed`, `previously failed`, `newly added`, or `directory now allowed` — with the skill name + directory name as structured fields. A user running `tome sync --verbose` can grep the output for `cause=` to see exactly why each re-emit happened.
  5. The `tome sync` final summary block (printed at `info` level — visible by default) includes a reconcile classification breakdown line: `reconcile: N match · M drift · K vanished · L missing-from-machine` immediately above the existing per-bucket cleanup summary. Counts come from `ReconcileReport` (already populated in Phase 13); no new computation, just surfacing.
**Plans**: 3 plans
- [x] 18-01-tracing-substrate-and-reconcile-proof-PLAN.md — Substrate (Cargo.toml deps, `tracing_init.rs`, `LogLevel::directive`, main.rs wiring) + reconcile.rs proof migration (OBS-01 substrate, OBS-02)
- [x] 18-02-migration-sweep-spans-cause-and-reconcile-line-PLAN.md — Sweep `lib.rs::sync` + `library.rs` + `distribute.rs` + `cleanup.rs`; add `change_cause.rs`; emit OBS-04 events; wrap 5 step spans; relocate OBS-05 reconcile line into `render_sync_report` (OBS-01 sweep, OBS-03, OBS-04, OBS-05)
- [x] 18-03-verification-and-changelog-PLAN.md — Integration test pinning OBS-03 span emission + CHANGELOG.md entry under `[Unreleased]` (OBS-01..OBS-05 verification anchor)

### Phase 19: Doctor/status surface + bugfix bundle
**Goal**: Land the richer `tome doctor` / `tome status` surface and clear the v0.10-surfaced bug backlog in a single pass. `tome doctor` gains issue categorization (closing the #530 "auto-fixable" contradiction in the same change); `tome status` gains per-directory counts and a last-sync timestamp. Five independent bugfixes ship alongside.
**Depends on**: Phase 18 (logging substrate available so doctor/status warnings route through `tracing` consistently; reconcile/categorization vocabulary settled). Phase 19 can start once Phase 18 lands; the bugfix work inside this phase is independent and parallelizable internally.
**Requirements**: OBS-06, OBS-07, FIX-01, FIX-02, FIX-03, FIX-04, FIX-05, FIX-06
**Success Criteria** (what must be TRUE):
  1. `tome doctor` text output groups issues by category — **Library** (orphans, manifest corruption, broken symlinks inside `library_dir`), **Directory** (missing source paths, override targets), **Config** (validation failures), **Foreign-symlink** (D-DIST-2 carry-over) — with per-category counts in the summary line. `tome doctor --json` adds a `category` string field per issue, and the JSON `summary` object exposes per-category counts. The shipped contradiction — "N auto-fixable issues" followed by "(no auto-repair available)" — is gone (closes [#530](https://github.com/MartinP7r/tome/issues/530)); the auto-fixable count surfaces *only* items that actually have an auto-repair path, and the prompt is skipped entirely when that count is zero.
  2. `tome status` text output adds a per-directory skill-count column to the Directories section (existing `(override)` annotation from PORT-05 preserved) and a top-line `Last sync: <RFC-3339 timestamp>` (or `never` if no manifest entries yet); `tome status --json` shape gains the matching fields (`directories[].skill_count`, `last_sync`). When the unowned set is non-empty, both text and JSON continue to surface it per UNOWN-03.
  3. The five bugfix items land cleanly, each with a regression test pinning the fix:
     - [#511](https://github.com/MartinP7r/tome/issues/511) `browse::app::tests::copy_path_retry_helper_returns_within_bound` timing flake fixed via deterministic clock injection (or a relaxed bound with an explicit comment naming the parallel-contention root cause). Test passes 100 consecutive `cargo test -p tome browse::app -- --test-threads=8` runs locally.
     - [#532](https://github.com/MartinP7r/tome/issues/532) `tome doctor` no longer reports `N managed symlink(s) tracked in git` post-v0.10 migration — the check is either removed or rewritten to detect a different real failure mode; an integration test asserts a clean v0.10-shape library produces zero such warnings.
     - [#454](https://github.com/MartinP7r/tome/issues/454) Wizard summary table columns align in interactive TTY mode — ANSI bold escapes are no longer counted as visible width. Fix uses ANSI-aware width measurement (e.g., `strip-ansi-escapes` before `tabled` width calc, or an explicit width hint per cell). Verified by a snapshot test that emits a styled summary and asserts column alignment.
     - [#453](https://github.com/MartinP7r/tome/issues/453) + [#456](https://github.com/MartinP7r/tome/issues/456) Wizard `configure_library` derives the library default from the resolved `tome_home` (i.e., `<tome_home>/skills`) instead of hardcoding `~/.tome/skills`. A user who sets `tome_home` to `~/dev/coding-agent-files/.tome` sees `~/dev/coding-agent-files/.tome/skills` proposed as the library default. Verified by a wizard integration test driving a custom `tome_home` in `--no-input` mode.
     - [#533](https://github.com/MartinP7r/tome/issues/533) `make release VERSION=X.Y.Z` automatically replaces `[Unreleased]` with `[X.Y.Z] - YYYY-MM-DD` in `CHANGELOG.md` during the version-bump PR. Verified by a script-level test (or a documented dry-run) showing the substitution against a fixture changelog.
  4. CI green on all platforms (ubuntu-latest, macos-latest); clippy `-D warnings` clean; test count grows by at least one regression test per FIX item plus the OBS-06/07 JSON/text shape tests (target: ≥1000 tests at v0.11 ship time, was 987 at v0.10.0).
**Plans**: 7 plans

- [x] 19-01-doctor-substrate-categorization-and-repair-PLAN.md — Wave 1A: doctor.rs substrate (IssueCategory + RepairKind enums, 8 emit sites retrofit, dispatcher rewrite, FIX-03 stale check deletion) — OBS-06 + FIX-01 + FIX-03
- [x] 19-02-makefile-release-changelog-stamp-PLAN.md — Wave 1B: Makefile inline sed for CHANGELOG date-stamp + 3 regression tests — FIX-06
- [x] 19-03-status-last-sync-and-per-directory-counts-PLAN.md — Wave 2A: manifest.last_synced_at additive field + sync() stamp + StatusReport.last_sync + SKILLS column in Directories table — OBS-07
- [x] 19-04-flake-bounds-relaxation-PLAN.md — Wave 2B: browse test bound 600ms→2000ms + reproduce-first backup test fix per actual root cause — FIX-02
- [x] 19-05-wizard-ansi-aware-width-PLAN.md — Wave 2C: reproduce-first then either strip-ansi-escapes dep + strip call OR administrative close path; snapshot test ships either way — FIX-04
- [ ] 19-06-wizard-library-default-pinning-test-PLAN.md — Wave 2D: integration test pinning wizard.rs:637 existing TOME_HOME-following behavior — FIX-05
- [ ] 19-07-changelog-and-phase-verification-PLAN.md — Wave 3: CHANGELOG [Unreleased] Phase 19 entries + REQUIREMENTS.md Traceability flip Pending→Done + make ci + human checkpoint against ROADMAP success criteria 1-4


## Progress

**Execution Order:**
Phases execute in numeric order: 11 → 12 → 13 (alpha) → 14 → 15 (beta) → 16 (rc) → 17 (v0.10 final) → 18 → 19 (v0.11 final)

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Unified Directory Foundation | v0.6 | 3/5 | Complete | 2026-04-14 |
| 2. Git Sources & Selection | v0.6 | 4/4 | Complete | 2026-04-15 |
| 3. Import, Reassignment & Browse Polish | v0.6 | 2/2 | Complete | 2026-04-16 |
| 4. Wizard Correctness | v0.7 | 3/3 | Complete | 2026-04-19 |
| 5. Wizard Test Coverage | v0.7 | 4/4 | Complete | 2026-04-20 |
| 6. Display Polish & Docs | v0.7 | 2/2 | Complete | 2026-04-22 |
| 7. Wizard UX (Greenfield / Brownfield / Legacy) | v0.8 | 4/4 | Complete | 2026-04-23 |
| 8. Safety Refactors (Partial-Failure Visibility & Cross-Platform) | v0.8 | 3/3 | Complete | 2026-04-24 |
| 8.1. v0.8.1 hotfix — lockfile regen + save chain | v0.8 | 3/3 | Complete | 2026-04-27 |
| 9. Cross-Machine Path Overrides | v0.9 | 3/3 | Complete | 2026-04-28 |
| 10. Phase 8 Review Tail — Type Design, TUI Polish & Test Coverage | v0.9 | 3/3 | Complete | 2026-04-29 |
| 11. Library-canonical core | v0.10 | 5/5 | Complete    | 2026-05-03 |
| 12. Marketplace adapter | v0.10 | 4/4 | Complete    | 2026-05-05 |
| 13. Lockfile-authoritative sync (alpha) | v0.10 | 5/5 | Complete    | 2026-05-05 |
| 14. Unowned-library lifecycle | v0.10 | 8/8 | Complete    | 2026-05-07 |
| 15. CLI hardening (beta) | v0.10 | 6/6 | Complete    | 2026-05-08 |
| 16. Cleanup-message UX + docs (rc) | v0.10 | 5/5 | Complete    | 2026-05-08 |
| 17. Migration polish + UAT + release (v0.10 final) | v0.10 | 5/5 | Complete    | 2026-05-12 |
| 18. Observability foundation + sync diagnostics | v0.11 | 3/3 | Complete    | 2026-05-12 |
| 19. Doctor/status surface + bugfix bundle | v0.11 | 3/7 | In Progress|  |
