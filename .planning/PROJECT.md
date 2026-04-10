# tome v0.6 — Unified Directory Model

## What This Is

tome is a CLI tool that manages AI coding agent skills across multiple tools (Claude Code, Codex, Antigravity, Cursor, etc.). It discovers skills from sources, consolidates them into a central library, and distributes them to target tools via symlinks. v0.6 replaces the artificial source/target split with a unified directory model where each configured directory declares its relationship to tome.

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

### Active

- [ ] Unified `[directories.*]` config replacing `[[sources]]` + `[targets.*]` (#396)
- [ ] Wizard rewrite with merged KNOWN_DIRECTORIES registry (#362)
- [ ] Git sources — clone/pull remote skill repos (#58)
- [ ] Standalone SKILL.md import from GitHub repos (#92)
- [ ] Per-target skill selection in machine.toml (#253)
- [ ] `tome remove` — CLI to remove directories from config (#392)
- [ ] Change skill source after the fact (#395)
- [ ] Browse TUI visual polish (#365)

### Out of Scope

- Backward-compatible config parsing — single user, hard break with migration instructions
- Connector trait abstraction (#192) — deferred; unified directories solve config flexibility first
- Format transforms / rules syncing (#57, #193, #194) — different concern, deferred to post-v0.6
- Watch mode (#59) — low priority, deferred
- Skill composition / Wolpertinger (#267) — v0.7 experimental
- Config migration command (`tome migrate`) — not worth building for one user; manual migration sufficient

## Context

tome is at v0.5.4 with a mature sync pipeline. The codebase has ~17 source modules in a single Rust crate. The core insight driving v0.6 is that `~/.claude/skills` appears in config as **both** a source and a target — the wizard even has `find_source_target_overlaps()` to detect this. The unified directory model eliminates this artificial split.

The Rust codebase uses `anyhow` for errors, `serde`/`toml` for config, `clap` for CLI, and `ratatui` for the browser TUI. Tests use `assert_cmd` + `tempfile` + `insta` snapshots. CI runs on Ubuntu and macOS.

Current config has `sources: Vec<Source>` (ordered, first wins for duplicates) and `targets: BTreeMap<TargetName, TargetConfig>`. The unified model replaces both with `directories: BTreeMap<DirectoryName, DirectoryConfig>` where each entry has a `role` (managed/synced/source/target) and `type` (claude-plugins/directory/git).

## Constraints

- **Platform**: Unix-only (symlinks). No Windows support.
- **Rust edition**: 2024. Strict clippy with `-D warnings`.
- **Single user**: Martin is the sole user. This unblocks hard-breaking changes but means there's no migration tooling.
- **No nested git**: Git source clones go to `~/.tome/repos/`, not inside the library dir (which may be its own git repo).
- **Backward compat**: None. Old `tome.toml` files will fail to parse. Migration is documented, not automated.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Hard break, no backward compat | Single user; migration tooling not worth the cost | — Pending |
| BTreeMap (alphabetical) for duplicate priority | Simplest; conflicts rare; `priority` field can be added later | — Pending |
| Auto-assign roles in wizard, show summary | Faster UX; known directories get sensible defaults | — Pending |
| Per-target selection in machine.toml | Per-machine concern, not portable across machines | — Pending |
| Git clones in `~/.tome/repos/<hash>/` | Avoids nested git repos inside library; `.git` intact for pull | — Pending |
| One atomic PR for foundation (#396 + #362) | Cohesive change; easier to review as a unit | — Pending |
| Remove `TargetMethod` enum | Only `Symlink` variant exists; path lives in `DirectoryConfig.path` | — Pending |
| Default roles: ClaudePlugins→Managed, Directory→Synced, Git→Source | Sensible defaults matching typical usage patterns | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd:transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-04-10 after initialization*
