# tome — Unified Directory Model

## What This Is

tome is a CLI tool that manages AI coding agent skills across multiple tools (Claude Code, Codex, Antigravity, Cursor, etc.). It discovers skills from configured directories, consolidates them into a central library, and distributes them to target tools via symlinks. The unified directory model (shipped in v0.6) replaces the old separate source/target config with a single `[directories.*]` map where each entry declares its type and role.

## Core Value

Every AI coding tool on a developer's machine shares the same skill library without manual copying or per-tool configuration. One config, one library, every tool.

## Requirements

### Validated

- ✓ Skill discovery from ClaudePlugins and Directory sources — v0.1
- ✓ Library consolidation (copy local, symlink managed) with content hashing — v0.2
- ✓ Symlink distribution to multiple targets — v0.2
- ✓ Interactive wizard for config setup with auto-discovery — v0.1
- ✓ Per-machine skill/target disable via machine.toml — v0.3.x
- ✓ Lockfile diffing and interactive triage — v0.3.x
- ✓ Auto-install managed plugins from lockfile — v0.5
- ✓ Git-backed backup with remote sync — v0.5
- ✓ Frontmatter parsing and `tome lint` — v0.4.2
- ✓ Interactive TUI browser (`tome browse`) — v0.4.1
- ✓ Config-based tool root detection — v0.5.4
- ✓ `--json` output for list/status/doctor — v0.5.4
- ✓ XDG config for tome_home — v0.5.4
- ✓ Unified `[directories.*]` config replacing `[[sources]]` + `[targets.*]` — v0.6
- ✓ Git sources — clone/pull remote skill repos — v0.6
- ✓ Per-directory skill selection (`enabled`/`disabled` in machine.toml) — v0.6
- ✓ `tome add` — register git skill repos from URL — v0.6
- ✓ `tome remove` — remove directories from config with cleanup — v0.6
- ✓ `tome reassign` / `tome fork` — change skill provenance — v0.6
- ✓ Browse TUI polish (theming, scrollbar, fuzzy highlighting, markdown preview) — v0.6

### Validated in v0.8

