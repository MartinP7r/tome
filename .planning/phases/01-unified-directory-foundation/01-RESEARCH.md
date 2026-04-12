# Phase 1: Unified Directory Foundation - Research

**Researched:** 2026-04-12
**Domain:** Rust config model refactoring, CLI wizard rewrite, sync pipeline adaptation
**Confidence:** HIGH

## Summary

Phase 1 replaces tome's artificial `[[sources]]` + `[targets.*]` config split with a unified `[directories.*]` BTreeMap model. Every directory declares a path, type, and role. The sync pipeline (discover, consolidate, distribute, cleanup) and all state schemas (manifest, lockfile, status, doctor, machine preferences) must be adapted. The wizard merges `KNOWN_SOURCES` + `KNOWN_TARGETS` into a single `KNOWN_DIRECTORIES` registry. Old-format configs must fail with a migration hint.

This is a codebase-wide refactor touching ~15 source files. The design is fully specified in `docs/v06-implementation-plan.md` and constrained by user decisions in CONTEXT.md. No external dependencies, no new crates, no API research needed -- this is purely an internal restructuring using existing patterns.

**Primary recommendation:** Follow the module conversion order from D-10: config.rs (new types) -> discover.rs -> library.rs -> distribute.rs -> cleanup/manifest/lockfile -> wizard.rs -> status/doctor -> cli.rs integration tests. Each module should compile and pass tests before moving to the next.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Let `toml::from_str` fail naturally via `#[serde(deny_unknown_fields)]` on `Config`. After a parse failure, check the raw TOML for `[[sources]]` or `[targets.` and append a migration hint to the error message.
- **D-02:** The hint should read: `hint: tome v0.6 replaced [[sources]] and [targets.*] with [directories.*]. See CHANGELOG.md for migration instructions.`
- **D-03:** `deny_unknown_fields` catches typos and future format drift beyond just old-format keys.
- **D-04:** Auto-assign roles from the `KNOWN_DIRECTORIES` registry. Show a summary with inline descriptions per directory explaining the role in plain english.
- **D-05:** Role names are internal jargon -- every user-facing display of a role MUST include a parenthetical plain-english explanation.
- **D-06:** When user edits a directory's role, use a `dialoguer::Select` menu showing each valid role (filtered by directory type) with its one-line description.
- **D-07:** ClaudePlugins directories can only be Managed (no role picker shown for them).
- **D-08:** Migration documented in CHANGELOG.md "Breaking Changes" section with before/after config examples. No standalone MIGRATION.md.
- **D-09:** `tome sync` with no config prints "no config found, run `tome init`" and exits. No auto-launch of wizard. Same as current behavior.
- **D-10:** Rewrite tests in lockstep with module conversion. Order: config.rs -> discover.rs -> library.rs -> distribute.rs -> cleanup/manifest/lockfile -> wizard.rs -> status/doctor -> cli.rs integration tests.
- **D-11:** Delete old insta snapshots and regenerate fresh via `cargo insta review`. Old snapshot diffs not useful given the scope of format changes.

