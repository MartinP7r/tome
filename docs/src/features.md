# Feature List

This page inventories the current CLI behavior. Desktop remains outside this
CLI release scope.

## Shared Library and Sync

Tome consolidates skills into one canonical library and distributes selected
portable skills through Unix symlinks.

| Capability | Current behavior |
|---|---|
| Discovery | Reads native plugin caches, ordinary directories, and shared Git repositories |
| Canonical storage | Stores managed and local skills as real directory copies |
| Provenance | Tracks source ownership and content hashes in `.tome-manifest.json` and `tome.lock` |
| Classification | Stores shared user-managed tags on each manifest skill |
| Distribution | Routes skills to destinations by OR-matching manifest tags |
| Exceptions | A per-destination skill exclusion overrides a matching tag |
| Conflict handling | Shared exclusions and source pins resolve pool membership |

Native tool plugins remain tool-specific installation adapters. Tome tracks the
desired plugin and skill state, reconciles through those adapters when local
consent permits, and routes portable library copies to tools that consume
`SKILL.md` packages.

## Layered Configuration

Normal commands resolve four layers:

| Layer | Purpose |
|---|---|
| `tome.toml` | Shared repository policy, Git sources, library path, exclusions, and source pins |
| `machines/<profile>.toml` | Selected profile's machine-wide sources, destinations, and routes |
| Nearest `.tome.toml` | Additive project-only destinations and routes, discovered upward from the working directory |
| `~/.config/tome/settings.toml` | Active profile and local Git, plugin-install, and backup consent |

Invalid project configuration fails explicitly. Project configuration cannot
define sources or replace profile destinations. Released commands do not read
or write `machine.toml`, and the CLI has no `--machine` option.

## Tag Routing

Sources determine library membership and provenance, not distribution. New
skills enter the manifest with no tags. Use `tome tag` to classify them and
`tome route` to select tags for a destination.

```bash
tome tag add rust-cli coding
tome tag add rust-cli portable
tome route tag add --to codex coding
tome route exclude add --to codex claude-only-skill
```

A configured route uses OR matching, so either `coding` or `portable` can
select a skill when both tags appear in the route. Untagged skills stay in the
library but are not linked into routed destinations. Destinations without a
route retain unrestricted legacy distribution behavior.

## Sync Pipeline

| Stage | What it does |
|---|---|
| Reconcile | Compares managed state with `tome.lock` and invokes native adapters when consent permits |
| Discover | Scans shared Git sources and selected-profile directory sources |
| Consolidate | Copies skills into the canonical library while preserving manifest tags |
| Distribute | Applies destination routes, explicit exclusions, and existing origin safety checks |
| Cleanup | Removes stale Tome-owned links, including links that become unrouted |
| Lockfile | Writes reproducible provenance state for the next reconciliation |

## Command Surface

### Setup and inspection

| Command | Current feature |
|---|---|
| `tome init` | Interactive setup |
| `tome profile create\|list\|select` | Manage committed profiles and local selection |
| `tome sync` | Resolve layers and run the full sync pipeline |
| `tome status` | Show the selected profile, effective directories, library health, and last sync |
| `tome config` | Show effective configuration or its shared-policy path |
| `tome list` | List discovered skills; supports JSON output |
| `tome browse` | Browse skills with fuzzy search and markdown preview |
| `tome --version` | Print the version through Clap's standard flag |

### Sources, tags, and routes

| Command | Current feature |
|---|---|
| `tome add <git-url>` | Add a repository-owned Git source; no `--to` or `--role` |
| `tome add <path> [--role]` | Add a local directory to the selected profile |
| `tome tag add\|remove\|list` | Manage tags stored in the shared manifest |
| `tome route tag add\|remove` | Manage destination tag selectors in the owning profile or project |
| `tome route exclude add\|remove` | Manage explicit per-destination skill exclusions |
| `tome pool exclude\|restore` | Remove a skill from shared discovery or restore it |

### Library lifecycle and recovery

| Command | Current feature |
|---|---|
| `tome remove dir <name>` | Remove a configured directory and preserve its skills as Unowned |
| `tome remove skill <name>` | Delete an Unowned skill from library, manifest, distributions, and lockfile |
| `tome reassign <skill> --to <dir>` | Re-anchor an owned or Unowned skill |
| `tome fork <skill> --to <dir>` | Convert a managed skill into a local editable copy |
| `tome doctor` | Diagnose and safely repair supported library and destination problems |
| `tome lint [path]` | Validate `SKILL.md` frontmatter in text or JSON form |
| `tome relocate <path>` | Move the library and repair downstream links |
| `tome eject` | Remove Tome-owned distribution symlinks without deleting the library |
| `tome backup ...` | Create, list, diff, and restore Git-backed library snapshots |
| `tome completions <shell>` | Install or print shell completions |

The obsolete `tome migrate-library`, `tome version`, and `tome remove pool`
commands are not part of the current CLI.

## Safety and Operability

- Checked atomic writes for repository policy, profiles, project config, local
  settings, the manifest, and the lockfile.
- `--dry-run` for state-changing flows and `--no-input` for automation.
- `--no-install` and `--git-sync` one-run consent overrides.
- Deterministic SHA-256 hashing for idempotent consolidation.
- Foreign-symlink protection before cleanup removes destination entries.
- Structured JSON output for status, doctor, list, and lint.
- `tracing` logging through `--verbose`, `--quiet`, and `TOME_LOG`.

## Platform Boundary

Tome is Unix-only because distribution uses symlinks. The CLI remains pre-1.0
and does not promise backward compatibility. Desktop code exists in the
workspace but is paused and outside this CLI release.
