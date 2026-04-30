# Architecture

**Analysis Date:** 2026-04-30 (v0.9.0)

## Pattern Overview

**Overall:** Two-tier discovery → library → distribution pipeline. Each configured directory declares a `type` and a `role` (`managed`/`synced`/`source`/`target`); the pipeline asks each directory's role what to do with it.

**Key Characteristics:**
- Unix-only symlink-based distribution (uses `std::os::unix::fs::symlink`)
- Idempotent consolidation with SHA-256 content hashing
- Managed vs. local dual consolidation strategies
- Data-driven directory configuration (BTreeMap-based, role-driven)
- Per-machine path overrides via `[directory_overrides.<name>]` in `machine.toml` (PORT-01..05, v0.9)
- Dry-run threading throughout all operations
- Atomic file writes with temp+rename pattern
- Plan / render / execute pattern for `add`, `remove`, `reassign`, `relocate`, `eject`

## Layers

**CLI Layer:**
- Purpose: Parse command arguments and dispatch to domain logic
- Location: `crates/tome/src/cli.rs`, `crates/tome/src/main.rs`
- Contains: `Cli` struct (parsed args), `Command` enum (subcommands), thin `main.rs` wrapper
- Depends on: Domain modules (sync, status, doctor, lint, browse, etc.)
- Used by: Entry point only

**Configuration Layer:**
- Purpose: Load, validate, and manage TOML config files
- Location: `crates/tome/src/config.rs`, `crates/tome/src/paths.rs`, `crates/tome/src/machine.rs`
- Contains: `Config` (`directories: BTreeMap<DirectoryName, DirectoryConfig>`), `DirectoryName`, `DirectoryType`, `DirectoryRole`, `SkillName`, `TomePaths` (path bundling), `MachinePrefs` (per-machine disable lists + `[directory_overrides.<name>]` path remapping)
- Depends on: `serde`, file I/O, tilde expansion
- Used by: All domain operations

**Skill Discovery:**
- Purpose: Scan sources and identify available skills
- Location: `crates/tome/src/discover.rs`
- Contains: `DiscoveredSkill`, `SkillName`, source scanners (ClaudePlugins, Directory)
- Depends on: `config.rs` (source definitions), `walkdir` (filesystem traversal)
- Used by: Sync pipeline, browse command, lint

**Consolidation Layer:**
- Purpose: Copy or symlink skills into the library
- Location: `crates/tome/src/library.rs`
- Contains: Two strategies—**managed** (symlink library→source), **local** (copy to library). Manifest-driven idempotency via content hashing.
- Depends on: `discover.rs`, `manifest.rs`, `paths.rs`
- Used by: Sync pipeline

**Distribution Layer:**
- Purpose: Distribute library skills to target tool directories
- Location: `crates/tome/src/distribute.rs`
- Contains: Symlink creation (library→target), circular symlink detection (`shares_tool_root`, `find_tool_dir`)
- Depends on: `manifest.rs`, `machine.rs` (disabled skills), `config.rs` (target config)
- Used by: Sync pipeline

**Metadata & State Management:**
- Purpose: Track skill provenance and history
- Location: `crates/tome/src/manifest.rs`, `crates/tome/src/lockfile.rs`
- Contains: `.tome-manifest.json` (SHA-256 hashes, source names, sync timestamps), `tome.lock` (reproducible snapshots)
- Depends on: `serde_json`, `sha2`, filesystem I/O
- Used by: Consolidate, distribute, cleanup, update

**Cleanup & Verification:**
- Purpose: Remove stale entries and broken symlinks
- Location: `crates/tome/src/cleanup.rs`, `crates/tome/src/doctor.rs`
- Contains: Stale skill removal, broken symlink detection, orphan directory identification
- Depends on: `manifest.rs`, `paths.rs` (symlink verification)
- Used by: Sync pipeline, doctor command

**Interactive UI:**
- Purpose: Browse and manage skills interactively
- Location: `crates/tome/src/browse/` (mod.rs, app.rs, ui.rs, fuzzy.rs)
- Contains: TUI app state (ratatui), fuzzy search (nucleo-matcher), keyboard event handling
- Depends on: `ratatui`, `crossterm` (terminal control), `nucleo-matcher` (fuzzy matching)
- Used by: browse command only

