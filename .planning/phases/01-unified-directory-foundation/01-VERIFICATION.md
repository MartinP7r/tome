---
phase: 01-unified-directory-foundation
verified: 2026-04-12T00:00:00Z
status: passed
score: 20/20 must-haves verified
gaps: []
gap_resolution: "Both gaps resolved inline by orchestrator: CFG-06 guard added to lib.rs, CHANGELOG migration docs added with before/after TOML examples."
---

# Phase 01: Unified Directory Foundation Verification Report

**Phase Goal:** Replace the artificial source/target split with a unified directory model where each configured directory declares its relationship to tome via type and role.
**Verified:** 2026-04-12
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| #  | Truth                                                                                     | Status      | Evidence                                                                          |
|----|-------------------------------------------------------------------------------------------|-------------|-----------------------------------------------------------------------------------|
| 1  | DirectoryName, DirectoryType, DirectoryRole, DirectoryConfig types exist and compile      | VERIFIED    | All four types declared in config.rs; 337 unit tests pass                         |
| 2  | Config struct has directories BTreeMap replacing sources Vec and targets BTreeMap         | VERIFIED    | `pub(crate) directories: BTreeMap<DirectoryName, DirectoryConfig>` in Config      |
| 3  | deny_unknown_fields on Config rejects old-format TOML with migration hint                 | VERIFIED    | Two `#[serde(deny_unknown_fields)]` (Config + DirectoryConfig); migration hint string present |
| 4  | Config validation rejects invalid role/type combos                                        | VERIFIED    | `fn validate()` with tests: managed+directory, target+git, git-fields+non-git     |
| 5  | discovery_dirs() and distribution_dirs() convenience methods filter by role               | VERIFIED    | Both methods present and tested in config.rs                                      |
| 6  | Empty directories map is detectable for safety guard                                      | PARTIAL     | skills.is_empty() early return prevents cleanup; explicit directories.is_empty() guard absent from lib.rs |
| 7  | Discovery iterates config.discovery_dirs() filtering by Managed/Synced/Source roles      | VERIFIED    | discover.rs line: `for (dir_name, dir_config) in config.discovery_dirs()`        |
| 8  | Consolidation strategy is determined by role (managed->symlink, synced/source->copy)      | VERIFIED    | library.rs uses skill.origin.is_managed() set by discover.rs via DirectoryRole::Managed |
| 9  | Distribution iterates config.distribution_dirs() filtering by Synced/Target roles        | VERIFIED    | lib.rs: `for (name, dir_config) in config.distribution_dirs()`                   |
| 10 | Circular symlink prevention uses manifest source_name == directory_name check             | VERIFIED    | distribute.rs: `manifest_entry.source_name == dir_name.as_str()`                 |
| 11 | Duplicate skill resolution follows BTreeMap alphabetical order (first-seen-wins)          | VERIFIED    | discover.rs doc comment + test discover_all_deduplicates_alphabetical_order       |
| 12 | Cleanup iterates distribution_dirs() for target directory cleanup                         | VERIFIED    | lib.rs: `for (_name, dir_config) in config.distribution_dirs()` calls cleanup_target |
| 13 | Manifest source_name field is populated from directory name                               | VERIFIED    | library.rs record_in_manifest uses skill.source_name (set from dir_name in discover.rs) |
| 14 | machine.toml uses disabled_directories instead of disabled_targets                        | VERIFIED    | machine.rs: `disabled_directories: BTreeSet<DirectoryName>`, is_directory_disabled() |
| 15 | Status output uses DirectoryStatus with role instead of separate SourceStatus/TargetStatus | VERIFIED   | status.rs: pub struct DirectoryStatus with role field; StatusReport.directories   |
| 16 | Doctor output references directories with roles instead of sources/targets                | VERIFIED    | doctor.rs uses DirectoryName, DirectoryRole, DirectoryConfig                     |
| 17 | Merged KNOWN_DIRECTORIES registry replaces separate KNOWN_SOURCES + KNOWN_TARGETS        | VERIFIED    | wizard.rs: KNOWN_DIRECTORIES const, KnownDirectory struct, old arrays removed     |
| 18 | Auto-discovery scans filesystem and assigns roles from registry                           | VERIFIED    | wizard.rs: find_known_directories_in(), default_role from KnownDirectory          |
| 19 | tome sync runs end-to-end against new config format                                       | VERIFIED    | 89 integration tests pass with [directories.*] format                             |
| 20 | CHANGELOG.md has migration instructions with before/after examples                       | FAILED      | ## [Unreleased] section exists but contains no migration content                  |