- ✓ **WUX-01** Wizard prompts for `tome_home` on greenfield — Phase 7 (2026-04-23)
- ✓ **WUX-02** Wizard detects existing `.tome/` config (brownfield: use / edit / reinitialize) — Phase 7 (2026-04-23)
- ✓ **WUX-03** Wizard detects legacy `~/.config/tome/config.toml` pre-v0.6 file and offers cleanup — Phase 7 (2026-04-23)
- ✓ **WUX-04** Wizard prints resolved `tome_home` up-front as info line — Phase 7 (2026-04-23)
- ✓ **WUX-05** Wizard offers to persist custom `tome_home` via XDG config write — Phase 7 (2026-04-23)
- ✓ **SAFE-01** `remove::execute` aggregates partial-cleanup failures and surfaces them (#413) — Phase 8 (2026-04-24)
- ✓ **SAFE-02** Browse UI `open` and `copy path` actions work on Linux (#414) — Phase 8 (2026-04-24); Linux runtime behavior flagged in `08-HUMAN-UAT.md` for hands-on verification (carry-over)
- ✓ **SAFE-03** `relocate.rs` surfaces `fs::read_link` errors instead of silently dropping (#449) — Phase 8 (2026-04-24)
- ✓ **HOTFIX-01/02/03** v0.8.1 hotfix — lockfile regen + save chain reorder + wording (#461) — Phase 8.1 (2026-04-27)

### Validated in v0.9

- ✓ **PORT-01** `[directory_overrides.<name>]` schema in `machine.toml` for per-machine path remapping (#458) — Phase 9 (2026-04-28)
- ✓ **PORT-02** Override application at config load (after tilde expansion, before validate) — Phase 9 (2026-04-28)
- ✓ **PORT-03** Typo-target stderr warning for unknown override directory names — Phase 9 (2026-04-28)
- ✓ **PORT-04** Distinct error wrapper naming `machine.toml` on override-induced validation failures — Phase 9 (2026-04-28)
- ✓ **PORT-05** `(override)` annotation in `tome status` and `tome doctor` (text + JSON) — Phase 9 (2026-04-28)
- ✓ **POLISH-01** "Opening: <path>..." pre-block status + tty drain in `tome browse` (D1, #463) — Phase 10 (2026-04-29)
- ✓ **POLISH-02** `StatusMessage` redesigned as `Success | Warning | Pending` enum with body/glyph/severity accessors (D2, #463) — Phase 10 (2026-04-29)
- ✓ **POLISH-03** `ClipboardOccupied` auto-retry with 100ms backoff (D3, #463) — Phase 10 (2026-04-29)
- ✓ **POLISH-04** `FailureKind::ALL` compile-enforced via exhaustive-match sentinel (D4, #463) — Phase 10 (2026-04-29)
- ✓ **POLISH-05** `RemoveFailure::new` `debug_assert!(path.is_absolute())` invariant (D5, #463) — Phase 10 (2026-04-29)
- ✓ **POLISH-06** `arboard` patch-pin (`>=3.6, <3.7`) with bump-review policy (D6, #463) — Phase 10 (2026-04-29)
- ✓ **TEST-01** Success-banner-absence assertion on partial-failure (P1, #462) — Phase 10 (2026-04-29)
- ✓ **TEST-02** Retry-after-fix end-to-end pinning I2/I3 retention (P2, #462) — Phase 10 (2026-04-29)
- ✓ **TEST-03** `status_message_from_open_result` helper + 3-arm unit tests (P3, #462) — Phase 10 (2026-04-29)
- ✓ **TEST-04** `regen_warnings` deferred until after success banner (P4, #462) — Phase 10 (2026-04-29)
- ✓ **TEST-05** Dead `SkillMoveEntry.source_path` field removed (P5, #462) — Phase 10 (2026-04-29)

### Active (v0.10 — Library-canonical Model)

v0.10 milestone in flight. Requirements defined in `.planning/REQUIREMENTS.md`. Design doc: `.planning/research/v0.10-library-canonical-design.md`. Closes epic [#459](https://github.com/MartinP7r/tome/issues/459).

### Validated in v0.10

- ✓ **LIB-01** Library is the single source of truth — managed and local skills are stored uniformly as real directory copies — Phase 11 (2026-05-03)
- ✓ **LIB-02** `managed: bool` is now an "update channel" indicator, not a storage-strategy switch — Phase 11 (2026-05-03)
- ✓ **LIB-03** Manifest + lockfile schema accept `source_name: Option<DirectoryName>` (Unowned state); `SkillEntry::new_unowned` constructor; `Manifest::skills_get_mut` accessor — Phase 11 (2026-05-03)
- ✓ **LIB-04** Source removal preserves library content — `tome remove <dir>` and `cleanup_library` (Case 1) transition manifest entries to `source_name = None` instead of deleting — Phase 11 (2026-05-03)
- ✓ **LIB-05** `tome migrate-library` one-shot CLI converts v0.9-shape libraries; `tome sync` refuses with Conflict/Why/Suggestion hint pointing at the command; broken-symlink preservation (D-04); SAFE-01 failure aggregation; idempotent re-runs — Phase 11 (2026-05-03)
- ✓ **ADP-01** `MarketplaceAdapter` trait (six locked methods: `id`, `current_version`, `install`, `update`, `list_installed`, `available`) + `InstalledPlugin` data type + `MockMarketplaceAdapter` test double — Phase 12 (2026-05-05)
- ✓ **ADP-02** `ClaudeMarketplaceAdapter` shells to `claude plugin install/update/list --json` with `stdin = /dev/null`, internal `RefCell` snapshot cache (auto-invalidates on Ok install/update), `available()` reads cached `errors[]` field for vanished signal (zero extra subprocess calls per D-02), heuristic stderr → `InstallFailureKind` mapping — Phase 12 (2026-05-05)
- ✓ **ADP-03** `GitAdapter` thin shim over `crate::git` helpers; D-05a regression contract honored (existing git-source integration tests pass byte-for-byte) — Phase 12 (2026-05-05)
- ✓ **ADP-04** `InstallFailure` aggregation (5 fields), `InstallOp { Install, Update }`, `InstallFailureKind { NotFound, NetworkError, PermissionDenied, Unknown }` with POLISH-04 `ALL` array + compile-time exhaustiveness sentinel, `render_install_failures()` SAFE-01-shaped grouped renderer — Phase 12 (2026-05-05)

### Backlog (deferred)

- Linux runtime UAT carry-over from v0.8: 2 items in `08-HUMAN-UAT.md` (clipboard + xdg-open) — pending Linux desktop hardware
- Expand `KNOWN_DIRECTORIES` registry (Cursor, Windsurf, Aider — if they have skill paths)
- Pre-existing flaky test: `backup::tests::push_and_pull_roundtrip` — passes in isolation, intermittent in full suite. Worth a separate investigation pass.

### Validated in v0.7

- ✓ Validate wizard output against `Config::validate()` before save — Phase 4 (WHARD-01)
- ✓ Detect overlap between `library_dir` and distribution directories (Cases A/B/C) — Phase 4 (WHARD-02, WHARD-03)
- ✓ Pure wizard helpers (`find_known_directories_in`, `KNOWN_DIRECTORIES` registry, `assemble_config`) have unit test coverage — Phase 5 (WHARD-04)
- ✓ Headless `tome init --no-input` integration test validates generated config round-trips — Phase 5 (WHARD-05)
- ✓ Exhaustive `(DirectoryType, DirectoryRole)` matrix test locks in `valid_roles()` ↔ `validate()` agreement — Phase 5 (WHARD-06)

### Previously Validated (re-verified in v0.7 research)

- ✓ Merged `KNOWN_DIRECTORIES` registry (shipped silently in v0.6, now formally validated)
- ✓ Auto-discovery with role auto-assignment
- ✓ Summary table before confirmation
- ✓ Custom directory addition with role selection
- ✓ Removed `find_source_target_overlaps()` dead code

### Hardened in v0.7

The wizard-surface work below shipped in v0.6 (as WIZ-01–05) but lacked validation, circular-path detection, and test coverage. v0.7 closed those gaps. All items are now shipped AND hardened — Shipped v0.6, hardened v0.7 (Phases 4+5).

- ✓ **WIZ-01** — Merged `KNOWN_DIRECTORIES` registry replacing the split `KNOWN_SOURCES` / `KNOWN_TARGETS` arrays. Shipped v0.6, hardened v0.7: formal unit-test coverage for registry invariants and `find_known_directories_in` (Phase 5 / WHARD-04).
- ✓ **WIZ-02** — Auto-discovery with role auto-assignment (ClaudePlugins→Managed, Directory→Synced, Git→Source) at wizard time. Shipped v0.6, hardened v0.7: `(DirectoryType, DirectoryRole)` combo-matrix test locks in `valid_roles()` ↔ `Config::validate()` agreement across all 12 combos (Phase 5 / WHARD-06).
- ✓ **WIZ-03** — Custom directory addition with role selection during `tome init`. Shipped v0.6, hardened v0.7: invalid type/role combos are now rejected by `Config::validate()` before `save()` instead of being silently written (Phase 4 / WHARD-01).
- ✓ **WIZ-04** — Summary table before confirmation. Shipped v0.6, hardened v0.7: migrated to `tabled` with `Style::rounded()` and terminal-width-aware truncation (Phase 6 / WHARD-07).
- ✓ **WIZ-05** — Removal of the legacy source/target split mental model, including dead-code cleanup of `find_source_target_overlaps()`. Shipped v0.6, hardened v0.7: replaced with `Config::validate()` Cases A/B/C path-overlap detection and `Config::save_checked` TOML round-trip (Phase 4 / WHARD-02/03).

*v0.7 hardening deliverables:* (a) `Config::validate()` path-overlap checks (Phase 4), (b) `Config::save_checked` with TOML round-trip (Phase 4), (c) `--no-input` plumbing (Phase 5), (d) unit + integration test coverage for pure wizard helpers (Phase 5), (e) 12-combo validation matrix (Phase 5), (f) `tabled` summary migration (Phase 6).

## Current State

**Shipped:** v0.9.0 (2026-04-29)

v0.9 milestone complete — Cross-Machine Config Portability & Polish. 16 requirements shipped (5 PORT + 6 POLISH + 5 TEST) across Phases 9-10. Archive: [`milestones/v0.9-ROADMAP.md`](milestones/v0.9-ROADMAP.md).

**Highlights:**
- A single `tome.toml` checked into dotfiles now works across machines via per-machine `[directory_overrides.<name>]` blocks in `machine.toml` — overrides apply once at config load, every downstream command (`sync`, `status`, `doctor`, `lockfile::generate`) sees the merged result
- `tome status` and `tome doctor` mark overridden directories with `(override)` in text output and `override_applied: bool` in JSON
- `tome browse open` paints "Opening: <path>..." before blocking on `xdg-open`; `StatusMessage` redesigned as `Success | Warning | Pending` enum; `ClipboardOccupied` auto-retries with 100ms backoff
- `FailureKind::ALL` compile-enforced via exhaustive-match sentinel; `RemoveFailure::new` gains `path.is_absolute()` debug invariant; `arboard` patch-pinned with bump-review policy
- `regen_warnings` deferred until after the success banner; partial-failure success-banner-absence + retry-after-fix end-to-end tests pin the I2/I3 retention contract
- Bare-slug `tome add` (PR #471) bundled in — `tome add planetscale/database-skills` expands to `https://github.com/planetscale/database-skills`

**Carry-over:** 2 Linux-runtime UAT items in `08-HUMAN-UAT.md` (clipboard runtime + xdg-open runtime) still pending Linux desktop hardware. Accepted as carry-over for the third consecutive milestone.

## Current Milestone: v0.10 — Library-canonical Model + Cross-Machine Plugin Reconciliation

**Goal:** Make tome's library a single source of truth (real directory copies for both managed and local skills), with a lockfile-authoritative `tome sync` flow that reconciles installed plugins to the lockfile state on every machine via marketplace adapters. Closes the library-as-dotfiles workflow gap surfaced in the post-v0.9 codebase review.

**Target features:**

- **Library-canonical model** — managed skills become real directory copies in the library, not symlinks into machine-specific cache paths. Survives plugin uninstall, version churn, cross-machine sync.
- **Lockfile-authoritative cross-machine sync** — `tome.lock` becomes the truth (Cargo.lock-shaped). `tome sync` reconciles installed plugins to lockfile state via marketplace adapters.
- **MarketplaceAdapter trait** — `ClaudeMarketplaceAdapter` (shells out to `claude plugin install/update/list --json`) + `GitAdapter` (wraps existing `git.rs`).
- **Unowned-library lifecycle** — `tome reassign <skill> --to <dir>` (Unowned input, per Phase 14 D-API-1) and `tome remove skill <skill>` (per Phase 14 D-API-2). Source removal preserves library content; lifecycle is explicit, not implicit. The merge folds the originally proposed `tome adopt` / `tome forget` verbs into existing commands — see `.planning/phases/14-unowned-library-lifecycle/14-CONTEXT.md`.
- **Migration UX** — first-sync auto-migrate symlink library → real-copy library, with guarded prompt and persisted consent in `machine.toml`.
- **CLI hardening bundle** — 19 review-followups (#485–#503) + ~10 older bug backlog issues (#416, #430, #433, #447, #454, #456, #457, etc.).
- **Cleanup-message UX rewrite** — partition "stale" skills into removed-from-config / missing-from-disk / now-excluded with clear per-bucket messaging (the original trigger for this milestone discussion).

**Key context:**

- Closes epic [#459](https://github.com/MartinP7r/tome/issues/459) (cross-machine library-as-dotfiles).
- v1.0 (Tauri Desktop GUI) drafted in [`milestones/v1.0-REQUIREMENTS.md`](milestones/v1.0-REQUIREMENTS.md) + [`milestones/v1.0-ROADMAP.md`](milestones/v1.0-ROADMAP.md) but **deferred to after v0.10 ships**. Library-canonical work changes the public type surface (`SkillEntry`, `LockEntry`, `RemovePlan`) that the GUI's Tauri IPC will expose; settling the model first means GUI types don't churn mid-build.
- Hard upstream constraint: `claude plugin install/update` don't accept `--version` flags. Adapter installs/updates to "latest" only; lockfile records actual installed version + surfaces drift. True version pinning is an upstream Claude Code feature request.
- Backward compat: None (per project policy). Migration hard-cuts on first v0.10 sync; no compat shim for old library shape.
- Behavior change worth flagging in release notes: plugin updates no longer auto-propagate via symlink — they require `tome sync` to reach Claude Code skills. Users opt into upstream changes instead of being subject to them.
- Sized at ~7 phases, ~10–14 weeks of focused work. Phase numbering continues from 10 → starts at Phase 11.
- Design doc: [`.planning/research/v0.10-library-canonical-design.md`](research/v0.10-library-canonical-design.md) (468 lines, 9 open questions resolved with rationale + alternatives).

**Phase progress (v0.10):**

- [x] Phase 11: Library-canonical core (LIB-01..05) — completed 2026-05-03
- [x] Phase 12: Marketplace adapter (ADP-01..04) — completed 2026-05-05
- [x] Phase 13: Lockfile-authoritative sync (RECON-01..05) — **alpha cut** completed 2026-05-05; `reconcile.rs` (1714 LOC, 28 unit tests) replaces deleted `install.rs`; `cli_sync_reconcile.rs` integration suite (10 tests) green; `MockMarketplaceAdapter` lifted into feature-gated `pub mod testing`; `--no-install` flag + `auto_install_plugins` consent persisted in `machine.toml`
- [ ] Phase 14: Unowned-library lifecycle (UNOWN-01..03)
- [ ] Phase 15: CLI hardening (HARD-01..22) — **beta cut**
- [ ] Phase 16: Cleanup-message UX + docs (UX-01..02, DOC-01..03) — **rc cut**
- [ ] Phase 17: Migration polish + UAT + release (REL-01..05) — **v0.10 final**

<details>
<summary>Previous milestones (recap)</summary>

- v0.6 Unified Directory Model (Phases 1-3, shipped 2026-04-16) — `[directories.*]` BTreeMap config, git sources, per-directory selection, `tome add`/`remove`/`reassign`/`fork`, browse TUI polish
- v0.7 Wizard Hardening (Phases 4-6, shipped 2026-04-22) — `Config::validate()` Conflict+Why+Suggestion errors, `Config::save_checked` round-trip, `--no-input` plumbing, 12-combo matrix test, `tabled` summary
- v0.8 Wizard UX & Safety Hardening (Phases 7-8 + 8.1 hotfix, shipped 2026-04-27) — wizard greenfield/brownfield/legacy flows, partial-failure visibility, cross-platform browse, lockfile regen safety
- v0.9 Cross-Machine Config Portability & Polish (Phases 9-10, shipped 2026-04-29) — `[directory_overrides.<name>]` schema, override surfacing, Phase 8 review tail (StatusMessage redesign, FailureKind compile-enforcement, RemoveFailure invariant, arboard patch-pin)

</details>

<details>
<summary>Previous milestones (recap)</summary>

- v0.6 Unified Directory Model (Phases 1-3, shipped 2026-04-16) — `[directories.*]` BTreeMap config, git sources, per-directory selection, `tome add`/`remove`/`reassign`/`fork`, browse TUI polish
- v0.7 Wizard Hardening (Phases 4-6, shipped 2026-04-22) — `Config::validate()` Conflict+Why+Suggestion errors, `Config::save_checked` round-trip, `--no-input` plumbing, 12-combo matrix test, `tabled` summary
- v0.8 Wizard UX & Safety Hardening (Phases 7-8 + 8.1 hotfix, shipped 2026-04-27) — wizard greenfield/brownfield/legacy flows, partial-failure visibility, cross-platform browse, lockfile regen safety

</details>

### Out of Scope

- Backward-compatible config parsing — single user; hard break with migration docs
- Connector trait abstraction (#192) — unified directories solve config flexibility
- Format transforms / rules syncing (#57, #193, #194) — different concern, post-v0.6
- Watch mode (#59) — low priority
- Config migration command (`tome migrate`) — not worth building for one user

## Context

tome is at Cargo.toml `0.9.0` (released 2026-04-29 via cargo-dist). Codebase: ~26k lines of Rust across 20+ source modules in a single crate. v0.6 introduced the unified directory model; v0.7 hardened the wizard surface; v0.8 closed the new-machine/dotfiles-sync UX gap and shipped partial-failure visibility + cross-platform browse actions; v0.9 shipped per-machine `[directory_overrides.<name>]` for cross-machine portability and cleared the v0.8 review tail.

The Rust codebase uses `anyhow` for errors, `serde`/`toml` for config, `clap` for CLI, `ratatui` for the TUI browser, and `nucleo-matcher` for fuzzy search. Tests use `assert_cmd` + `tempfile` + `insta` snapshots. CI runs on Ubuntu and macOS. 662 tests total (526 unit + 136 integration as of v0.9.0).

Config is `directories: BTreeMap<DirectoryName, DirectoryConfig>` where each entry has a `role` (managed/synced/source/target) and `type` (claude-plugins/directory/git). `Config::save_checked` enforces expand → `validate()` → TOML round-trip → write; no invalid config can reach disk.

## Constraints

- **Platform**: Unix-only (symlinks). No Windows support.
- **Rust edition**: 2024. Strict clippy with `-D warnings`.
- **Single user**: Martin is the sole user. Unblocks hard-breaking changes.
- **No nested git**: Git source clones go to `~/.tome/repos/`, not inside the library dir.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Hard break, no backward compat | Single user; migration tooling not worth the cost | ✓ Good — clean implementation |
| BTreeMap (alphabetical) for duplicate priority | Simplest; conflicts rare | ✓ Good — works in practice |
| Per-directory selection in machine.toml | Per-machine concern, not portable | ✓ Good — `enabled`/`disabled` with locality principle |
| Git clones in `~/.tome/repos/<sha256>/` | Avoids nested git repos; `.git` intact for pull | ✓ Good |
| Remove `TargetMethod` enum | Only `Symlink` variant existed | ✓ Good — eliminated dead code |
| Default roles: ClaudePlugins→Managed, Directory→Synced, Git→Source | Sensible defaults | ✓ Good — no manual role needed |
| Plan/render/execute pattern for destructive commands | Separation of planning from execution | ✓ Good — reused for remove, reassign, fork |
| Manifest-based circular prevention | Replaces `shares_tool_root()` path heuristic | ✓ Good — more reliable |
| Git env clearing pattern | Every `Command::new("git")` clears GIT_DIR etc. | ✓ Good — prevents nesting bugs |
| Defer wizard rewrite (WIZ-01–05) | Old wizard still works; low priority | ✓ Resolved v0.7 — kept existing wizard code, hardened in-place (validation + tests + tabled polish) instead of rewriting |
| D-10 Conflict+Why+Suggestion error template (v0.7 Phase 4) | Validation errors should name what conflicts, explain why, and suggest a fix | ✓ Good — applied to all 4 `Config::validate()` bail sites |
| TOML round-trip byte-equality over `PartialEq` (v0.7 Phase 4) | Avoids deriving PartialEq cascade across all config types; compares emitted strings | ✓ Good — `Config::save_checked` enforces in ~20 lines |
| `--no-input` flag over separate non-interactive binary (v0.7 Phase 5) | One wizard, two modes — integration tests drive the same code path users run | ✓ Good — 12-combo matrix test possible because of this |
| `tabled` `Style::rounded()` for wizard summary, `Style::blank()` stays for `tome status` (v0.7 Phase 6) | Ceremonial one-shot summary deserves visual weight; repeated inspection wants lightweight | ✓ Good — matching pattern not required |
| Brownfield default = "use existing" (v0.8 Phase 7 / WUX-02) | Safest for the dotfiles-sync workflow that triggered v0.8; never silently overwrites a valid config | ✓ Good — `--no-input` defaults to use-existing too |
| Legacy config: warn + offer delete, not silent auto-delete (v0.8 Phase 7 / WUX-03) | File may contain user-valued data worth manual review | ✓ Good |
| `tome_home` prompt writes XDG config, not `TOME_HOME` env-var injection (v0.8 Phase 7 / WUX-05 / D-2) | XDG file is shell-agnostic; propagates to cron/editor/subshells | ✓ Good |
| `arboard` (default-features = false) for cross-platform clipboard (v0.8 Phase 8 / SAFE-02) | Replaces `sh -c \| pbcopy` command-injection vector; no `image` crate in dep tree | ✓ Good — Linux runtime carry-over flagged in HUMAN-UAT |
| Glyph-prefix dispatch (✓ → `theme.accent`, ⚠ → `theme.alert`) for status bar (v0.8 Phase 8 / SAFE-02) | Reuses existing theme fields; no new `theme.warning` needed | ✓ Good |
| Lockfile-as-cache for offline resolved-paths recovery (v0.8.1 Phase 8.1 / HOTFIX-01) | Reads previous lockfile + on-disk repo cache; no `git fetch` from destructive commands; per-directory warnings replace silent skip | ✓ Good — closes #461 H1 silent-drop regression |
| `if !result.failures.is_empty()` block fires before save chain in `Command::Remove` (v0.8.1 Phase 8.1 / HOTFIX-02) | Save-chain `?` propagation was masking the I2/I3 retention messaging on disk-write errors | ✓ Good — closes #461 H2 |
| `[directory_overrides.<name>]` lives in `machine.toml` (v0.9 Phase 9 / PORT-01) | Sync boundary already correct — `machine.toml` is per-machine and never synced; reusing it for path overrides preserves the "tome.toml is portable" invariant | ✓ Good — closes #458 cross-machine portability epic |
| `Config::apply_machine_overrides` between `expand_tildes()` and `validate()` (v0.9 Phase 9 / PORT-02) | Single insertion point in the load pipeline guarantees all downstream code sees the merged result; no second code path can observe pre-override paths | ✓ Good — single source of truth |
| `override_applied: bool` on `DirectoryConfig` with `#[serde(skip)]` (v0.9 Phase 9 / PORT-05) | Single source of truth for status/doctor surfacing; round-trip byte-equality of `tome.toml` preserved | ✓ Good — chosen over snapshot-diff |
| Override-induced validation errors via message-content wrapper, not typed enum (v0.9 Phase 9 / PORT-04) | Matches existing tome convention (`anyhow::Result` + grep-able message templates); typed-enum migration noted as v1.0 follow-up if a programmatic consumer needs it | ✓ Good — defensible per plan-checker |
| `StatusMessage = Success \| Warning \| Pending` enum with `body()`/`glyph()`/`severity()` accessors (v0.9 Phase 10 / POLISH-02) | Removes dual-source-of-truth between severity and pre-formatted glyph in body; UI formats `"{glyph} {body}"` at render time | ✓ Good |
| Closure-callback redraw threading for pre-block TUI updates (v0.9 Phase 10 / POLISH-01) | Keeps `App` independent of `ratatui::DefaultTerminal`; redraw fires BEFORE `.status()` blocks (a flag-based approach would only redraw on the next event loop iteration, too late) | ✓ Good — chosen over `pending_redraw` flag and `&mut Terminal` threading |
| `FailureKind::ALL` compile-enforced via exhaustive-match sentinel + const-len assert (v0.9 Phase 10 / POLISH-04) | Catches "added a variant without updating ALL" at compile time; no `strum` dep needed | ✓ Good — chosen over runtime canary |
| `arboard` patch-pin (`>=3.6, <3.7`) with bump-review comment (v0.9 Phase 10 / POLISH-06) | Prevents silent variant addition (`arboard::Error` is `#[non_exhaustive]`); review-on-bump policy documented in `Cargo.toml` | ✓ Good |
| Defer `regen_warnings` until after success banner (v0.9 Phase 10 / TEST-04) | Success banner is the user's anchor; warnings as a footnote feel more natural than scoped-prefix on every line. Source-byte regression test anchored to `Command::Remove` region for false-positive resistance | ✓ Good — chosen over `[lockfile regen]` prefix |
| Insert v0.10 (library-canonical) before v1.0 GUI (2026-05-02) | v0.10 reshapes the public Rust types (`SkillEntry`, `LockEntry`, `RemovePlan`) that the GUI's Tauri IPC will expose. Settling the model first means GUI types don't churn mid-build. Library-as-dotfiles workflow gap is also blocking for the project's primary user — fixing it before adding a GUI surface is the right order. | TBD — outcome at v0.10 ship |
| Keep GUI = v1.0 naming despite v0.10 inserted between v0.9 and v1.0 (2026-05-02) | Library-canonical work earns the v1.0 framing (stable Rust types, durable library, reproducible cross-machine install). Calling the GUI release v1.0 *because it ships on solid ground* is honest, not just naming-ceremony. v0.10 lays the foundation; v1.0 ships the visible product on top of it. | TBD — outcome at v1.0 ship |
| Library = single source of truth (managed-as-copy), not consolidated cache (v0.10 / D-LIB-01) | Today's symlink-managed library breaks library-as-dotfiles (machine-specific paths in git), loses content on plugin uninstall/version-churn, and provides no resilience against vanished plugins. Library-canonical model fixes all three at the cost of disk space (~5–50 MB) and update-timing change (sync required for upstream changes to propagate). | ✓ Validated in Phase 11 (2026-05-03) |
| Lockfile-authoritative cross-machine reproducibility (Cargo.lock-shaped) (v0.10 / D-LIB-02) | `tome.lock` becomes the authoritative state for what's installed on every machine. `tome sync` reconciles drift via marketplace adapters. Mirrors `cargo build` semantics; user mental model already familiar. | Pending — implementation in Phase 13 |
| MarketplaceAdapter trait, not direct shell-out (v0.10 / D-LIB-03) | Trait isolates marketplace-specific install logic (Claude CLI, git, future: npm) behind a stable interface. Production adapter shells out; tests use `MockMarketplaceAdapter`. v0.10 ships `ClaudeMarketplaceAdapter` + `GitAdapter` (wrap existing `git.rs`). | Pending — implementation in Phase 12 |
| ~~`tome adopt`/`forget`~~ → `tome reassign` (Unowned input, D-API-1) and `tome remove skill` (D-API-2) for unowned library entries (v0.10 / D-LIB-04; merge per Phase 14 D-API-1/-2) | Source removal no longer auto-deletes library content. Library entries enter `Unowned` state; explicit commands manage the lifecycle. Avoids silent data loss when user removes a directory entry from `tome.toml`. The merge folds the proposed `tome adopt`/`tome forget` verbs into existing commands — smaller CLI surface, single re-anchor verb. | ✓ Validated in Phase 14 |
| Per-machine first-time auto-install consent persisted in `machine.toml` (v0.10 / D-LIB-05) | Balances zero-touch new-machine onboarding (auto-install missing plugins) against surprise-action-prevention on shared/CI machines. Prompts once per machine; remembers consent. Honors global `--no-install` opt-out. | Pending — implementation in Phase 13 |

## Evolution

This document evolves at phase transitions and milestone boundaries.

---
*Last updated: 2026-05-05 — Phase 12 (Marketplace adapter) complete. All 4 ADP-* requirements validated: `MarketplaceAdapter` trait + `InstalledPlugin` + `MockMarketplaceAdapter` (ADP-01); `ClaudeMarketplaceAdapter` with `RefCell` snapshot cache, stdin-closed subprocess invocation, `errors[]`-field-based vanished signal, heuristic stderr classifier (ADP-02); `GitAdapter` thin shim with byte-for-byte D-05a regression contract honored (ADP-03); `InstallFailure` aggregation + POLISH-04 `ALL` exhaustiveness sentinel + SAFE-01-shaped grouped renderer (ADP-04). 599 unit + 141 integration tests pass. Phase 12 ships the trait + adapters in `crates/tome/src/marketplace.rs` only — no `lib.rs::sync()` call-site changes; Phase 13 (lockfile-authoritative sync — alpha cut) wires the D-11 dispatcher next.*

*Last updated: 2026-05-03 — Phase 11 (Library-canonical core) complete. All 5 LIB-* requirements validated: managed and local skills now stored uniformly as real directory copies (LIB-01); `managed: bool` reframed as update-channel indicator (LIB-02); `Option<DirectoryName>` schema for Unowned state (LIB-03); source removal preserves library content (LIB-04); `tome migrate-library` one-shot with sync refuse-with-hint, broken-symlink preservation, SAFE-01 aggregation, idempotent re-runs (LIB-05). 558 unit + 141 integration tests pass. Phase 12 (Marketplace adapter) is next; alpha cut comes at Phase 13.*

*Last updated: 2026-05-02 — v0.10 milestone started. Goal: library-canonical model (managed-as-copy, source removal preserves content) + lockfile-authoritative cross-machine sync via marketplace adapters + CLI hardening bundle (19 review-followups + ~10 older bug backlog issues). Closes epic #459 (cross-machine library-as-dotfiles). v1.0 (Tauri GUI) deferred — drafted in `milestones/v1.0-{REQUIREMENTS,ROADMAP}.md`, ratifies after v0.10 ships so it can build on the stable type surface and durable library. Design doc: `.planning/research/v0.10-library-canonical-design.md` (468 lines, 9 OQs resolved). Phase 11 is the next planning unit. Discussion lineage: PR #484 codebase review → "no longer configured" UX question → library-as-dotfiles realization → managed-as-copy resilience requirement → cross-machine reconciliation requirement.*

*Last updated: 2026-04-29 after v0.9 milestone — v0.9.0 shipped via cargo-dist (commits c183e3f Phase 10 + 0ae6288 version bump on main). v0.9 milestone archived: 16 v0.9 requirements (5 PORT + 6 POLISH + 5 TEST) shipped across Phases 9 and 10 (10 in 1 wave, 9 in 2 waves). 662 tests passing (526 unit + 136 integration). Linux-runtime UAT items in `08-HUMAN-UAT.md` carried over for the third consecutive milestone (still pending hardware). Ready for v1.0 — Tauri GUI milestone artifacts already drafted in `milestones/v1.0-{REQUIREMENTS,ROADMAP}.md`; ratify via `/gsd:new-milestone` to start phase planning.*

*Last updated: 2026-05-05 — Phase 13 complete (Lockfile-authoritative sync, **v0.10 alpha cut**). All 5 RECON requirements shipped: classification (Match/Drift/Vanished/MissingFromMachine), `auto_install_plugins` consent + `--no-install` flag, drift apply with re-hash, vanished warnings + preserved distribution, edit-in-library 3-way prompt with `--no-input` skip-default. New `reconcile.rs` module (1714 LOC, 28 unit tests) replaces deleted `install.rs` (-312 LOC); `MockMarketplaceAdapter` lifted into feature-gated `pub mod testing`; new `cli_sync_reconcile.rs` integration suite (10 tests). Test count: 781 (630 unit + 141 integration cli + 10 cli_sync_reconcile). Modulo pre-existing HARD-14 timing flake.*

*Last updated: 2026-04-29 — Phase 10 complete (Phase 8 Review Tail). All 11 v0.8 review-tail items shipped: POLISH-01..06 (#463 D1-D6) + TEST-01..05 (#462 P1-P5). 662 tests passing. v0.9 milestone functionally complete — ready for milestone closure.*

*Last updated: 2026-04-28 — Phase 9 complete (Cross-Machine Path Overrides). All 5 PORT requirements shipped: `[directory_overrides.<name>]` schema in machine.toml, override-apply timing in load pipeline, typo warning, distinct machine.toml error class, and `(override)` annotation in `tome status`/`tome doctor` (text + JSON). 648 tests passing (514 unit + 134 integration; +58 since Phase 9 start). Phase 10 (#462 + #463 polish bundle) is the remaining v0.9 work.*

*Last updated: 2026-04-28 — v0.9 milestone started. Goal: cross-machine config portability (#458) bundled with #463 (type design + TUI architecture polish, 6 items) and #462 (test coverage + dead code polish, 5 items) from the v0.8 post-merge review. Bare-slug `tome add` improvement (PR #471, merged 2026-04-27) ships with v0.9 — no v0.8.2 patch release planned.*

*Last updated: 2026-04-27 after v0.8 milestone — v0.8.1 shipped via cargo-dist (commits e13eb31 + 231e52d on main). v0.8 milestone archived: 8 v0.8 requirements (WUX-01..05 + SAFE-01..03) + 3 v0.8.1 hotfix requirements (HOTFIX-01..03) shipped across Phases 7, 8, and 8.1. 590 tests passing (464 unit + 126 integration). Linux-runtime UAT items in `08-HUMAN-UAT.md` accepted as carry-over pending hardware.*
