# Codebase Structure

**Analysis Date:** 2025-04-05

## Directory Layout

```
/Users/martin/code/opensource/tome/
├── crates/                              # Workspace members
│   └── tome/                            # Main crate (binary + library)
│       ├── src/                         # Source code
│       │   ├── main.rs                  # Binary entry point
│       │   ├── lib.rs                   # Library re-exports + run() entry point
│       │   ├── cli.rs                   # CLI argument parsing
│       │   ├── config.rs                # TOML config structs
│       │   ├── paths.rs                 # Path utilities + TomePaths
│       │   ├── discover.rs              # Skill discovery from sources
│       │   ├── library.rs               # Consolidation (copy/symlink)
│       │   ├── distribute.rs            # Distribution to targets
│       │   ├── cleanup.rs               # Stale entry removal
│       │   ├── manifest.rs              # .tome-manifest.json tracking
│       │   ├── lockfile.rs              # tome.lock generation
│       │   ├── machine.rs               # Per-machine preferences
│       │   ├── status.rs                # Status reporting
│       │   ├── doctor.rs                # Diagnostics + repair
│       │   ├── lint.rs                  # Validation + reporting
│       │   ├── skill.rs                 # SKILL.md parsing
│       │   ├── validation.rs            # Shared validation logic
│       │   ├── update.rs                # Lockfile diffing + triage
│       │   ├── browse/                  # TUI skill browser
│       │   │   ├── mod.rs               # Entry point + event loop
│       │   │   ├── app.rs               # App state machine
│       │   │   ├── ui.rs                # Rendering (ratatui)
│       │   │   └── fuzzy.rs             # Fuzzy search
│       │   ├── backup.rs                # Git-backed snapshots
│       │   ├── eject.rs                 # Symlink removal
│       │   ├── relocate.rs              # Library relocation
│       │   └── install.rs               # Shell completions
│       ├── tests/
│       │   ├── cli.rs                   # Integration tests (assert_cmd)
│       │   └── snapshots/               # Insta snapshot fixtures
│       └── Cargo.toml                   # Package manifest
├── docs/                                # Documentation
│   ├── src/                             # MDBook source
│   │   ├── architecture.md              # Detailed architecture
│   │   ├── commands.md                  # CLI command reference
│   │   ├── configuration.md             # Config file guide
│   │   ├── test-setup.md                # Test architecture
│   │   └── [other docs]
│   └── gfx/, visuals/, architecture/    # Diagrams + images
├── openspec/                            # OpenSpec tracking
│   ├── changes/                         # Change proposals
│   └── specs/                           # Detailed specifications
├── .claude/                             # Claude-specific config
├── .planning/                           # GSD planning directory
│   └── codebase/                        # GSD codebase docs (ARCHITECTURE.md, STRUCTURE.md, etc.)
├── .github/                             # GitHub Actions + issue templates
├── Cargo.toml                           # Workspace manifest
├── Cargo.lock                           # Dependency lock
├── Makefile                             # Make targets (build, test, lint, release)
├── ROADMAP.md                           # Version-by-version roadmap
├── CHANGELOG.md                         # Release history
└── README.md                            # Project overview
```

## Directory Purposes

**crates/tome/src/:**
- Purpose: All Rust source code for the CLI binary and library
- Contains: 27 .rs files implementing the sync pipeline, CLI, config, and utilities
- Key files: `lib.rs` (public API), `cli.rs` (arg parsing), `main.rs` (entry point)

**crates/tome/tests/:**
- Purpose: Integration tests exercising the CLI binary
- Contains: `cli.rs` (assert_cmd-based tests) + snapshot fixtures for insta
- Key files: `cli.rs` with TestEnvBuilder pattern for reproducing complex scenarios

**docs/src/:**
- Purpose: MDBook documentation source (public-facing)
- Contains: Architecture deep-dive, CLI reference, config guide, test setup, development workflow
- Key files: `architecture.md` (detailed sync pipeline), `configuration.md` (TOML schema)

**openspec/:**
- Purpose: Formal specification and change tracking
- Contains: Proposal documents, spec files, change archive
- Not generated; committed to git

**.planning/codebase/:**
- Purpose: GSD agent-generated codebase analysis documents
- Contains: ARCHITECTURE.md, STRUCTURE.md, CONVENTIONS.md, TESTING.md, CONCERNS.md
- Consumed by /gsd:plan-phase and /gsd:execute-phase commands
- Not committed to git (generated per-analysis)

