# Requirements: tome v0.6 — Unified Directory Model

**Defined:** 2026-04-10
**Core Value:** Every AI coding tool on a developer's machine shares the same skill library without manual copying or per-tool configuration.

## v1 Requirements

Requirements for v0.6 release. Each maps to roadmap phases.

### Config Model

- [x] **CFG-01**: Config uses `[directories.*]` BTreeMap replacing `[[sources]]` + `[targets.*]`
- [x] **CFG-02**: Each directory entry has `path`, `type` (claude-plugins/directory/git), and `role` (managed/synced/source/target)
- [x] **CFG-03**: Default roles inferred from type: ClaudePlugins→Managed, Directory→Synced, Git→Source
- [x] **CFG-04**: Config validation rejects invalid combos (managed only with claude-plugins, target incompatible with git, git fields only with git type)
- [x] **CFG-05**: `deny_unknown_fields` or equivalent catches old-format config files with clear error
- [x] **CFG-06**: Empty directories map triggers safety guard in cleanup (prevents library deletion)

### Pipeline

- [x] **PIPE-01**: Discovery iterates `discovery_dirs()` (managed/synced/source roles)
- [x] **PIPE-02**: Distribution iterates `distribution_dirs()` (synced/target roles)
- [x] **PIPE-03**: Circular symlink prevention uses manifest-based origin check (skip distributing skill to its discovery directory) replacing `shares_tool_root()` path heuristic
- [x] **PIPE-04**: Consolidation strategy determined by role: managed→symlink, synced/source→copy
- [x] **PIPE-05**: Duplicate skill resolution uses alphabetical directory name order (BTreeMap)

### Wizard

- [ ] **WIZ-01**: Merged `KNOWN_DIRECTORIES` registry replaces separate `KNOWN_SOURCES` + `KNOWN_TARGETS`
- [ ] **WIZ-02**: Auto-discovers directories from filesystem, auto-assigns roles from registry
- [ ] **WIZ-03**: Shows summary table (name | path | type | role) before confirmation
- [ ] **WIZ-04**: Custom directory addition flow includes role selection
- [ ] **WIZ-05**: `find_source_target_overlaps()` eliminated — not needed with unified model

### Machine Preferences

- [x] **MACH-01**: `disabled_targets` renamed to `disabled_directories` in machine.toml
- [ ] **MACH-02**: Per-directory `disabled` set (blocklist) in machine.toml
- [ ] **MACH-03**: Per-directory `enabled` set (exclusive allowlist) in machine.toml
- [ ] **MACH-04**: `disabled` + `enabled` on same directory = validation error
- [ ] **MACH-05**: Resolution: global disabled → per-directory disabled → per-directory enabled (allowlist)

### Git Sources

- [x] **GIT-01**: `type = "git"` directory config with URL in `path` field
- [x] **GIT-02**: Shallow clone (`--depth 1`) to `~/.tome/repos/<sha256(url)>/` with `.git` intact
- [x] **GIT-03**: Update via `git fetch --depth 1 origin <ref> && git reset --hard FETCH_HEAD` (not `git pull`)
- [x] **GIT-04**: Branch/tag/SHA pinning via `branch`, `tag`, `rev` fields (mutually exclusive)
- [x] **GIT-05**: Resolved commit SHA recorded in lockfile for reproducibility
- [x] **GIT-06**: All git Commands clear `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE` env vars
- [ ] **GIT-07**: Git resolution runs as pre-discovery step, resolves URLs to local cache paths
- [ ] **GIT-08**: Failed git operations fall back to cached state, don't abort sync of local directories

### Management CLI

- [ ] **CLI-01**: `tome remove <directory-name>` removes entry from config, cleans up library + symlinks
- [ ] **CLI-02**: `tome add <github-url>` creates git directory entry in config from URL
- [ ] **CLI-03**: `tome reassign <skill-name> --to <directory-name>` changes skill provenance

### State Schema

