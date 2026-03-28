# Commands

| Command | Description |
|---------|-------------|
| `tome init` | Interactive wizard to configure sources and targets |
| `tome sync` | Discover, consolidate, triage changes, and distribute skills |
| `tome status` | Show library, sources, targets, and health summary |
| `tome list` (alias: `ls`) | List all discovered skills with sources (supports `--json`) |
| `tome doctor` | Diagnose and repair broken symlinks or config issues |
| `tome config` | Show current configuration |

## Global Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--config <path>` | | Path to config file (default: `~/.tome/tome.toml`) |
| `--machine <path>` | | Path to machine preferences file (default: `~/.config/tome/machine.toml`) |
| `--dry-run` | | Preview changes without modifying filesystem |
| `--verbose` | `-v` | Detailed output |
| `--quiet` | `-q` | Suppress non-error output (conflicts with `--verbose`) |

## Command Details

### `tome sync`

Runs the full pipeline: discover skills from sources, consolidate into the library, diff the lockfile to surface changes, distribute to targets, and clean up stale entries. When new or changed skills are detected, an interactive triage prompt lets you disable unwanted skills. Generates a `tome.lock` lockfile for reproducible snapshots.

| Flag | Short | Description |
|------|-------|-------------|
| `--force` | `-f` | Recreate all symlinks even if they appear up-to-date |
| `--no-triage` | | Skip interactive triage of new/changed skills (for CI/scripts) |

### `tome list`

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON |

### `tome config`

| Flag | Description |
|------|-------------|
| `--path` | Print config file path only |
