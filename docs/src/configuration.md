# Configuration

Tome resolves four configuration layers. Shared policy and profiles are meant
to be version-controlled together; project configuration is scoped to one
project tree; local settings stay on the current machine.

| Layer | Default path | Owns |
|---|---|---|
| Repository policy | `~/.tome/tome.toml` | Library path, shared Git sources, shared exclusions, backup policy, conflict pins |
| Selected profile | `~/.tome/machines/<profile>.toml` | Machine-wide local sources, destinations, and destination routes |
| Project | Nearest ancestor `.tome.toml` | Additive project-only destinations and routes |
| Local settings | `~/.config/tome/settings.toml` | Selected profile and runtime consent |

Released CLI commands do not read or write `machine.toml`, and there is no
global `--machine` option. Use `--config`, `--settings`, or `--tome-home` when
the default locations are unsuitable.

## Repository Policy

The shared `tome.toml` is the repository policy:

```toml
library_dir = "~/.tome/skills"
exclude = ["deprecated-skill"]

[directories.team-skills]
path = "https://github.com/myorg/team-skills"
type = "git"
role = "source"
branch = "main"
subdir = "skills"
```

Only Git discovery sources belong in its `[directories.<name>]` map. A Git
source may use one of `branch`, `tag`, or `rev`, plus an optional `subdir`.
`tome add <git-url>` writes this layer. It registers provenance only: Git adds
have no `--to` routing option and reject `--role` because their role is always
`source`.

Top-level repository-policy fields:

| Field | Description |
|---|---|
| `library_dir` | Path to the canonical skill library; supports `~` expansion |
| `exclude` | Shared skill names omitted from discovery; manage with `tome pool exclude` and `tome pool restore` |
| `backup` | Shared backup configuration |
| `source_pins` | Persisted conflict choices for duplicate skill sources |
| `directories` | Shared Git sources only |

## Machine Profiles

Profiles are committed as `machines/<profile>.toml`. Select the active profile
with `tome profile select <name>`; `tome profile create` and
`tome profile list` manage the available files.

```toml
[directories.claude-plugins]
path = "~/.claude/plugins/cache"
type = "claude-plugins"
role = "managed"

[directories.local-skills]
path = "~/.claude/skills"
type = "directory"
role = "source"

[directories.codex]
path = "~/.codex/skills"
type = "directory"
role = "target"

[routes.codex]
tags = ["portable", "coding"]
exclude = ["claude-only-skill"]
```

Ordinary directory sources are profile-specific because their paths and tool
roles depend on the machine. Each entry combines a type and role:

| Type | Description |
|---|---|
| `claude-plugins` | Reads Claude Code's `installed_plugins.json`; role is `managed` |
| `directory` | Scans a normal directory for `*/SKILL.md` |
| `git` | Reserved for repository policy; shallow-cloned into `~/.tome/repos/<sha256>/` |

| Role | Discovery | Distribution |
|---|---:|---:|
| `managed` | Yes, read-only upstream | No |
| `source` | Yes | No |
| `target` | No | Yes |
| `synced` | Yes | Yes |

Use `tome add <path> [--role <role>]` to add an explicit local path to the
selected profile.

## Shared Skill Tags

Tags classify individual library skills independently of their source. They
are stored in each `.tome-manifest.json` skill entry and survive content
updates from the same source. New skills have an empty tag set.

```bash
tome tag add rust-cli coding
tome tag add rust-cli portable
tome tag list rust-cli
tome tag remove rust-cli coding
```

The lockfile remains provenance-only; tags are not copied into `tome.lock`.

## Destination Routes

A `[routes.<destination>]` table belongs to the same profile or project file as
the destination. Its `tags` field is an OR selector: a skill is eligible when
at least one manifest tag intersects the selected tags. `exclude` lists skills
that must not reach this destination even when a tag matches.

```toml
[routes.codex]
tags = ["portable", "coding"]
exclude = ["claude-only-skill"]
```

```bash
tome route tag add --to codex portable
tome route tag remove --to codex coding
tome route exclude add --to codex claude-only-skill
tome route exclude remove --to codex claude-only-skill
```

Untagged skills remain in the library but are not linked into destinations
that have a route. A destination without a route retains unrestricted legacy
distribution behavior, so define a route for every destination that should use
tag-based selection.

Route commands validate that the destination exists in the active profile or
nearest project layer. A route tag must already be assigned to a manifest
skill, and an excluded skill must exist in the manifest. Checked writes are
atomic.

## Project Configuration

Starting at the current working directory, Tome searches upward for the
nearest `.tome.toml`. The project layer is additive and may contain only target
directories and routes owned by those project destinations:

```toml
[directories.project-codex]
path = ".codex/skills"
type = "directory"
role = "target"

[routes.project-codex]
tags = ["project", "portable"]
exclude = ["global-only-skill"]
```

Project configuration cannot define sources, select a profile, replace a
profile destination, or route to an unknown destination. Invalid project TOML
fails the command; Tome does not silently fall back to profile-only behavior.
Running `tome route ...` inside the project tree updates the project file when
that file owns the named destination.

## Local Settings

`~/.config/tome/settings.toml` selects the active profile and stores local
runtime consent:

```toml
profile = "work"
git_sync = "ask"
managed_plugin_install = "ask"
backup_runtime = "ask"
```

| Field | Values | Description |
|---|---|---|
| `profile` | Profile name | Selects `machines/<profile>.toml` |
| `git_sync` | `always`, `ask`, `never` | Controls synchronization of the shared Tome repository |
| `managed_plugin_install` | `always`, `ask`, `never` | Controls native adapter install/update actions |
| `backup_runtime` | `always`, `ask`, `never` | Controls local backup runtime behavior |

Native plugin systems remain tool-specific installation adapters. Tome tracks
the desired skill and plugin state, invokes an adapter when consent allows,
and owns routing of portable library copies to other tools.

## Lockfile and Manifest

`tome sync` writes `tome.lock`, a reproducible provenance snapshot containing
skill names, content hashes, sources, and upstream metadata. The shared
`.tome-manifest.json` tracks the current library entry, including its tags.

Both files support a multi-machine repository workflow. `tome.lock` drives
managed-plugin reconciliation; the manifest's tags drive destination routing.

## Library `.gitignore`

`tome sync` maintains the library `.gitignore` for transient files such as
temporary lockfile writes. The canonical library contains real directory copies
for both managed and local skills.
