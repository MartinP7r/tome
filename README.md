<p align="right">
  <a href="https://github.com/MartinP7r/tome/actions/workflows/ci.yml"><img src="https://github.com/MartinP7r/tome/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
  <a href="https://github.com/MartinP7r/tome/releases"><img src="https://img.shields.io/github/v/release/MartinP7r/tome" alt="Latest release" /></a>
  <img src="https://img.shields.io/github/downloads/MartinP7r/tome/total" alt="Downloads" />
  <img src="https://img.shields.io/badge/License-MIT-yellow" alt="License: MIT" />
</p>

# tome 📖

*Your skills, leather-bound.*

<p align="center">
  <img src="docs/gfx/mage.svg" alt="tome mascot" width="560" />
</p>

Sync AI coding skills across tools. Discover skills from Claude Code plugins, standalone directories, and custom locations — then distribute them to every AI coding tool that supports the SKILL.md format.

> [!WARNING]
> **Beta software.** tome is under active development and may contain bugs that could break your local skills setup or interfere with other tooling. Back up your skills directories (e.g., with git) before running `tome sync` for the first time. Use `--dry-run` to preview changes without modifying anything.

## Why

AI coding tools (Claude Code, Codex, Antigravity) each use SKILL.md packages to provide context. But skills get siloed:

- Plugin skills live in cache directories you never see
- Standalone skills only exist for one tool
- Switching tools means losing access to your skill library

**tome** consolidates all skills into a single library and distributes them everywhere.

## Install

**Homebrew** (macOS/Linux):
```bash
brew install MartinP7r/tap/tome
```

## Quick Start

```bash
# Interactive setup — discovers sources, configures targets
tome init

# Sync skills to all configured targets
tome sync

# Check what's configured
tome status
```

## Development

For repository workflow guidance, see [docs/src/development-workflow.md](docs/src/development-workflow.md). It explains when `tome` uses GitHub Issues vs OpenSpec vs Beads, and how to link them cleanly in commits and PRs.

## Commands

| Command       | Description                                              |
| ------------- | -------------------------------------------------------- |
| `tome init`   | Interactive wizard to configure sources and targets      |
| `tome sync`   | Discover, consolidate, triage changes, and distribute    |
| `tome status` | Show library, sources, targets, and health               |
| `tome list`   | List all discovered skills with sources                  |
| `tome doctor` | Diagnose and repair broken symlinks or config issues     |
| `tome config` | Show current configuration                               |

All commands support `--dry-run`, `--verbose`, `--quiet`, `--config <path>`, and `--machine <path>`.

## How It Works

```mermaid
graph LR
    subgraph Sources
        S1["Plugin cache<br/>(23 skills)"]
        S2["~/.claude/skills<br/>(8 skills)"]
        S3["~/my-skills<br/>(18 skills)"]
    end

    subgraph Library
        L["Consolidated<br/>skill library<br/>(copies + symlinks)"]
    end

    subgraph Targets
        T1["Antigravity<br/>(symlinks)"]
        T2["Codex<br/>(symlinks)"]
        T3["OpenClaw<br/>(symlinks)"]
    end

    S1 --> L
    S2 --> L
    S3 --> L
    L --> T1
    L --> T2
    L --> T3
```

1. **Discover** — Scan configured sources for `*/SKILL.md` directories
2. **Consolidate** — Gather skills into a central library: local skills are copied, managed (plugin) skills are symlinked; deduplicates with first source winning
3. **Distribute** — Create symlinks in each target tool's skills directory (respects per-machine disabled list)
4. **Cleanup** — Remove stale entries and broken symlinks from library and targets

## Configuration

TOML at `~/.tome/tome.toml`:

```toml
library_dir = "~/.tome/skills"
exclude = ["deprecated-skill"]

[[sources]]
name = "claude-plugins"
path = "~/.claude/plugins/cache"
type = "claude-plugins"

[[sources]]
name = "standalone"
path = "~/.claude/skills"
type = "directory"

[targets.antigravity]
enabled = true
method = "symlink"
skills_dir = "~/.gemini/antigravity/skills"
```

## Per-Machine Preferences

Control which skills are active on each machine via `~/.config/tome/machine.toml`:

```toml
disabled = ["noisy-skill", "work-only-skill"]
disabled_targets = ["openclaw"]
```

Disabled skills stay in the library but are skipped during distribution. `tome sync` automatically diffs the lockfile and offers interactive triage when new or changed skills are detected.

## License

MIT
