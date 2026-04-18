# Architecture Patterns

**Domain:** Rust CLI wizard for unified directory model
**Researched:** 2026-04-16
**Status:** Wizard rewrite is COMPLETE (shipped in v0.6 phase 01-04)

## Current State Assessment

The wizard rewrite from `KNOWN_SOURCES` + `KNOWN_TARGETS` to a merged `KNOWN_DIRECTORIES` registry is **already implemented** in `crates/tome/src/wizard.rs`. All five WIZ requirements (WIZ-01 through WIZ-05) were satisfied during v0.6 phase 01-04. The current code:

- Uses a single `KNOWN_DIRECTORIES: &[KnownDirectory]` const array (11 entries)
- Auto-discovers directories via `find_known_directories_in()` with `std::fs::metadata()`
- Assigns roles from registry metadata (`default_role` field on each `KnownDirectory`)
- Shows a summary table with name/path/type/role columns
- Supports custom directory addition with role picker filtered by `DirectoryType::valid_roles()`
- Offers post-summary role editing (except ClaudePlugins which is locked to Managed)
- `find_source_target_overlaps()` has been eliminated entirely

### Data Structure: KnownDirectory

The current registry entry struct:

```rust
struct KnownDirectory {
    name: &'static str,           // BTreeMap key (e.g. "claude-plugins")
    display: &'static str,        // Human label (e.g. "Claude Code Plugins")
    default_path: &'static str,   // Relative to $HOME (e.g. ".claude/plugins")
    directory_type: DirectoryType, // ClaudePlugins | Directory | Git
    default_role: DirectoryRole,   // Managed | Synced | Source | Target
}
```

This is the right shape. Each field has a clear purpose, maps directly to `DirectoryConfig`, and the `default_role` + `directory_type` combination determines behavior. No changes needed to this struct.

### Auto-Role Assignment Logic

Role assignment follows `DirectoryType::default_role()`:

| DirectoryType | Default Role | Rationale |
|---------------|-------------|-----------|
| ClaudePlugins | Managed | Package manager owns files; read-only |
| Directory | Synced | Bidirectional: discovers AND receives skills |
| Git | Source | Read-only clone; discovers but does not receive |

The `valid_roles()` method constrains what users can change to:

| DirectoryType | Valid Roles | Locked? |
|---------------|------------|---------|
| ClaudePlugins | Managed only | Yes |
| Directory | Synced, Source, Target | No |
| Git | Source only | Yes |

This is correct. ClaudePlugins directories must be Managed (they are symlinked, not copied). Git directories must be Source (the clone is read-only). Directory types are flexible.

### Wizard Flow

```
1. configure_directories()
   - find_known_directories() scans $HOME for KNOWN_DIRECTORIES paths
   - MultiSelect: user picks which to include (all pre-selected)
   - Each selected entry becomes DirectoryConfig with default_role

2. discover_all() -- pre-scan for exclusion step
   - Creates temporary Config from selected directories
   - Discovers skills (warns on error, continues gracefully)

3. configure_library()
   - Select: default (~/.tome/skills) or custom path

4. configure_exclusions()
   - MultiSelect: pick skills to exclude (none pre-selected)

5. Summary table + role editing loop
   - show_directory_summary(): name | path | type | role
   - Confirm: edit roles? (Select directory, Select new role)
   - Repeat until user says no

6. Custom directory addition loop
   - Input: name, path
   - Select: type (directory | claude-plugins)
   - Select: role (filtered by type)
   - Repeat until user says no

7. Save config (or dry-run preview)
8. Optional git init for backup tracking
```

### Component Boundaries

| Component | Responsibility | Status |
|-----------|---------------|--------|
| `KnownDirectory` struct | Registry entry shape | Complete, no changes needed |
| `KNOWN_DIRECTORIES` const | Registry of known tool directories | Complete, may need new entries |
| `find_known_directories_in()` | Filesystem scanning for auto-discovery | Complete |
| `configure_directories()` | MultiSelect UI for directory selection | Complete |
| `show_directory_summary()` | Summary table rendering | Complete |
| `configure_library()` | Library path selection | Complete |
| `configure_exclusions()` | Skill exclusion picker | Complete |
| `run()` | Top-level wizard orchestration | Complete |

