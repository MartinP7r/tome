# Cross-machine sync

Tome's shared repository holds the canonical library, repository policy,
machine profiles, manifest, and lockfile. Each machine keeps only profile
selection and runtime consent in local `settings.toml`.

## Shared and Local State

Commit these shared files together:

| Path | Purpose |
|---|---|
| `tome.toml` | Repository policy, library path, shared Git sources, exclusions, and source pins |
| `machines/<profile>.toml` | Machine-wide directory topology and destination routes |
| `skills/` | Canonical real-directory copies of managed and local skills |
| `.tome-manifest.json` | Current provenance, content hashes, and shared skill tags |
| `tome.lock` | Reproducible provenance snapshot for managed reconciliation |

Do not commit `~/.config/tome/settings.toml` with the shared repository. It
selects the active profile and stores local consent:

```toml
profile = "personal-macos"
git_sync = "ask"
managed_plugin_install = "ask"
backup_runtime = "ask"
```

Released CLI commands do not use `machine.toml`; there is no `--machine`
option.

## Configure the First Machine

Create repository policy and a profile, then select it locally:

```bash
tome init
tome profile create personal-macos
tome profile select personal-macos
```

Add Git repositories as shared sources. Git registration has no destination
selection and no `--to` flag:

```bash
tome add https://github.com/my-org/my-skills.git
tome add MartinP7r/tome --subdir skills
```

Add explicit local paths to the selected profile:

```bash
tome add ~/.claude/skills --role source
tome add ~/.pfw/skills --role managed
```

Define profile destinations in `machines/personal-macos.toml`, then route tags
to them:

```toml
[directories.codex]
path = "~/.codex/skills"
type = "directory"
role = "target"

[routes.codex]
tags = ["portable", "coding"]
exclude = ["claude-only-skill"]
```

Sync once to import skills, classify them, and sync again to distribute:

```bash
tome sync
tome tag add using-tome portable
tome tag add rust-cli coding
tome route tag add --to codex portable
tome sync
```

Tags are shared manifest state. Source provenance does not decide routing. A
routed destination receives a skill when any selected tag matches, unless that
skill appears in the destination's explicit exclusion list. New upstream skills
arrive untagged and stay library-only for routed destinations until classified.

Commit the shared repository after reviewing the result:

```bash
git add tome.toml machines skills .tome-manifest.json tome.lock
git commit -m "Update Tome library"
git push
```

## Bootstrap Another Machine

Install Tome, clone the shared repository into the location used as Tome home,
and select a profile in local settings:

```bash
brew install MartinP7r/tap/tome
git clone git@github.com:you/your-tome-repository.git ~/.tome
tome profile select work-linux
tome status
tome sync --dry-run --no-install
tome sync
```

The selected `machines/work-linux.toml` can use Linux-specific paths while
sharing the same repository-owned Git sources, library tags, and lockfile. Use
separate committed profiles when machines need different paths or destinations.

`git_sync` controls whether Tome synchronizes the shared repository:

| Value | Behavior |
|---|---|
| `always` | Pull shared state before sync and publish successful changes |
| `ask` | Request consent before repository synchronization |
| `never` | Leave Git operations to the user |

For one sync, `tome sync --git-sync <always|ask|never>` overrides the local
setting without changing it.

## Project Destinations

A project may commit `.tome.toml` at its root to add destinations used only
inside that project tree:

```toml
[directories.project-codex]
path = ".codex/skills"
type = "directory"
role = "target"

[routes.project-codex]
tags = ["project", "portable"]
exclude = ["global-only-skill"]
```

Tome searches upward from the command's working directory and uses the nearest
`.tome.toml`. This layer is additive: it cannot add sources, replace profile
destinations, or select a profile. Invalid project configuration fails instead
of falling back silently.

## Native Plugin Reconciliation

Native plugins remain installed and updated through each tool's own adapter.
Tome owns the desired state represented by the shared library and lockfile, but
does not turn those plugins into a cross-tool plugin format. Portable skills
copied into the library can be routed to other `SKILL.md` destinations.

`managed_plugin_install` controls adapter actions:

```toml
managed_plugin_install = "always" # apply without prompting
managed_plugin_install = "ask"    # ask when reconciliation needs an action
managed_plugin_install = "never"  # report drift without installing
```

`tome sync --no-install` forces no adapter installs for one invocation and
does not alter local settings.

## Lockfile Semantics

Each `tome.lock` entry records the skill name, content hash, source, previous
source, version, registry identity, and Git commit when available. The lockfile
is provenance-only; shared routing tags live in `.tome-manifest.json`.

Reconciliation compares content hashes rather than treating a display version
as a complete pin. If a native adapter is unavailable, Tome reports the
adapter error. A vanished plugin can continue using its preserved canonical
library copy.

## Routing Changes Across Machines

Tag changes are shared immediately through the manifest. Route changes are
shared through the profile or project file that owns the destination. On the
next sync, Tome creates newly eligible links and removes stale Tome-owned links
that no longer match. Foreign symlinks remain untouched.

Use explicit exclusions for one destination:

```bash
tome route exclude add --to codex claude-only-skill
tome route exclude remove --to codex claude-only-skill
```

Use shared pool exclusions only when a skill should not enter the library at
all:

```bash
tome pool exclude unwanted-skill
tome pool restore unwanted-skill
```
