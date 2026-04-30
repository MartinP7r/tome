# Architecture

> **[System Diagram (Excalidraw)](https://excalidraw.com/#json=5-pjpDsna4Way3lfGW5km,p0bQwpcJEl6do68RrnKAgw)** — interactive diagram showing the two-tier discovery → library → distribution flow.

Rust workspace (edition 2024) with a single crate producing one binary.

## `crates/tome` — CLI (`tome`)

The main binary. All domain logic lives here as a library (`lib.rs` re-exports all modules) with a thin `main.rs` that parses CLI args and calls `tome::run()`.

### Sync Pipeline

The core flow that `tome sync` and `tome init` both invoke (`lib.rs::sync`):

1. **Discover** (`discover.rs`) — Walk every directory whose role is `managed`, `synced`, or `source` looking for `*/SKILL.md`. Three directory types: `ClaudePlugins` (reads `installed_plugins.json`), `Directory` (flat walkdir scan), and `Git` (shallow-clones into `~/.tome/repos/<sha256>/` and then scans the clone). First directory wins on name conflicts; the `exclude` list is applied.
2. **Consolidate** (`library.rs`) — Two strategies depending on directory role: **managed** skills (Claude plugins, git clones) are symlinked from library → source dir so the package manager continues to own the bytes; **local** skills (`directory`/`synced` sources) are copied into the library (the library is the canonical home). A manifest (`.tome-manifest.json`) tracks SHA-256 content hashes for idempotent updates: unchanged skills are skipped, changed skills are re-copied or re-linked. Stale directory state (e.g. a plain directory where a symlink should be) is automatically repaired.
3. **Distribute** (`distribute.rs`) — Push library skills to every directory whose role is `synced` or `target` via symlinks. Skills disabled in `machine.toml` (globally or per-directory) are skipped, as are directories on the `disabled_directories` list.
4. **Cleanup** (`cleanup.rs`) — Remove stale entries from the library (skills no longer in any source), broken symlinks from distribution directories, and disabled-skill symlinks. Verifies that every symlink points into the library before removing it.
5. **Lockfile** (`lockfile.rs`) — Generate `tome.lock` capturing a reproducible snapshot of the library state for diffing on the next sync.

### Other Modules

- `wizard.rs` — Interactive `tome init` setup using `dialoguer` (MultiSelect, Input, Confirm, Select). Uses the merged `KNOWN_DIRECTORIES` registry (WIZ-01, hardened in v0.7) to auto-discover common tool locations (`~/.claude/plugins/cache`, `~/.claude/skills`, `~/.codex/skills`, `~/.gemini/antigravity/skills`, etc.). Detects pre-v0.6 legacy configs and offers cleanup (WUX-03).
- `config.rs` — TOML config at `~/.tome/tome.toml`. `Config::load_or_default` handles missing files gracefully. Defines `DirectoryName`, `DirectoryType` (`ClaudePlugins`/`Directory`/`Git`), `DirectoryRole` (`Managed`/`Synced`/`Source`/`Target`), and `DirectoryConfig`. All path fields support `~` expansion. `Config::apply_machine_overrides` merges `[directory_overrides.<name>]` from `machine.toml` after expansion and before validation (PORT-01..04).
- `add.rs` / `remove.rs` / `reassign.rs` — `tome add`, `tome remove`, `tome reassign`, `tome fork` commands. All use the plan/render/execute pattern so dry-run is free and tests are trivial. `remove` aggregates partial-cleanup failures into a `Vec<RemoveFailure>` and surfaces a `⚠ N operations failed` summary.
- `git.rs` — Git clone / pull for `type = "git"` directories. Shallow clones to `~/.tome/repos/<sha256>/`, with `branch`/`tag`/`rev` ref pinning and SHA captured in the lockfile.
- `doctor.rs` — Diagnoses library issues (orphan directories, missing manifest entries, broken legacy symlinks, missing directory paths); interactive per-item repair for orphans. Annotates `(override)` for paths sourced from `machine.toml` (PORT-05).
- `status.rs` — Read-only summary of library, directories (with type/role + override annotations), and health. Single-pass directory scan for efficiency.
- `manifest.rs` — Library manifest (`.tome-manifest.json`): tracks provenance, content hashes, and sync timestamps for each skill. Provides `hash_directory()` for deterministic SHA-256 of directory contents. Atomic temp+rename writes.
- `lockfile.rs` — Generates and loads `tome.lock` files. Each entry records skill name, content hash, source directory, and provenance metadata (registry id, version, git commit SHA). Atomic temp+rename writes.
- `machine.rs` — Per-machine preferences (`~/.config/tome/machine.toml`). Tracks `disabled` skill set, `disabled_directories` set, per-directory `disabled`/`enabled` skill filtering (`DirectoryPrefs`, MACH-04), and `[directory_overrides.<name>]` path remapping (PORT-01). Atomic temp+rename writes.
- `update.rs` — Lockfile diffing and interactive triage logic, invoked by `tome sync` to surface added/changed/removed skills and offer to disable unwanted new skills.
- `paths.rs` — `TomePaths` struct bundling `tome_home`/`library_dir`/`config_dir` to prevent parameter swaps. Symlink path utilities: resolves relative symlink targets to absolute paths and checks whether a symlink points to a given destination.
- `relocate.rs` — Move the skill library to a new path with full safety guarantees: detects cross-filesystem moves, re-anchors all distribution symlinks, warns on unreadable managed-skill symlinks instead of silently dropping provenance.
- `eject.rs` — Remove all of tome's distribution symlinks (reversible via `tome sync`).
- `backup.rs` — Git-backed snapshot/restore/diff for the library. The pre-restore safety snapshot is the only recovery path if a restore was accidental, so `restore` aborts if the snapshot fails (#415).
- `browse/` — TUI browser (`tome browse`): `app.rs` (state + key handling), `ui.rs` (ratatui rendering), `theme.rs` (adaptive dark/light), `fuzzy.rs` (nucleo-matcher), `markdown.rs` (preview rendering). The status bar uses a `StatusMessage { Success | Warning | Pending }` enum (POLISH-02) so glyph + colorization stay consistent across pre-block and post-block states.
- `lint.rs` — Validates SKILL.md frontmatter; CI-friendly exit codes.
- `install.rs` — Shell completion installation.

## Key Patterns

- **Two-tier model**: Discovery directories →(consolidate)→ Library →(distribute)→ Distribution directories. The library is the source of truth. Managed skills (Claude plugins, git clones) are symlinked from library → source dir; local skills (`directory`/`synced` sources) are copied into the library. Distribution always uses Unix symlinks (`std::os::unix::fs::symlink`) pointing into the library. Unix-only.
- **Directories are data-driven**: `config::directories` is a `BTreeMap<DirectoryName, DirectoryConfig>` — any tool can be added as a directory with a role without code changes. The wizard's `KNOWN_DIRECTORIES` registry is used purely for auto-discovery convenience.
- **Roles, not "sources vs targets"**: A directory can be `managed` (read-only source), `source` (discovery only), `target` (distribution only), or `synced` (both — same dir is read AND written, e.g. `~/.claude/skills`). The pipeline asks each directory's role what to do with it; there is no separate "sources" vs "targets" config.
- **`dry_run` threading**: Most operations accept a `dry_run: bool` that skips filesystem writes but still counts what *would* change. Results report the same counts either way.
- **Atomic writes**: `manifest.json`, `tome.lock`, and `machine.toml` are always written via temp file + rename. The temp file is in the same directory as the target so the rename is atomic on POSIX.
- **Plan/render/execute**: `add`, `remove`, `reassign`, `relocate`, `eject` build an explicit plan, render it for the user, and only then execute. Dry-run is free; tests can assert plan structure without touching the filesystem.
- **Newtypes at boundaries**: `SkillName`, `DirectoryName`, `ContentHash`, `TomePaths` validate at construction so downstream code doesn't have to. The shared `validate_identifier` rejects empty names, path separators, `.`, and `..`.
- **Error handling**: `anyhow` for the application; `.with_context()` adds path context to every fs error. Missing sources/paths produce stderr warnings rather than hard errors. Symlink operations always verify the link points into the library before deleting.
- **Per-machine portability**: The portable `tome.toml` describes the abstract topology; `machine.toml` provides path overrides (`[directory_overrides.<name>]`) and machine-local opt-outs. Override application happens at config load, before validation, so all downstream code sees post-override paths.

## Testing

Unit tests are co-located with each module (`#[cfg(test)] mod tests`). Integration tests in `crates/tome/tests/cli.rs` exercise the binary via `assert_cmd`. Snapshot tests use `insta` (filtered for tmpdir paths). Tests use `tempfile::TempDir` and `assert_fs::TempDir` for filesystem isolation — no cleanup needed.

## CI

GitHub Actions runs on both `ubuntu-latest` and `macos-latest`: fmt check, clippy with `-D warnings`, tests, and release build.
