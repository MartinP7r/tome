# Architecture Patterns

**Domain:** Rust CLI config model refactor (unified directory model)
**Researched:** 2026-04-10

## Recommended Architecture

The unified directory model replaces two parallel config collections (`sources: Vec<Source>` + `targets: BTreeMap<TargetName, TargetConfig>`) with a single `directories: BTreeMap<DirectoryName, DirectoryConfig>`. The sync pipeline stages remain the same, but their inputs come from role-based iterators over the unified map instead of separate config fields.

### Core Concept: Role Determines Pipeline Participation

Each directory declares a `role` that determines which pipeline stages it participates in:

| Role | Discovers From? | Distributes To? | Consolidation Strategy |
|------|----------------|-----------------|----------------------|
| `Managed` | Yes | No | Symlink library -> source (package manager owns files) |
| `Synced` | Yes | Yes | Copy into library (library is canonical home) |
| `Source` | Yes | No | Copy into library (read-only external, e.g., git clone) |
| `Target` | No | Yes | N/A (receive-only) |

**Key insight:** `Synced` is the role that replaces today's source-target overlap (e.g., `~/.claude/skills` appears as both source and target). A synced directory both contributes skills to the library AND receives all library skills back. This eliminates `find_source_target_overlaps()` entirely.

### Component Boundaries

| Component | Responsibility | Communicates With | Changes in v0.6 |
|-----------|---------------|-------------------|-----------------|
| `config.rs` | Parse TOML, expose `DirectoryConfig` | All modules | **Major rewrite.** `Source`, `SourceType`, `TargetConfig`, `TargetMethod` replaced by `DirectoryConfig`, `DirectoryRole`, `DirectoryType`. New `DirectoryName` newtype. |
| `config.rs` (iterators) | Role-based directory filtering | discover, distribute, cleanup | **New.** `discovery_dirs()` -> dirs with role in {Managed, Synced, Source}. `distribution_dirs()` -> dirs with role in {Synced, Target}. |
| `discover.rs` | Scan directories for skills | config (via iterators), library | **Moderate.** `discover_all()` takes `impl Iterator<Item = &DirectoryConfig>` instead of `&Config`. `DiscoveredSkill.source_name` becomes `directory_name: DirectoryName`. |
| `library.rs` | Consolidate skills into library | discover, manifest | **Minor.** Consolidation strategy derived from `DirectoryRole` instead of `SourceType`. Managed -> symlink, Synced/Source -> copy. |
| `distribute.rs` | Push library skills to directories | config (via iterators), manifest, machine | **Moderate.** Iterates `distribution_dirs()` instead of `config.targets`. Circular symlink detection uses `DirectoryRole::Synced` check instead of path-based `shares_tool_root()`. |
| `cleanup.rs` | Remove stale entries | manifest, config | **Minor.** Cleanup targets come from `distribution_dirs()`. |
| `manifest.rs` | Track skill provenance | library, lockfile | **Field rename.** `SkillEntry.source_name` -> `directory_name`. Manifest schema version bump. |
| `lockfile.rs` | Reproducible snapshot | manifest, discover | **Field rename.** `LockEntry.source_name` -> `directory_name`. Lockfile schema version bump. |
| `machine.rs` | Per-machine disables | distribute, cleanup | **Rename.** `disabled_targets` -> `disabled_directories` (or keep both during transition). |
| `wizard.rs` | Interactive setup | config | **Major rewrite.** Merged `KNOWN_SOURCES` + `KNOWN_TARGETS` into `KNOWN_DIRECTORIES` with default roles. |
| `paths.rs` | Path bundling | all | **No change.** `TomePaths` stays as-is. |
| `lib.rs` (sync) | Pipeline orchestration | all | **Moderate.** Replace `config.sources` / `config.targets` iteration with `config.discovery_dirs()` / `config.distribution_dirs()`. |

### Data Flow

**Current flow:**
```
Config.sources ---> discover_all() ---> consolidate() ---> manifest
Config.targets ---> distribute_to_target() (loop) ---> target dirs
```

**Proposed flow:**
```
Config.directories
  |
  ├── .discovery_dirs() ---> discover_all() ---> consolidate() ---> manifest
  |                                                                    |
  └── .distribution_dirs() ---> distribute_to_target() (loop) <-------+
                                        |
                                        v
                                   target/synced dirs
```

The manifest and lockfile sit between consolidation and distribution, unchanged in role but referencing `directory_name` instead of `source_name`.

