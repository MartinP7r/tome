# Agent Instructions

This file provides guidance to Claude Code (claude.ai/code) and other AI agents when working with code in this repository.

## Current State

**v0.6.0 (unreleased)** — Unified Directory Model milestone complete. Config uses `[directories.*]` BTreeMap replacing separate `[[sources]]` + `[targets.*]`. Git-backed skill repos with shallow clone, ref pinning, SHA in lockfile. Per-directory skill filtering (`enabled`/`disabled` in `machine.toml`). CLI commands: `tome add`, `tome remove`, `tome reassign`, `tome fork`. Browse TUI: adaptive theming, fuzzy match highlighting, scrollbar, markdown preview, help overlay. Known gap: wizard rewrite (WIZ-01–05) deferred.

## Quick Reference

| Document | Purpose |
|----------|---------|
| `CHANGELOG.md` | Release history and what changed per version |
| `docs/src/architecture.md` | Detailed sync pipeline and module breakdown |
| `docs/src/test-setup.md` | Test architecture, module coverage, CI pipeline |
| `docs/src/configuration.md` | TOML config format and examples |
| `docs/src/commands.md` | CLI command reference |
| `.planning/PROJECT.md` | Project context, requirements, decisions |
| `.planning/ROADMAP.md` | Milestone roadmap and phase tracking |

## Non-Interactive Shell Commands

**ALWAYS use non-interactive flags** with file operations to avoid hanging on confirmation prompts.

Shell commands like `cp`, `mv`, and `rm` may be aliased to include `-i` (interactive) mode on some systems, causing the agent to hang indefinitely waiting for y/n input.

**Use these forms instead:**
```bash
# Force overwrite without prompting
cp -f source dest           # NOT: cp source dest
mv -f source dest           # NOT: mv source dest
rm -f file                  # NOT: rm file

# For recursive operations
rm -rf directory            # NOT: rm -r directory
cp -rf source dest          # NOT: cp -r source dest
```

**Other commands that may prompt:**
- `scp` - use `-o BatchMode=yes` for non-interactive
- `ssh` - use `-o BatchMode=yes` to fail instead of prompting
- `apt-get` - use `-y` flag
- `brew` - use `HOMEBREW_NO_AUTO_UPDATE=1` env var

## Project & Task Workflow

- Tasks and roadmap tracked via **GitHub Issues** with milestones (v0.4.1, v0.4.2, v0.5, etc.)
- Project board: **"tome Execution Board"** on GitHub Projects
- Labels: `bug`, `enhancement`, `architecture`, `testing`, `documentation`, `dependencies`
- Default workflow for substantial changes: GitHub issue/idea → OpenSpec change → GSD phase/plans → implementation → archive/close
- Reference doc: `docs/src/development-workflow.md`
- Small fixes (typos, tiny bugs, narrowly scoped cleanups) do **not** need full OpenSpec + GSD overhead

## Tech Stack

Rust edition 2024. Key crates: `clap` (CLI), `dialoguer` (interactive prompts), `indicatif` (progress bars), `tabled` (table output), `walkdir` (dir traversal), `sha2` (hashing), `serde`/`toml` (config). Test crates: `assert_cmd`, `tempfile`, `assert_fs`.

## OpenSpec + Traceability

For substantial changes (new features, significant refactors, architecture-impacting work, or process changes), use the repo workflow described in `docs/src/development-workflow.md`.

### OpenSpec

Use OpenSpec to capture the planning layer:
- proposal
- design
- task checklist
- any spec deltas needed for changed behavior

Typical commands:
```bash
openspec new change <change-id>
openspec show <change-id>
openspec status --change <change-id>
openspec validate <change-id>
openspec archive <change-id>
```

### Traceability Convention

For meaningful changes, link the layers when they exist:
- GitHub issue: `#123`
- OpenSpec change: `<change-id>`
- GSD phase: `.planning/phases/<NN>-<name>/`
- Requirement IDs: as defined in `.planning/REQUIREMENTS.md`
- Commit / PR: implementation evidence

