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

### Active

- [ ] Merged KNOWN_DIRECTORIES registry replacing KNOWN_SOURCES + KNOWN_TARGETS
- [ ] Auto-discovery with role auto-assignment from registry
- [ ] Summary table (name | path | type | role) before confirmation
- [ ] Custom directory addition with role selection
- [ ] Remove find_source_target_overlaps() dead code

## Current Milestone: v0.7 Wizard Rewrite

**Goal:** Rewrite `tome init` wizard to use the unified directory model, replacing the legacy source/target mental model.

**Target features:**
- Merged `KNOWN_DIRECTORIES` registry (replaces `KNOWN_SOURCES` + `KNOWN_TARGETS`)
- Auto-discovery with role auto-assignment from registry
- Summary table before confirmation
- Custom directory addition with role selection
- Remove `find_source_target_overlaps()` dead code

### Known Gaps (deferred from v0.6)

- WIZ-01 through WIZ-05: Wizard rewrite with merged `KNOWN_DIRECTORIES` registry. The old wizard code still works but uses the legacy source/target mental model. Low priority since `tome init` is a one-time operation.

### Out of Scope

- Backward-compatible config parsing — single user; hard break with migration docs
- Connector trait abstraction (#192) — unified directories solve config flexibility
- Format transforms / rules syncing (#57, #193, #194) — different concern, post-v0.6
- Watch mode (#59) — low priority
- Config migration command (`tome migrate`) — not worth building for one user

## Context

tome is at v0.6.1 with ~20k lines of Rust across 20+ source modules in a single crate. The unified directory model eliminated the source/target config split. Git-backed skill repos are supported with shallow clones and ref pinning.

The Rust codebase uses `anyhow` for errors, `serde`/`toml` for config, `clap` for CLI, `ratatui` for the TUI browser, and `nucleo-matcher` for fuzzy search. Tests use `assert_cmd` + `tempfile` + `insta` snapshots. CI runs on Ubuntu and macOS.

Config is `directories: BTreeMap<DirectoryName, DirectoryConfig>` where each entry has a `role` (managed/synced/source/target) and `type` (claude-plugins/directory/git).

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
| Defer wizard rewrite (WIZ-01–05) | Old wizard still works; low priority | ⚠️ Revisit — tech debt |

## Evolution

This document evolves at phase transitions and milestone boundaries.

---
*Last updated: 2026-04-18 — v0.7 milestone started*