### Git Source Resolution in the Pipeline

Git directories (`type = "git"`) introduce a new pre-discovery step:

```
Config.directories
  |
  ├── .git_dirs() ---> git_resolve() ---> clone/pull to ~/.tome/repos/<hash>/
  |                         |
  |                    updates DirectoryConfig.effective_path (runtime only)
  |
  ├── .discovery_dirs() ---> discover_all() ... (uses effective_path)
  └── .distribution_dirs() ---> distribute_to_target() ...
```

**Where git resolution fits:** Before discovery, after config load. It is a separate step because:
1. It requires network I/O (clone/pull), which is slow and fallible
2. It mutates runtime state (the effective path to scan) but NOT the config file
3. It needs its own progress indicator and error handling (network failures are warnings, not fatal)

**Implementation approach:** Add a `resolve_git_directories()` function called from `sync()` between config load and discovery. It returns a `ResolvedDirectories` struct that wraps the config's directories with effective paths (original path for non-git dirs, `~/.tome/repos/<hash>/` for git dirs). Discovery then takes `&ResolvedDirectories` instead of raw config iterators.

### DirectoryConfig Structure

```rust
/// Unified directory configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryConfig {
    /// Filesystem path to the directory (or git URL for git type)
    pub path: PathBuf,
    /// What kind of directory this is
    #[serde(rename = "type")]
    pub dir_type: DirectoryType,
    /// How this directory participates in the sync pipeline
    pub role: DirectoryRole,
    /// Whether this directory is enabled (default: true)
    #[serde(default = "defaults::enabled")]
    pub enabled: bool,
}

/// The type of a directory - determines discovery mechanism.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DirectoryType {
    /// Reads installed_plugins.json for discovery
    ClaudePlugins,
    /// Scans for */SKILL.md directly
    Directory,
    /// Git repository (clone/pull)
    Git,
}

/// How a directory participates in the pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DirectoryRole {
    /// Package-managed source. Discovers skills. Symlinked into library.
    Managed,
    /// Bidirectional. Discovers skills AND receives distribution.
    Synced,
    /// Read-only source. Discovers skills. Copied into library.
    Source,
    /// Receive-only. Gets symlinks from library.
    Target,
}

/// A validated directory name (replaces both SourceName and TargetName for config keys).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DirectoryName(String);
```

### TOML Config Shape

```toml
library_dir = "~/.tome/skills"

[directories.claude-plugins]
path = "~/.claude/plugins"
type = "claude-plugins"
role = "managed"

[directories.claude-skills]
path = "~/.claude/skills"
type = "directory"
role = "synced"

[directories.codex]
path = "~/.codex/skills"
type = "directory"
role = "target"

[directories.community-skills]
path = "https://github.com/user/skills-repo.git"
type = "git"
role = "source"

[backup]
enabled = true
```

## Patterns to Follow

### Pattern 1: Role-Based Iterator Methods on Config

**What:** Config exposes filtered iterator methods instead of exposing the raw BTreeMap to pipeline stages.

**When:** Any module needs to know which directories participate in its stage.

**Example:**
```rust
impl Config {
    /// Directories that contribute skills (discovery sources).
    pub fn discovery_dirs(&self) -> impl Iterator<Item = (&DirectoryName, &DirectoryConfig)> {
        self.directories.iter().filter(|(_, d)| {
            d.enabled && matches!(d.role, DirectoryRole::Managed | DirectoryRole::Synced | DirectoryRole::Source)
        })
    }

    /// Directories that receive skills (distribution targets).
    pub fn distribution_dirs(&self) -> impl Iterator<Item = (&DirectoryName, &DirectoryConfig)> {
        self.directories.iter().filter(|(_, d)| {
            d.enabled && matches!(d.role, DirectoryRole::Synced | DirectoryRole::Target)
        })
    }

    /// Directories that need git resolution before discovery.
    pub fn git_dirs(&self) -> impl Iterator<Item = (&DirectoryName, &DirectoryConfig)> {
        self.directories.iter().filter(|(_, d)| {
            d.enabled && d.dir_type == DirectoryType::Git
        })
    }
}
```

**Why this pattern:** Pipeline stages should not know about roles they do not care about. Discovery should not know what a Target is. Distribution should not know what a Source is. The iterator methods enforce this boundary.

### Pattern 2: Consolidation Strategy Derived from Role

**What:** The consolidation strategy (symlink vs. copy) is determined by directory role, not a separate type enum.

