# skync

Sync AI coding skills across tools. Discover skills from Claude Code plugins, standalone directories, and custom locations — then distribute them to every AI coding tool that supports the SKILL.md format.

## Why

AI coding tools (Claude Code, Codex, Antigravity) each use SKILL.md packages to provide context. But skills get siloed:

- Plugin skills live in cache directories you never see
- Standalone skills only exist for one tool
- Switching tools means losing access to your skill library

**skync** consolidates all skills into a single library and distributes them everywhere.

## Install

```bash
cargo install skync
```

## Quick Start

```bash
# Interactive setup — discovers sources, configures targets
skync init

# Sync skills to all configured targets
skync sync

# Check what's configured
skync status
```

## Commands

| Command | Description |
|---------|-------------|
| `skync init` | Interactive wizard to configure sources and targets |
| `skync sync` | Discover, consolidate, and distribute skills |
| `skync status` | Show library, sources, targets, and health |
| `skync list` | List all discovered skills with sources |
| `skync doctor` | Diagnose and repair broken symlinks |
| `skync serve` | Start the MCP server (stdio) |
| `skync config` | Show current configuration |

All commands support `--dry-run`, `--verbose`, and `--config <path>`.

## How It Works

```
Sources                    Library                  Targets
┌─────────────────┐       ┌──────────────┐       ┌─────────────────┐
│ Plugin cache     │──┐   │              │   ┌──▶│ Antigravity     │
│ (23 skills)      │  │   │  Consolidated│   │   │ (symlinks)      │
├─────────────────┤  ├──▶│  skill       ├───┤   ├─────────────────┤
│ ~/.claude/skills │  │   │  library     │   │   │ Codex           │
│ (8 skills)       │──┤   │              │   └──▶│ (MCP config)    │
├─────────────────┤  │   │  (symlinks)  │       ├─────────────────┤
│ ~/my-skills      │──┘   │              │       │ OpenClaw        │
│ (18 skills)      │      └──────────────┘       │ (MCP config)    │
└─────────────────┘                               └─────────────────┘
```

1. **Discover** — Scan configured sources for `*/SKILL.md` directories
2. **Consolidate** — Symlink discovered skills into a central library (deduplicates, first source wins)
3. **Distribute** — Create symlinks or MCP config entries in each target tool's directory
4. **Cleanup** — Remove stale symlinks for skills that no longer exist

## Configuration

TOML at `~/.config/skync/config.toml`:

```toml
library_dir = "~/.local/share/skync/skills"
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

[targets.codex]
enabled = true
method = "mcp"
mcp_config = "~/.codex/.mcp.json"
```

## MCP Server

skync includes a built-in MCP server for tools that support the Model Context Protocol:

```bash
# Standalone binary
skync-mcp

# Or via the CLI
skync serve
```

The server exposes two tools:
- `list_skills` — List all discovered skills
- `read_skill` — Read a skill's SKILL.md content by name

## License

MIT
