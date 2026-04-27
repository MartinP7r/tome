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

### Active (next milestone)

- [ ] Cross-machine config portability via `machine.toml` path overrides (#458) — primary v0.9 driver

### Backlog (deferred)

- v0.8.x polish: Phase 8 test coverage + wording + dead code (#462) — 5 items from the post-merge review (P1-P5): success-banner-absence assertion, retry end-to-end test, ViewSource .status() middle-branch coverage, regen-warning ordering, dead `source_path` field.
- v0.9 polish: Phase 8 type design + TUI architecture (#463) — 6 items (D1-D6): .status() TUI blocking, StatusMessage type redesign, clipboard auto-retry, FailureKind::ALL compile-enforcement, RemoveFailure::new justification, arboard drift hygiene.
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

**Shipped:** v0.8.1 (2026-04-27)

v0.8 milestone complete — Wizard UX & Safety Hardening. 8 requirements shipped (WUX-01..05 + SAFE-01..03) across Phases 7+8, plus the v0.8.1 hotfix (HOTFIX-01..03 across Phase 8.1) closing 3 post-merge findings from #461. Archive: [`milestones/v0.8-ROADMAP.md`](milestones/v0.8-ROADMAP.md).

**Highlights:**
- Wizard handles greenfield, brownfield, and legacy machine states — no more silent overwrites or default-path footguns
- `tome remove` aggregates partial-cleanup failures with grouped stderr summary + non-zero exit; save chain reordered so retention messaging surfaces before save errors
- `tome browse` `open` + `copy path` work on Linux (`xdg-open` + `arboard`), with success/failure surfacing in TUI status bar
- `relocate.rs` surfaces `read_link` failures instead of silently dropping
- Lockfile regen for `tome remove`/`reassign`/`fork` no longer silently drops git-sourced skills

**Carry-over:** 2 Linux-runtime UAT items in `08-HUMAN-UAT.md` (clipboard runtime + xdg-open runtime) pending Linux desktop hardware. Accepted as carry-over.

## Next Milestone Goals

**v0.9 Cross-Machine Config Portability** — epic [#458](https://github.com/MartinP7r/tome/issues/458)

Primary driver: `machine.toml` path overrides so the same `tome.toml` can be checked into dotfiles and applied across machines with different `~/.tome` layouts. Larger design — new schema fields + override-apply timing in the load pipeline. Intentionally deferred from v0.8.

Secondary candidates (from v0.8 post-merge review):
- v0.9 polish (#463): 6 type design + TUI architecture items from Phase 8 review
- v0.8.x polish (#462): 5 test coverage + wording + dead code items — could ship as a v0.8.x patch instead

Run `/gsd:new-milestone` to plan v0.9.

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

tome is at Cargo.toml `0.8.1` (released 2026-04-27 via cargo-dist). Codebase: ~25.2k lines of Rust across 20+ source modules in a single crate. v0.6 introduced the unified directory model; v0.7 hardened the wizard surface; v0.8 closed the new-machine/dotfiles-sync UX gap and shipped partial-failure visibility + cross-platform browse actions.

The Rust codebase uses `anyhow` for errors, `serde`/`toml` for config, `clap` for CLI, `ratatui` for the TUI browser, and `nucleo-matcher` for fuzzy search. Tests use `assert_cmd` + `tempfile` + `insta` snapshots. CI runs on Ubuntu and macOS. 590 tests total (464 unit + 126 integration as of v0.8.1).

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

## Evolution

This document evolves at phase transitions and milestone boundaries.

---
*Last updated: 2026-04-27 after v0.8 milestone — v0.8.1 shipped via cargo-dist (commits e13eb31 + 231e52d on main). v0.8 milestone archived: 8 v0.8 requirements (WUX-01..05 + SAFE-01..03) + 3 v0.8.1 hotfix requirements (HOTFIX-01..03) shipped across Phases 7, 8, and 8.1. 590 tests passing (464 unit + 126 integration). Linux-runtime UAT items in `08-HUMAN-UAT.md` accepted as carry-over pending hardware. Ready for v0.9 (cross-machine config portability, #458).*