### Claude's Discretion
- Exact wording of role descriptions (as long as they're plain-english and non-jargon)
- Internal module organization (e.g. whether `DirectoryName` lives in config.rs or gets its own module)
- Whether to keep `TargetName` as a type alias during transition or remove it immediately

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CFG-01 | Config uses `[directories.*]` BTreeMap replacing `[[sources]]` + `[targets.*]` | New `DirectoryName`, `DirectoryType`, `DirectoryRole`, `DirectoryConfig` types in config.rs; `Config.directories: BTreeMap<DirectoryName, DirectoryConfig>` replaces `sources` + `targets` |
| CFG-02 | Each directory entry has `path`, `type`, `role` | `DirectoryConfig` struct with serde rename `type` for `directory_type` field; `DirectoryRole` enum |
| CFG-03 | Default roles inferred from type | `DirectoryType::default_role()` method: ClaudePlugins->Managed, Directory->Synced, Git->Source |
| CFG-04 | Config validation rejects invalid combos | `Config::validate()` extended: Managed only with ClaudePlugins, Target incompatible with Git, git fields only with Git type |
| CFG-05 | `deny_unknown_fields` catches old-format configs | `#[serde(deny_unknown_fields)]` on Config struct + post-failure raw TOML check for `[[sources]]`/`[targets.` with migration hint (D-01, D-02) |
| CFG-06 | Empty directories map triggers safety guard | Cleanup checks `config.directories.is_empty()` before proceeding; prevents library deletion |
| PIPE-01 | Discovery iterates `discovery_dirs()` | `Config::discovery_dirs()` returns iterator over Managed/Synced/Source roles |
| PIPE-02 | Distribution iterates `distribution_dirs()` | `Config::distribution_dirs()` returns iterator over Synced/Target roles |
| PIPE-03 | Circular symlink prevention uses manifest-based origin check | Replace `shares_tool_root()` with manifest lookup: skip distributing skill X to directory Y if X was discovered from Y |
| PIPE-04 | Consolidation strategy determined by role | Managed->symlink, Synced/Source->copy (replaces SourceType-based routing) |
| PIPE-05 | Duplicate skill resolution uses BTreeMap order | BTreeMap iteration is alphabetical by DirectoryName -- first discovery wins |
| WIZ-01 | Merged `KNOWN_DIRECTORIES` registry | Single `KnownDirectory` struct + const array replacing `KNOWN_SOURCES` + `KNOWN_TARGETS` |
| WIZ-02 | Auto-discovers directories, auto-assigns roles | `find_known_directories_in()` scans filesystem, assigns roles from registry |
| WIZ-03 | Shows summary table before confirmation | `tabled` table with columns: name, path, type, role (with description) |
| WIZ-04 | Custom directory addition includes role selection | `dialoguer::Select` with role options filtered by type (D-06) |
| WIZ-05 | `find_source_target_overlaps()` eliminated | Function removed entirely -- concept doesn't exist in unified model |
| MACH-01 | `disabled_targets` renamed to `disabled_directories` | `MachinePrefs.disabled_targets: BTreeSet<TargetName>` becomes `disabled_directories: BTreeSet<DirectoryName>` |
| STATE-01 | Manifest `source_name` populated from directory name | `SkillEntry.source_name` field name preserved, value comes from `DirectoryName` |
| STATE-02 | Lockfile `source_name` populated from directory name | `LockfileEntry.source_name` field name preserved, value comes from `DirectoryName` |
| STATE-03 | Status output merges into DirectoryStatus | `SourceStatus` + `TargetStatus` collapsed into single `DirectoryStatus` with role field |
</phase_requirements>

## Standard Stack

No new crates needed. All required functionality exists in the current dependency set.

### Core (already in Cargo.toml)
| Library | Version | Purpose | Why Used Here |
|---------|---------|---------|---------------|
| serde + toml | 1.0.228 / 1.1.2 | Config serialization | `#[serde(deny_unknown_fields)]`, `#[serde(rename)]`, custom Deserialize for DirectoryName |
| serde_json | 1.0.149 | Manifest and lockfile I/O | Field name preservation (`source_name`) |
| dialoguer | 0.12.0 | Interactive wizard | `Select` for role picker, `MultiSelect` for directory selection |
| tabled | 0.20.0 | Table output | Summary table in wizard (name/path/type/role) |
| clap | 4.6.0 | CLI parsing | No structural changes needed |
| anyhow | 1.0.102 | Error handling | Context on parse failures for migration hint |
| walkdir | 2.5.0 | Directory traversal | Unchanged usage in discover |

### Not Needed
| Library | Reason |
|---------|--------|
| Any migration crate | Single user, hard break with docs only |
| New test crates | Existing assert_cmd + tempfile + insta sufficient |

## Architecture Patterns

### Module Conversion Order (from D-10)
```
1. config.rs       -- New types (DirectoryName, DirectoryType, DirectoryRole, DirectoryConfig)
2. discover.rs     -- Switch from Source/SourceType to DirectoryConfig/DirectoryName
3. library.rs      -- Consolidation strategy by role instead of source_type
4. distribute.rs   -- Iterate distribution_dirs(), replace shares_tool_root()
5. cleanup.rs      -- Iterate distribution_dirs() for target cleanup
   manifest.rs     -- source_name from directory name (field unchanged)
   lockfile.rs     -- source_name from directory name (field unchanged)
6. wizard.rs       -- KNOWN_DIRECTORIES, merged flow, role picker
7. status.rs       -- DirectoryStatus replacing SourceStatus/TargetStatus
   doctor.rs       -- Directory-aware diagnostics
8. lib.rs          -- Sync pipeline: discovery_dirs(), distribution_dirs()
   cli.rs          -- Update any source/target-specific args
9. tests/cli.rs    -- Rewrite all config TOML strings, update snapshot expectations
```

### Type Hierarchy

```rust
// New types in config.rs

pub struct DirectoryName(String);  // Same validation as TargetName

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DirectoryType {
    ClaudePlugins,   // reads installed_plugins.json
    Directory,       // scans */SKILL.md (default)
    Git,             // placeholder for Phase 2
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DirectoryRole {
    Managed,   // read-only, package manager owns
    Synced,    // discover + distribute
    Source,    // discover only
    Target,    // distribute only
}

#[derive(Serialize, Deserialize)]
pub struct DirectoryConfig {
    pub path: PathBuf,
    #[serde(rename = "type", default)]
    pub directory_type: DirectoryType,
    #[serde(default)]  // default inferred from type
    pub role: DirectoryRole,
    // Git fields (Phase 2, defined as Option now for forward compat)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]  // D-01, D-03
pub struct Config {
    #[serde(default = "defaults::library_dir")]
    pub(crate) library_dir: PathBuf,
    #[serde(default)]
    pub(crate) exclude: BTreeSet<SkillName>,
    #[serde(default, deserialize_with = "deserialize_directories")]
    pub(crate) directories: BTreeMap<DirectoryName, DirectoryConfig>,
    #[serde(default)]
    pub(crate) backup: BackupConfig,
}
```

### Config Convenience Methods

```rust
impl Config {
    /// Directories that participate in discovery: Managed, Synced, Source
    pub fn discovery_dirs(&self) -> impl Iterator<Item = (&DirectoryName, &DirectoryConfig)> {
        self.directories.iter().filter(|(_, c)| matches!(
            c.role, DirectoryRole::Managed | DirectoryRole::Synced | DirectoryRole::Source
        ))
    }

    /// Directories that receive distributed skills: Synced, Target
    pub fn distribution_dirs(&self) -> impl Iterator<Item = (&DirectoryName, &DirectoryConfig)> {
        self.directories.iter().filter(|(_, c)| matches!(
            c.role, DirectoryRole::Synced | DirectoryRole::Target
        ))
    }

    /// Directories with managed role only
    pub fn managed_dirs(&self) -> impl Iterator<Item = (&DirectoryName, &DirectoryConfig)> {
        self.directories.iter().filter(|(_, c)| matches!(
            c.role, DirectoryRole::Managed
        ))
    }
}
```

### Old Config Detection Pattern (D-01, D-02)

```rust
// In Config::load()
pub fn load(path: &Path) -> Result<Self> {
    if path.exists() {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        match toml::from_str::<Config>(&content) {
            Ok(mut config) => {
                config.expand_tildes()?;
                Ok(config)
            }
            Err(e) => {
                // Check if this looks like an old-format config
                if content.contains("[[sources]]") || content.contains("[targets.") {
                    Err(e).with_context(|| {
                        "hint: tome v0.6 replaced [[sources]] and [targets.*] \
                         with [directories.*]. See CHANGELOG.md for migration instructions."
                    })
                } else {
                    Err(e).with_context(|| format!("failed to parse {}", path.display()))
                }
            }
        }
    } else {
        // ...
    }
}
```

### Circular Symlink Prevention (PIPE-03)

The current `shares_tool_root()` function in distribute.rs uses path heuristics. The new approach uses manifest-based origin check:

```rust
// In distribute.rs, replace shares_tool_root() block with:
if let Some(manifest_entry) = manifest.get(skill_name_str.as_ref()) {
    // Skip if skill was discovered from this same directory
    if manifest_entry.source_name == directory_name {
        // Skill originated from this directory; don't distribute back
        result.skipped_managed += 1;
        continue;
    }
}
```

This is simpler, more correct, and doesn't need `source_paths` threading or `canonicalize()` calls.

### discover.rs Adaptation

```rust
// Change from:
pub fn discover_all(config: &Config, warnings: &mut Vec<String>) -> Result<Vec<DiscoveredSkill>>

// The function iterates config.discovery_dirs() instead of config.sources.
// Each directory is dispatched by directory_type (ClaudePlugins vs Directory).
// DiscoveredSkill.source_name populated from DirectoryName.as_str().
// DiscoveredSkill.origin determined by role: Managed -> SkillOrigin::Managed, else -> SkillOrigin::Local
```

### Wizard KNOWN_DIRECTORIES Registry

```rust
struct KnownDirectory {
    name: &'static str,
    display: &'static str,
    default_path: &'static str,   // relative to $HOME
    directory_type: DirectoryType,
    default_role: DirectoryRole,
}

// ~12-13 entries, deduplicated from current 10 KNOWN_SOURCES + 9 KNOWN_TARGETS
// Examples:
// ("claude-plugins", "Claude Code Plugins", ".claude/plugins", ClaudePlugins, Managed)
// ("claude-skills", "Claude Code Skills", ".claude/skills", Directory, Synced)
// ("antigravity", "Antigravity", ".gemini/antigravity/skills", Directory, Synced)
// ("codex-source", "Codex (source)", ".codex/skills", Directory, Source)
// ("codex-target", "Codex (target)", ".agents/skills", Directory, Target)
```

Note: Some tools have separate source/target paths (e.g., Codex reads from `.agents/skills` but stores in `.codex/skills`). These remain as two separate directory entries with different roles.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| TOML validation for old-format detection | Manual string parsing of old config fields | `#[serde(deny_unknown_fields)]` + post-failure content check | Serde handles all validation; raw string check only needed for migration hint |
| Newtype validation | Custom validator per type | Reuse `validate_identifier()` from `validation.rs` | Already handles empty, path separators, dots, whitespace |
| Role defaulting | Complex default logic | `#[serde(default)]` + `DirectoryType::default_role()` method | Serde default handles missing field; method centralizes the logic |
| Deduplication priority | Custom priority system | BTreeMap alphabetical iteration | BTreeMap is already ordered; first-seen-wins with iter order |

## Common Pitfalls

### Pitfall 1: serde(deny_unknown_fields) Interaction with Defaults
**What goes wrong:** Adding `#[serde(deny_unknown_fields)]` to Config means ALL fields must be listed in the struct or the parse fails. The `backup` field uses `#[serde(default)]` which is fine -- missing fields are ok. But extra/unknown fields in TOML will fail.
**How to avoid:** Test that minimal configs (just `library_dir`) still parse. The `#[serde(default)]` on `directories`, `exclude`, and `backup` handles missing fields. `deny_unknown_fields` only rejects *extra* unexpected fields.
**Warning signs:** Existing valid configs that happen to have extra keys start failing.

### Pitfall 2: Role Default When type Is Omitted
**What goes wrong:** If both `type` and `role` are omitted from a `[directories.foo]` entry, the defaults must chain correctly: type defaults to Directory, role defaults based on type (Synced).
**How to avoid:** Implement `Default for DirectoryType` (returns `Directory`) and use a custom deserialize or post-parse step for role defaulting that reads the resolved type. Cannot use simple `#[serde(default)]` for role because the default depends on `type`.
**Warning signs:** Role is always `Synced` regardless of type, or role default panics when type is `ClaudePlugins`.

### Pitfall 3: discover_all Duplicate Resolution Order Change
**What goes wrong:** Current `discover_all` uses Vec<Source> iteration order (user-defined) for first-wins deduplication. New `BTreeMap<DirectoryName, _>` uses alphabetical order. This changes which skill "wins" when duplicates exist.
**How to avoid:** This is intentional (PIPE-05). Document the change. Since conflicts are rare and warned about, this is acceptable.
**Warning signs:** None -- this is the designed behavior.

### Pitfall 4: DiscoveredSkill.source_name Type Mismatch
**What goes wrong:** `DiscoveredSkill.source_name` is currently `String`. If `DirectoryName` is used as the key during discovery but the field stays `String`, there's a type mismatch at the boundary. If you change it to `DirectoryName`, then manifest/lockfile serialization must handle it.
**How to avoid:** Keep `source_name` as `String` in `DiscoveredSkill`, `SkillEntry`, and lockfile entries. Populate from `directory_name.as_str().to_string()`. This matches the existing pattern and preserves serialization compatibility.
**Warning signs:** Compilation errors around `String` vs `DirectoryName` at module boundaries.

### Pitfall 5: machine.toml Backward Compatibility
**What goes wrong:** Existing `machine.toml` files have `disabled_targets` field. If the struct changes to `disabled_directories`, loading old files fails.
**How to avoid:** Since this is single-user and a hard break, just document it. Alternatively, use `#[serde(alias = "disabled_targets")]` for a softer transition. Given the project's explicit "no backward compat" stance, a clean rename is fine.
**Warning signs:** `machine::load()` fails on existing machine.toml.

### Pitfall 6: Integration Tests Use Hardcoded TOML Strings
**What goes wrong:** Every integration test in `tests/cli.rs` builds config TOML strings with `[[sources]]` and `[targets.*]`. All of these must be rewritten to `[directories.*]` format.
**How to avoid:** D-10 says integration tests are rewritten last. By then, all modules compile. Use a test helper function that builds valid TOML strings from parameters.
**Warning signs:** 50+ test failures all at once. This is expected and handled by doing it last.

### Pitfall 7: Empty Directories Safety Guard (CFG-06)
**What goes wrong:** If `config.directories` is empty (no directories configured), cleanup might delete the entire library thinking everything is stale.
**How to avoid:** Add an early return in `cleanup_library` (or in `sync` before cleanup) when `config.directories.is_empty()`. Print a warning: "no directories configured, skipping cleanup to protect library."
**Warning signs:** Running `tome sync` with a minimal config (only `library_dir`) wipes the library.

### Pitfall 8: Config::load Returns Defaults for Missing File
**What goes wrong:** Current `Config::load` returns `Config::default()` when file is missing. The default Config has empty `sources` and `targets` (now empty `directories`). With `deny_unknown_fields`, this path doesn't go through TOML parsing at all, so it's fine. But `Config::load_or_default` with an explicit path that doesn't exist checks parent dir existence -- this logic is unchanged.
**How to avoid:** No action needed, but verify that the "no config" path still works (D-09).

## Code Examples

### New Config TOML Format
```toml
# Minimal (defaults: type = "directory", role = "synced")
[directories.claude-skills]
path = "~/.claude/skills"

# Explicit managed directory
[directories.claude-plugins]
path = "~/.claude/plugins"
type = "claude-plugins"
role = "managed"

# Target-only directory
[directories.antigravity]
path = "~/.gemini/antigravity/skills"
role = "target"

# Source-only directory
[directories.codex-source]
path = "~/.codex/skills"
role = "source"
```

### Role Description Strings (D-04, D-05)
```rust
impl DirectoryRole {
    pub fn description(&self) -> &'static str {
        match self {
            Self::Managed => "Managed (read-only, owned by package manager)",
            Self::Synced => "Synced (skills discovered here AND distributed here)",
            Self::Source => "Source (skills discovered here, not distributed here)",
            Self::Target => "Target (skills distributed here, not discovered here)",
        }
    }
}
```

### DirectoryName Newtype (Reuse TargetName Pattern)
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize)]
#[serde(transparent)]
pub struct DirectoryName(String);

