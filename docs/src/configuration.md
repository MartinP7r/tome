# Configuration

## Main Config

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

### Fields

| Field | Description |
|-------|-------------|
| `library_dir` | Path to the consolidated skill library. Supports `~` expansion. |
| `exclude` | List of skill names to skip during discovery. |

### Source Types

| Type | Description |
|------|-------------|
| `claude-plugins` | Reads `installed_plugins.json` from the Claude Code plugin cache. Supports v1 (flat array) and v2 (namespaced object) formats. |
| `directory` | Flat scan for `*/SKILL.md` directories. |

### Target Methods

| Method | Fields | Description |
|--------|--------|-------------|
| `symlink` | `skills_dir` | Creates symlinks in the target's skills directory pointing into the library. |

Targets are data-driven — any tool can be added without code changes. The `tome init` wizard auto-discovers common tool locations via a built-in `KnownTarget` registry.

## Machine Preferences

Per-machine opt-in/opt-out at `~/.config/tome/machine.toml`:

```toml
disabled = ["noisy-skill", "work-only-skill"]
disabled_targets = ["openclaw"]
```

| Field | Description |
|-------|-------------|
| `disabled` | List of skill names to skip during distribution (no symlinks created in targets). |
| `disabled_targets` | List of target names to skip entirely on this machine. |

Disabled skills remain in the library but are skipped during distribution.

This allows sharing a single library (e.g., via git) across machines while customizing which skills are active on each one.

Use `tome update` to interactively review new or changed skills and disable unwanted ones. The `--machine <path>` global flag overrides the default machine preferences path.

## Lockfile

`tome sync` generates a `tome.lock` file in the tome home directory (`~/.tome/tome.lock`). This lockfile captures a reproducible snapshot of all skills — their names, content hashes, sources, and provenance metadata. It is used by `tome update` to diff against the current state and surface changes.

The lockfile is designed to be committed to version control alongside the library, enabling multi-machine workflows where `tome update` on a new machine can detect what changed since the last sync.

## Library `.gitignore`

`tome sync` automatically generates a `.gitignore` in the library directory:

- **Managed skills** (symlinked from package managers) are gitignored — they are recreated by `tome sync`
- **Local skills** (copied into the library) are tracked in version control
- **Temporary files** (`.tome-manifest.tmp`, `tome.lock.tmp`) are always ignored

This allows the library directory to serve as a git repository for portable skill management while keeping transient entries out of version control.