## Key File Locations

**Entry Points:**
- `crates/tome/src/main.rs`: Binary entry point (thin wrapper calling `tome::run()`)
- `crates/tome/src/lib.rs`: Library entry point (`pub fn run(cli)` dispatches all commands)
- `crates/tome/src/cli.rs`: clap argument parsing (defines Cli struct and Command enum)

**Configuration:**
- `Cargo.toml`: Workspace manifest (workspace.dependencies, package metadata)
- `crates/tome/Cargo.toml`: Crate manifest (depends on workspace packages)
- `Makefile`: Build targets (make build, make test, make release)
- `deny.toml`: Dependency auditing config

**Core Logic:**
- `crates/tome/src/discover.rs`: Source scanning (ClaudePlugins, Directory)
- `crates/tome/src/library.rs`: Consolidation (copy vs. symlink strategies)
- `crates/tome/src/distribute.rs`: Target distribution (symlink creation)
- `crates/tome/src/cleanup.rs`: Stale removal and verification
- `crates/tome/src/manifest.rs`: Manifest persistence (.tome-manifest.json)
- `crates/tome/src/lockfile.rs`: Lockfile generation (tome.lock)

**Testing:**
- `crates/tome/tests/cli.rs`: Integration test suite with TestEnvBuilder
- `crates/tome/tests/snapshots/`: Insta snapshot fixtures (golden files)
- Unit tests co-located in each module (`#[cfg(test)] mod tests`)

**Interactive UI:**
- `crates/tome/src/browse/mod.rs`: Event loop + entry point
- `crates/tome/src/browse/app.rs`: App state machine
- `crates/tome/src/browse/ui.rs`: ratatui rendering
- `crates/tome/src/browse/fuzzy.rs`: nucleo-matcher fuzzy search

**Utilities & Helpers:**
- `crates/tome/src/config.rs`: TOML loading, TargetName, SkillName, tilde expansion
- `crates/tome/src/paths.rs`: TomePaths bundling, symlink resolution
- `crates/tome/src/machine.rs`: Per-machine preferences (machine.toml)
- `crates/tome/src/validation.rs`: Shared validation (identifiers, content hashing)
- `crates/tome/src/skill.rs`: SKILL.md frontmatter parsing

**Special Features:**
- `crates/tome/src/wizard.rs`: Interactive setup (dialoguer-based)
- `crates/tome/src/status.rs`: Read-only library summary
- `crates/tome/src/doctor.rs`: Diagnostics and repair
- `crates/tome/src/lint.rs`: Frontmatter validation and error reporting
- `crates/tome/src/update.rs`: Lockfile diffing for triage
- `crates/tome/src/backup.rs`: Git-backed snapshots
- `crates/tome/src/eject.rs`: Symlink removal
- `crates/tome/src/relocate.rs`: Library relocation with safety checks
- `crates/tome/src/install.rs`: Shell completion generation

## Naming Conventions

**Files:**
- `mod.rs`: Module entry point (used in `src/browse/mod.rs`)
- `<feature>.rs`: One feature per file, e.g., `discover.rs`, `manifest.rs`
- `cli.rs`: Dedicated to CLI parsing (not mixed with other logic)
- `tests/cli.rs`: Integration tests only (not unit tests)
- `snapshots/`: Insta snapshot directory (golden files)

**Directories:**
- `src/`: Rust source code only
- `tests/`: Integration tests (run binary, not unit tests)
- `docs/src/`: MDBook markdown source
- `crates/`: Workspace members (currently one: tome)
- `.planning/codebase/`: GSD analysis documents (generated)

**Modules (internal naming):**
- Private modules: Single letter or short name, no prefixes (e.g., `pub(crate) mod cleanup`)
- Public modules: Re-exported from `lib.rs` for external API (e.g., `pub mod config`, `pub mod cli`)
- Submodules: Namespace under parent (e.g., `browse/mod.rs`, `browse/app.rs`)

**Types:**
- Newtype wrappers: `SkillName(String)`, `TargetName(String)`, `ContentHash(String)`
- Enums: `Command` (subcommands), `SourceType` (discovery strategy), `TargetMethod` (distribution method)
- Structs: Descriptive names, e.g., `DiscoveredSkill`, `SkillEntry`, `ConsolidateResult`

