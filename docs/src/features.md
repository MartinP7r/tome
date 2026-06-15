# Feature List

This page is a practical inventory of the features currently shipped in `tome` as represented by the repository's current workspace version, `0.16.0`. It focuses on implemented behavior in the CLI, config model, docs, and tests. Roadmap-only items are called out separately.

## Current Scope

- `tome` is a Unix-only CLI for sharing `SKILL.md`-based agent skills across tools.
- The shipped product is the CLI plus its docs; the desktop GUI is still a roadmap item, not a current user-facing feature.
- The project is explicitly pre-`1.0`; backward compatibility is not guaranteed between releases.

## Core Product Capabilities

### 1. Unified skill library across tools

`tome` consolidates skills from multiple upstream locations into one canonical library, then distributes them to every configured tool directory.

| Capability | Current behavior |
|---|---|
| Discovery inputs | Reads skills from managed plugin caches, ordinary directories, and git repositories |
| Canonical storage | Stores all library entries as real directory copies in the library |
| Distribution model | Creates symlinks from target tools back into the library |
| Conflict handling | First discovered directory wins on duplicate skill names |
| Exclusions | Global exclude list can skip named skills during discovery |

### 2. Library-canonical sync pipeline

The main `tome sync` flow is organized into five functional stages plus lockfile generation:

| Stage | What it does |
|---|---|
| Reconcile | Compares managed skills against `tome.lock`, detects drift or vanished upstream entries, and can install or update managed plugins |
| Discover | Scans configured directories for `*/SKILL.md` skill packages |
| Consolidate | Copies managed and local skills into the library as real directories |
| Distribute | Symlinks library skills into `target` and `synced` directories |
| Cleanup | Removes stale links, cleans broken state, and transitions removed sources to Unowned entries when appropriate |
| Lockfile | Writes `tome.lock` so future syncs can diff against a reproducible snapshot |

### 3. Data-driven directory model

Instead of hardcoding individual tools, `tome` treats every configured location as a directory with a `type` and a `role`.

#### Directory types

| Type | Purpose |
|---|---|
| `claude-plugins` | Reads Claude Code marketplace plugin installs from `installed_plugins.json` |
| `directory` | Scans a normal filesystem directory for skills |
| `git` | Clones a remote repo into `~/.tome/repos/<sha256>/` and scans the clone |

#### Directory roles

| Role | Discovery | Distribution | Typical use |
|---|---|---|---|
| `managed` | Yes | No | Read-only upstream package-manager source |
| `source` | Yes | No | Local or remote skill source |
| `target` | No | Yes | Tool that only receives symlinks |
| `synced` | Yes | Yes | Tool directory that is both read from and written to |

This model lets `tome` support new tools by configuration rather than by adding new hardcoded target structs.

## Current Command Surface

### Setup and daily sync

| Command | Current feature |
|---|---|
| `tome init` | Interactive wizard that discovers common tool paths and writes config |
| `tome sync` | Main reconcile and distribution command |
| `tome status` | Summary of library health, directories, counts, and last sync time |
| `tome list` | Enumerates discovered skills; supports JSON output |
| `tome browse` | Full-screen TUI browser with fuzzy search and skill actions |

### Source and library management

| Command | Current feature |
|---|---|
| `tome add <url-or-slug>` | Adds a git-backed directory entry; supports GitHub slugs, SSH/HTTPS URLs, `/tree/<ref>/<subdir>` parsing, `--subdir`, and `--role` |
| `tome remove dir <name>` | Removes a configured directory and transitions its owned skills to Unowned |
| `tome remove skill <name>` | Deletes an Unowned skill from the library and distribution targets |
| `tome reassign <skill> --to <dir>` | Re-anchors an owned or Unowned skill to a different directory |
| `tome fork <skill> --to <dir>` | Converts a managed skill into a local editable copy |
| `tome migrate-library` | One-shot v0.9 to v0.10 library-shape migration tool |
| `tome relocate <path>` | Safely moves the library and repairs downstream links |
| `tome eject` | Removes `tome`'s distribution symlinks from targets without deleting the library |

### Inspection, validation, and repair

| Command | Current feature |
|---|---|
| `tome doctor` | Diagnoses broken paths, orphaned entries, missing sources, foreign symlinks, broken frontmatter, and target real-directory collisions; includes auto-repair paths where safe |
| `tome lint [path]` | Validates `SKILL.md` frontmatter and can emit text or JSON |
| `tome config` | Prints config or config-path information |
| `tome version` | Prints version information |