### Integration Points

The wizard integrates with:

1. **`config.rs`** -- Creates `DirectoryConfig` structs, uses `DirectoryName::new()` for validation, calls `Config::save()`
2. **`discover.rs`** -- Calls `discover_all()` during step 2 for exclusion picking
3. **`paths.rs`** -- Uses `collapse_home_path()` for portable path serialization
4. **`backup.rs`** -- Calls `backup::init()` for optional git tracking setup
5. **`config::expand_tilde()`** -- Expands user-provided custom paths

### Registry Completeness

Current 11 entries:

| Name | Path | Type | Default Role |
|------|------|------|-------------|
| claude-plugins | .claude/plugins | ClaudePlugins | Managed |
| claude-skills | .claude/skills | Directory | Synced |
| antigravity | .gemini/antigravity/skills | Directory | Synced |
| codex | .codex/skills | Directory | Synced |
| codex-agents | .agents/skills | Directory | Synced |
| openclaw | .openclaw/skills | Directory | Synced |
| goose | .config/goose/skills | Directory | Synced |
| gemini-cli | .gemini/skills | Directory | Synced |
| amp | .config/amp/skills | Directory | Synced |
| opencode | .config/opencode/skills | Directory | Synced |
| copilot | .copilot/skills | Directory | Synced |

**Potential gaps** (LOW confidence -- need filesystem verification):
- Cursor: may have a global skills directory (e.g. `~/.cursor/skills/`)
- Windsurf: may have a global skills directory
- Aider: may have a skills/config directory

These would need investigation per the ROADMAP.md "Tentative" section.

## Remaining Work Assessment

The wizard architecture is complete. What remains is **not architectural** but rather:

1. **Registry expansion** -- Adding new tool entries to `KNOWN_DIRECTORIES` as tools adopt skill directories (pure data, no code changes)
2. **PROJECT.md cleanup** -- The "Active" items list WIZ-01 through WIZ-05 as incomplete, but all are done
3. **Test coverage** -- Current tests verify registry invariants (no duplicates, valid names, filesystem scanning). Interactive flow is not tested (dialoguer does not support test mode). This is acceptable for a one-time wizard.

## Patterns Already Implemented

### Pattern: Registry-Driven Auto-Discovery
The `KNOWN_DIRECTORIES` const array drives auto-discovery. Adding a new tool requires only a new `KnownDirectory` entry -- no code changes. The `find_known_directories_in()` function iterates the array and checks `std::fs::metadata()` for each path.

### Pattern: Type-Constrained Role Selection
`DirectoryType::valid_roles()` constrains role selection per type. The wizard uses this to filter the role picker, making invalid states unrepresentable in the UI.

### Pattern: Default-On Selection
MultiSelect defaults all discovered directories to selected (`defaults: vec![true; found.len()]`). This is the right UX -- users opt out rather than opt in.

### Pattern: Config Preview Before Save
The summary table + role editing loop lets users review and adjust before saving. Dry-run mode prints the generated TOML without writing.

## Anti-Patterns Avoided

### No Git Type in Wizard
The wizard does not offer Git as a directory type during custom addition. This is correct -- git repos need URLs, not filesystem paths, and `tome add <url>` handles that flow. The wizard is for local directories.

### No enabled/disabled Toggle in Wizard
The wizard does not expose per-directory enable/disable. This belongs in `machine.toml` (per-machine concern) not the portable config. Correct separation.

## Scalability Considerations

Not applicable. Single-digit directory count, one-time operation.

## Sources

- Direct source code analysis: `crates/tome/src/wizard.rs` (603 lines, current)
- Direct source code analysis: `crates/tome/src/config.rs` (DirectoryType, DirectoryRole, DirectoryConfig)
- `.planning/phases/01-unified-directory-foundation/01-04-SUMMARY.md` (completion evidence)
- `.planning/phases/01-unified-directory-foundation/01-VERIFICATION.md` (WIZ-01-05 all SATISFIED)
- HIGH confidence: all findings from direct codebase analysis

---

*Architecture analysis: 2026-04-16*