- [x] **STATE-01**: Manifest `source_name` field populated from directory name (field name preserved)
- [x] **STATE-02**: Lockfile `source_name` field populated from directory name (field name preserved)
- [x] **STATE-03**: Status output merges SourceStatus/TargetStatus into DirectoryStatus with role

### Browse Polish

- [ ] **BROWSE-01**: Theming support (configurable color scheme)
- [ ] **BROWSE-02**: Fuzzy match highlighting in skill list
- [ ] **BROWSE-03**: Scrollbar indicator for long lists
- [ ] **BROWSE-04**: Markdown syntax rendering in preview panel

## v2 Requirements

Deferred to future releases. Tracked but not in current roadmap.

### Override Semantics

- **OVER-01**: `override_enable` field in machine.toml to re-enable globally disabled skills for specific directories
- **OVER-02**: Explicit `tool` field on directory entries for grouping multiple directories per tool

### Expanded Features

- **EXP-01**: `priority` field on directory entries for explicit duplicate resolution ordering
- **EXP-02**: Format transforms / rules syncing (#57, #193, #194)
- **EXP-03**: Connector trait abstraction (#192)
- **EXP-04**: Watch mode (#59)

## Out of Scope

| Feature | Reason |
|---------|--------|
| Backward-compatible config parsing | Single user; hard break with migration docs |
| Config migration command (`tome migrate`) | Not worth building for one user |
| Template engine for per-machine config | Complexity without proportional value |
| Dependency resolution between skills | Skills are flat documents, not code with imports |
| Registry / marketplace hosting | Ecosystem has Skills.sh; different product |
| Bidirectional file sync (two-way merge) | Library is canonical; sources flow in, targets flow out |
| Nested git repos in library | Git submodules cause confusion; clones go to `~/.tome/repos/` |
| Format transforms in v0.6 | Separate concern; defers complexity |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| CFG-01 | Phase 1 | Complete |
| CFG-02 | Phase 1 | Complete |
| CFG-03 | Phase 1 | Complete |
| CFG-04 | Phase 1 | Complete |
| CFG-05 | Phase 1 | Complete |
| CFG-06 | Phase 1 | Complete |
| PIPE-01 | Phase 1 | Complete |
| PIPE-02 | Phase 1 | Complete |
| PIPE-03 | Phase 1 | Complete |
| PIPE-04 | Phase 1 | Complete |
| PIPE-05 | Phase 1 | Complete |
| WIZ-01 | Phase 1 | Pending |
| WIZ-02 | Phase 1 | Pending |
| WIZ-03 | Phase 1 | Pending |
| WIZ-04 | Phase 1 | Pending |
| WIZ-05 | Phase 1 | Pending |
| MACH-01 | Phase 1 | Complete |
| STATE-01 | Phase 1 | Complete |
| STATE-02 | Phase 1 | Complete |
| STATE-03 | Phase 1 | Complete |
| MACH-02 | Phase 2 | Pending |
| MACH-03 | Phase 2 | Pending |
| MACH-04 | Phase 2 | Pending |
| MACH-05 | Phase 2 | Pending |
| GIT-01 | Phase 2 | Complete |
| GIT-02 | Phase 2 | Complete |
| GIT-03 | Phase 2 | Complete |
| GIT-04 | Phase 2 | Complete |
| GIT-05 | Phase 2 | Complete |
| GIT-06 | Phase 2 | Complete |
| GIT-07 | Phase 2 | Pending |
| GIT-08 | Phase 2 | Pending |
| CLI-01 | Phase 2 | Pending |
| CLI-02 | Phase 3 | Pending |
| CLI-03 | Phase 3 | Pending |
| BROWSE-01 | Phase 3 | Pending |
| BROWSE-02 | Phase 3 | Pending |
| BROWSE-03 | Phase 3 | Pending |
| BROWSE-04 | Phase 3 | Pending |

**Coverage:**
- v1 requirements: 39 total
- Mapped to phases: 39
- Unmapped: 0

---
*Requirements defined: 2026-04-10*
*Last updated: 2026-04-10 after roadmap creation*
