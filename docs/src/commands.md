# Commands

| Command | Description |
|---------|-------------|
| `tome init` | Interactive wizard to configure directories |
| `tome sync` | Reconcile, discover, consolidate, distribute, and clean up skills |
| `tome add <url\|slug>` | Register a git skill repository in `tome.toml` |
| `tome remove dir <name>` | Remove a directory entry (manifest entries transition to Unowned per LIB-04) |
| `tome remove skill <name>` | Delete an Unowned skill from the library (manifest + library + distribution + lockfile + machine.toml cleanup) |
| `tome reassign <skill> --to <directory>` | Reassign a skill to a different directory (accepts Owned + Unowned input per UNOWN-01) |
| `tome fork <skill> --to <local-directory>` | Fork a managed skill to a local directory for customization |
| `tome migrate-library` | Convert a v0.9-shape library (managed skills as symlinks) to v0.10 real-directory copies (idempotent on re-run) |
| `tome status` | Show library, directories, last-sync, and health summary |
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

#### URL forms

```bash
tome add https://github.com/user/skills           # full HTTPS URL
tome add user/skills                              # bare slug → github.com
tome add git@github.com:user/skills.git           # SSH URL
tome add user/skills/tree/main/skills             # /tree/<ref>/<subdir> shortcut (v0.13+)
tome add user/skills --subdir skills              # explicit --subdir flag (v0.13+)
```

The `/tree/<ref>/<subdir>` URL form mimics how GitHub renders navigation into a subdirectory in your browser — copy-paste from `github.com/owner/repo/tree/main/skills` and it just works. Extracted `<ref>` becomes the default branch; `<subdir>` becomes the discovery subdirectory. Explicit `--branch` / `--subdir` flags override URL-embedded values (with a warning).

#### Auto-detection of common subdirs (v0.13+)

If `tome sync` finds zero skills at a directory's root AND no `subdir` is configured, it probes common Claude Code plugin layouts (`skills/`, `.claude-plugin/skills/`) and emits a `subdir = "..."` hint if any candidate has skills inside. Catches the "I added a Claude plugin repo and got zero skills" case without forcing the user to know the convention up front.

#### Flags

| Flag | Description |
|------|-------------|
| `URL` | Git repository URL or `owner/repo` slug (optionally with `/tree/<ref>/<subdir>` suffix) |
| `--name <name>` | Custom directory name (default: extracted from URL) |
| `--branch <branch>` | Track a specific branch (overrides URL-embedded `/tree/<ref>/...`) |
| `--tag <tag>` | Pin to a specific tag |
| `--rev <sha>` | Pin to a specific commit SHA |
| `--subdir <path>` | Restrict discovery to `<clone>/<path>/*/SKILL.md` (v0.13+, overrides URL-embedded subdir) |
| `--role <role>` | Override the type-default role (v0.14+). Validated against `valid_roles()` for the chosen type. |

`--branch`, `--tag`, `--rev` are mutually exclusive.

#### Choosing the right role (v0.14+)

The `role` field decides what tome does with a configured directory:

| Role | Behavior |
|------|----------|
| `managed` | Read-only upstream (package manager owns content). Discovery only. |
| `synced` | Both discovery AND distribution — skills found here are pulled into the library, and distribution symlinks are also written back into this dir. |
| `source` | Discovery only. tome reads but never writes here. |
| `target` | Distribution only. tome writes symlinks here but doesn't scan for skills. |

**The defaults bite if you don't know them.** When you omit `--role`, the directory's role falls back to its type default:

- `claude-plugins` → `managed`
- `directory` → `synced`
- `git` → `source`

The `directory → synced` default is the one that surprises people. If you `tome add` a local directory owned by a package manager (e.g. `~/.pfw/skills/`), the `synced` default writes ~170 distribution symlinks INTO that source directory — polluting it with content tome propagated from other configured directories. **Use `--role source` for read-only package manager directories** to keep them clean.

```bash
# WRONG (default role = synced; tome writes BACK into ~/.pfw/skills/)
tome add ~/.pfw/skills

# RIGHT (explicit source; tome only reads from ~/.pfw/skills/)
tome add ~/.pfw/skills --role source
```