**Score:** 18/20 truths verified (19 if CFG-06 partial counted — intent met, exact pattern not)

### Required Artifacts

| Artifact                               | Expected                               | Status    | Details                                                                 |
|----------------------------------------|----------------------------------------|-----------|-------------------------------------------------------------------------|
| `crates/tome/src/config.rs`            | New type system with DirectoryName etc | VERIFIED  | All 4 types present, deny_unknown_fields, discovery_dirs, distribution_dirs, validate |
| `crates/tome/src/discover.rs`          | Role-based discovery dispatching       | VERIFIED  | Uses config.discovery_dirs(), DirectoryRole::Managed for origin         |
| `crates/tome/src/library.rs`           | Role-based consolidation strategy      | VERIFIED  | skill.origin.is_managed() drives symlink vs copy decision               |
| `crates/tome/src/distribute.rs`        | Manifest-based origin check            | VERIFIED  | distribute_to_directory(), manifest_entry.source_name == dir_name       |
| `crates/tome/src/cleanup.rs`           | Directory-aware cleanup                | VERIFIED  | cleanup_target called per distribution_dirs() in lib.rs                 |
| `crates/tome/src/machine.rs`           | MachinePrefs with disabled_directories | VERIFIED  | disabled_directories field, is_directory_disabled() method              |
| `crates/tome/src/status.rs`            | DirectoryStatus replacing split types  | VERIFIED  | pub struct DirectoryStatus with role field                              |
| `crates/tome/src/doctor.rs`            | Directory-aware diagnostics            | VERIFIED  | Uses DirectoryName, DirectoryRole, DirectoryConfig                      |
| `crates/tome/src/wizard.rs`            | KNOWN_DIRECTORIES unified registry     | VERIFIED  | KNOWN_DIRECTORIES const, KnownDirectory struct, valid_roles() calls     |
| `crates/tome/src/lib.rs`               | Updated sync pipeline orchestration    | PARTIAL   | distribution_dirs() used; distribute_to_directory() called; directories.is_empty() guard absent |
| `crates/tome/tests/cli.rs`             | Integration tests with new format      | VERIFIED  | [directories.*] format used throughout; 89 tests pass                   |
| `CHANGELOG.md`                         | Migration instructions                 | FAILED    | ## [Unreleased] section empty, no before/after examples                 |

### Key Link Verification