impl DirectoryName {
    pub fn new(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        crate::validation::validate_identifier(&name, "directory name")?;
        Ok(Self(name))
    }
    pub fn as_str(&self) -> &str { &self.0 }
}
// + Display, AsRef<str>, AsRef<Path>, Borrow<str>, TryFrom<String>, custom Deserialize
// (identical trait impls to current TargetName)
```

### Sync Pipeline Changes (lib.rs)
```rust
// Old:
let source_paths: Vec<PathBuf> = config.sources.iter().map(|s| s.path.clone()).collect();
for (name, target) in config.targets.iter() { ... }

// New:
for (dir_name, dir_config) in config.distribution_dirs() {
    if machine_prefs.is_directory_disabled(dir_name.as_str()) { continue; }
    let result = distribute::distribute_to_directory(
        paths.library_dir(),
        dir_name.as_str(),
        dir_config,
        &manifest,
        &machine_prefs,
        dry_run,
        force,
    )?;
    distribute_results.push(result);
}
```

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| `Vec<Source>` (ordered, user-defined priority) | `BTreeMap<DirectoryName, DirectoryConfig>` (alphabetical) | Duplicate resolution now alphabetical; PIPE-05 |
| `TargetMethod` enum (only Symlink variant) | Removed entirely | Path comes directly from `DirectoryConfig.path`; simplification |
| `shares_tool_root()` path heuristic | Manifest-based origin check (`source_name == directory_name`) | Simpler, more correct; PIPE-03 |
| Separate `KNOWN_SOURCES` + `KNOWN_TARGETS` | Merged `KNOWN_DIRECTORIES` registry | Eliminates overlap concept; WIZ-01 |
| `disabled_targets` in machine.toml | `disabled_directories` in machine.toml | Semantic rename; MACH-01 |

## Open Questions

1. **DirectoryType default deserialization**
   - What we know: `#[serde(default)]` on `directory_type` gives `Directory`. `#[serde(default)]` on `role` should give the role for the *resolved* type, not a static default.
   - What's unclear: Serde deserializes fields independently -- `role`'s default can't depend on `type` during deserialization.
   - Recommendation: Use `Option<DirectoryRole>` for `role` in the raw struct, then resolve defaults in a post-deserialization step (in `Config::load` after parsing, before returning). This is the cleanest approach.

