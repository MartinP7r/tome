# Configuration

tome reads two TOML files:

- `~/.tome/tome.toml` — the **portable** config (intended to be shared via dotfiles across machines).
- `~/.config/tome/machine.toml` — **machine-local** preferences and path overrides (do *not* share this).

The split is intentional: the portable config describes the abstract topology (which directories tome cares about, what role each plays), while `machine.toml` describes how that topology maps onto *this* machine's filesystem.

## `tome.toml` — Portable Config

```toml
library_dir = "~/.tome/skills"
exclude = ["deprecated-skill"]

[directories.claude-plugins]
path = "~/.claude/plugins/cache"
type = "claude-plugins"

[directories.local-skills]
path = "~/.claude/skills"
type = "directory"
role = "synced"

[directories.team-skills]
path = "https://github.com/myorg/team-skills"
type = "git"
branch = "main"

[directories.antigravity]
path = "~/.gemini/antigravity/skills"
type = "directory"
role = "target"
```

> **Migrating from v0.5 or earlier?** The `[[sources]]` and `[targets.*]` sections were replaced with a single `[directories.<name>]` map in v0.6. tome will refuse to load old-format configs and print a migration hint. There is no automated migration tool — copy each `[[sources]]` entry to a `[directories.<name>]` entry with `role = "source"` (or `"managed"` for `claude-plugins`), and each `[targets.<name>]` entry to a `[directories.<name>]` entry with `role = "target"`.

### Top-level fields

| Field | Description |
|-------|-------------|
| `library_dir` | Path to the consolidated skill library. Supports `~` expansion. |
| `exclude` | List of skill names to skip during discovery. |

### `[directories.<name>]` — entries

A `<name>` is a kebab-case identifier. Each entry combines a `type` (how skills are discovered) with a `role` (whether it's a source, a target, or both).

| Field | Required | Description |
|-------|----------|-------------|
| `path` | Yes | Filesystem path (or git URL when `type = "git"`). Tilde-expanded. |
| `type` | No (defaults to `"directory"`) | One of `claude-plugins`, `directory`, `git`. |
| `role` | No (each `type` has a default) | One of `managed`, `synced`, `source`, `target`. |
| `branch` / `tag` / `rev` | No (`git` only, mutually exclusive) | Pin a git directory to a branch, tag, or commit SHA. |
| `subdir` | No (`git` only) | If the repo nests skills under a subdirectory. |

### Directory `type`

| Type | Description |
|------|-------------|
| `claude-plugins` | Reads `installed_plugins.json` from the Claude Code plugin cache. Supports v1 (flat array) and v2 (namespaced object) formats. Always `role = "managed"`. |
| `directory` | Flat scan for `*/SKILL.md` directories. Default. |
| `git` | Shallow-clones a remote repo into `~/.tome/repos/<sha256>/` and treats the clone as a `directory` source. Always `role = "source"`. |

### Directory `role`

| Role | Discovery | Distribution | Typical use |
|------|-----------|--------------|-------------|
| `managed` | ✓ (read-only) | — | Plugin cache (e.g. Claude Code) |
| `synced` | ✓ | ✓ | A directory that is both a skill source AND a tool that consumes them (e.g. `~/.claude/skills`) |
| `source` | ✓ | — | A skill repo or local skill directory |
| `target` | — | ✓ | A tool that only receives skills (e.g. Codex, Antigravity) |

`tome init` picks a sensible default role from the type, but you can override it per directory.

The directory model is fully data-driven: any new tool can be supported by adding a `[directories.<name>]` entry — no code changes required. The `tome init` wizard auto-discovers common tool locations via the built-in `KNOWN_DIRECTORIES` registry.

## `machine.toml` — Machine-Local Preferences

```toml
# Skip these skills entirely on this machine
disabled = ["noisy-skill", "work-only-skill"]

# Don't distribute to these directories on this machine
disabled_directories = ["openclaw"]

# Per-directory skill filtering (mutually exclusive: pick disabled OR enabled per directory)
[directory.antigravity]
disabled = ["claude-only-skill"]

[directory.work-laptop]
enabled = ["work-skill-a", "work-skill-b"]  # allowlist — ONLY these are distributed

# Per-machine path overrides for `tome.toml::directories.<name>.path` (PORT-01..05, v0.9)
[directory_overrides.local-skills]
path = "/Users/alice-corp/.claude/skills"

[directory_overrides.team-skills]
path = "/opt/shared/team-skills"
```

| Field | Description |
|-------|-------------|
| `disabled` | List of skill names to skip during distribution (no symlinks created in any target). |
| `disabled_directories` | List of directory names to skip entirely on this machine. |
| `[directory.<name>].disabled` | Skills to exclude from a single directory (blocklist). |
| `[directory.<name>].enabled` | Allowlist — ONLY these skills are distributed to this directory. Mutually exclusive with `disabled` per directory (MACH-04). |
| `[directory_overrides.<name>].path` | Replaces `directories.<name>.path` on this machine. Useful when the same `tome.toml` is shared across machines with different home layouts. Unknown override names emit a typo-target stderr warning. |

Override application happens at config load (after tilde expansion, before `Config::validate`), so all downstream code sees the canonical post-override paths. Any validation failure caused by an override is wrapped with an error attributing the problem to `machine.toml` rather than the portable `tome.toml`.

`tome status` and `tome doctor` annotate `(override)` next to any path that came from `machine.toml`, so you can tell at a glance which paths are portable and which are machine-local.

The `--machine <path>` global flag overrides the default machine preferences path.

## Lockfile

`tome sync` generates a `tome.lock` file in the tome home directory (`~/.tome/tome.lock`). This lockfile captures a reproducible snapshot of all skills — their names, content hashes, sources, and provenance metadata. Each sync diffs the new lockfile against the previous one and surfaces changes interactively.

The lockfile is designed to be committed to version control alongside the library, enabling multi-machine workflows where `tome sync` on a new machine can detect what changed since the last sync.

## Library `.gitignore`

`tome sync` automatically generates a `.gitignore` in the library directory:

- **Managed skills** (symlinked from package managers) are gitignored — they are recreated by `tome sync`
- **Local skills** (copied into the library) are tracked in version control
- **Temporary files** (`tome.lock.tmp`) are always ignored

This allows the library directory to serve as a git repository for portable skill management while keeping transient entries out of version control.