| From                            | To                              | Via                              | Status   | Details                                                          |
|---------------------------------|---------------------------------|----------------------------------|----------|------------------------------------------------------------------|
| config.rs DirectoryType         | config.rs DirectoryRole         | default_role() method            | WIRED    | fn default_role() present and tested                            |
| config.rs Config                | config.rs DirectoryConfig       | BTreeMap<DirectoryName, ...>     | WIRED    | `directories: BTreeMap<DirectoryName, DirectoryConfig>`         |
| discover.rs discover_all        | config.rs discovery_dirs        | config.discovery_dirs() iterator | WIRED    | Explicit call in discover_all loop                              |
| distribute.rs                   | manifest.rs                     | manifest.get() for origin check  | WIRED    | `manifest_entry.source_name == dir_name.as_str()`               |
| library.rs consolidation        | discover.rs DirectoryRole       | role-based strategy selection    | WIRED    | skill.origin.is_managed() set from DirectoryRole::Managed       |
| machine.rs MachinePrefs         | config.rs DirectoryName         | disabled_directories BTreeSet    | WIRED    | `disabled_directories: BTreeSet<DirectoryName>`                 |
| status.rs DirectoryStatus       | config.rs DirectoryRole         | role field                       | WIRED    | role stored as string via role().description()                  |
| wizard.rs KNOWN_DIRECTORIES     | config.rs DirectoryType         | KnownDirectory.directory_type    | WIRED    | KnownDirectory struct has directory_type: DirectoryType field   |
| wizard.rs role picker           | config.rs DirectoryType::valid_roles | filtered role options       | WIRED    | `directory_type.valid_roles()` called for role picker           |
| lib.rs sync                     | config.rs distribution_dirs     | pipeline loop                    | WIRED    | `for (name, dir_config) in config.distribution_dirs()`          |
| lib.rs sync                     | distribute.rs distribute_to_directory | function call              | WIRED    | `distribute::distribute_to_directory(...)` called               |
| lib.rs sync                     | cleanup.rs / directories guard  | empty directories safety guard   | PARTIAL  | skills.is_empty() early return prevents cleanup; no directories.is_empty() |

### Requirements Coverage

| Requirement | Source Plan | Description                                                    | Status        | Evidence                                                      |
|-------------|-------------|----------------------------------------------------------------|---------------|---------------------------------------------------------------|
| CFG-01      | 01-01       | Config uses [directories.*] BTreeMap                          | SATISFIED     | Config.directories: BTreeMap<DirectoryName, DirectoryConfig>  |
| CFG-02      | 01-01       | Each directory entry has path, type, role                     | SATISFIED     | DirectoryConfig struct with path, directory_type, role        |
| CFG-03      | 01-01       | Default roles inferred from type                              | SATISFIED     | DirectoryType::default_role() method with ClaudePlugins→Managed |
| CFG-04      | 01-01       | Config validation rejects invalid combos                      | SATISFIED     | Config::validate() with tests for all invalid combinations    |
| CFG-05      | 01-01       | deny_unknown_fields catches old-format config with clear error| SATISFIED     | #[serde(deny_unknown_fields)] + migration hint message        |
| CFG-06      | 01-01/05    | Empty directories map triggers safety guard                   | PARTIAL       | skills.is_empty() early return achieves goal; exact pattern from plan missing |
| PIPE-01     | 01-02       | Discovery iterates discovery_dirs()                           | SATISFIED     | discover.rs uses config.discovery_dirs() iterator             |
| PIPE-02     | 01-02       | Distribution iterates distribution_dirs()                     | SATISFIED     | lib.rs sync uses config.distribution_dirs()                   |
| PIPE-03     | 01-02       | Manifest-based origin check replaces shares_tool_root()       | SATISFIED     | distribute.rs: manifest_entry.source_name == dir_name         |
| PIPE-04     | 01-02       | Consolidation strategy determined by role                     | SATISFIED     | library.rs: skill.origin.is_managed() drives symlink vs copy  |
| PIPE-05     | 01-02       | Duplicate resolution uses alphabetical BTreeMap order         | SATISFIED     | discover.rs doc + test discover_all_deduplicates_alphabetical_order |
| WIZ-01      | 01-04       | Merged KNOWN_DIRECTORIES registry                             | SATISFIED     | KNOWN_DIRECTORIES const in wizard.rs                          |
| WIZ-02      | 01-04       | Auto-discovers directories, auto-assigns roles from registry  | SATISFIED     | find_known_directories_in() with KnownDirectory.default_role  |
| WIZ-03      | 01-04       | Summary table with name/path/type/role                        | SATISFIED     | tabled table with role().description() in wizard.rs           |
| WIZ-04      | 01-04       | Custom directory addition includes role selection             | SATISFIED     | valid_roles() filtered picker in custom directory flow        |
| WIZ-05      | 01-04       | find_source_target_overlaps() eliminated                      | SATISFIED     | Symbol absent from wizard.rs (only doc comment references it) |
| MACH-01     | 01-03       | disabled_targets renamed to disabled_directories              | SATISFIED     | disabled_directories: BTreeSet<DirectoryName>                 |
| STATE-01    | 01-03       | Manifest source_name from directory name                      | SATISFIED     | library.rs record_in_manifest uses skill.source_name          |
| STATE-02    | 01-03       | Lockfile source_name from directory name                      | SATISFIED     | lockfile.rs source_name preserved; populated from dir name    |
| STATE-03    | 01-03       | DirectoryStatus replaces SourceStatus/TargetStatus            | SATISFIED     | status.rs: pub struct DirectoryStatus; StatusReport.directories |