**Linting & Validation:**
- Purpose: Validate SKILL.md frontmatter and directory structure
- Location: `crates/tome/src/lint.rs`, `crates/tome/src/skill.rs`, `crates/tome/src/validation.rs`
- Contains: Frontmatter parsing (YAML), content hashing, skill name / directory name validation (shared `validate_identifier`)
- Depends on: `serde_yaml`, `sha2`, regex patterns
- Used by: Lint command, consolidate (validation)

**Sync Coordination:**
- Purpose: Orchestrate the full sync pipeline
- Location: `crates/tome/src/lib.rs` (sync function), `crates/tome/src/update.rs`
- Contains: Discover → consolidate → triage (via lockfile diff) → distribute → cleanup → save flow
- Depends on: All above layers
- Used by: `run()` entry point (init, sync commands)

**Utilities:**
- Purpose: Shared helpers and backup functionality
- Location: `crates/tome/src/backup.rs`, `crates/tome/src/eject.rs`, `crates/tome/src/relocate.rs`, `crates/tome/src/install.rs`
- Contains: Git-backed snapshots (backup), symlink removal (eject), library relocation (relocate), shell completion (install)
- Depends on: Core modules, git operations, shell integration
- Used by: Individual commands

## Data Flow

**Sync Pipeline (main flow):**

1. **Discover** → Scan all configured sources for `*/SKILL.md` directories. ClaudePlugins sources read `installed_plugins.json`; Directory sources use walkdir scan. Deduplicated by skill name (first source wins). Returns `Vec<DiscoveredSkill>`.

2. **Consolidate** → For each discovered skill, either:
   - **Managed** (from ClaudePlugins): Create symlink in library pointing to source directory. Package manager owns the files.
   - **Local** (from Directory): Copy entire directory into library. Library is the canonical home.
   - Check manifest for previous content hash. Skip if unchanged. Repair destination state if it's a stale plain directory where a symlink should be.
   - Records SHA-256 hash and provenance in manifest.

3. **Load/Diff Lockfile** → Load previous `tome.lock` (if exists) to identify new/changed/removed skills. If not first run and interactive TTY, offer user chance to disable new skills via machine.toml.

4. **Distribute** → For each library skill, create symlink in each target's skills directory (e.g., `~/.claude/skills/my-skill` → `~/.tome/skills/my-skill`). Skip skills disabled in machine preferences. Skip targets disabled in machine preferences. Detect circular symlinks (when source and target are both under same tool root, e.g., `~/.claude/`).

5. **Cleanup** → Remove stale entries:
   - Delete library skills no longer in any source (unless skipped).
   - Delete broken symlinks in target directories.
   - Delete disabled skill symlinks from target directories.
   - Verify symlinks point into library before removing.

6. **Save** → Write manifest, lockfile, and `.gitignore` to config directory.

**State Management:**
- **Manifest** (`.tome-manifest.json`): Single source of truth for what's in the library. Tracks per-skill: source path, source name, SHA-256 hash, sync timestamp, managed flag.
- **Lockfile** (`tome.lock`): Reproducible snapshot for version control. Tracks per-skill: source name, content hash, registry ID, version, git commit SHA (for managed plugins).
- **Machine Preferences** (`~/.config/tome/machine.toml`): Machine-specific disables. Separate from portable tome home so skills list stays complete across machines.
- **TomePaths**: Bundles `tome_home`, `library_dir`, `config_dir` to prevent parameter swaps.

## Key Abstractions

**SkillName:**
- Purpose: Validated, type-safe skill identifier
- Examples: `crates/tome/src/discover.rs` (SkillName type)
- Pattern: Newtype wrapper with `new()` constructor, lenient validation (rejects empty + path separators), strict convention checking (lowercase + digits + hyphens)

**DirectoryName:**
- Purpose: Validated, type-safe directory identifier
- Examples: `crates/tome/src/config.rs` (DirectoryName type)
- Pattern: Same as SkillName; prevents accidental string parameter mixing. Used as the key in `Config::directories`.

**DirectoryType:**
- Purpose: Enum-based discovery strategy
- Examples: `crates/tome/src/config.rs` (DirectoryType enum)
- Pattern: Variants = `ClaudePlugins` (reads `installed_plugins.json`), `Directory` (flat walkdir), `Git` (shallow clone into `~/.tome/repos/<sha256>/` then walk). Determines consolidation strategy.