Commit-body / PR-footer shapes used in recent PRs (pick one — don't invent a new shape):
```text
Refs #123
OpenSpec: <change-id>
```
or, for PRs that close a phase:
```text
## Traceability
- Requirements: WHARD-04, WHARD-05, WHARD-06
- Phase artifacts: .planning/phases/05-wizard-test-coverage/
```

This repo uses:
- **GitHub Issues** for backlog / roadmap intent
- **OpenSpec** for requirements + design + checklist
- **GSD** (`.planning/` + `/gsd:*` commands) for phase/plan execution state
- **git / PRs** for shipped evidence

## Build & Development Commands

```bash
make build          # cargo build
make test           # cargo test (unit + integration)
make lint           # cargo clippy --all-targets -- -D warnings
make fmt            # cargo fmt
make fmt-check      # cargo fmt -- --check
make ci             # fmt-check + lint + test (matches CI pipeline)
make install        # install binary via cargo install
make build-release  # cargo build --release (LTO + strip)
make release VERSION=0.1.4  # bump version, PR, merge, tag, push (triggers CI release)
```

Run a single test:
```bash
cargo test test_name                          # by test function name
cargo test -p tome -- discover::tests        # module-scoped in a crate
cargo test -p tome --test cli                # integration tests only
```

## Architecture

> For the full deep-dive, see `docs/src/architecture.md`.

Rust workspace (edition 2024) with a single crate:

### `crates/tome` — CLI (`tome`)
The main binary. All domain logic lives here as a library (`lib.rs` re-exports all modules) with a thin `main.rs` that parses CLI args and calls `tome::run()`.

**Sync pipeline** (`lib.rs::sync`) — the core flow that `tome sync` and `tome init` both invoke:
1. **Discover** (`discover.rs`) — Scan configured sources for `*/SKILL.md` dirs. Two source types: `ClaudePlugins` (reads `installed_plugins.json`) and `Directory` (flat walkdir scan). First source wins on name conflicts; exclusion list applied.
2. **Consolidate** (`library.rs`) — Two strategies based on source type: managed skills (ClaudePlugins) are symlinked from library → source dir (package manager owns the files); local skills (Directory) are copied into the library (library is the canonical home). A manifest (`.tome-manifest.json`) tracks SHA-256 content hashes for idempotent updates: unchanged skills are skipped, changed skills are re-copied or re-linked.
3. **Distribute** (`distribute.rs`) — Push library skills to target tools via symlinks in each target's skills directory. Skills disabled in machine preferences are skipped.
4. **Cleanup** (`cleanup.rs`) — Remove stale entries from library (skills no longer in any source), broken symlinks from targets, and disabled skill symlinks from target directories. Interactive in TTY mode; auto-removes with warning otherwise.

**Other modules:**
- `wizard.rs` — Interactive `tome init` setup using `dialoguer`. Auto-discovers known directory locations. (Note: still uses legacy source/target model — wizard rewrite deferred.)
- `config.rs` — TOML config at `~/.tome/tome.toml`. `Config::load_or_default` handles missing files gracefully. All path fields support `~` expansion. `DirectoryName`, `DirectoryType`, `DirectoryRole`, `DirectoryConfig` types.
- `manifest.rs` — Library manifest (`.tome-manifest.json`): tracks provenance, content hashes, and sync timestamps for each skill. Provides `hash_directory()` for deterministic SHA-256 of directory contents.
- `doctor.rs` — Diagnoses library issues (orphan directories, missing manifest entries, broken legacy symlinks) and missing directory paths; interactive per-item repair for orphans.
- `status.rs` — Read-only summary of library, directories, and health. Single-pass directory scan for efficiency.
- `lockfile.rs` — Generates and loads `tome.lock` files. Each entry records skill name, content hash, source, and provenance metadata. Uses atomic temp+rename writes.
- `machine.rs` — Per-machine preferences (`~/.config/tome/machine.toml`). Tracks `disabled` skill set, `disabled_directories` set, and per-directory `enabled`/`disabled` skill lists. Uses atomic temp+rename writes.
- `update.rs` — Implements `tome update`: loads the previous lockfile, diffs against current state, presents changes interactively, and offers to disable unwanted new skills.
- `paths.rs` — Defines `TomePaths` (bundles `tome_home`, `library_dir`, `config_dir` to prevent parameter swaps). Also provides symlink path utilities.
- `git.rs` — Git clone/update for `type = "git"` directories. Shallow clones to `~/.tome/repos/<sha256>/`, ref pinning (branch/tag/rev), SHA reading.
- `add.rs` — `tome add <url>`: registers a git directory in config from a URL.
- `remove.rs` — `tome remove <name>`: plan/render/execute pattern for directory removal with full cleanup.
- `reassign.rs` — `tome reassign` and `tome fork`: plan/render/execute pattern for changing skill provenance.
- `browse/` — TUI browser (`tome browse`): `app.rs` (state/keys), `ui.rs` (ratatui rendering), `theme.rs` (adaptive dark/light), `fuzzy.rs` (nucleo-matcher), `markdown.rs` (preview rendering).

## Key Patterns

- **Two-tier model**: Discovery dirs →(consolidate)→ Library →(distribute)→ Distribution dirs. The library is the source of truth. Managed skills (from package managers) are symlinked from library → source dir; local skills are copied into the library. Distribution to targets uses Unix symlinks (`std::os::unix::fs::symlink`) pointing into the library. Unix-only.
- **Directories are data-driven**: `config::directories` is a `BTreeMap<DirectoryName, DirectoryConfig>` — any tool can be added as a directory with a role without code changes.
- **`dry_run` threading**: Most operations accept a `dry_run: bool` that skips filesystem writes but still counts what *would* change. Results report the same counts either way.
- **Error handling**: `anyhow` for the application. Missing sources/paths produce warnings (stderr) rather than hard errors.

## Testing

> For test architecture details, see `docs/src/test-setup.md`.

Unit tests are co-located with each module (`#[cfg(test)] mod tests`). Integration tests in `crates/tome/tests/cli.rs` exercise the binary via `assert_cmd`. Tests use `tempfile::TempDir` for filesystem isolation — no cleanup needed.

## CI

GitHub Actions runs on both `ubuntu-latest` and `macos-latest`: fmt check, clippy with `-D warnings`, tests, and release build.

## Releases

Releases are managed by [cargo-dist](https://opensource.axo.dev/cargo-dist/). The release workflow (`release.yml`) is **generated and owned by cargo-dist** — don't edit it manually.

**Important:** When bumping `cargo-dist-version` in `Cargo.toml`, always run `cargo dist init` afterwards to regenerate `release.yml`. We use `allow-dirty = ["ci"]` to tolerate Dependabot action bumps, but cargo-dist upgrades may require real workflow changes that won't be applied automatically.

## Issue Tracking

This project uses **GitHub Issues** for backlog and roadmap intent, and **GSD** (`.planning/` + `/gsd:*` commands) for phase-level execution state. Do NOT create markdown TODOs or parallel task trackers — they fall out of sync.

- **Backlog / product intent**: GitHub Issues + Milestones (`gh issue create`, project board "tome Execution Board")
- **Milestone/phase execution**: GSD planning artifacts under `.planning/` (ROADMAP.md, PROJECT.md, STATE.md, REQUIREMENTS.md) and the `/gsd:*` commands (`/gsd:progress`, `/gsd:plan-phase`, `/gsd:execute-phase`, `/gsd:verify-work`)
- **Substantial changes** also flow through OpenSpec — see `docs/src/development-workflow.md`
- **Small fixes** (typos, tiny bugs, narrowly scoped cleanups) do NOT need full OpenSpec + GSD overhead — issue → code → PR is fine

## Session Completion

**When ending a work session**, work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File follow-up issues** — Open GitHub issues (or add `/gsd:add-backlog` entries) for anything discovered that won't ship in this session.
2. **Run quality gates** — `make ci` (fmt-check, clippy -D warnings, tests) if code changed.
3. **Update planning artifacts** — Mark completed plans/phases in `.planning/`. `/gsd:execute-phase` handles this automatically on success; otherwise update STATE.md and ROADMAP.md manually.
4. **PUSH TO REMOTE** — This is MANDATORY:
   ```bash
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** — Clear stashes, prune remote branches.
6. **Verify** — All changes committed AND pushed.
7. **Hand off** — Provide context for the next session.

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds.
- NEVER stop before pushing — that leaves work stranded locally.
- NEVER say "ready to push when you are" — YOU must push.
- If push fails, resolve and retry until it succeeds.

<!-- GSD:project-start source:PROJECT.md -->
## Project

**tome v0.6 — Unified Directory Model**

tome is a CLI tool that manages AI coding agent skills across multiple tools (Claude Code, Codex, Antigravity, Cursor, etc.). It discovers skills from configured directories, consolidates them into a central library, and distributes them to target tools via symlinks. v0.6 shipped the unified directory model where each configured directory declares its type and role.

**Core Value:** Every AI coding tool on a developer's machine shares the same skill library without manual copying or per-tool configuration. One config, one library, every tool.

### Constraints

- **Platform**: Unix-only (symlinks). No Windows support.
- **Rust edition**: 2024. Strict clippy with `-D warnings`.
- **Single user**: Martin is the sole user. This unblocks hard-breaking changes but means there's no migration tooling.
- **No nested git**: Git source clones go to `~/.tome/repos/`, not inside the library dir (which may be its own git repo).
- **Backward compat**: None. Old `tome.toml` files will fail to parse. Migration is documented, not automated.
<!-- GSD:project-end -->

<!-- GSD:stack-start source:codebase/STACK.md -->
## Technology Stack

## Languages
- Rust 1.85.0+ (Edition 2024) - CLI binary (`crates/tome`) with library re-exports
## Runtime
- Standalone binary (no runtime required beyond OS)
- Targets: `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`
- Cargo (Rust 1.85.0+)
- Lockfile: `Cargo.lock` (present)
## Frameworks
- `clap` 4 - CLI argument parsing with derive macros
- `clap_complete` 4 - Shell completion generation
- `ratatui` 0.30 - Terminal UI framework (TUI) for `tome browse` command
- `crossterm` 0.29 - Terminal event handling and cursor control
- `nucleo-matcher` 0.3 - Fuzzy matching for interactive search in browse view
- `serde` 1 with derive - Serialization/deserialization framework
- `toml` 1 - TOML configuration parsing (`~/.tome/tome.toml`)
- `serde_json` 1 - JSON for manifest files (`.tome-manifest.json`, lockfiles)
- `serde_yaml` 0.9 - YAML frontmatter parsing from SKILL.md files
- `walkdir` 2 - Recursive directory traversal
- `dirs` 6 - Platform-aware home directory detection
- `tempfile` 3 (dev) - Temporary file creation for tests
- `dialoguer` 0.12 - Interactive prompts (MultiSelect, Input, Confirm, Select) in wizard
- `indicatif` 0.18 - Progress bars and spinners
- `console` 0.16 - Terminal colors and formatting
- `tabled` 0.20 - ASCII table output for `tome list` and `tome status`
- `sha2` 0.11 - SHA-256 hashing for content integrity (skill directory hashes)
- `anyhow` 1 - Error handling and context propagation
## Testing
- `assert_cmd` 2 - CLI binary assertion testing
- `assert_fs` 1 - Filesystem assertion helpers (TempDir)
- `insta` 1 with json+filters features - Snapshot testing with path redaction
- `predicates` 3 - Assertion predicates for test conditions
- Unit tests: co-located in modules via `#[cfg(test)] mod tests`
- Integration tests: `crates/tome/tests/cli.rs` exercises binary via `assert_cmd`
- Snapshot tests: stored in `crates/tome/tests/snapshots/`
## Key Dependencies
- `serde` + `toml` - Config loading/saving; schema validation via deserialization
- `walkdir` - Skill discovery from configured sources
- `sha2` - Content hashing for idempotent sync (detects unchanged skills)
- `clap` - CLI parsing and help text generation
- `dialoguer` - Interactive setup (wizard) via `tome init`
- `ratatui`/`crossterm` - Terminal UI for `tome browse` command
- `indicatif` - Progress feedback during long operations
## Build System
- Workspace manifest: `Cargo.toml` (root, defines all dependencies)
- Crate manifest: `crates/tome/Cargo.toml` (binary-specific)
- Profile configuration in root `Cargo.toml`:
- `cargo-dist` 0.30.3 - Artifact building and release automation
- Targets: Homebrew (primary), GitHub Releases (hosting)
- CI: GitHub Actions (ubuntu-latest, macos-latest)
## Configuration
- Primary config: `~/.tome/tome.toml` (TOML format)
- Per-machine prefs: `~/.config/tome/machine.toml` (disabled skills/targets)
- Library manifest: `~/.tome/.tome-manifest.json` (provenance + hashes)
- Lockfile: `~/.tome/tome.lock` (reproducibility snapshot)
- Rust formatting: `cargo fmt` (no separate prettier/rustfmt.toml)
- Linting: `cargo clippy --all-targets -- -D warnings`
- Dependency auditing: `cargo deny` (policy in `deny.toml`)
- Typo checking: `typos` CLI
- Unused dependency detection: `cargo machete`
## Platform Requirements
- Rust 1.85.0+ (via `dtolnay/rust-toolchain@stable` in CI)
- macOS (tested) or Linux (tested) — Unix-only (`std::os::unix::fs::symlink`)
- Cargo and workspace resolver v3
- macOS 10.15+ (aarch64-apple-darwin, x86_64-apple-darwin)
- Linux x86_64 (GNU libc, x86_64-unknown-linux-gnu)
- No external services or network requirements
## Dependency Audit Policy
- Multiple versions of the same crate trigger warnings (highlight all)
- Unknown registries and git sources trigger warnings
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

## Naming Patterns
- Lowercase snake_case for all module files: `discover.rs`, `library.rs`, `cleanup.rs`
- Tests co-located in same file using `#[cfg(test)] mod tests { }` blocks
- Integration tests in separate `tests/cli.rs` directory
- Lowercase snake_case: `hash_directory()`, `resolve_machine_path()`, `expand_tilde()`
- Descriptive action verbs: `discover_`, `consolidate_`, `distribute_`, `cleanup_`
- Helper functions marked with `pub(crate)` for internal use
- Lowercase snake_case: `tmp_dir`, `source_path`, `skill_name`
- Single-letter loop variables acceptable in short contexts: `for (k, v) in...`
- Collection variables use plural forms: `sources`, `targets`, `skills`, `directories`
- PascalCase for struct/enum names: `SkillName`, `DirectoryName`, `DiscoveredSkill`, `SkillOrigin`, `SyncReport`
- Newtype wrappers use transparent repr: `pub struct SkillName(String);`
- Enums descriptive and specific: `DirectoryType::ClaudePlugins`, `SkillOrigin::Managed { provenance }`
## Code Style
- `cargo fmt` (rustfmt default settings)
- No explicit `.rustfmt.toml` — uses Rust edition 2024 defaults
- Max line length: implicit, around 100-120 characters
- `cargo clippy --all-targets -- -D warnings` enforced in CI
- Clippy warnings treated as build failures (`-D warnings`)
- Use `#[allow(dead_code)]` or `#[allow(unused)]` with justification when necessary (e.g., builder pattern with optional methods)
## Import Organization
- No module path aliases used
- Full qualified paths preferred for clarity: `crate::validation::validate_identifier()`
## Error Handling
- `anyhow::Result<T>` used throughout for application-level error handling
- `anyhow::Context` trait for adding context: `.context("description of what failed")?` or `.with_context(|| format!(...))?`
- `anyhow::ensure!()` macro for validation: `ensure!(condition, "error message")`
- `anyhow::bail!()` for error returns: `bail!("descriptive error")`
- `Option::is_some_and()` for conditional checks: `p.parent().is_some_and(|d| d.exists())`
- Centralized in `crate::validation` module
- `validate_identifier()` function rejects: empty names, `.` and `..`, whitespace-only, path separators
- Newtype types enforce validation at construction time
## Logging
- User-facing errors: `eprintln!("error: {e:#}");` with debug formatting for context
- Progress/feedback: spinners via `indicatif::ProgressBar`
- Status messages: colored text via `console::style()`
- Verbose output: conditioned on `--verbose` flag
## Comments
- Above functions with `///` doc comments explaining purpose, parameters, examples
- Module-level `//!` doc comments in each module file
- Inline comments for non-obvious logic or workarounds
- Avoid redundant comments that simply restate code
- Comprehensive doc comments on all public types and functions
- Doc comments include `# Examples` sections for complex functionality
- Code examples in doc comments are formatted as executable code
## Function Design
- Accept references or owned types depending on lifetime needs: `&Path` vs `PathBuf`
- Generic constraints used where appropriate: `impl Into<String>`
- Builder patterns for complex initialization
- `anyhow::Result<T>` for fallible operations
- `Option<T>` for optional values (not defaults)
- Struct types with public fields (e.g., `SyncReport`, `DiscoveredSkill`)
## Module Design
- `pub` for public API items
- `pub(crate)` for internal-only helpers (not exported from crate root)
- `pub(crate)` on internal struct fields that should not be directly accessed
- Minimal public surface area
- No barrel re-exports (no `pub use`)
- Crate root (`lib.rs`) explicitly lists all modules and re-exports key types
## Type Safety
- Used for domain types to prevent mixing (e.g., `SkillName`, `DirectoryName`, `ContentHash`)
- Provides validation at construction time
- Implements `AsRef<str>`, `Display`, `Borrow<str>` for ergonomics
- Custom `Deserialize` impl validates on deserialization
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize)]
#[serde(transparent)]
## Trait Implementations
- `Debug` always derived
- `Clone` derived unless expensive (rare)
- `Default` implemented for configuration structs
- `Display` implemented for user-facing types
- `AsRef<T>`, `Borrow<T>`, `TryFrom<T>` for ergonomics
- `Serialize`, `Deserialize` derived for data-holding structs
- `#[serde(transparent)]` for newtype wrappers
- `#[serde(default)]` for optional fields
- Custom deserialize impls validate during parsing
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