**When:** `library::consolidate()` decides how to bring a skill into the library.

**Example:**
```rust
fn strategy_for_role(role: &DirectoryRole) -> ConsolidationStrategy {
    match role {
        DirectoryRole::Managed => ConsolidationStrategy::Symlink,
        DirectoryRole::Synced | DirectoryRole::Source => ConsolidationStrategy::Copy,
        DirectoryRole::Target => unreachable!("targets don't consolidate"),
    }
}
```

**Why:** Today `SourceType::ClaudePlugins` implies Managed implies Symlink. With the unified model, the role is the semantic signal, not the type. A `DirectoryType::ClaudePlugins` directory is almost always `Managed`, but the role is what determines behavior.

### Pattern 3: Circular Symlink Prevention via Role

**What:** Replace path-based `shares_tool_root()` detection with a simple role check.

**When:** During distribution, deciding whether to skip a skill for a particular directory.

**Example:**
```rust
// Old: path-based heuristic
if skill_entry.managed && shares_tool_root(&source_paths, &skill_entry.source_path, skills_dir) {
    skipped_managed += 1;
    continue;
}

// New: role-based check
// If the skill came from this same directory (name match) and it's Synced, skip it
// to avoid circular symlinks
if skill_entry.directory_name == target_dir_name
    && matches!(target_dir.role, DirectoryRole::Synced) {
    skipped_circular += 1;
    continue;
}
```

**Why:** The path-based approach is fragile (relies on parent directory comparison). The role-based approach is explicit: a Synced directory that contributed a skill should not receive a symlink back to that same skill.

### Pattern 4: DirectoryName Replaces Both SourceName and TargetName in State

**What:** Manifest and lockfile entries reference skills by `directory_name` (a `DirectoryName`) instead of `source_name` (a plain `String`).

**When:** Any time provenance is recorded or queried.

**Example:**
```rust
// manifest.rs
pub struct SkillEntry {
    pub source_path: PathBuf,
    pub directory_name: DirectoryName,  // was: source_name: String
    pub content_hash: ContentHash,
    pub synced_at: String,
    pub managed: bool,
}

// lockfile.rs
pub struct LockEntry {
    pub directory_name: DirectoryName,  // was: source_name: String
    pub content_hash: ContentHash,
    pub registry_id: Option<String>,
    pub version: Option<String>,
    pub git_commit_sha: Option<String>,
}
```

## Anti-Patterns to Avoid

### Anti-Pattern 1: Leaking Role Knowledge Across Pipeline Stages

**What:** Distribution code checking `DirectoryRole::Managed` or `DirectoryRole::Source`.

**Why bad:** Distribution should only know about directories it receives from `distribution_dirs()`. If it's checking whether something is Managed or Source, the boundary is wrong.

**Instead:** Let the iterator filter handle role selection. Distribution code treats all directories it receives identically (create symlinks).

### Anti-Pattern 2: Storing Effective Paths in Config

**What:** Mutating `DirectoryConfig.path` after git resolution to store the cloned repo path.

**Why bad:** Config represents what the user wrote in `tome.toml`. Runtime state (resolved clone paths) should not mutate config. This breaks dry-run, re-serialization, and mental model.

**Instead:** Use a `ResolvedDirectories` wrapper that pairs each `DirectoryConfig` with an `effective_path: PathBuf`. Non-git directories have `effective_path == config.path`. Git directories have `effective_path == ~/.tome/repos/<hash>/`.

### Anti-Pattern 3: Preserving TargetName/SourceName Alongside DirectoryName

**What:** Keeping `TargetName` and `SourceType` types alive for backward compatibility.

**Why bad:** Single user, hard break. Dual type systems create confusion and bugs. The old types add no value.

**Instead:** Delete `TargetName`, `Source`, `SourceType`, `TargetConfig`, `TargetMethod` entirely. Replace with `DirectoryName`, `DirectoryConfig`, `DirectoryRole`, `DirectoryType`. Clean break.

### Anti-Pattern 4: Priority Field for Duplicate Resolution

**What:** Adding a `priority: u32` field to `DirectoryConfig` for duplicate skill resolution.

**Why bad:** Premature complexity. BTreeMap alphabetical ordering is deterministic and sufficient. Conflicts are rare. If needed later, a `priority` field can be added without breaking changes.

**Instead:** Document that BTreeMap key order (alphabetical) determines priority. First matching directory wins. Log a warning on conflicts.

