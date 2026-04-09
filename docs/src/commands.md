# Commands

| Command | Description |
|---------|-------------|
| `tome init` | Interactive wizard to configure sources and targets |
| `tome sync` | Discover, consolidate, triage changes, and distribute skills |
| `tome status` | Show library, sources, targets, and health summary |
| `tome list` (alias: `ls`) | List all discovered skills with sources (supports `--json`) |
| `tome browse` | Interactively browse discovered skills with fuzzy search |
| `tome doctor` | Diagnose and repair broken symlinks or config issues |
| `tome lint` | Validate skill frontmatter and report issues |
| `tome config` | Show current configuration |
| `tome backup` | Git-backed backup and restore for the skill library |
| `tome eject` | Remove tome's symlinks from all targets (reversible via `tome sync`) |
| `tome relocate <path>` | Move the skill library to a new location |
| `tome completions <shell>` | Install shell completions (bash, zsh, fish, powershell) |
| `tome version` | Print version information |

## Global Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--config <path>` | | Path to config file (default: `~/.tome/tome.toml`) |
| `--tome-home <path>` | | Override tome home directory (default: `~/.tome/`, or `TOME_HOME` env var) |
| `--machine <path>` | | Path to machine preferences file (default: `~/.config/tome/machine.toml`) |
| `--dry-run` | | Preview changes without modifying filesystem |
| `--no-input` | | Disable all interactive prompts (implies `--no-triage` for sync) |
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

### `tome browse`

Full-screen interactive skill browser using fuzzy search. Supports sorting, grouping by source, and per-skill actions (view source, copy path, disable/enable).

### `tome lint`

| Flag | Description |
|------|-------------|
| `PATH` | Specific skill directory to lint (default: entire library) |
| `--format text\|json` | Output format (default: `text`) |

Validates SKILL.md frontmatter: missing/mismatched names, description length, non-standard fields, Unicode tag codepoints. Exits with code 1 on errors (CI-friendly).

### `tome config`

| Flag | Description |
|------|-------------|
| `--path` | Print config file path only |

### `tome backup`

Git-backed backup and restore. Subcommands:

| Subcommand | Description |
|------------|-------------|
| `tome backup init` | Initialize git repo in the library for backup tracking |
| `tome backup snapshot [-m MSG]` | Create a snapshot of the current library state |
| `tome backup list [-n COUNT]` | Show backup history (default: 10 entries) |
| `tome backup restore [REF]` | Restore library to a previous snapshot (default: `HEAD~1`) |
| `tome backup diff [REF]` | Show changes since last backup (default: `HEAD`) |

### `tome eject`

Removes all of tome's symlinks from target tool directories. Reversible — run `tome sync` to recreate them.

### `tome relocate`

Moves the skill library to a new path, updating symlinks in all targets.

### `tome completions`

| Flag | Description |
|------|-------------|
| `SHELL` | Shell to install for: `bash`, `zsh`, `fish`, `powershell` |
| `--print` | Print completions to stdout instead of installing |