2. **TargetName removal timing**
   - What we know: `TargetName` is used in config.rs, machine.rs, distribute.rs, lib.rs. `DirectoryName` replaces it everywhere.
   - What's unclear: Whether to keep `TargetName` as a type alias during development or remove immediately.
   - Recommendation: Remove immediately. The atomic PR approach (D-10 order) means everything compiles together. A type alias adds no value and creates confusion.

## Project Constraints (from CLAUDE.md)

- **Platform:** Unix-only (`std::os::unix::fs::symlink`). No Windows support.
- **Rust edition:** 2024. Strict clippy with `-D warnings`.
- **Non-interactive shell commands:** Use `cp -f`, `mv -f`, `rm -f` to avoid hanging.
- **Build commands:** `make ci` (fmt-check + lint + test), `cargo test -p tome`.
- **Issue tracking:** Use `bd` (beads) for ALL task tracking. No markdown TODO lists.
- **Git:** Never commit directly to `main`. Always create a feature branch.
- **Test pattern:** Unit tests co-located with modules (`#[cfg(test)] mod tests`). Integration tests in `tests/cli.rs`.
- **Error handling:** `anyhow::Result<T>` throughout. `.with_context()` for operation context.
- **Newtype pattern:** Validated at construction. Custom Deserialize for parse-time validation. `#[serde(transparent)]`.
- **Atomic writes:** temp+rename for manifest, lockfile, machine.toml.
- **Insta snapshots:** Delete old, regenerate with `cargo insta review` (D-11).
- **GSD Workflow:** Use `/gsd:quick`, `/gsd:debug`, or `/gsd:execute-phase` entry points.
- **OpenSpec + Traceability:** For substantial changes, link GitHub issue, OpenSpec change, Beads task, commit/PR.
- **Session completion:** MUST push to remote before ending session.

