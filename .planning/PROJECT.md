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

### Active (v0.8)

- [x] **WUX-01** Wizard prompts for `tome_home` on greenfield (#453) — *Validated in Phase 7 (2026-04-23)*
- [x] **WUX-02** Wizard detects existing `.tome/` config (brownfield: use / edit / reinitialize) (#453) — *Validated in Phase 7 (2026-04-23)*
- [x] **WUX-03** Wizard detects legacy `~/.config/tome/config.toml` pre-v0.6 file and offers cleanup (#453) — *Validated in Phase 7 (2026-04-23)*
- [x] **WUX-04** Wizard prints resolved `tome_home` up-front as info line (#453) — *Validated in Phase 7 (2026-04-23)*
- [x] **WUX-05** Wizard offers to persist custom `tome_home` via XDG config write (#453) — *Validated in Phase 7 (2026-04-23)*
- [ ] **SAFE-01** `remove::execute` aggregates partial-cleanup failures and surfaces them (#413)
- [ ] **SAFE-02** Browse UI `open` and `copy path` actions work on Linux (#414)
- [ ] **SAFE-03** `relocate.rs` surfaces `fs::read_link` errors instead of silently dropping (#449)

### Backlog (not in v0.8)

- Expand `KNOWN_DIRECTORIES` registry (Cursor, Windsurf, Aider — if they have skill paths)
- v0.7.1 hotfix (PR #455): tabled ANSI width fix — prerequisite patch release
- v0.7.2 quick wins (#456 library default derivation, #457 tilde preservation on save) — prereq patch release
- v0.9 scope: cross-machine config portability via machine.toml path overrides (#458)

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

## Current Milestone: v0.8 Wizard UX & Safety Hardening

**Goal:** Close the new-machine / dotfiles-sync UX gap on the wizard and address the P0 safety refactors from the v0.7 whole-codebase review that didn't fit in the v0.7.1 hotfix.

**Target features:**
- Wizard prompts for `tome_home` on new machines; detects existing `.tome/` configs and offers use / edit / reinitialize flows (brownfield support)
- Wizard surfaces the resolved `tome_home` up-front and offers to persist via XDG config (`~/.config/tome/config.toml`)
- Wizard detects and cleans up the legacy pre-v0.6 `~/.config/tome/config.toml` file (silent ignore is a footgun)
- `remove::execute` aggregates partial-cleanup failures and reports them — no more silent "success" when symlinks fail to delete
- Browse UI's `open` and `copy path` actions work cross-platform (currently macOS-only with silent no-op on Linux)
- `relocate.rs` surfaces `fs::read_link` failures instead of silently dropping them

**Scope anchor:** Epic issue [#459](https://github.com/MartinP7r/tome/issues/459)

**Key context:**
- v0.7.1 (tabled ANSI width hotfix) and v0.7.2 (library_dir default + tilde preservation, #456/#457) are patch cuts that precede this milestone
- v0.9 will tackle cross-machine config portability via machine.toml path overrides (#458) — larger design, intentionally deferred

### Shipped in v0.7 (1-line recap)

v0.7 Wizard Hardening — 8 requirements (WHARD-01..08) across Phases 4-6, shipped 2026-04-22. Archive: [`milestones/v0.7-ROADMAP.md`](milestones/v0.7-ROADMAP.md). Wizard now refuses invalid configs at save time; 525 tests passing; summary renders via `tabled` with rounded borders.

### Out of Scope

- Backward-compatible config parsing — single user; hard break with migration docs
- Connector trait abstraction (#192) — unified directories solve config flexibility
- Format transforms / rules syncing (#57, #193, #194) — different concern, post-v0.6
- Watch mode (#59) — low priority
- Config migration command (`tome migrate`) — not worth building for one user

## Context

tome is at Cargo.toml `0.6.2` pending the v0.7.0 release cut. Codebase: ~21.8k lines of Rust across 20+ source modules in a single crate. v0.6 introduced the unified directory model; v0.7 hardened the wizard surface around it.

The Rust codebase uses `anyhow` for errors, `serde`/`toml` for config, `clap` for CLI, `ratatui` for the TUI browser, and `nucleo-matcher` for fuzzy search. Tests use `assert_cmd` + `tempfile` + `insta` snapshots. CI runs on Ubuntu and macOS. 525 tests total (417 unit + 108 integration).

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

## Evolution

This document evolves at phase transitions and milestone boundaries.

---
*Last updated: 2026-04-23 — Phase 7 complete (Wizard UX: greenfield prompt, brownfield 4-way decision, legacy detection, resolved `tome_home` info line, XDG persistence). WUX-01..05 all shipped and validated. Phase 8 (Safety Refactors) next.*