The success message now echoes the resolved role so you see what you got:

```
✓ Added directory 'pfw' (git: https://..., role: source)
  → Source (skills discovered here, not distributed here)
```

### `tome remove`

Split into two subcommands since v0.10 (Phase 14, D-API-2):

#### `tome remove dir <name>`

Remove a configured directory entry from `tome.toml`. Manifest entries owned by that directory transition to **Unowned** (per LIB-04) — library content is preserved on disk; only the `source_name` linkage is cleared. Aggregates partial-cleanup failures and exits non-zero with a `⚠ N operations failed` summary if any cleanup step fails. For git directories, the cached clone in `~/.tome/repos/<sha256>/` is removed.

| Flag | Description |
|------|-------------|
| `NAME` | Directory name to remove (as shown in `tome status`) |
| `--yes` / `-y` | Skip confirmation prompt |

#### `tome remove skill <name>`

Delete an **Unowned** skill from the library entirely — clears the manifest entry, removes the library directory, removes downstream distribution symlinks, removes the lockfile entry, and removes any `machine.toml` memberships. Refuses to operate on Owned skills with a hint to run `tome remove dir` first (per D-B2).

| Flag | Description |
|------|-------------|
| `NAME` | Skill name to delete |
| `--yes` / `-y` | Skip confirmation prompt (default: no) |

### `tome reassign`

Reassign a skill to a different directory — useful when the same skill appears under multiple sources and you want to pin which directory owns it. Accepts both **Owned** skills (re-anchor between configured directories) and **Unowned** skills (re-anchor a previously-stranded skill back to a configured directory, per UNOWN-01 / D-API-1).

| Flag | Description |
|------|-------------|
| `SKILL` | Skill name to reassign |
| `--to <directory>` | Target directory name (required) |
| `--force` | Overwrite if the target already has a different-content skill of the same name (per D-A1) |

### `tome fork`

Fork a managed (read-only) skill into a local directory so it can be edited. The local copy supersedes the managed one in the library.

| Flag | Description |
|------|-------------|
| `SKILL` | Skill name to fork |
| `--to <local-directory>` | Target local directory name (required) |
| `--yes` | Skip confirmation prompt |

### `tome migrate-library`

One-shot migration: convert a **v0.9-shape library** (where managed skills lived as symlinks pointing into the package manager's cache) to the **v0.10 library-canonical model** (real-directory copies). Run once after upgrading from v0.9.x; idempotent on re-run.

Shows a plan summary (skill count + per-skill disk estimate via `walkdir` + `metadata().len()`) before any conversion, then prompts for confirmation. Broken symlinks are preserved in place per Phase 11 D-04.

| Flag | Description |
|------|-------------|
| `--yes` / `-y` | Skip the confirmation prompt (bypasses the UX-02 confirm gate) |
| `--dry-run` | Render the plan; make no filesystem changes |

### `tome list`

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON |

### `tome browse`

Full-screen interactive skill browser using fuzzy search. Supports sorting, grouping by source, and per-skill actions (view source, copy path, disable/enable).

### `tome doctor`

Diagnose library state. When run interactively (no `--no-input`, no `--dry-run`), surfaces issues and offers per-category repair prompts.

#### Orphan-directory repair (v0.14+)

When `tome doctor` finds a directory in the library that has no matching manifest entry (an "orphan"), it offers four choices per orphan:

- **`claim`** — Register the orphan in the manifest as an Unowned skill (v0.14+). Hashes the directory, writes a `SkillEntry::new_unowned`, and the entry distributes to your `target` / `synced` directories on the next `tome sync`. This is the proper fix when the orphan represents a real skill you want to keep (e.g., a directory you copied in by hand, or one whose source was removed but you want to preserve it).
- **`keep`** — Leave the directory on disk; `tome sync` will re-register it IF it discovers the orphan from a configured source. Useful when you know the orphan's source got temporarily disconnected and will come back. **Note:** for library-canonical orphans with no upstream source, this option is a no-op until you `claim` it or add a source that covers it.
- **`delete`** — Remove the directory from disk permanently.
- **`skip`** — Leave the orphan as-is; doctor will surface it again on the next run.

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