**DirectoryRole:**
- Purpose: Enum-based pipeline role
- Examples: `crates/tome/src/config.rs` (DirectoryRole enum)
- Pattern: Variants = `Managed` (read-only source), `Synced` (source AND target — same dir is both read AND written), `Source` (discovery only), `Target` (distribution only). `is_discovery()` / `is_distribution()` accessors.

**DirectoryOverride:**
- Purpose: Per-machine path remapping for a single `[directories.<name>]` entry
- Examples: `crates/tome/src/machine.rs` (DirectoryOverride struct)
- Pattern: Lives in `MachinePrefs::directory_overrides` (`BTreeMap<DirectoryName, DirectoryOverride>`). Currently only `path` is supported (PORT-01); applied at config load before validation.

**TomePaths:**
- Purpose: Bundle tome_home + library_dir + config_dir to prevent swaps
- Examples: `crates/tome/src/paths.rs` (TomePaths struct)
- Pattern: Newtype-like pattern; absolute path validation in constructor; smart config_dir detection (either tome_home or tome_home/.tome/)

**ContentHash:**
- Purpose: SHA-256 digest for idempotent content comparison
- Examples: `crates/tome/src/validation.rs`, `crates/tome/src/manifest.rs`
- Pattern: Serialized as hex string; computed via `hash_directory()` for deterministic directory hashing

**Discovered Skill:**
- Purpose: Metadata about a discovered skill before consolidation
- Examples: `crates/tome/src/discover.rs` (DiscoveredSkill struct)
- Pattern: Captures name, path, source name, origin (managed vs. local), provenance metadata (registry_id, version, git_commit_sha)

## Entry Points

**CLI (main):**
- Location: `crates/tome/src/main.rs`
- Triggers: Binary execution
- Responsibilities: Parse CLI args via clap, call `tome::run()`

**Run Function:**
- Location: `crates/tome/src/lib.rs::run(cli: Cli)`
- Triggers: All CLI commands
- Responsibilities: Resolve paths (tome_home, config), load config, dispatch to subcommand handlers (sync, status, doctor, lint, browse, etc.)

**Sync Function:**
- Location: `crates/tome/src/lib.rs::sync(config, paths, options)`
- Triggers: `tome init`, `tome sync`
- Responsibilities: Orchestrate the full pipeline: discover → consolidate → triage → distribute → cleanup → save

**Init Wizard:**
- Location: `crates/tome/src/wizard.rs::run(dry_run)`
- Triggers: `tome init`
- Responsibilities: Interactive setup with dialoguer; auto-discovers known source/target locations; writes config

**Browse:**
- Location: `crates/tome/src/browse/mod.rs::browse(skills, manifest)`
- Triggers: `tome browse`
- Responsibilities: Launch ratatui TUI; display skill list with fuzzy search; show metadata (source, path, sync timestamp)

## Error Handling

**Strategy:** `anyhow` for application errors. Missing sources produce warnings (stderr) rather than hard errors.

**Patterns:**
- `.with_context()` to add operation context to errors
- `dry_run` parameter allows skipping filesystem writes while still counting changes
- Atomic writes (temp+rename) prevent partial updates
- Symlink verification before removal prevents cascading failures
- Manifest/lockfile parsing errors fail fast (corrupt config is unrecoverable)

## Cross-Cutting Concerns

**Logging:** Uses `eprintln!()` for warnings/errors. Progress bars via `indicatif::ProgressBar` for long operations. Controllable via `--verbose` and `--quiet` flags.

**Validation:** 
- Skill/target names: `crate::validation::validate_identifier()` (rejects empty + path separators)
- SKILL.md frontmatter: `serde_yaml::from_str()` with strict mode
- Config TOML: `toml::from_str()` with custom deserialization
- Paths: Absolute path requirements in TomePaths constructor
- Symlinks: `symlink_points_to()` verifies destination before operations

**Authentication:** None. Tome is purely filesystem-based. External tools (Claude, Gemini, etc.) manage their own auth.

**Dry-Run:** Threading throughout—`discover()`, `consolidate()`, `distribute()`, `cleanup()` all accept `dry_run: bool`. When true, reports counts without writing. Manifest/lockfile are never written in dry-run, so downstream steps see the would-be state.

---

*Architecture analysis: 2025-04-05*