## Pattern Overview
- Unix-only symlink-based distribution (uses `std::os::unix::fs::symlink`)
- Idempotent consolidation with SHA-256 content hashing
- Managed vs. local dual consolidation strategies
- Data-driven target configuration (BTreeMap-based)
- Dry-run threading throughout all operations
- Atomic file writes with temp+rename pattern
## Layers
- Purpose: Parse command arguments and dispatch to domain logic
- Location: `crates/tome/src/cli.rs`, `crates/tome/src/main.rs`
- Contains: `Cli` struct (parsed args), `Command` enum (subcommands), thin `main.rs` wrapper
- Depends on: Domain modules (sync, status, doctor, lint, browse, etc.)
- Used by: Entry point only
- Purpose: Load, validate, and manage TOML config files
- Location: `crates/tome/src/config.rs`, `crates/tome/src/paths.rs`, `crates/tome/src/machine.rs`
- Contains: `Config` (directories), `DirectoryName`, `SkillName`, `TomePaths` (path bundling), `MachinePrefs` (per-machine disable lists)
- Depends on: `serde`, file I/O, tilde expansion
- Used by: All domain operations
- Purpose: Scan sources and identify available skills
- Location: `crates/tome/src/discover.rs`
- Contains: `DiscoveredSkill`, `SkillName`, source scanners (ClaudePlugins, Directory)
- Depends on: `config.rs` (source definitions), `walkdir` (filesystem traversal)
- Used by: Sync pipeline, browse command, lint
- Purpose: Copy or symlink skills into the library
- Location: `crates/tome/src/library.rs`
- Contains: Two strategies—**managed** (symlink library→source), **local** (copy to library). Manifest-driven idempotency via content hashing.
- Depends on: `discover.rs`, `manifest.rs`, `paths.rs`
- Used by: Sync pipeline
- Purpose: Distribute library skills to target tool directories
- Location: `crates/tome/src/distribute.rs`
- Contains: Symlink creation (library→target), circular symlink detection (`shares_tool_root`, `find_tool_dir`)
- Depends on: `manifest.rs`, `machine.rs` (disabled skills), `config.rs` (target config)
- Used by: Sync pipeline
- Purpose: Track skill provenance and history
- Location: `crates/tome/src/manifest.rs`, `crates/tome/src/lockfile.rs`
- Contains: `.tome-manifest.json` (SHA-256 hashes, source names, sync timestamps), `tome.lock` (reproducible snapshots)
- Depends on: `serde_json`, `sha2`, filesystem I/O
- Used by: Consolidate, distribute, cleanup, update
- Purpose: Remove stale entries and broken symlinks
- Location: `crates/tome/src/cleanup.rs`, `crates/tome/src/doctor.rs`
- Contains: Stale skill removal, broken symlink detection, orphan directory identification
- Depends on: `manifest.rs`, `paths.rs` (symlink verification)
- Used by: Sync pipeline, doctor command
- Purpose: Browse and manage skills interactively
- Location: `crates/tome/src/browse/` (mod.rs, app.rs, ui.rs, fuzzy.rs)
- Contains: TUI app state (ratatui), fuzzy search (nucleo-matcher), keyboard event handling
- Depends on: `ratatui`, `crossterm` (terminal control), `nucleo-matcher` (fuzzy matching)
- Used by: browse command only
- Purpose: Validate SKILL.md frontmatter and directory structure
- Location: `crates/tome/src/lint.rs`, `crates/tome/src/skill.rs`, `crates/tome/src/validation.rs`
- Contains: Frontmatter parsing (YAML), content hashing, skill name/target name validation
- Depends on: `serde_yaml`, `sha2`, regex patterns
- Used by: Lint command, consolidate (validation)
- Purpose: Orchestrate the full sync pipeline
- Location: `crates/tome/src/lib.rs` (sync function), `crates/tome/src/update.rs`
- Contains: Discover → consolidate → triage (via lockfile diff) → distribute → cleanup → save flow
- Depends on: All above layers
- Used by: `run()` entry point (init, sync commands)
- Purpose: Shared helpers and backup functionality
- Location: `crates/tome/src/backup.rs`, `crates/tome/src/eject.rs`, `crates/tome/src/relocate.rs`, `crates/tome/src/install.rs`
- Contains: Git-backed snapshots (backup), symlink removal (eject), library relocation (relocate), shell completion (install)
- Depends on: Core modules, git operations, shell integration
- Used by: Individual commands
## Data Flow
- **Manifest** (`.tome-manifest.json`): Single source of truth for what's in the library. Tracks per-skill: source path, source name, SHA-256 hash, sync timestamp, managed flag.
- **Lockfile** (`tome.lock`): Reproducible snapshot for version control. Tracks per-skill: source name, content hash, registry ID, version, git commit SHA (for managed plugins).
- **Machine Preferences** (`~/.config/tome/machine.toml`): Machine-specific disables. Separate from portable tome home so skills list stays complete across machines.
- **TomePaths**: Bundles `tome_home`, `library_dir`, `config_dir` to prevent parameter swaps.
## Key Abstractions
- Purpose: Validated, type-safe skill identifier
- Examples: `crates/tome/src/discover.rs` (SkillName type)
- Pattern: Newtype wrapper with `new()` constructor, lenient validation (rejects empty + path separators), strict convention checking (lowercase + digits + hyphens)
- Purpose: Validated, type-safe target identifier
- Examples: `crates/tome/src/config.rs` (DirectoryName type)
- Pattern: Same as SkillName; prevents accidental string parameter mixing
- Purpose: Enum-based source discovery strategy
- Examples: `crates/tome/src/config.rs` (DirectoryType enum)
- Pattern: Variants = ClaudePlugins (plugin-based), Directory (flat walkdir), Git (remote repo). Determines consolidation strategy.
- Purpose: Bundle tome_home + library_dir + config_dir to prevent swaps
- Examples: `crates/tome/src/paths.rs` (TomePaths struct)
- Pattern: Newtype-like pattern; absolute path validation in constructor; smart config_dir detection (either tome_home or tome_home/.tome/)
- Purpose: SHA-256 digest for idempotent content comparison
- Examples: `crates/tome/src/validation.rs`, `crates/tome/src/manifest.rs`
- Pattern: Serialized as hex string; computed via `hash_directory()` for deterministic directory hashing
- Purpose: Metadata about a discovered skill before consolidation
- Examples: `crates/tome/src/discover.rs` (DiscoveredSkill struct)
- Pattern: Captures name, path, source name, origin (managed vs. local), provenance metadata (registry_id, version, git_commit_sha)
## Entry Points
- Location: `crates/tome/src/main.rs`
- Triggers: Binary execution
- Responsibilities: Parse CLI args via clap, call `tome::run()`
- Location: `crates/tome/src/lib.rs::run(cli: Cli)`
- Triggers: All CLI commands
- Responsibilities: Resolve paths (tome_home, config), load config, dispatch to subcommand handlers (sync, status, doctor, lint, browse, etc.)
- Location: `crates/tome/src/lib.rs::sync(config, paths, options)`
- Triggers: `tome init`, `tome sync`
- Responsibilities: Orchestrate the full pipeline: discover → consolidate → triage → distribute → cleanup → save
- Location: `crates/tome/src/wizard.rs::run(dry_run)`
- Triggers: `tome init`
- Responsibilities: Interactive setup with dialoguer; auto-discovers known directory locations; writes config
- Location: `crates/tome/src/browse/mod.rs::browse(skills, manifest)`
- Triggers: `tome browse`
- Responsibilities: Launch ratatui TUI; display skill list with fuzzy search; show metadata (source, path, sync timestamp)
## Error Handling
- `.with_context()` to add operation context to errors
- `dry_run` parameter allows skipping filesystem writes while still counting changes
- Atomic writes (temp+rename) prevent partial updates
- Symlink verification before removal prevents cascading failures
- Manifest/lockfile parsing errors fail fast (corrupt config is unrecoverable)
## Cross-Cutting Concerns
- Skill/target names: `crate::validation::validate_identifier()` (rejects empty + path separators)
- SKILL.md frontmatter: `serde_yaml::from_str()` with strict mode
- Config TOML: `toml::from_str()` with custom deserialization
- Paths: Absolute path requirements in TomePaths constructor
- Symlinks: `symlink_points_to()` verifies destination before operations
<!-- GSD:architecture-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd:quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd:debug` for investigation and bug fixing
- `/gsd:execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->

<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd:profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
