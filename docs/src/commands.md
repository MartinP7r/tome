# Commands

| Command | Description |
|---------|-------------|
| `tome init` | Interactive wizard to configure sources and targets |
| `tome sync` | Discover, consolidate, and distribute skills |
| `tome update` | Review library changes and sync with interactive triage |
| `tome status` | Show current state of skills, symlinks, and targets |
| `tome list` (alias: `ls`) | List all discovered skills with sources |
| `tome doctor` | Diagnose and repair broken symlinks or config issues |
| `tome serve` | Start the MCP server (stdio) |
| `tome config` | Show or edit configuration |

## Global Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--config <path>` | | Path to config file (default: `~/.config/tome/config.toml`) |
| `--machine <path>` | | Path to machine preferences file (default: `~/.config/tome/machine.toml`) |
| `--dry-run` | | Preview changes without modifying filesystem |
| `--verbose` | `-v` | Detailed output |
| `--quiet` | `-q` | Suppress non-error output (conflicts with `--verbose`) |

## Command Details

### `tome sync`

Runs the full pipeline: discover skills from sources, consolidate into the library, distribute to targets, and clean up stale entries. Generates a `tome.lock` lockfile for reproducible snapshots.

| Flag | Short | Description |
|------|-------|-------------|
| `--force` | `-f` | Recreate all symlinks even if they appear up-to-date |

### `tome update`

Loads the existing `tome.lock` lockfile, diffs against the current state, and presents added/changed/removed skills interactively. Offers to disable unwanted new skills via machine preferences.

### `tome list`

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON |

### `tome config`

| Flag | Description |
|------|-------------|
| `--path` | Print config file path only |
