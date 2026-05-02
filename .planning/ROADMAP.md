# Roadmap: tome

## Milestones

- ✅ **v0.6 Unified Directory Model** — Phases 1-3 (shipped 2026-04-16) — [archive](milestones/v0.6-ROADMAP.md)
- ✅ **v0.7 Wizard Hardening** — Phases 4-6 (shipped 2026-04-22) — [archive](milestones/v0.7-ROADMAP.md)
- ✅ **v0.8 Wizard UX & Safety Hardening** — Phases 7-8 + 8.1 hotfix (shipped 2026-04-27) — [archive](milestones/v0.8-ROADMAP.md)
- ✅ **v0.9 Cross-Machine Config Portability & Polish** — Phases 9-10 (shipped 2026-04-29) — [archive](milestones/v0.9-ROADMAP.md)
- 🚧 **v0.10 Library-canonical Model + Cross-Machine Plugin Reconciliation** — Phases 11-17 (in progress, started 2026-05-02) — closes epic [#459](https://github.com/MartinP7r/tome/issues/459)
- 📋 **v1.0 tome Desktop (Tauri GUI)** — drafted, deferred to after v0.10 ships — see [milestones/v1.0-REQUIREMENTS.md](milestones/v1.0-REQUIREMENTS.md) and [milestones/v1.0-ROADMAP.md](milestones/v1.0-ROADMAP.md)

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

### 🚧 v0.10 Library-canonical Model + Cross-Machine Plugin Reconciliation (In Progress)

**Milestone Goal:** Make tome's library a single source of truth (real directory copies for both managed and local skills), with a lockfile-authoritative `tome sync` flow that reconciles installed plugins to the lockfile state on every machine via marketplace adapters. Closes the library-as-dotfiles workflow gap surfaced in the post-v0.9 codebase review. Closes epic [#459](https://github.com/MartinP7r/tome/issues/459).

**Cuts:** Phase 13 = **alpha** · Phase 15 = **beta** · Phase 16 = **rc** · Phase 17 = **v0.10 final**

**Phase Numbering:** Continues from Phase 10 (v0.9). Phase 11 is the first new phase.

- [ ] **Phase 11: Library-canonical core** — Managed skills become real directory copies; source removal preserves library content; first-sync migration converts symlink libraries (LIB-01..05)
- [ ] **Phase 12: Marketplace adapter** — `MarketplaceAdapter` trait + `ClaudeMarketplaceAdapter` + `GitAdapter`; aggregated install/update failure surfacing (ADP-01..04)
- [ ] **Phase 13: Lockfile-authoritative sync** — `tome sync` reconciles installed plugins to lockfile state; Match/Drift/Vanished classification; auto-install consent; edit-in-library detection (RECON-01..05) — **alpha cut**
- [ ] **Phase 14: Unowned-library lifecycle** — `tome adopt` / `tome forget` commands; `tome status` and `tome doctor` surface the unowned set (UNOWN-01..03)
- [ ] **Phase 15: CLI hardening** — 22 review-followups + older bug backlog: refactors (#485-#487, #491-#493), safety (#488, #494, #495), test coverage (#496-#500), polish (#501-#503), older bugs (#416, #430, #433, #447, #457) (HARD-01..22) — **beta cut**
- [ ] **Phase 16: Cleanup-message UX + docs** — Three-bucket cleanup partition with actionable hints; migration prompt summary table; architecture, changelog, cross-machine docs (UX-01..02, DOC-01..03) — **rc cut**
- [ ] **Phase 17: Migration polish + UAT + release** — In-flight PR landing; issue triage; Linux UAT; real-library migration smoke-test; cargo-dist v0.10.0 release (REL-01..05) — **v0.10 final**

## Phase Details

### Phase 11: Library-canonical core
**Goal**: The library becomes the single source of truth — managed and local skills are stored uniformly as real directories. Source removal no longer deletes content. Existing symlink-based libraries migrate cleanly on first sync.
**Depends on**: Nothing new (foundation for the rest of v0.10; builds on shipped v0.9 manifest/sync infrastructure)
**Requirements**: LIB-01, LIB-02, LIB-03, LIB-04, LIB-05
**Success Criteria** (what must be TRUE):
  1. After `tome sync` completes, the library on disk contains zero symlinks for managed skills — `library_dir/<skill>/` is a real directory copy of source content for every entry, verified via `find <library_dir> -type l | wc -l == 0`.
  2. Removing a `[directories.*]` entry from `tome.toml` and running `tome sync` preserves all library content originally sourced from that directory; an integration test removes a directory entry and asserts every previously-discovered skill remains on disk with content_hash unchanged.
  3. `Manifest` deserialization accepts both old (`source_name: DirectoryName`) and new (`source_name: Option<DirectoryName>`) shapes via `#[serde(default)]`; entries with `source_name: None` are correctly classified as `Unowned`.
  4. On a machine with an existing v0.9-shape library (mix of symlinks + real dirs), `tome sync` detects the symlink entries, prompts with a diff summary listing affected skills and approximate disk delta, and on user consent persists `migration_v010_acknowledged: true` in `machine.toml`. Subsequent syncs are idempotent (no re-prompt, no re-conversion).
  5. The cleanup phase no longer auto-deletes orphan library entries; orphans surface in `tome status` and `tome doctor` output as the new unowned set (Phase 14 wires the surfacing; this phase ensures cleanup leaves them in place).
**Plans**: TBD

### Phase 12: Marketplace adapter
**Goal**: A pluggable `MarketplaceAdapter` trait isolates marketplace-specific install/update logic. v0.10 ships two production adapters (Claude marketplace, git) plus partial-failure aggregation matching the v0.8 SAFE-01 pattern.
**Depends on**: Phase 11 (manifest changes for unowned/managed semantics must be in place before adapter hooks discovery)
**Requirements**: ADP-01, ADP-02, ADP-03, ADP-04
**Success Criteria** (what must be TRUE):
  1. `MarketplaceAdapter` trait exists in `crates/tome/src/marketplace.rs` with `id()`, `current_version()`, `install()`, `update()`, `list_installed()`, `available()` methods, all returning `anyhow::Result`. A `MockMarketplaceAdapter` test double exercises the trait shape in unit tests.
  2. `ClaudeMarketplaceAdapter::install` shells out to `claude plugin install <plugin>@<marketplace>` and returns success on exit-0; `ClaudeMarketplaceAdapter::list_installed` parses `claude plugin list --json` and returns a `Vec<InstalledPlugin>` with id, version, install path. Missing `claude` on PATH produces a clear actionable error message naming the binary.
  3. `GitAdapter` implements `MarketplaceAdapter` by delegating to `crates/tome/src/git.rs::clone_repo` / `update_repo`; behavior for existing git directories is byte-for-byte unchanged from v0.9 (regression-tested via existing git-source integration tests).
  4. When `tome sync` invokes an adapter and one or more `install`/`update` calls fail, failures aggregate into a `Vec<InstallFailure>`, render as a grouped summary (`⚠ N install operations failed`) in stderr, and `tome sync` exits non-zero. Library distribution still completes for skills whose adapter calls succeeded.
**Plans**: TBD

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
**Plans**: TBD

### Phase 14: Unowned-library lifecycle
**Goal**: Two new commands explicitly manage skills whose source has been removed. The unowned set is a first-class concept surfaced in status/doctor.
**Depends on**: Phase 11 (unowned state must exist in manifest before commands can manipulate it)
**Requirements**: UNOWN-01, UNOWN-02, UNOWN-03
**Success Criteria** (what must be TRUE):
  1. `tome adopt <skill> <directory>` re-anchors an unowned skill to a configured directory: manifest `source_name` updates from `None` to `Some(<directory>)`, the skill content is copied into the directory's path on disk, and the skill leaves the unowned set on next discovery. `tome adopt foo nonexistent-dir` fails fast with a clear error naming the missing directory.
  2. `tome forget <skill>` deletes an unowned skill: manifest entry removed, library directory removed, downstream distribution symlinks removed. Interactive confirmation prompt unless `--yes` is passed. `tome forget` on a still-owned skill fails fast with a message directing the user to remove the source directory first.
  3. `tome status` and `tome doctor` text output include an `Unowned skills (N):` section listing each unowned skill with its last-known source name; JSON output of both commands includes an `unowned: [SkillSummary]` array. When the unowned set is empty, the section omits cleanly (no empty header).
**Plans**: TBD

### Phase 15: CLI hardening
**Goal**: Bundle of v0.9-review followups (#485-#503) plus older bug backlog (#416, #430, #433, #447, #457) lands as a single hardening pass. Most touch the same modules as the library-canonical work, so doing them together is more efficient than serializing.
**Depends on**: Phase 13 (sync architecture must be settled before refactoring `lib.rs::run` and `config.rs`; lockfile shape stable before tightening visibility)
**Requirements**: HARD-01, HARD-02, HARD-03, HARD-04, HARD-05, HARD-06, HARD-07, HARD-08, HARD-09, HARD-10, HARD-11, HARD-12, HARD-13, HARD-14, HARD-15, HARD-16, HARD-17, HARD-18, HARD-19, HARD-20, HARD-21, HARD-22
**Success Criteria** (what must be TRUE):
  1. The "architecture" cluster lands cleanly: `skill::parse` returns `anyhow::Result`, `lib.rs::run()` decomposes into per-subcommand `cmd_<name>` helpers (no single match arm exceeds ~30 lines), `config.rs` splits into `config/{mod,types,overrides,validate}.rs`, `process::exit(1)` in lint flow replaced with downcastable `LintFailed` error, `scan_for_skills` adopts a `ScanMode` enum, `Lockfile` fields are `pub(crate)` with mirroring accessors, and `(verbose, quiet)` flags collapse into a `LogLevel` enum. All 22 closed GitHub issues link to the merging PRs.
  2. The "safety + tests" cluster lands cleanly: atomic-save preservation regression test exists for manifest+lockfile+machine.toml; `distribute` refuses to clobber pre-existing symlinks pointing outside the current library; hostile-input tests cover `..` traversal, symlink loops, and same-target overrides for `[directory_overrides]`; `tome remove <git-dir>` and `tome remove <claude-plugins-dir>` end-to-end integration tests pass; `browse/ui.rs` has ratatui `TestBackend` + `insta` snapshot coverage for status dashboard, skill list, detail pane, help overlay; `tests/cli.rs` (was 5580 LOC) splits into per-domain `cli_*.rs` files with shared `common/` helpers; `backup::tests::push_and_pull_roundtrip` flake fixed via local-config-disabled git signing.
  3. The "polish + older bugs" cluster lands cleanly: `wizard.rs` diagnostic prints converted to `eprintln!`; `relocate.rs::provenance_from_link_result` renamed to `warn_if_unreadable_symlink`; `TryFrom<String>` impls for `SkillName`/`DirectoryName`; `tome relocate` cross-fs cleanup recovery hint surfaces; `tome reassign` plan/execute reads filesystem state once (no plan/execute drift); manifest epoch-0 timestamp surfaces a warning instead of silent garbage; browse UI Disable/Enable actions wired up (no `#[allow(dead_code)]`); `Config::save_checked` preserves tilde-shaped paths instead of expanding to absolute (#457 dotfiles regression closed).
  4. CI green on all three platforms (ubuntu-latest, macos-latest) for the entire HARD bundle; clippy-D-warnings clean; test count grows by at least the snapshot + integration additions (target: ≥720 tests at end of Phase 15, was 662 at v0.9.0).
**Plans**: TBD

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
**Plans**: TBD

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
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 11 → 12 → 13 (alpha) → 14 → 15 (beta) → 16 (rc) → 17 (v0.10 final)

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
| 11. Library-canonical core | v0.10 | 0/TBD | Not started | - |
| 12. Marketplace adapter | v0.10 | 0/TBD | Not started | - |
| 13. Lockfile-authoritative sync (alpha) | v0.10 | 0/TBD | Not started | - |
| 14. Unowned-library lifecycle | v0.10 | 0/TBD | Not started | - |
| 15. CLI hardening (beta) | v0.10 | 0/TBD | Not started | - |
| 16. Cleanup-message UX + docs (rc) | v0.10 | 0/TBD | Not started | - |
| 17. Migration polish + UAT + release (v0.10 final) | v0.10 | 0/TBD | Not started | - |
