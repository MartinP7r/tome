# Requirements: tome v0.10 — Library-canonical Model + Cross-Machine Plugin Reconciliation

**Defined:** 2026-05-02
**Source:** [`.planning/research/v0.10-library-canonical-design.md`](research/v0.10-library-canonical-design.md)
**Closes epic:** [#459](https://github.com/MartinP7r/tome/issues/459) (cross-machine library-as-dotfiles)

## v0.10 Requirements

Requirements for the v0.10 release. Grouped by category; each will map to one or more roadmap phases.

### Library-canonical core (LIB)

The library becomes the single source of truth. Managed and local skills are stored uniformly as real directories. Source removal no longer deletes content.

- [ ] **LIB-01**: All library entries are real directories — managed skills are copies of their source content (not symlinks into machine-specific cache paths). `consolidate_managed` writes a copy + records `content_hash`; `consolidate_local` is unchanged.
- [ ] **LIB-02**: `Manifest.managed: bool` semantics shift from "stored as symlink" to "update channel" (managed = upstream sync feeds updates into the library; local = library is canonical). Field name kept; documentation updated.
- [ ] **LIB-03**: `SkillEntry.source_name` becomes `Option<DirectoryName>` (`None` = unowned, source removed but library content preserved). Existing manifest schema accepts both shapes via `#[serde(default)]`.
- [ ] **LIB-04**: Source removal (removing a `[directories.*]` entry from `tome.toml`) preserves library content. Manifest entries enter `Unowned` state. The cleanup phase no longer auto-deletes orphaned skills.
- [ ] **LIB-05**: First-sync migration converts existing symlink-based libraries to real-directory copies. Detection via `dest.is_symlink() && manifest.contains_key(name)`. Prompt with diff summary; persist consent in `machine.toml::migration_v010_acknowledged: true`. Idempotent (no-op on subsequent syncs).

### Marketplace adapter (ADP)

A pluggable adapter trait isolates marketplace-specific install/update logic. v0.10 ships two adapters; future marketplaces extend the trait without core changes.

- [ ] **ADP-01**: `MarketplaceAdapter` trait defined in a new `crates/tome/src/marketplace.rs` module: `id()`, `current_version()`, `install()`, `update()`, `list_installed()`, `available()`. All methods return `anyhow::Result`.
- [ ] **ADP-02**: `ClaudeMarketplaceAdapter` shells out to `claude plugin install <plugin>@<marketplace>`, `claude plugin update <plugin>`, and parses `claude plugin list --json` for current state. Surfaces `claude` not on `PATH` as a clear error message.
- [ ] **ADP-03**: `GitAdapter` wraps existing `crates/tome/src/git.rs::clone_repo` / `update_repo` into the `MarketplaceAdapter` trait shape. Behavior unchanged for existing git directories.
- [ ] **ADP-04**: Adapter `install`/`update` failures aggregate into a `Vec<InstallFailure>` and surface as a grouped failure summary (mirrors v0.8 SAFE-01 `RemoveFailure` pattern). `tome sync` exits non-zero on partial install failure but library distribution still completes.

### Lockfile-authoritative reconciliation (RECON)

`tome sync` reconciles installed plugin state against the lockfile (Cargo.lock-shaped). Drift surfaces interactively; user controls when to apply.

- [ ] **RECON-01**: For each managed skill in the lockfile, classify the actual installation as Match (version + hash agree) / Drift (version differs or older) / Vanished (`available()` returns false). Output a per-class summary on every sync.
- [ ] **RECON-02**: First-time-on-machine prompt: "Auto-install missing plugins on every sync? [Y/n/never]". Persists choice in `machine.toml::auto_install_plugins`. Honors `--no-install` global flag override.
- [ ] **RECON-03**: When auto-install consent is set and drift is detected, render a diff summary ("plugin X: 5.0.5 → 5.0.7"), apply via adapter, re-discover, and verify resulting `content_hash` against lockfile. When auto-install is off, surface drift as warnings without modification.
- [ ] **RECON-04**: Vanished plugins (no longer available from marketplace) emit a clear stderr warning ("plugin X vanished from marketplace Y; using preserved library copy"). Distribution continues from library copy.
- [ ] **RECON-05**: Edit-in-library detection: when `managed: true` and `content_hash(library/<skill>) != lockfile.content_hash`, prompt the user with three choices: fork (default — promote to local via existing `tome fork` semantics), revert (overwrite with marketplace copy), skip (warn and don't touch this entry this sync). In `--no-input` mode, default to skip with a warning.

### Unowned-library lifecycle (UNOWN)

Two new commands explicitly manage skills whose source has been removed.

- [ ] **UNOWN-01**: `tome adopt <skill> <directory>` re-anchors an unowned skill to a configured directory. Updates manifest `source_name` from `None` to `Some(<directory>)` and copies the skill content into the directory's path on disk. Skill leaves the unowned set.
- [ ] **UNOWN-02**: `tome forget <skill>` explicitly deletes an unowned skill from the library. Confirms via interactive prompt unless `--yes`. Removes manifest entry, library directory, and downstream distribution symlinks.
- [ ] **UNOWN-03**: `tome status` and `tome doctor` surface the unowned set: count + per-skill list with last-known source. JSON output includes an `unowned: [SkillSummary]` array.

### Cleanup UX rewrite (UX)

The original trigger of this milestone discussion: the "no longer configured" cleanup message conflates three distinct cases.

- [ ] **UX-01**: `tome sync` cleanup phase partitions stale-candidate skills into three buckets — removed-from-config / missing-from-disk-while-source-still-configured / now-in-`exclude`-list — with per-bucket messaging. Each entry shows actionable resolution hints (re-add directory, restore file from backup, remove from exclude list).
- [ ] **UX-02**: First-sync v0.10 migration prompt (LIB-05) renders a summary table: "62 symlinks → real directories, ~30 MB additional disk". User confirms or aborts before any conversion runs. Aborted migrations leave the library state unchanged.

### CLI hardening (HARD)

Bundle of v0.9-review followups + older bug backlog. Each requirement closes one or more existing GitHub issues; full mapping in Traceability.

- [ ] **HARD-01**: `skill::parse` returns `anyhow::Result` instead of `Result<_, String>`. Closes [#485](https://github.com/MartinP7r/tome/issues/485).
- [ ] **HARD-02**: `lib.rs::run()` decomposed into per-subcommand `cmd_<name>(...)` helpers. Closes [#486](https://github.com/MartinP7r/tome/issues/486).
- [ ] **HARD-03**: `config.rs` split into `config/{mod,types,overrides,validate}.rs`. Closes [#487](https://github.com/MartinP7r/tome/issues/487).
- [ ] **HARD-04**: `process::exit(1)` in `lib.rs::run()` (lint command) replaced with downcastable `LintFailed` error; `main.rs` maps to exit code 1. Closes [#488](https://github.com/MartinP7r/tome/issues/488).
- [ ] **HARD-05**: `scan_for_skills(Option<Option<SkillProvenance>>)` replaced with named `ScanMode` enum. Closes [#491](https://github.com/MartinP7r/tome/issues/491).
- [ ] **HARD-06**: `Lockfile.{skills,version}` tightened to `pub(crate)` with accessors mirroring `Manifest`. Closes [#492](https://github.com/MartinP7r/tome/issues/492).
- [ ] **HARD-07**: `(verbose: bool, quiet: bool)` flags replaced with `LogLevel` enum (`Quiet | Normal | Verbose`). Closes [#493](https://github.com/MartinP7r/tome/issues/493).
- [ ] **HARD-08**: Atomic-save preservation regression test (manifest, lockfile, machine.toml all preserve previous contents on rename failure). Closes [#494](https://github.com/MartinP7r/tome/issues/494).
- [ ] **HARD-09**: `distribute` refuses to clobber pre-existing symlinks pointing outside the current library (foreign tome install / stale relocate protection). Closes [#495](https://github.com/MartinP7r/tome/issues/495).
- [ ] **HARD-10**: Hostile-input tests for `[directory_overrides]` (`..` traversal, symlink loops, two directories overriding to the same path). Closes [#496](https://github.com/MartinP7r/tome/issues/496).
- [ ] **HARD-11**: `tome remove <git-dir>` and `tome remove <claude-plugins-dir>` end-to-end integration tests. Closes [#497](https://github.com/MartinP7r/tome/issues/497).
- [ ] **HARD-12**: `browse/ui.rs` rendering tests via ratatui `TestBackend` + `insta` snapshots (status dashboard, skill list, detail pane, help overlay). Closes [#498](https://github.com/MartinP7r/tome/issues/498).
- [ ] **HARD-13**: `tests/cli.rs` (5580 LOC) split into per-domain integration test files (`cli_sync.rs`, `cli_doctor.rs`, etc.) with shared `common/` helpers. Closes [#499](https://github.com/MartinP7r/tome/issues/499).
- [ ] **HARD-14**: `backup::tests::push_and_pull_roundtrip` and `diff_shows_changes` flake fix — disable git signing in test repos via local config. Closes [#500](https://github.com/MartinP7r/tome/issues/500).
- [ ] **HARD-15**: `wizard.rs` diagnostic `println!` calls converted to `eprintln!` for stdout/stderr discipline. Closes [#501](https://github.com/MartinP7r/tome/issues/501).
- [ ] **HARD-16**: Rename `relocate.rs::provenance_from_link_result` → `warn_if_unreadable_symlink` so the side-effect intent is in the function name. Closes [#502](https://github.com/MartinP7r/tome/issues/502).
- [ ] **HARD-17**: `impl TryFrom<String> for SkillName` and `DirectoryName` to avoid clones at owned-string construction sites. Closes [#503](https://github.com/MartinP7r/tome/issues/503).
- [ ] **HARD-18**: `tome relocate` cross-fs cleanup recovery hint when the orphan-copy preservation logic kicks in. Closes [#416](https://github.com/MartinP7r/tome/issues/416).
- [ ] **HARD-19**: `tome reassign` plan/execute reads filesystem state once (eliminate drift risk between plan and execute). Closes [#430](https://github.com/MartinP7r/tome/issues/430).
- [ ] **HARD-20**: Manifest epoch-0 timestamp fallback fixed — surfaces as warning rather than silent garbage data in future diffs. Closes [#433](https://github.com/MartinP7r/tome/issues/433).
- [ ] **HARD-21**: Browse UI Disable/Enable actions wired up (currently stubbed with `#[allow(dead_code)]`). Closes [#447](https://github.com/MartinP7r/tome/issues/447).
- [ ] **HARD-22**: `Config::save_checked` preserves tilde-shaped paths instead of writing expanded absolute paths (breaks dotfiles sync). Closes [#457](https://github.com/MartinP7r/tome/issues/457).

### Documentation (DOC)

- [ ] **DOC-01**: `docs/src/architecture.md` updated for library-canonical model — managed-as-copy, lockfile-authoritative reconciliation, marketplace adapter trait, unowned lifecycle.
- [ ] **DOC-02**: `CHANGELOG.md` v0.10 release notes explicitly call out the behavior change (plugin updates no longer auto-propagate via symlink — `tome sync` required) and the migration step (first-sync conversion of symlink library to real copies).
- [ ] **DOC-03**: New page `docs/src/cross-machine-sync.md` documents the dotfiles workflow: library committed to git, `tome.lock` authoritative, `machine.toml::auto_install_plugins` consent, expected behavior on new-machine bootstrap.

### Release (REL)

- [ ] **REL-01**: PRs #484 (chore/v0.10-prep doc drift + safety fixes) and #504 (refactor/v0.10-phase-c type lifts) merged before phase planning starts.
- [ ] **REL-02**: Issue triage pass: close already-shipped issues (#392, #365, #396, #459 once retitled, #463); de-dup review-followups vs older refactors (#419/#488, #423/#489, #427/#491, #432/#485, #441/#487, #428+#429/#486).
- [ ] **REL-03**: Linux UAT items from v0.8 carry-over re-evaluated: verify on Linux hardware OR document explicit deferral to v1.0.
- [ ] **REL-04**: Migration smoke-test on real library (Martin's coding-agent-files dotfiles) — verify 62 symlinks convert cleanly, distribution targets re-symlinked, no skill content lost.
- [ ] **REL-05**: cargo-dist release for v0.10.0 (Homebrew + GitHub Releases, signed/notarized for macOS).

## Future Requirements

Deferred to post-v0.10. Surface as candidates for v0.11 / v1.0 / v2.

- **ADP-FUTURE-01**: NPM-backed marketplace adapter (e.g. for skills.sh-style ecosystems).
- **ADP-FUTURE-02**: Generic-URL marketplace adapter (HTTPS pull with checksum verification).
- **LIB-FUTURE-01**: True version pinning per managed skill — blocked on `claude plugin install` upstream supporting `--version` flag.
- **HARD-FUTURE-01**: `[directories.claude-plugins.pins]` override table in `tome.toml` for cross-machine version pins — blocked on LIB-FUTURE-01.
- **WATCH-FUTURE-01**: File watcher for auto-sync when source directories or library change ([#59](https://github.com/MartinP7r/tome/issues/59)) — orthogonal product direction; defer to v1.x.
- **INSTR-FUTURE-01**: Instruction file syncing (CLAUDE.md ↔ AGENTS.md ↔ GEMINI.md, [#194](https://github.com/MartinP7r/tome/issues/194)) — separate concern; potential v0.11 or v0.12 milestone.
- **EDITING-FUTURE-01**: Edit-skill-in-library workflow with markdown editor (today: edits prompt for fork; future: integrated editor with frontmatter form). Aligns with v1.0 GUI-EDIT-01.

## Out of Scope (v0.10)

| Item | Reason |
|------|--------|
| Tauri Desktop GUI | Deferred to v1.0 — drafted in `milestones/v1.0-{REQUIREMENTS,ROADMAP}.md`. v0.10 prepares the stable Rust type surface (`SkillEntry`, `LockEntry`, `SkillOrigin::Unowned`, `SyncPlan`) that v1.0 will consume via Tauri IPC + `specta` bindings. |
| Watch mode / auto-sync on FS change ([#59](https://github.com/MartinP7r/tome/issues/59)) | Orthogonal feature; needs file watcher and event debouncing. v1.x or v2. |
| Plugin marketplace ([#309](https://github.com/MartinP7r/tome/issues/309)) | Different product direction (tome as marketplace vs tome as cross-tool consolidator). v2+. |
| `tome.toml` library-canonical schema migration tool | Out of scope per project policy (Backward compat: None). Migration is documented + auto-detected on first v0.10 sync (LIB-05). |
| Per-marketplace dependency resolution | Plugin A depends on plugin B at version C — too complex for v0.10. Defer to LIB-FUTURE-01 era. |
| Adapter for non-Claude/non-git marketplaces | Trait designed for extensibility, but no second non-trivial adapter implementation in v0.10. Validates trait shape only. |
| Real-time sync between machines | tome is local-machine + dotfiles. Sync happens at `tome sync` time, not continuously. |

## Traceability

(Filled by `gsd-roadmapper` in Step 10 of new-milestone workflow.)

| Requirement | Phase | GitHub Issue | Status |
|-------------|-------|--------------|--------|
| LIB-01..05 | TBD | #459 | Pending |
| ADP-01..04 | TBD | — | Pending |
| RECON-01..05 | TBD | — | Pending |
| UNOWN-01..03 | TBD | — | Pending |
| UX-01..02 | TBD | — | Pending |
| HARD-01..22 | TBD | #485-503, #416, #430, #433, #447, #457 | Pending |
| DOC-01..03 | TBD | — | Pending |
| REL-01..05 | TBD | — | Pending |

**Coverage:**
- v0.10 requirements: 49 total (5 LIB + 4 ADP + 5 RECON + 3 UNOWN + 2 UX + 22 HARD + 3 DOC + 5 REL)
- Mapped to phases: 0 / 49 (pending roadmapper)
- GitHub issues closed: 22 review-followups + ~5 older bugs = ~27 issues

---

*Requirements defined: 2026-05-02 from `.planning/research/v0.10-library-canonical-design.md`. Ready for `gsd-roadmapper` to derive phases.*