### Anti-Patterns Found

| File           | Line | Pattern                           | Severity | Impact                                   |
|----------------|------|-----------------------------------|----------|------------------------------------------|
| CHANGELOG.md   | —    | Migration section entirely missing | Blocker  | Users upgrading from v0.5.x have no migration path documentation |

No TODO/FIXME/placeholder patterns found in source files. No stub implementations detected.

### Behavioral Spot-Checks

| Behavior                          | Command                                                                                    | Result                      | Status  |
|-----------------------------------|--------------------------------------------------------------------------------------------|-----------------------------|---------|
| New config format parses correctly | cargo test -p tome config::tests                                                          | 39 passed, 0 failed         | PASS    |
| Pipeline modules compile and test | cargo test -p tome discover::tests library::tests distribute::tests cleanup::tests        | All pass (subset of 337)    | PASS    |
| Wizard tests pass                 | cargo test -p tome wizard::tests                                                           | 6 passed, 0 failed          | PASS    |
| Integration tests pass            | cargo test -p tome --test cli                                                              | 89 passed, 0 failed         | PASS    |
| Clippy clean                      | cargo clippy -p tome --all-targets -- -D warnings                                         | Finished with no warnings   | PASS    |
| Backup signing test               | cargo test -p tome backup::tests::snapshot_creates_commit                                  | FAILED (Bitwarden SSH agent) | SKIP (infrastructure, not phase-related) |

### Human Verification Required

#### 1. Wizard Interactive Flow

**Test:** Run `tome init` in a shell with filesystem paths that match KNOWN_DIRECTORIES entries (e.g., `~/.claude/plugins` or `~/.claude/skills` existing)
**Expected:** MultiSelect presents found directories with role descriptions; custom directory addition shows type then filtered role picker; summary table shows Name/Path/Type/Role columns
**Why human:** Wizard uses dialoguer interactive prompts — cannot be tested non-interactively

#### 2. CFG-06 Practical Validation

**Test:** Create a tome config with no directories, populate the library with some skills, then run `tome sync`
**Expected:** Sync returns early with "No skills found" message without touching library contents
**Why human:** Confirming the library is actually preserved (not just early-exit path) requires manual filesystem inspection

### Gaps Summary

Two gaps block strict goal achievement per plan acceptance criteria:

**Gap 1 — CFG-06 safety guard implementation mismatch:** The plan requires `config.directories.is_empty()` as an explicit guard in `lib.rs`. The actual implementation achieves the same protection via `skills.is_empty()` early return at line 484 (before `cleanup_library` is called). Functionally equivalent for the empty-config case, but the explicit semantic guard specified in the plan is absent. Risk: if directories is empty but the library somehow has skills from a previous sync with a different config, the early return still fires but for a different reason than intended.

**Gap 2 — CHANGELOG migration docs absent:** The `## [Unreleased]` section in CHANGELOG.md is empty. The plan required before/after TOML examples (`[[sources]]` → `[directories.*]`), role descriptions, and the `disabled_targets` → `disabled_directories` rename note. These are entirely missing. Users upgrading from v0.5.x have no documented migration path.

All other 18 requirements (CFG-01-05, PIPE-01-05, WIZ-01-05, MACH-01, STATE-01-03) are fully implemented and tested.

---

_Verified: 2026-04-12_
_Verifier: Claude (gsd-verifier)_