**Functions:**
- Operations: Active verbs, e.g., `discover()`, `consolidate()`, `distribute()`, `cleanup()`
- Helpers: Descriptive, e.g., `hash_directory()`, `symlink_points_to()`, `shares_tool_root()`
- Constructors: `new()` for types with validation
- Getters: `path()`, `library_dir()`, `config_dir()`

## Where to Add New Code

**New Feature (command or subcommand):**
- Implementation: `crates/tome/src/<feature>.rs` (new module)
- CLI args: Add variant to `Command` enum in `crates/tome/src/cli.rs`
- Entry point: Add handler branch in `crates/tome/src/lib.rs::run()` match statement
- Tests: Add to `crates/tome/tests/cli.rs` using TestEnvBuilder pattern
- Example: `eject.rs`, `relocate.rs`, `backup.rs` follow this pattern

**New validation rule:**
- Shared logic: `crates/tome/src/validation.rs`
- Skill-specific: `crates/tome/src/skill.rs` (frontmatter)
- Command-specific: `crates/tome/src/lint.rs` (linting rules)

**New report/output format:**
- Report type: Define in relevant module (e.g., `ConsolidateResult` in `library.rs`)
- Rendering: Separate function (e.g., `status::show()`, `doctor::diagnose()`)
- Snapshots: Add to `crates/tome/tests/snapshots/` with insta

**Utilities and helpers:**
- Path utilities: `crates/tome/src/paths.rs`
- Validation logic: `crates/tome/src/validation.rs`
- Configuration parsing: `crates/tome/src/config.rs`
- Machine preferences: `crates/tome/src/machine.rs`

**UI components (TUI):**
- Implementation: `crates/tome/src/browse/<component>.rs`
- Event handling: Add to `App::handle_key()` in `browse/app.rs`
- Rendering: Update `browse/ui.rs::render()` function
- Search: Fuzzy matching in `browse/fuzzy.rs`

## Special Directories

**target/:**
- Purpose: Cargo build artifacts
- Generated: Yes (cargo build output)
- Committed: No (.gitignore'd)

**.planning/codebase/:**
- Purpose: GSD analysis documents
- Generated: Yes (by /gsd:map-codebase)
- Committed: No (.gitignore'd)

**docs/gfx/, docs/visuals/, docs/architecture/:**
- Purpose: Images, diagrams, architectural sketches
- Generated: No (created manually, sometimes with Excalidraw)
- Committed: Yes

## Module Dependencies

```
main.rs
  └─→ lib.rs::run()
        ├─→ cli parsing + dispatch
        ├─→ config loading
        ├─→ sync pipeline
        │     ├─→ discover.rs
        │     ├─→ library.rs
        │     ├─→ distribute.rs
        │     ├─→ cleanup.rs
        │     ├─→ manifest.rs
        │     ├─→ lockfile.rs
        │     ├─→ update.rs (triage)
        │     └─→ machine.rs (prefs)
        ├─→ status.rs (read-only)
        ├─→ doctor.rs (diagnostics)
        ├─→ lint.rs (validation)
        ├─→ browse.rs (TUI)
        ├─→ wizard.rs (init)
        ├─→ backup.rs (snapshots)
        ├─→ eject.rs (removal)
        └─→ relocate.rs (move library)

All paths resolved via:
  paths.rs (TomePaths)
  config.rs (tilde expansion)

All filesystem I/O:
  Atomic writes (temp+rename)
  Symlinks validated with symlink_points_to()
```

## Test Organization

**Integration Tests:**
- Location: `crates/tome/tests/cli.rs`
- Pattern: TestEnvBuilder for setup, `tome()` runner for execution
- Assertions: Snapshot-based (insta) for large outputs, predicates for simple checks
- Coverage: All commands (sync, status, doctor, list, lint, browse, eject, relocate, backup, completions)

**Unit Tests:**
- Location: `#[cfg(test)] mod tests` within each module
- Pattern: Inline with implementation
- Examples: `discover.rs::tests`, `skill.rs::tests`, `manifest.rs::tests`, `machine.rs::tests`

**Snapshots:**
- Location: `crates/tome/tests/snapshots/`
- Format: insta golden files (reviewed + committed)
- Redaction: Tmpdir paths replaced with `[TMPDIR]` for portability

---

*Structure analysis: 2025-04-05*
