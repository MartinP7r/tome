# Roadmap: tome

## Milestones

- ✅ **v0.6 Unified Directory Model** — Phases 1-3 (shipped 2026-04-16) — [archive](milestones/v0.6-ROADMAP.md)
- ✅ **v0.7 Wizard Hardening** — Phases 4-6 (shipped 2026-04-22) — [archive](milestones/v0.7-ROADMAP.md)
- ✅ **v0.8 Wizard UX & Safety Hardening** — Phases 7-8 + 8.1 hotfix (shipped 2026-04-27) — [archive](milestones/v0.8-ROADMAP.md)
- ✅ **v0.9 Cross-Machine Config Portability & Polish** — Phases 9-10 (shipped 2026-04-29) — [archive](milestones/v0.9-ROADMAP.md)
- ✅ **v0.10 Library-canonical Model + Cross-Machine Plugin Reconciliation** — Phases 11-17 (shipped 2026-05-11) — closes epic [#459](https://github.com/MartinP7r/tome/issues/459) — [archive](milestones/v0.10-ROADMAP.md)
- ✅ **v0.11 Polish + Observability** — Phases 18-19 (shipped 2026-05-14, v0.11.1 patch 2026-05-15)
- ✅ **v0.12 Pre-v1.0 Review Polish** — whole-codebase audit fix bundle (shipped 2026-05-17 as v0.12.0; trivial version-bump-only patch v0.12.1 same day). Deferred items tracked in [#542](https://github.com/MartinP7r/tome/issues/542) for v1.0 Phase 10 absorption.
- ✅ **v0.13 `tome add` UX** — 3-layer `tome add` improvement (PR #547): GitHub `/tree/<ref>/<subdir>` URL parsing, `--subdir` flag, auto-detect + warn-on-zero hint (shipped 2026-05-19). Open follow-up [#548](https://github.com/MartinP7r/tome/issues/548) tracks the role-transition cleanup gap surfaced during dogfooding.
- ✅ **v0.14 Polish: type+role UX + doctor claim-orphan** — Phases 20-21 (shipped 2026-05-20; promoted from backlog 999.5 + 999.2 on 2026-05-19; PR #550 + PR #551)
- ✅ **v0.15 Generic managed source directory** — Phase 22 (shipped 2026-05-20; promoted from backlog 999.4 on 2026-05-20; PR #553). Allows `[directories.<name>] type = "directory" role = "managed"` so pfw-style package managers are first-class. Two deferred follow-ups: `is_foreign_symlink` managed-source recognition + detect-and-warn for upstream-own-distribution conflict.
- ✅ **v0.16 Doctor diagnostics expansion** — Phases 23-24 (shipped 2026-05-20; promoted from backlog 999.1 + 999.3 on 2026-05-20; PR #555). Phase 23: `tome doctor` surfaces skills with unparsable SKILL.md frontmatter as Library Warnings (no auto-fix). Phase 24: `tome doctor` consolidates target real-dirs into symlinks when content matches the library byte-for-byte; diverging content surfaces as no-repair Warning. New `RepairKind::ConsolidateTargetRealDirToSymlink` variant.
- 🚧 **v1.0 tome Desktop (Tauri GUI)** — Phases 25–31 (ratified 2026-05-23 via `/gsd-new-milestone`; drafted 2026-04-28 in [milestones/v1.0-REQUIREMENTS.md](milestones/v1.0-REQUIREMENTS.md) and [milestones/v1.0-ROADMAP.md](milestones/v1.0-ROADMAP.md)). Tauri 2 desktop GUI over the existing CLI library. 32 requirements across 8 categories (CORE / VIEW / SYNC / CFG / OPS / BAK / DIST / NF). macOS-only for v1.0 (Linux deferred to v2). Cuts: alpha (Phases 25–26) → beta (27–28) → rc (29–30) → v1.0 (31).

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

### ✅ v0.11 Polish + Observability (Shipped 2026-05-14)

**Milestone Goal:** Ship the v0.10-surfaced bug bundle and adopt structured logging across the codebase so `tome sync`/`doctor`/`status` give clearer signal — laying groundwork for the v1.0 GUI's IPC + log-capture needs. Scope discipline: instrument existing output, not redesign it.

**Phase Numbering:** Continues from Phase 17 (v0.10). Phase 18 is the first new phase.

- [x] **Phase 18: Observability foundation + sync diagnostics** — Adopt `tracing` + `tracing-subscriber`; wire `--verbose`/`--quiet`/`TOME_LOG` to subscriber filter; per-pipeline-step spans with elapsed-ms; change-cause attribution in `info!`; reconcile classification breakdown in `tome sync` summary (OBS-01..05) (completed 2026-05-12)
- [x] **Phase 19: Doctor/status surface + bugfix bundle** — Richer `tome doctor` (per-category counts + JSON `category` field; folds in #530 auto-fixable contradiction fix); richer `tome status` (per-directory counts, last-sync timestamp, JSON parity); plus the v0.11 bugfix backlog: #511 browse copy-path timing flake, #532 stale managed-symlinks-in-git check, #454 wizard summary ANSI width, #453+#456 library-default follows `tome_home`, #533 `make release` CHANGELOG date stamp (OBS-06..07 + FIX-01..06) (completed 2026-05-13)

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
- [x] 19-06-wizard-library-default-pinning-test-PLAN.md — Wave 2D: integration test pinning wizard.rs:637 existing TOME_HOME-following behavior — FIX-05
- [x] 19-07-changelog-and-phase-verification-PLAN.md — Wave 3: CHANGELOG [Unreleased] Phase 19 entries + REQUIREMENTS.md Traceability flip Pending→Done + make ci + human checkpoint against ROADMAP success criteria 1-4 (approved 2026-05-15, retrospective — v0.11 shipped 2026-05-14 via PR #534 + #535)

### ✅ v0.14 Polish: type+role UX + doctor claim-orphan (Shipped 2026-05-20)

**Milestone Goal:** Speedrun two backlog items surfaced by post-v0.13 dogfooding. Both are small, well-scoped UX fixes that close real user-friction surfaces. Promoted from backlog (999.5 + 999.2) on 2026-05-19. Shipped via PR #550 + PR #551.

- [x] **Phase 20: Type+role explanation in `tome add`** (promoted from 999.5) — Surface what `type` and `role` mean + their defaults at the moments users are choosing them. Shipped: CLI `--role` flag + `tome add` success message echoing role + docs section in `commands.md` ("Choosing the right role").
- [x] **Phase 21: Doctor repair — claim orphan into manifest** (promoted from 999.2) — `tome doctor` now offers a `claim` option that registers an orphan library directory into the manifest as Unowned. Closes the dead-end where library-canonical orphans with no upstream source had no recovery path.

### ✅ v0.15 Generic managed source directory (Shipped 2026-05-20)

**Milestone Goal:** Make the `Managed` role first-class for any flat-directory package manager (pfw, etc.), not just `claude-plugins`. Shipped via PR #553.

- [x] **Phase 22: Generic managed source directory** (promoted from 999.4) — Relaxed `valid_roles()` for `DirectoryType::Directory` to include `Managed`. Dropped the `validate.rs` reject rule. Discovery + consolidate already keyed on `role() == Managed` end-to-end, so the change was small: validation surface + tests + docs. **Out of scope (deferred to follow-ups):** (a) `is_foreign_symlink` refinement to recognize managed-source paths as legitimate-origin; (b) detect-and-warn for "pfw's own distribution is fighting tome".

### ✅ v0.16 Doctor diagnostics expansion (SHIPPED 2026-05-20)

**Milestone Goal:** Two doctor improvements promoted from backlog. Phase 23 surfaces skills with broken SKILL.md frontmatter (the original 999.1 framing about "discover drops them" was wrong — they pass through; the real gap was doctor didn't surface them post-sync). Phase 24 turns the existing `sync` real-dir-collision warning into an actionable doctor repair. Shipped via PR #555.

- [x] **Phase 23: Loosen frontmatter cascade** (promoted from 999.1) — `tome doctor` now walks every manifest-tracked library skill and parses its SKILL.md. YAML/delimiter errors and missing files surface as Library Warnings (`'<skill>' has unparsable SKILL.md frontmatter: …`). Not auto-repairable — the user must edit the file. The "loosen" framing was based on a misread of `discover.rs`: skills with broken frontmatter ALREADY pass through to library + distribution today (no filter drops them). The actionable bit is the doctor diagnostic.
- [x] **Phase 24: Doctor repair — consolidate target real-dir into symlink** (promoted from 999.3) — Extended `check_distribution_dir` to detect non-symlink entries whose contents are byte-identical to a library skill, and offers an auto-repair (`RepairKind::ConsolidateTargetRealDirToSymlink`) to delete the real dir + replace with a symlink. Diverging-content real dirs surface as a no-repair Warning so the user can reconcile manually.

### 🚧 v1.0 tome Desktop (Tauri GUI) — Phases 25–31 (Ratified 2026-05-23)

**Milestone Goal:** Make the skill library *visible*. Tauri 2 desktop GUI over the existing CLI library; the Rust core is reshaped to return structured types callable from any front-end. CLI continues to ship unchanged from `crates/tome`; new `crates/tome-desktop` workspace member hosts the app. macOS-only for v1.0 (Linux deferred to v2). Cuts: alpha (Phases 25–26) → beta (27–28) → rc (29–30) → v1.0 (31). Full requirements in [`REQUIREMENTS.md`](REQUIREMENTS.md); full per-phase plan in [`milestones/v1.0-ROADMAP.md`](milestones/v1.0-ROADMAP.md).

- [x] **Phase 25: Rust core extraction + Tauri integration spike** (CORE-01..05) — Decompose `lib.rs::run` into CLI presenter + structured-type domain calls; add `crates/tome-desktop` Tauri 2 scaffold; wire `specta` + `tauri-specta` for `bindings.ts` generation; Tauri event channel for progress; `TomeError` enum with stable codes. Frontend framework decided via spike (D-GUI-04). **No production UI in this phase.** (completed 2026-05-27)
- [x] **Phase 26: Read-only views — alpha cut** (VIEW-01..06 + NF-01..03, NF-05) — Status dashboard, virtualised skill list (2000 skills @ 60fps on M1), detail pane + markdown preview, doctor health pane with one-click fixes, file watcher auto-refresh. Keyboard + VoiceOver. **v1.0-alpha cut.** (completed 2026-05-29)
- [x] **Phase 27: Sync + triage UI** (SYNC-01..05) — Per-stage progress, lockfile diff with per-skill triage decisions, previewable `machine.toml` diff, cancellable sync, per-stage failure summary + retry. Highest-UX-risk phase. (completed 2026-06-07)
- [ ] **Phase 28: Configuration UI — beta cut** (CFG-01..05 + NF-04) — First-run wizard (greenfield/brownfield/legacy), directory editor with live validation, add-git-repo form, machine prefs editor with diff preview. All writes route through `Config::save_checked`. **v1.0-beta cut.**
- [ ] **Phase 29: Mutating operations UI** (OPS-01..04 + NF-04) — Remove/reassign/fork/relocate/eject with plan-preview-confirm flows. Partial-failure aggregation (SAFE-01 semantics) with retry-per-item.
- [ ] **Phase 30: Backup UI — rc cut** (BAK-01..04 + NF-04) — Backup history view, snapshot action, diff view, restore flow with automatic post-restore sync. **v1.0-rc cut.**
- [ ] **Phase 31: Distribution — v1.0 ship** (DIST-01..05) — Sign + notarize + DMG (aarch64 + x86_64), `tauri-plugin-updater` auto-update with signed manifest, combined GitHub Actions release workflow (CLI cargo-dist outputs preserved), first-launch UX, embedded CLI with "Show in terminal" affordances. **v1.0 ship.**

**Open questions (Q1–Q7 from `milestones/v1.0-ROADMAP.md`):**

- Q1: Frontend framework choice — **RESOLVED in Phase 25 spike (25-06): React** (D-GUI-04, irreversible from Phase 26). See `.planning/research/v1.0-frontend-framework-decision.md`.
- Q2: Tauri minor-version pinning policy
- Q3: How `tome lint` failures surface (separate view / integrated into doctor / both)
- Q4: Tray-icon presence (default: no, launch on demand)
- Q5: Linux as stretch goal vs strict v2 (default: strict v2)
- Q6: Sparkle vs `tauri-plugin-updater` (default Tauri; revisit in Phase 31)
- Q7: Telemetry / crash reporting (out of scope v1.0; flag for v2)

### Phase 25: Rust core extraction + Tauri integration spike
**Goal**: Reshape `crates/tome` so its domain operations return structured types callable from any front-end, add `crates/tome-desktop` as a sibling Tauri 2 app crate, wire `specta` + `tauri-specta` bindings, add a progress-event channel and a stable `TomeError` boundary, and pick the frontend framework via a 3-way spike. **No production UI ships in this phase** — the spike apps are throwaway except the winner's scaffold.
**Depends on**: v0.16 shipped (current `main`). Builds on the v0.10 library-canonical types (`SkillEntry`, `RemovePlan`, `StatusReport`) and the v0.11 `tracing`/`LogLevel` substrate. Decisions locked in `25-CONTEXT.md` (D-01..D-17) and `milestones/v1.0-REQUIREMENTS.md` (D-GUI-01..09).
**Requirements**: CORE-01, CORE-02, CORE-03, CORE-04, CORE-05
**Success Criteria** (what must be TRUE):
  1. Each top-level CLI command has a corresponding domain function returning a structured Rust type (`StatusReport`, `ListReport`, `LintReport`, `RemovePlan`, etc.). Existing CLI is rewired to call these and format their output; the full `crates/tome/tests/cli*.rs` integration suite still passes (no CLI regression). The `#542` Owned/Unowned migration lands as part of this: `SkillEntry::source_name: Option<DirectoryName>` becomes `ownership: SkillOwnership { Owned { source }, Unowned { last_owner } }` (CONTEXT.md D-08; named `SkillOwnership` to avoid colliding with the existing `discover.rs::SkillProvenance` struct).
  2. `crates/tome-desktop` builds a debug `.app` on macOS via `cargo tauri dev`. The app opens a window displaying a real `StatusReport` from `tome::status::gather` against the user's actual `tome_home`. `crates/tome-desktop` depends on `crates/tome` as a path dep with `features = ["bindings"]` (CONTEXT.md D-05, D-06).
  3. `bindings.ts` is generated by `specta` + `tauri-specta` via a `gen-bindings` bin that shares a `make_builder()` fn with `main.rs` (NOT `build.rs` — it can't see the `#[tauri::command]` fns; CONTEXT.md D-07), committed at `crates/tome-desktop/ui/src/bindings.ts`, and contains TypeScript definitions for every type that crosses the IPC boundary. CI fails if the generated bindings are out of date (`cargo run -p tome-desktop --bin gen-bindings` then `git diff --exit-code -- crates/tome-desktop/ui/src/bindings.ts`). `specta::Type` derives are gated behind the optional `bindings` cargo feature so the CLI binary takes no specta cost (CONTEXT.md D-06, D-07).
  4. Long-running operations emit progress via an injected `ProgressSink` trait (`crates/tome/src/progress.rs`); the domain stays synchronous (no tokio dep in `crates/tome`). A per-op typed `ProgressEvent` enum carries stage/op detail. CLI uses an `IndicatifSink`; a `NullSink` serves tests + `--quiet`; the spike app subscribes to `sync-progress` Tauri events via a `TauriEventSink` and renders a placeholder progress bar. A `&CancellationToken` arg is threaded alongside `sink` (real cancel behavior lands in Phase 27/SYNC-04) (CONTEXT.md D-09..D-12).
  5. Errors crossing into the front-end are classified at the IPC boundary into `TomeError { code: ErrorCode, message: String, context: Vec<String> }` — the domain keeps `anyhow::Result` internally (zero-refactor, no CLI regression). `ErrorCode` is a coarse ~6-variant enum (`Validation`, `NotFound`, `Permission`, `Conflict`, `Git`, `Io`, `Internal`); classification uses typed `DomainErrorKind` sentinels via `downcast_ref` (not message string-matching), unmatched → `Internal`. Front-end can pattern-match on `code` without inspecting the message (CONTEXT.md D-13..D-16).
  6. **Frontend framework decision (D-GUI-04):** the spike builds the same single-page `StatusReport` view in all three candidates (React, Solid, Svelte), scores each 1–5 across four criteria (bindings.ts ergonomics, bundle size + cold-start, dev-loop speed, ecosystem fit for v1.0 reqs), and records the winner + rationale + invalidation conditions in an ADR at `.planning/research/v1.0-frontend-framework-decision.md`. D-GUI-04 in `milestones/v1.0-REQUIREMENTS.md` is updated with the chosen framework; the two losing spikes are deleted (CONTEXT.md D-01..D-04).
**Plans**: 6 plans (Wave 1: 25-01, 25-02 — foundational, parallel; Wave 2: 25-03 — lib.rs decomposition + sink threading; Wave 3: 25-04 — tome-desktop scaffold + bindings + CI; Wave 4: 25-05 — TomeError boundary; Wave 5: 25-06 — framework spike + ADR)
- [x] 25-01-PLAN.md — `SkillOwnership` migration (#542, D-08) + gated `specta::Type` on cross-boundary types + `bindings` cargo feature (CORE-01)
- [x] 25-02-PLAN.md — `progress.rs`: `ProgressSink` trait + typed `ProgressEvent`/`SyncStage` + `CancelToken` (no tokio) + `NullSink`/`RecordingSink` (CORE-04)
- [x] 25-03-PLAN.md — `lib.rs` presenter decomposition + thread `sink`/`cancel` through `sync()` + `IndicatifSink` (CORE-01, CORE-04)
- [x] 25-04-PLAN.md — `crates/tome-desktop` Tauri 2 scaffold + `specta`/`tauri-specta` `bindings.ts` via `gen-bindings` bin + `get_status` + `TauriEventSink` + CI freshness gate + cargo-dist opt-out (CORE-02, CORE-03, CORE-04)
- [x] 25-05-PLAN.md — `TomeError`/`ErrorCode` boundary + `DomainErrorKind` sentinels via anyhow downcast (CORE-05)
- [x] 25-06-PLAN.md — 3-way frontend framework spike (React/Solid/Svelte) + scoring + ADR + D-GUI-04 update (D-GUI-04)

### Phase 26: Read-only views — alpha cut
**Goal**: Ship the read-only half of the GUI: status dashboard, virtualised skill list, detail pane with markdown preview, doctor health view, and on-disk file watcher. After this phase, the desktop app is useful for inspection but does not mutate state. First user-visible UI; built on the React scaffold chosen in Phase 25.
**Depends on**: Phase 25 (Rust core extraction + Tauri integration spike) — structured domain types (`StatusReport`, etc.), `bindings.ts` generation, `ProgressSink`, `TomeError` boundary, and the React `ui/` scaffold all land there.
**Requirements**: VIEW-01, VIEW-02, VIEW-03, VIEW-04, VIEW-05, VIEW-06, NF-01 (perf budget), NF-02 (a11y), NF-03 (HIG), NF-05 (concurrency)
**Success Criteria** (what must be TRUE):
  1. Status dashboard renders all fields from `tome status --json` with role/type badges, last-sync time, and lockfile state. Refreshes within 200 ms of an external `tome sync` from the CLI.
  2. Skill list view handles a 2000-skill library at sustained 60 fps during search-as-you-type on M1 (8 GB) — verified via a synthetic-skills bench in CI (NF-01).
  3. Detail pane renders frontmatter (parsed by existing `lint.rs`), source path, content hash, last sync, managed/local badge, and disabled state. Three actions (open source, copy path, disable on this machine) wired to the same handlers as the existing browse TUI.
  4. Markdown preview renders SKILL.md body with the same Markdown subset as `browse/markdown.rs`. Code blocks, headings, lists, links all render.
  5. Health pane lists all `tome doctor` findings with one-click fix actions. Fix actions go through the same repair handlers used by interactive `tome doctor`.
  6. File watcher reloads UI state when manifest, lockfile, or library content changes externally. No stale UI after CLI sync (VIEW-06).
  7. Every interactive element is keyboard-accessible and has a VoiceOver label (NF-02). Native menu bar shows File / Edit / View / Library / Help (NF-03).
**Cut**: **v1.0-alpha**. Internal release; CLI still required for sync and edit.
**Plans**: 8 plans (Wave 1: 26-01 — StatusReport extension unblocks UI plans; Wave 2: 26-02..26-06 — UI feature plans + Rust watcher in parallel, all depend on 26-01's StatusReport shape; Wave 3: 26-07 + 26-08 — a11y/HIG audit + perf bench depend on all UI plans landing first)
- [x] 26-01-PLAN.md — Status dashboard view + StatusReport lockfile/machine-prefs extension (VIEW-01)
- [x] 26-02-PLAN.md — Shell (3-col NavigationSplitView + tokens) + virtualised skill list + fuzzy search (VIEW-02, NF-01 setup)
- [x] 26-03-PLAN.md — Detail pane + per-skill actions (open/copy/disable) via tome::actions module + Tauri opener+clipboard plugins (VIEW-03)
- [x] 26-04-PLAN.md — Markdown preview component (react-markdown SC#4 subset) + REQUIREMENTS.md VIEW-04 cleanup (VIEW-04)
- [x] 26-05-PLAN.md — Doctor health pane + per-item PreviewPopover fixes + content-aware FindingId enum (VIEW-05)
- [x] 26-06-PLAN.md — Rust-side file watcher (notify 8.2 + debouncer-full 0.7) + 4 typed events + per-hook subscriptions (VIEW-06, NF-05)
- [x] 26-07-PLAN.md — Native macOS menu bar + keyboard-shortcut audit (Pitfall 9) + axe-core/playwright a11y CI gate (NF-02, NF-03)
- [x] 26-08-PLAN.md — Synthetic 2000-skill fixture + Playwright FPS bench + macOS-only perf CI workflow (NF-01)

### Phase 27: Sync + triage UI
**Goal**: Replace `tome sync`'s CLI flow (and `update.rs`'s interactive triage prompt) with a visual flow showing per-stage progress, lockfile diff with per-skill triage decisions, previewable `machine.toml` writes, cancellation, and failure-summary + retry. Highest-UX-risk phase of the v1.0 milestone — first cross-stage *mutating* pipeline rendered in the GUI.
**Depends on**: Phase 26 (file watcher for post-sync reloads, Health-pane patterns for failure surfacing, alpha shell + 3-column layout for the new sidebar section, virtualised list primitives for the triage panel). Built on the Phase 25 substrate: `ProgressSink` / `ProgressEvent` / `SyncStage` / `CancellationToken` (`crates/tome/src/progress.rs`), `TauriEventSink` (`crates/tome-desktop/src/`), `TomeError` boundary, committed `bindings.ts` + CI freshness gate.
**Requirements**: SYNC-01, SYNC-02, SYNC-03, SYNC-04, SYNC-05
**Phase 26 carryovers folded in** (see `.planning/phases/26-read-only-views-alpha-cut/deferred-items.md`): VIEW-02 group-by (Source/Role) section headers + VIEW-02 "Recent" sort via a new `synced_at` field on the manifest's per-skill provenance. Closing these alongside SYNC-02's "sectioned pending changes" surface keeps the section-header abstraction one implementation, not two; the `synced_at` field also feeds SYNC-02's per-skill diff metadata.
**Success Criteria** (what must be TRUE):
  1. "Sync" action runs the full pipeline (discover → consolidate → distribute → cleanup → save) with per-stage progress and a current-directory indicator. The CLI's interactive triage is *not* invoked — the GUI's triage panel replaces it (SYNC-01).
  2. Lockfile diff produces a triage panel listing new / changed / removed skills with diff metadata (source, hash, timestamp). Per-skill default action is "keep"; alternates are "disable on this machine" and (for git-sourced skills) "view source". Bulk actions (e.g. "disable all new from `<directory>`") work (SYNC-02).
  3. Triage decisions render as a `machine.toml` diff before save. User clicks "apply" to write; "cancel" abandons without side-effects. No silent writes (SYNC-03).
  4. Cancel action during sync stops the pipeline at the current stage boundary and leaves library state consistent (no half-written manifest, no partial lockfile). Verified via integration test (SYNC-04).
  5. Failed sync surfaces a per-stage failure summary (matching CLI's `⚠ K operations failed` SAFE-01 semantics) with a retry action that resumes from the failed stage where possible — re-running discover + consolidate is acceptable; rerunning distribute on a partial manifest is not (SYNC-05).
  6. VIEW-02 carryover closure: picking `Group = Source` or `Group = Role` renders `SectionHeader` rows between skill spans (VoiceOver heading landmarks, per-group totals); picking `Sort = Recent` produces a stable ordering keyed on the per-skill `synced_at` timestamp (most-recent first, alphabetical-name tiebreaker). REQUIREMENTS.md VIEW-02 flips from `partial` → `complete`.
**Cut**: none (v1.0-beta cut lands at the end of Phase 28).
**Plans**: 7 plans (Wave 1: 27-01a — Rust domain types (`item: Option<String>` + `synced_at` + sink fold-in); Wave 2: 27-01b — Tauri boundary commands + React skeleton + `bindings.ts` regen + axe scan; Wave 3: 27-02 (triage panel + `SectionHeader` extension), 27-02b (SkillsView VIEW-02 closure), 27-03 (`machine.toml` diff preview + similar crate) — three plans in parallel; Wave 4: 27-04 — cancellation invariant integration test + StageStepper + SyncToast; Wave 5: 27-05 — `SyncOutcome` wrapping struct + partial-failure rendering + retry handlers)
- [x] 27-01a-PLAN.md — Rust domain types: `item: Option<String>` on `ProgressEvent::SyncStageProgress` (D-08), `synced_at: Option<String>` on `DiscoveredSkill` (D-16), sink-side D-09 fold-in, Pitfall 4 ordering test (SYNC-01)
- [x] 27-01b-PLAN.md — Tauri boundary `start_sync` (spawn_blocking) + `cancel_sync` + `MenuAction::JumpSync` (⌘3 re-anchor); React `SyncView` skeleton + `useSync` hook (Pitfall 6 discipline); Sidebar 4th NavItem; `bindings.ts` regen; axe scan (SYNC-01)
- [x] 27-02-PLAN.md — Triage panel with lockfile diff + per-skill actions + bulk actions on NEW only + GridList primitive + reusable `SectionHeader` extension (SYNC-02)
- [x] 27-02b-PLAN.md — SkillsView VIEW-02 closure: Sort=Recent via `synced_at`, group-by Source/Role with SectionHeader, REQUIREMENTS.md flip (VIEW-02 carryover)
- [x] 27-03-PLAN.md — `machine.toml` diff preview + `similar` crate + `PreviewPopover` slot refactor + `MachineTomlDiff` component (SYNC-03)
- [x] 27-04-PLAN.md — Cancellation invariant integration test + `StageStepper` + `SyncToast` hand-rolled (SYNC-04)
- [x] 27-05-PLAN.md — `SyncOutcome` wrapping struct + partial-failure rendering (D-20) + retry-from-stage and retry-failed-items handlers (SYNC-05)

## Backlog

Unsequenced ideas captured for future planning. Promote via `/gsd:review-backlog` when ready.

_(v0.12 dogfooding backlog 999.1-999.5 all promoted and shipped in v0.14-v0.16. Numbering resets — sparse 999.x is fine.)_


### Phase 999.1: Per-project local config (`.tome.toml`) (BACKLOG)

**Goal:** [Captured for future planning] Let projects ship their own committed `tome` config so a `git clone` of a project pulls down a known-good skill manifest without manual `tome init` ceremony. Analogous to `Cargo.toml` for `cargo`, `package.json` for `npm`, or `.nvmrc` for `nvm` — the project declares what it needs, the tool picks it up.

**Why it matters:**
- **Team consistency** — everyone working on the same repo gets the same baseline skill set, with pinned versions.
- **Onboarding** — new contributor runs `tome <some-command>` in the repo, gets the project's expected AI coding skills installed locally without reading docs.
- **Per-project scope** — a Rust project pulls in Rust-focused skills; a TypeScript project pulls in TS-focused skills, without polluting either's neighbor.
- **Reproducibility** — the committed config + lockfile snapshot mean "what skills was this repo built with on date X" is answerable.

**Open design questions** (for future `/gsd:discuss-phase 999.1`):
1. **Filename:** `.tome.toml`? `.tomerc`? `tome.toml` (no dot, Cargo-style)? User flagged both `.tomerc` and `.tome.toml` as candidates.
2. **Registration command + discovery mechanism:** How does a project get its committed tracking file written? Candidate UX (user, 2026-06-24): run a command *inside* the project dir — `cd ~/dev/jibiki/jibiki && tome track` (or reuse `tome add`) — which writes the project-local file in place, then commits with the repo. Candidate verbs: `tome track`, `tome add`, `tome use <path>`. And how is it found on later invocations: walk-up-from-cwd (git-style automatic), explicit command, or hybrid (automatic in the project dir + explicit command for non-cwd projects)? Both the registration verb and the discovery contract are undecided.
3. **Scope of the local config:** Just additional skill sources? Or also overrides for global directory roles, disabled-skill lists, or wholesale tome-home redirection?
4. **Merge semantics vs `~/.tome/tome.toml`:** Project augments global (additive), project overrides global (replacement), or layered (project precedence with explicit opt-out for "ignore global entirely")?
5. **Interaction with the existing per-machine `~/.config/tome/machine.toml`:** Where do per-machine *overrides* sit when a project-local config also exists? Three-layer model (global → project → machine) needs careful precedence rules.
6. **Lockfile handling:** Does the project config get its own `tome.lock` committed alongside? Or does it inherit the global lockfile?
7. **Multi-project on one machine:** Can two projects with conflicting local configs coexist? Single-active-project model vs. simultaneous-projects model.

**Related context:**
- v0.6 unified directory model already supports per-directory `enabled`/`disabled` skill lists in `machine.toml` — that pattern can extend, but is currently per-machine, not per-project.
- v0.9 cross-machine path overrides (`[directory_overrides.<name>]`) addresses a different axis (path portability across machines for the *same* user); per-project config addresses portability across *users* of the same repo.
- The v1.0 Tauri GUI milestone is the natural moment to surface "currently active project" in the UI if discovery is automatic.

**Requirements:** TBD (to be derived during `/gsd:discuss-phase 999.1`)
**Plans:** 0 plans

Plans:
- [ ] TBD (promote with `/gsd:review-backlog` when ready)

### Phase 999.2: Sync idle hero — surface past-run history (BACKLOG)

**Goal:** [Captured for future planning] The Phase-27 Sync idle hero passes its UI contract (last-synced line + "Run sync" CTA + recent-changes disclosure) but feels content-light on a no-history machine. User feedback during 27-UAT (2026-06-07): "the sync screen is a bit empty with no information about past runs." Strengthen the idle state so it conveys signal even before the first sync, and richer signal after.

**Source:** Phase 27 UAT test 3 passed-with-observation.

**Why it matters:**
- The current recent-changes disclosure relies on a prior run to have content. First-time or freshly-reset machines see an empty disclosure and nothing else of substance.
- "Past runs" is a natural mental model — users want to see when they synced, what changed, and whether errors were recovered — without leaving the Sync view.
- The data infrastructure already exists or is one small step away: `synced_at` per-skill (27-01a), `SyncOutcome.partial_failures` (27-05, structurally ready), and `tome.lock` history.

**Open design questions** (for future `/gsd:discuss-phase 999.2`):
1. **What counts as "past run history"** — last N runs with outcome + timestamp + diff summary? Or just last 1 with a "View all" disclosure?
2. **Where does the data live?** A new `~/.tome/sync-history.json` log, or derive from `tome.lock` snapshots + manifest timestamps already on disk?
3. **Persistence policy** — keep all runs, last N, or last 30 days? Rotate vs. never delete?
4. **First-time UX** — what does the hero say with zero history? Surface info about what sync WILL do (e.g. count of configured directories + estimated skill touch)?
5. **Inline diff preview from history** — clicking a past run shows its diff? Or just outcome summary?
6. **Failure recovery affordance** — surface past failed runs prominently with a one-click "retry from where it left off"?

**Related context:**
- Phase 27 27-01a's `DiscoveredSkill.synced_at: Option<String>` is the per-skill provenance field — already feeds SkillsView's Sort=Recent.
- Phase 27 27-05's `SyncOutcome` carries `retry_from` + `partial_failures` — the data model for failed-run replay already exists at the IPC boundary.
- Carry-forward from 27-05-SUMMARY ("`partial_failures` empty until `sync()` inline-surfaces SAFE-01") would benefit from resolution BEFORE this phase ships — past-run failure history is meaningless if the underlying types never populate.

**Requirements:** TBD (to be derived during `/gsd:discuss-phase 999.2`)
**Plans:** 0 plans

Plans:
- [ ] TBD (promote with `/gsd:review-backlog` when ready)

### Phase 999.3: StageStepper dwell time + post-hoc inspection (BACKLOG)

**Goal:** [Captured for future planning] The Phase-27 StageStepper renders per-stage progress correctly per its UI contract, but on small libraries each stage completes in milliseconds and the user only catches the terminal state. The "what is sync doing" feedback the stepper was supposed to provide isn't actually received. Make per-stage progress legible — either by minimum dwell time during the run, or by post-hoc inspection after the run completes.

**Source:** Phase 27 UAT test 4 passed-with-observation (2026-06-07). User feedback: "yes, this shows, though it's disappearing much too fast to have enough meaningful impact."

**Why it matters:**
- The stepper is the primary user-facing artifact of "sync is doing something" — when it fails to convey signal, users feel uncertain whether the operation actually happened.
- The current per-stage `item` field (D-08, shipped in 27-01a) is exactly the per-skill detail that *should* be the most valuable part of the visual feedback, but it flashes through in a single frame on a small library.
- This is a problem of information density vs. operation speed. The faster the underlying sync gets, the more it needs deliberate UX choices to remain comprehensible.

**Open design questions** (for future `/gsd:discuss-phase 999.3`):
1. **Minimum dwell time per stage** — enforce e.g. 200ms-400ms minimum visible duration per stage even when underlying work finishes faster? Animation pacing tradeoff: feels deliberate vs. feels slow on power users' machines.
2. **Post-hoc inspection** — keep the StageStepper visible after the run with stage-by-stage timings + per-skill items processed? Or a dedicated "View run details" disclosure on the SyncSummary?
3. **Item streaming pacing** — when the active stage processes 50 skills in 200ms, do we show every skill name flashing, or just the final count + a "view all" affordance?
4. **Skip the stepper for "trivial" runs** — if the whole sync takes <500ms with no changes, do we skip the in-progress UI entirely and just show "No changes" terminal state?
5. **Per-stage error visibility** — when a stage failed but recovery succeeded, is that captured anywhere the user can see post-run?

**Related context:**
- The `synced_at` per-skill field (27-01a) + `partial_failures` Vec on `SyncOutcome` (27-05) already provide the data model for post-hoc inspection. The UI is the gap.
- Cross-pollinates with backlog 999.2 (Sync idle hero past-run history) — both are about making sync's behavior more legible. They might merge or share a "Run details" view.

**Requirements:** TBD
**Plans:** 0 plans

Plans:
- [ ] TBD (promote with `/gsd:review-backlog` when ready)

### Phase 999.4: GUI hover tooltips for domain terminology (BACKLOG)

**Goal:** Surface short hover (and click-to-pin) tooltips in the desktop GUI that explain domain-specific phrases the user might not immediately recognise — e.g., "local override" (from `machine.toml` `[directory_overrides.<name>]`), "Owned" vs "Unowned" skills, "managed" vs "local" directories, "lockfile diff", and the six pipeline stage names (Reconcile / Discover / Consolidate / Distribute / Cleanup / Save). Optionally toggleable via a View-menu setting ("Show explanations" / "Hide tooltips") for users who prefer minimal chrome once they internalise the vocabulary.

**Requirements:** TBD
**Plans:** 0 plans

Plans:
- [ ] TBD (promote with /gsd:review-backlog when ready)

### Phase 999.5: Selective skill install from a plugin (BACKLOG)

**Goal:** [Captured for future planning] Allow installing only specific skills from a plugin/source rather than the full set. Today tome installs all skills from a managed directory; if a user only wants 2 of 50 skills from a marketplace plugin, the only workarounds are: (a) rename/remove unwanted skills manually after install, or (b) copy and customize as a local fork. Neither is ergonomic. Ideally tome (or the Claude marketplace) would let you check-off which skills to include at add-time or post-install.

**Context:**
- Unknown whether Claude marketplace supports per-skill selection at install time — worth investigating before designing the tome side.
- If marketplace adds this, tome's managed-skill model (symlink from library → plugin dir) may need to track per-skill enable/disable at the plugin level, separate from the machine-prefs `disabled` list.
- The existing per-directory `enabled`/`disabled` lists in `machine.toml` (v0.6) address a similar need for local directories but don't apply to managed (marketplace) plugins.
- Copy-and-customize (`tome fork`) already works for permanent customization; this feature targets transient "I just don't want this one skill from the package" use cases.

**Requirements:** TBD
**Plans:** 0 plans

Plans:
- [ ] TBD (promote with /gsd-review-backlog when ready)

### Phase 999.6: Warn about / auto-exclude repos/ from git (BACKLOG)

**Goal:** [Captured for future planning] Tome clones git-sourced directories into `<tome_home>/repos/<sha256>/`. When `tome_home` is inside a git repo (e.g. `~/dev/coding-agent-files/.tome/`), those clones land as embedded git repositories, triggering git's "adding embedded git repository" warning on every `git add`. Tome should either warn the user during `tome add <url>` when it detects this situation, or offer to write a `.gitignore` entry for `repos/` in the containing repo automatically.

**Context:**
- Observed: `~/dev/coding-agent-files/repos/b88590c4…` (last30days-skill clone) triggered the warning when running `git add` in `coding-agent-files/`.
- Workaround: add `repos/` to `.gitignore` in the containing repo manually.
- The `repos/` directory is tome's internal cache — it should never be committed to a user's dotfiles repo.
- Detection heuristic: during `tome add`, check if `<tome_home>` is inside a git worktree (`git -C tome_home rev-parse --is-inside-work-tree`). If yes, surface a one-time notice and offer to append `repos/` to the containing repo's `.gitignore`.

**Requirements:** TBD
**Plans:** 0 plans

Plans:
- [ ] TBD (promote with /gsd-review-backlog when ready)

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
| 19. Doctor/status surface + bugfix bundle | v0.11 | 7/7 | Complete    | 2026-05-13 |
