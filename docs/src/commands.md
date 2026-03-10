# Commands

| Command | Description |
|---------|-------------|
| `tome init` | Interactive wizard to configure sources and targets |
| `tome sync` | Discover, consolidate, and distribute skills |
| `tome status` | Show library, sources, targets, and health |
| `tome list` (alias: `ls`) | List all discovered skills with sources |
| `tome doctor` | Diagnose and repair library issues |
| `tome serve` | Start the MCP server (stdio) |
| `tome config` | Show current configuration |

## Global Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--config <path>` | | Path to config file (default: `~/.config/tome/config.toml`) |
| `--dry-run` | | Preview changes without modifying filesystem |
| `--verbose` | `-v` | Detailed output |
| `--quiet` | `-q` | Suppress non-error output (conflicts with `--verbose`) |

## Command-Specific Flags

### `tome sync`

| Flag | Short | Description |
|------|-------|-------------|
| `--force` | `-f` | Recreate all symlinks even if they appear up-to-date |

### `tome list`

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON |

### `tome config`

| Flag | Description |
|------|-------------|
| `--path` | Print config file path only |
