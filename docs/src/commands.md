# Commands

| Command | Description |
|---------|-------------|
| `tome init` | Interactive wizard to configure directories |
| `tome sync` | Discover, consolidate, triage changes, and distribute skills |
| `tome add <url\|slug>` | Register a git skill repository in `tome.toml` |
| `tome remove <name>` | Remove a directory entry and clean up its artifacts |
| `tome reassign <skill> <directory>` | Reassign a skill to a different directory |
| `tome fork <skill> <local-directory>` | Fork a managed skill to a local directory for customization |
| `tome status` | Show library, directories, and health summary |
| `tome list` (alias: `ls`) | List all discovered skills with their directories (supports `--json`) |
| `tome browse` | Interactively browse discovered skills with fuzzy search |
| `tome doctor` | Diagnose and repair broken symlinks or config issues |
| `tome lint` | Validate skill frontmatter and report issues |
| `tome config` | Show current configuration |
| `tome backup` | Git-backed backup and restore for the skill library |
| `tome eject` | Remove tome's symlinks from all distribution directories (reversible via `tome sync`) |
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

Runs the full pipeline: discover skills from configured directories, consolidate into the library, diff the lockfile to surface changes, distribute to targets, and clean up stale entries. When new or changed skills are detected, an interactive triage prompt lets you disable unwanted skills. Generates a `tome.lock` lockfile for reproducible snapshots.

| Flag | Short | Description |
|------|-------|-------------|
| `--force` | `-f` | Recreate all symlinks even if they appear up-to-date |
| `--no-triage` | | Skip interactive triage of new/changed skills (for CI/scripts) |

### `tome add`

Register a git skill repository in `tome.toml`. Accepts either a full git URL (`https://github.com/owner/repo`, `git@github.com:owner/repo.git`) or a bare GitHub slug (`owner/repo`), which is expanded to `https://github.com/owner/repo` (v0.8.2+). The clone is shallow and lives in `~/.tome/repos/<sha256>/`.

| Flag | Description |
|------|-------------|
| `URL` | Git repository URL or `owner/repo` slug |
| `--name <name>` | Custom directory name (default: extracted from URL) |
| `--branch <branch>` | Track a specific branch |
| `--tag <tag>` | Pin to a specific tag |
| `--rev <sha>` | Pin to a specific commit SHA |

`--branch`, `--tag`, `--rev` are mutually exclusive.

### `tome remove`

Remove a directory entry and clean up all its artifacts: distribution symlinks, library entries, library symlinks, and (for git directories) the cached clone. Aggregates partial-cleanup failures and exits non-zero with a `⚠ N operations failed` summary if any cleanup step fails (the directory's config entry and manifest entries are preserved on partial failure so the command can be re-run after fixing the underlying cause).

| Flag | Description |
|------|-------------|
| `NAME` | Directory name to remove (as shown in `tome status`) |
| `--yes` | Skip confirmation prompt |

### `tome reassign`

Reassign a skill to a different directory — useful when the same skill appears under multiple sources and you want to pin which directory owns it.

### `tome fork`

Fork a managed (read-only) skill into a local directory so it can be edited. The local copy supersedes the managed one in the library.

| Flag | Description |
|------|-------------|
| `--yes` | Skip confirmation prompt |

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

Removes all of tome's symlinks from distribution directories. Reversible — run `tome sync` to recreate them.

### `tome relocate`

Moves the skill library to a new path, updating symlinks in all distribution directories. Detects cross-filesystem moves and warns when target symlinks need to be re-anchored.

### `tome completions`

| Flag | Description |
|------|-------------|
| `SHELL` | Shell to install for: `bash`, `zsh`, `fish`, `powershell` |
| `--print` | Print completions to stdout instead of installing |