## Sources

### Primary (HIGH confidence)
- `docs/v06-implementation-plan.md` -- Type definitions, PR plan, design decisions
- `crates/tome/src/config.rs` -- Current Config, TargetName, Source, SourceType implementations
- `crates/tome/src/discover.rs` -- Current SkillName, DiscoveredSkill, discovery logic
- `crates/tome/src/wizard.rs` -- Current KNOWN_SOURCES, KNOWN_TARGETS, wizard flow
- `crates/tome/src/distribute.rs` -- Current distribution + shares_tool_root()
- `crates/tome/src/cleanup.rs` -- Current cleanup logic
- `crates/tome/src/manifest.rs` -- Manifest structure and SkillEntry
- `crates/tome/src/machine.rs` -- MachinePrefs with disabled_targets
- `crates/tome/src/lib.rs` -- Sync pipeline orchestration
- `crates/tome/src/status.rs` -- SourceStatus, TargetStatus structs
- `.planning/REQUIREMENTS.md` -- Phase 1 requirement IDs and descriptions
- `.planning/phases/01-unified-directory-foundation/01-CONTEXT.md` -- User decisions

### Secondary (MEDIUM confidence)
- `Cargo.lock` -- Verified dependency versions

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - No new dependencies, all existing crates verified in Cargo.lock
- Architecture: HIGH - Design fully specified in v06-implementation-plan.md, all source files read
- Pitfalls: HIGH - Derived from reading actual source code and understanding serde behavior

**Research date:** 2026-04-12
**Valid until:** 2026-05-12 (stable -- no external dependency changes expected)