## Suggested Build Order

The refactor has clear dependency layers. Build bottom-up:

### Phase 1: Config Foundation (must be first)

**Files:** `config.rs`, `validation.rs`

1. Define `DirectoryName`, `DirectoryConfig`, `DirectoryRole`, `DirectoryType`
2. Replace `Config.sources` + `Config.targets` with `Config.directories`
3. Add `discovery_dirs()`, `distribution_dirs()`, `git_dirs()` iterator methods
4. Delete `Source`, `SourceType`, `TargetConfig`, `TargetMethod`, `TargetName`
5. Update `Config` serde (de)serialization and `validate()` method

**Why first:** Every other module imports from config. This is the foundation.

**Risk:** This breaks every module simultaneously. The crate will not compile until all consumers are updated. Consider a scratch branch for this phase.

### Phase 2: State Schema Migration (depends on Phase 1)

**Files:** `manifest.rs`, `lockfile.rs`

1. Rename `SkillEntry.source_name` -> `directory_name: DirectoryName`
2. Rename `LockEntry.source_name` -> `directory_name: DirectoryName`
3. Bump manifest/lockfile schema versions
4. Update `generate()` functions to accept `DirectoryName`

**Why second:** Manifest and lockfile are consumed by consolidation, distribution, and cleanup. Their schema must be settled before pipeline stages are updated.

### Phase 3: Discovery Adaptation (depends on Phase 1)

**Files:** `discover.rs`

1. Change `discover_all()` to accept an iterator of `(&DirectoryName, &DirectoryConfig)` instead of `&Config`
2. Derive `SkillOrigin` from `DirectoryRole` instead of `SourceType`
3. Replace `DiscoveredSkill.source_name: String` with `directory_name: DirectoryName`
4. Keep `discover_source()` internal, dispatch on `DirectoryType`

**Why here:** Discovery is the first pipeline stage. It produces `DiscoveredSkill` which flows into consolidation and lockfile generation.

### Phase 4: Consolidation + Distribution (depends on Phases 1-3)

**Files:** `library.rs`, `distribute.rs`

1. `consolidate()`: derive strategy from `DirectoryRole` instead of `SourceType`
2. `distribute_to_target()`: accept `(&DirectoryName, &DirectoryConfig)` instead of `(&str, &TargetConfig)`
3. Replace `shares_tool_root()` with role-based circular detection
4. Remove `source_paths` parameter (no longer needed with role-based detection)

### Phase 5: Pipeline Orchestration (depends on Phases 1-4)

**Files:** `lib.rs` (sync function), `cleanup.rs`

1. Update `sync()` to use `config.discovery_dirs()` and `config.distribution_dirs()`
2. Update cleanup to iterate `distribution_dirs()` for target cleanup
3. Update `warn_unknown_disabled_targets()` to check against directory names
4. Update `SyncReport` if needed

### Phase 6: Wizard + Peripheral Modules (depends on Phases 1-5)

**Files:** `wizard.rs`, `status.rs`, `doctor.rs`, `eject.rs`, `browse/`, `machine.rs`

1. Merge `KNOWN_SOURCES` + `KNOWN_TARGETS` into `KNOWN_DIRECTORIES` with default roles
2. Update status/doctor/eject/browse to use `DirectoryName` and `DirectoryConfig`
3. Update `machine.rs` field names (`disabled_targets` -> `disabled_directories`)

### Phase 7: Git Sources (can be separate PR)

**Files:** `config.rs` (minor), new `git.rs` module, `lib.rs`

1. Add `resolve_git_directories()` function
2. Add `ResolvedDirectories` wrapper
3. Wire into sync pipeline between config load and discovery
4. Clone to `~/.tome/repos/<hash>/`, pull on subsequent syncs

**Why last:** Git sources are additive. They do not affect the core refactor. They can ship in a follow-up PR.

## Scalability Considerations

Not applicable for this single-user CLI tool. The directory count will remain in single digits. BTreeMap iteration order and in-memory filtering are more than sufficient.

## Sources

- Direct source code analysis of `crates/tome/src/` (config.rs, discover.rs, library.rs, distribute.rs, manifest.rs, lockfile.rs, lib.rs)
- `.planning/PROJECT.md` (v0.6 requirements and decisions)
- `.planning/codebase/ARCHITECTURE.md` (current architecture documentation)
- HIGH confidence: all findings are from direct codebase analysis, no external sources needed

---

*Architecture analysis: 2026-04-10*