### Recovery and workflow support

| Command | Current feature |
|---|---|
| `tome backup init` | Initializes a git-backed backup repo for the skill library |
| `tome backup snapshot` | Creates a named snapshot commit |
| `tome backup list` | Shows snapshot history |
| `tome backup restore` | Restores the library to an earlier git ref |
| `tome backup diff` | Diffs the current library against a chosen backup ref |
| `tome completions <shell>` | Installs or prints shell completions |

## Notable User-Facing Features

### Interactive setup and browsing

- Wizard auto-discovery for common tool locations such as Claude Code, Codex, and Antigravity.
- TUI browsing with fuzzy filtering, markdown preview, grouping, sorting, copy-path support, and enable/disable actions.
- Interactive triage during sync when new or changed skills are found.

### Managed plugin reconciliation

- Lockfile-authoritative drift detection for managed skills.
- Per-machine consent for automatic install and update behavior via `auto_install_plugins = "always" | "ask" | "never"`.
- `--no-install` escape hatch for one-off sync runs.
- "Edited in library" detection for managed skills, with fork/skip/revert decision paths.

### Unowned skill lifecycle

- Removing a source does not automatically destroy its library content.
- Skills whose source was removed can remain in the library as Unowned entries.
- `tome doctor` can claim orphaned directories into the manifest as Unowned skills.
- `tome remove skill` provides a cleanup path for Unowned entries when they are no longer needed.

### Git source ergonomics

- Shallow clone support for remote repos.
- `branch`, `tag`, and `rev` pinning.
- GitHub `/tree/<ref>/<subdir>` URL parsing.
- `subdir` scanning for repos that keep skills below the repo root.
- Zero-skill warnings with Claude-style subdir hints when a repo layout looks wrong.

## Configuration and Portability Features

### Portable config plus machine-local overrides

`tome` splits state between a shared config and machine-specific preferences:

| File | Purpose |
|---|---|
| `~/.tome/tome.toml` | Portable topology and directory definitions |
| `~/.config/tome/machine.toml` | Machine-local disables, allowlists/blocklists, overrides, and install consent |
| `~/.tome/.tome-manifest.json` | Library provenance and content-hash state |
| `~/.tome/tome.lock` | Reproducible snapshot used for reconcile and drift detection |

### Per-machine control

- Global disabled-skill list.
- Disabled-directory list.
- Per-directory allowlist or blocklist filtering.
- Path overrides via `[directory_overrides.<name>]`.
- Override-aware diagnostics in `tome status` and `tome doctor`.

### Cross-machine workflow support

- Portable `~/`-style config path handling.
- Lockfile and manifest state designed for multi-machine sync workflows.
- Library-as-dotfiles workflow documented in [Cross-machine sync](cross-machine-sync.md).

## Safety, Reliability, and Operability

- `--dry-run` support across most destructive or state-changing flows.
- `--no-input` support for non-interactive use.
- Atomic temp-and-rename writes for manifest, lockfile, and machine preferences.
- Deterministic SHA-256 directory hashing for idempotent sync behavior.
- Foreign-symlink protection before cleanup removes target entries.
- Partial-failure reporting instead of silent cleanup or install loss.
- Structured JSON output for `status`, `doctor`, `list`, and `lint`.
- `tracing`-based logging with `--verbose`, `--quiet`, and `TOME_LOG`.
- Graceful Ctrl-C handling.

## Documentation and Verification Surface

The repository ships more than command help:

- Reference docs for [Commands](commands.md), [Configuration](configuration.md), [Architecture](architecture.md), [Cross-machine sync](cross-machine-sync.md), and [Test Setup](test-setup.md).
- Research and comparison pages such as [Tool Landscape](tool-landscape.md), [Frontmatter Compatibility](frontmatter-compatibility.md), and [Vercel Skills Comparison](vercel-skills-comparison.md).
- mdBook-based documentation build via `mdbook build`.
- CI checks for formatting, clippy, tests, and release builds across macOS and Linux.

## Roadmap Boundary

These items are explicitly not part of the shipped CLI feature set yet:

| Area | Current status |
|---|---|
| Desktop GUI | Planned for `v1.0`; current repo includes groundwork such as the `bindings` feature and `tome-desktop` crate, but the CLI remains the shipped product |
| Stable external API | Not promised yet; the project still documents "Backward compat: None" |
| Windows support | Not supported; the distribution model depends on Unix symlinks |

For future work, see [Roadmap](roadmap.md), `.planning/ROADMAP.md`, and `CHANGELOG.md`.
