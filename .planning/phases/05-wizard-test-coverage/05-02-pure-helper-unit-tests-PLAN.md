---
phase: 5
plan: 2
type: execute
wave: 2
depends_on:
  - "05-01"
files_modified:
  - crates/tome/src/wizard.rs
requirements:
  - WHARD-04
autonomous: true
must_haves:
  truths:
    - "`cargo test -p tome --lib wizard::tests` exercises a table-driven test covering `find_known_directories_in` on empty HOME, HOME with multiple known dirs, and HOME with one of every `KNOWN_DIRECTORIES` entry"
    - "`cargo test -p tome --lib wizard::tests` asserts every `KNOWN_DIRECTORIES` entry's `(directory_type, default_role)` equals `DirectoryType::default_role()` for that type"
    - "`cargo test -p tome --lib wizard::tests::assemble_config_*` exercises `assemble_config` for: empty inputs, single entry, multiple entries, custom entry added, exclusions preserved"
    - "Every helper test uses `tempfile::TempDir` for HOME isolation (no reliance on the real user HOME)"
  artifacts:
    - path: "crates/tome/src/wizard.rs"
      provides: "Unit tests for `find_known_directories_in`, `KNOWN_DIRECTORIES` registry shape, and `assemble_config` — all co-located in the existing `#[cfg(test)] mod tests` block"
      contains: "assemble_config_"
    - path: "crates/tome/src/wizard.rs"
      provides: "Table-driven `find_known_directories_in` coverage"
      contains: "find_known_directories_in_"
    - path: "crates/tome/src/wizard.rs"
      provides: "Registry invariant test: `(directory_type, default_role)` matches `DirectoryType::default_role()`"
      contains: "known_directories_default_role_matches_type"
  key_links:
    - from: "crates/tome/src/wizard.rs::tests::assemble_config_*"
      to: "crates/tome/src/wizard.rs::assemble_config"
      via: "direct function call — pure, no dialoguer"
      pattern: "assemble_config\\("
    - from: "crates/tome/src/wizard.rs::tests::find_known_directories_in_*"
      to: "crates/tome/src/wizard.rs::find_known_directories_in"
      via: "direct function call with `TempDir` HOME"
      pattern: "find_known_directories_in\\("
    - from: "crates/tome/src/wizard.rs::tests::known_directories_default_role_matches_type"
      to: "crates/tome/src/config.rs::DirectoryType::default_role"
      via: "registry iteration + default_role comparison"
      pattern: "\\.default_role\\(\\)"
---

<objective>
Add unit tests for the pure (non-interactive) wizard helpers that close WHARD-04:

1. `find_known_directories_in(&Path) -> Result<Vec<(&KnownDirectory, PathBuf)>>` — already has 3 tests at `wizard.rs:570-604`; extend with (a) HOME containing multiple known dirs at once, (b) HOME containing one instance of every `KNOWN_DIRECTORIES` entry, and (c) a mixed regular-dir + file-at-expected-path HOME.
2. `KNOWN_DIRECTORIES` registry invariants — there is already a "claude-plugins always managed" spot-check at `wizard.rs:607`; add a registry-wide test that iterates every entry and asserts `kd.default_role == kd.directory_type.default_role()` (so future additions cannot drift).
3. `assemble_config(directories, library_dir, exclude) -> Config` (introduced in Plan 05-01) — add direct unit tests covering: empty inputs, one-entry selection, multi-entry selection, custom-dir entry added on top of auto-discovered, exclusions preserved.

All tests live in the existing `#[cfg(test)] mod tests` block at `wizard.rs:543-620`. `DirectoryType::default_role` and `DirectoryType::valid_roles` already have exhaustive tests at `config.rs:711` and `config.rs:724` — per CONTEXT.md "already satisfied"; this plan adds the cross-file assertion that `KNOWN_DIRECTORIES` entries agree with them, not duplicate `default_role` coverage.

Purpose: close WHARD-04's "unit test coverage for pure wizard helpers" gate. These tests do not go through dialoguer, do not spawn a binary, and do not touch the user's real HOME — they exercise in-process Rust functions with `TempDir` HOMEs and inline `BTreeMap` construction.

Output: wizard.rs `#[cfg(test)] mod tests` extended with ~8 new tests; `cargo test -p tome --lib wizard::tests` passes on ubuntu + macos under CI; no production code changes.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/phases/05-wizard-test-coverage/05-CONTEXT.md
@.planning/phases/05-wizard-test-coverage/05-01-no-input-plumbing-and-assemble-config-PLAN.md

<interfaces>
<!-- Load-bearing signatures the tests depend on. -->

From Plan 05-01 (prerequisite):
```rust
pub(crate) fn assemble_config(
    directories: BTreeMap<DirectoryName, DirectoryConfig>,
    library_dir: PathBuf,
    exclude: std::collections::BTreeSet<crate::discover::SkillName>,
) -> Config
```

From `crates/tome/src/wizard.rs:23-35` (`KnownDirectory` shape — accessed via the registry slice):
```rust
struct KnownDirectory {
    name: &'static str,
    display: &'static str,
    default_path: &'static str,
    directory_type: DirectoryType,
    default_role: DirectoryRole,
}
const KNOWN_DIRECTORIES: &[KnownDirectory] = &[ ... ];  // 11 entries as of 2026-04-19
```

From `crates/tome/src/wizard.rs:523`:
```rust
fn find_known_directories_in(home: &Path) -> Result<Vec<(&'static KnownDirectory, PathBuf)>>
```

From `crates/tome/src/config.rs:112-136` (authoritative, must agree):
```rust
impl DirectoryType {
    pub fn default_role(&self) -> DirectoryRole {
        match self {
            DirectoryType::ClaudePlugins => DirectoryRole::Managed,
            DirectoryType::Directory => DirectoryRole::Synced,
            DirectoryType::Git => DirectoryRole::Source,
        }
    }
}
```

From `crates/tome/src/config.rs:142-151` — `DirectoryRole` variants: `Managed | Synced | Source | Target`.

Current `KNOWN_DIRECTORIES` snapshot (from wizard.rs:41-119 — 11 entries, stable identifiers):
  1. claude-plugins       ClaudePlugins / Managed  → `.claude/plugins`
  2. claude-skills        Directory     / Synced   → `.claude/skills`
  3. antigravity          Directory     / Synced   → `.gemini/antigravity/skills`
  4. codex                Directory     / Synced   → `.codex/skills`
  5. codex-agents         Directory     / Synced   → `.agents/skills`
  6. openclaw             Directory     / Synced   → `.openclaw/skills`
  7. goose                Directory     / Synced   → `.config/goose/skills`
  8. gemini-cli           Directory     / Synced   → `.gemini/skills`
  9. amp                  Directory     / Synced   → `.config/amp/skills`
 10. opencode             Directory     / Synced   → `.config/opencode/skills`
 11. copilot              Directory     / Synced   → `.copilot/skills`

None have `DirectoryType::Git` — as of v0.7 the wizard does not auto-discover git sources
(git sources are added via `tome add`). `find_known_directories_in` therefore produces only
ClaudePlugins + Directory entries.

Existing 6 wizard tests at `wizard.rs:548-619` that MUST continue to pass:
- `known_directories_has_no_duplicate_names`
- `known_directories_all_have_valid_names`
- `find_known_directories_in_empty_home_returns_empty`
- `find_known_directories_in_discovers_existing_dirs`
- `find_known_directories_in_skips_files_with_same_name`
- `claude_plugins_always_managed`
</interfaces>
</context>

<tasks>

<task type="auto" tdd="false">
  <name>Task 1: Extend wizard.rs test module with registry invariant, multi-dir discovery, and assemble_config coverage</name>
  <files>
    crates/tome/src/wizard.rs
  </files>
  <read_first>
    - crates/tome/src/wizard.rs (focus on the `#[cfg(test)] mod tests` block starting at line 543, especially the existing 6 tests at 548-619, and the `KNOWN_DIRECTORIES` registry at 41-119, and the `find_known_directories_in` signature at 523, and the `assemble_config` helper added by Plan 05-01)
    - crates/tome/src/config.rs (focus on lines 112-136 — `DirectoryType::default_role` and `valid_roles`)
    - .planning/phases/05-wizard-test-coverage/05-CONTEXT.md (D-05, D-06, D-11 — authoritative)
    - .planning/phases/05-wizard-test-coverage/05-01-no-input-plumbing-and-assemble-config-PLAN.md (prerequisite — confirms the `assemble_config` signature this task asserts against)
  </read_first>
  <action>

### Part A — Append new tests to the existing `#[cfg(test)] mod tests` block in `crates/tome/src/wizard.rs`

All new tests live inside the block starting at `wizard.rs:543`, AFTER the existing
`claude_plugins_always_managed` test (line 619) and BEFORE the closing `}` of the mod. Append
exactly the test functions below. Do NOT reorder or modify the existing 6 tests.

Imports: the existing `use super::*;` (wizard.rs:545) already brings `KNOWN_DIRECTORIES`,
`KnownDirectory`, `DirectoryType`, `DirectoryRole`, `DirectoryName`, `DirectoryConfig`,
`Config`, `find_known_directories_in`, and `assemble_config` into scope. `tempfile::TempDir`
is already imported at wizard.rs:546. Add one more import inside the `use super::*;` region
at the top of the mod if not already present — `std::collections::BTreeMap` is already
imported at wizard.rs:10 (so `super::*` re-exports it), and `std::path::PathBuf` via
wizard.rs:11. No new imports needed.

Step A.1 — Add this registry-invariant test (ties KNOWN_DIRECTORIES to DirectoryType::default_role):

```rust
    #[test]
    fn known_directories_default_role_matches_type() {
        // For every entry, the registry's `default_role` must equal what
        // DirectoryType::default_role() returns for that entry's type.
        // Prevents silent drift when new entries are added or enum semantics change.
        for kd in KNOWN_DIRECTORIES {
            assert_eq!(
                kd.default_role,
                kd.directory_type.default_role(),
                "entry '{}': default_role {:?} disagrees with DirectoryType::default_role() for type {:?}",
                kd.name,
                kd.default_role,
                kd.directory_type,
            );
        }
    }
```

Step A.2 — Add this assertion that every entry's default role is in its type's `valid_roles()`:

```rust
    #[test]
    fn known_directories_default_role_is_in_valid_roles() {
        // Every KNOWN_DIRECTORIES default_role must be accepted by its type's valid_roles().
        // If this fails, `tome init` would produce a Config that Config::validate() rejects.
        for kd in KNOWN_DIRECTORIES {
            let valid = kd.directory_type.valid_roles();
            assert!(
                valid.contains(&kd.default_role),
                "entry '{}': default_role {:?} not in valid_roles for type {:?} ({:?})",
                kd.name,
                kd.default_role,
                kd.directory_type,
                valid,
            );
        }
    }
```

Step A.3 — Add a test that discovers every KNOWN_DIRECTORIES entry when HOME contains all of them:

```rust
    #[test]
    fn find_known_directories_in_discovers_every_registry_entry() {
        // Seed HOME with one instance of every registry entry's default_path,
        // then assert find_known_directories_in returns exactly KNOWN_DIRECTORIES.len()
        // results and each expected name appears exactly once.
        let tmp = TempDir::new().unwrap();
        for kd in KNOWN_DIRECTORIES {
            std::fs::create_dir_all(tmp.path().join(kd.default_path)).unwrap();
        }

        let found = find_known_directories_in(tmp.path()).unwrap();
        assert_eq!(
            found.len(),
            KNOWN_DIRECTORIES.len(),
            "expected {} entries, got {}: {:?}",
            KNOWN_DIRECTORIES.len(),
            found.len(),
            found.iter().map(|(kd, _)| kd.name).collect::<Vec<_>>(),
        );

        let mut found_names: Vec<&str> = found.iter().map(|(kd, _)| kd.name).collect();
        found_names.sort();
        let mut expected_names: Vec<&str> = KNOWN_DIRECTORIES.iter().map(|kd| kd.name).collect();
        expected_names.sort();
        assert_eq!(found_names, expected_names);

        // Returned PathBufs are absolute and point under tmp.path().
        for (kd, path) in &found {
            assert!(path.is_absolute(), "entry '{}' path not absolute: {:?}", kd.name, path);
            assert!(
                path.starts_with(tmp.path()),
                "entry '{}' path {:?} not inside TempDir {:?}",
                kd.name,
                path,
                tmp.path(),
            );
        }
    }
```

Step A.4 — Add a test covering discovery of multiple (but not all) known directories:

```rust
    #[test]
    fn find_known_directories_in_discovers_multiple_entries() {
        // Seed two known paths and assert exactly those two come back.
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".claude/skills")).unwrap();
        std::fs::create_dir_all(tmp.path().join(".codex/skills")).unwrap();

        let found = find_known_directories_in(tmp.path()).unwrap();
        let names: std::collections::BTreeSet<&str> =
            found.iter().map(|(kd, _)| kd.name).collect();

        assert_eq!(found.len(), 2, "expected 2 entries, got {}: {:?}", found.len(), names);
        assert!(names.contains("claude-skills"), "missing claude-skills: {:?}", names);
        assert!(names.contains("codex"), "missing codex: {:?}", names);
    }
```

Step A.5 — Add a mixed directory-plus-file test (one valid known dir, one path-occupied-by-file):

```rust
    #[test]
    fn find_known_directories_in_mixed_dir_and_file() {
        // .claude/skills is a valid directory; .codex/skills exists as a file, not a dir.
        // Expect exactly one result: the real directory, with the file-path silently skipped.
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".claude/skills")).unwrap();
        std::fs::create_dir_all(tmp.path().join(".codex")).unwrap();
        std::fs::write(tmp.path().join(".codex/skills"), "i am a file").unwrap();

        let found = find_known_directories_in(tmp.path()).unwrap();
        assert_eq!(found.len(), 1, "expected 1 entry, got {}: {:?}",
            found.len(),
            found.iter().map(|(kd, _)| kd.name).collect::<Vec<_>>(),
        );
        assert_eq!(found[0].0.name, "claude-skills");
    }
```

Step A.6 — Add the `assemble_config` tests. These exercise the Plan 05-01 helper without
dialoguer.

Note on the `DirectoryConfig` struct literal: its `role` field is `pub(crate)` (config.rs:203),
which is accessible from this crate. The wizard's own internal code already constructs
`DirectoryConfig` literals (e.g. wizard.rs:425-434 and 276-284); tests in `wizard::tests` have
the same access.

```rust
    // --- assemble_config tests (WHARD-04) ---

    fn test_dir(path: &str, kind: DirectoryType, role: DirectoryRole) -> DirectoryConfig {
        DirectoryConfig {
            path: PathBuf::from(path),
            directory_type: kind,
            role: Some(role),
            branch: None,
            tag: None,
            rev: None,
            subdir: None,
        }
    }

    #[test]
    fn assemble_config_empty_inputs_produces_empty_config() {
        let config = assemble_config(
            BTreeMap::new(),
            PathBuf::from("~/.tome/skills"),
            std::collections::BTreeSet::new(),
        );
        assert!(config.directories.is_empty());
        assert!(config.exclude.is_empty());
        assert_eq!(config.library_dir, PathBuf::from("~/.tome/skills"));
    }

    #[test]
    fn assemble_config_single_entry_is_preserved() {
        let mut dirs = BTreeMap::new();
        dirs.insert(
            DirectoryName::new("claude-skills").unwrap(),
            test_dir(
                "~/.claude/skills",
                DirectoryType::Directory,
                DirectoryRole::Synced,
            ),
        );

        let config = assemble_config(
            dirs,
            PathBuf::from("~/.tome/skills"),
            std::collections::BTreeSet::new(),
        );

        assert_eq!(config.directories.len(), 1);
        let entry = config
            .directories
            .get("claude-skills")
            .expect("claude-skills must be present");
        assert_eq!(entry.path, PathBuf::from("~/.claude/skills"));
        assert_eq!(entry.directory_type, DirectoryType::Directory);
        assert_eq!(entry.role(), DirectoryRole::Synced);
    }

    #[test]
    fn assemble_config_multi_entry_preserves_all() {
        let mut dirs = BTreeMap::new();
        dirs.insert(
            DirectoryName::new("claude-plugins").unwrap(),
            test_dir(
                "~/.claude/plugins",
                DirectoryType::ClaudePlugins,
                DirectoryRole::Managed,
            ),
        );
        dirs.insert(
            DirectoryName::new("claude-skills").unwrap(),
            test_dir(
                "~/.claude/skills",
                DirectoryType::Directory,
                DirectoryRole::Synced,
            ),
        );
        dirs.insert(
            DirectoryName::new("codex").unwrap(),
            test_dir(
                "~/.codex/skills",
                DirectoryType::Directory,
                DirectoryRole::Synced,
            ),
        );

        let config = assemble_config(
            dirs,
            PathBuf::from("~/.tome/skills"),
            std::collections::BTreeSet::new(),
        );

        assert_eq!(config.directories.len(), 3);
        assert!(config.directories.contains_key("claude-plugins"));
        assert!(config.directories.contains_key("claude-skills"));
        assert!(config.directories.contains_key("codex"));
        assert_eq!(
            config
                .directories
                .get("claude-plugins")
                .unwrap()
                .directory_type,
            DirectoryType::ClaudePlugins,
        );
    }

    #[test]
    fn assemble_config_custom_entry_alongside_known() {
        // Custom dir has a non-registry name but a valid identifier.
        let mut dirs = BTreeMap::new();
        dirs.insert(
            DirectoryName::new("claude-skills").unwrap(),
            test_dir(
                "~/.claude/skills",
                DirectoryType::Directory,
                DirectoryRole::Synced,
            ),
        );
        dirs.insert(
            DirectoryName::new("my-custom").unwrap(),
            test_dir(
                "~/work/team-skills",
                DirectoryType::Directory,
                DirectoryRole::Source,
            ),
        );

        let config = assemble_config(
            dirs,
            PathBuf::from("~/.tome/skills"),
            std::collections::BTreeSet::new(),
        );

        assert_eq!(config.directories.len(), 2);
        let custom = config.directories.get("my-custom").unwrap();
        assert_eq!(custom.path, PathBuf::from("~/work/team-skills"));
        assert_eq!(custom.directory_type, DirectoryType::Directory);
        assert_eq!(custom.role(), DirectoryRole::Source);
    }

    #[test]
    fn assemble_config_exclusions_preserved() {
        let mut exclude = std::collections::BTreeSet::new();
        exclude.insert(crate::discover::SkillName::new("skill-a").unwrap());
        exclude.insert(crate::discover::SkillName::new("skill-b").unwrap());

        let config = assemble_config(
            BTreeMap::new(),
            PathBuf::from("~/.tome/skills"),
            exclude.clone(),
        );

        assert_eq!(config.exclude.len(), 2);
        for name in &exclude {
            assert!(
                config.exclude.contains(name),
                "exclude set missing {:?}",
                name,
            );
        }
    }

    #[test]
    fn assemble_config_library_dir_passed_through_verbatim() {
        // assemble_config does NOT expand tildes or collapse home paths — it's a pure
        // plumbing helper. Verify it leaves library_dir byte-identical to input.
        let lib_tilde = PathBuf::from("~/custom/location");
        let config = assemble_config(
            BTreeMap::new(),
            lib_tilde.clone(),
            std::collections::BTreeSet::new(),
        );
        assert_eq!(config.library_dir, lib_tilde);

        let lib_abs = PathBuf::from("/opt/skills");
        let config = assemble_config(
            BTreeMap::new(),
            lib_abs.clone(),
            std::collections::BTreeSet::new(),
        );
        assert_eq!(config.library_dir, lib_abs);
    }
```

### Part B — Run CI equivalent

```bash
cd /Users/martin/dev/opensource/tome
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test -p tome --lib wizard::tests
```

Every new test must pass. The six pre-existing tests must still pass unchanged.

Do NOT:
- Modify any production code in `wizard.rs` (this plan is tests-only).
- Add tests for `DirectoryType::default_role` in isolation (already covered at `config.rs:711`).
- Add tests for `Config::validate` combo coverage (that's Plan 05-04's job).
- Add integration tests invoking the binary (that's Plan 05-03's job).
- Touch `KNOWN_DIRECTORIES` or any registry ordering.
  </action>
  <verify>
    <automated>cd /Users/martin/dev/opensource/tome && cargo fmt -- --check && cargo clippy --all-targets -- -D warnings && cargo test -p tome --lib wizard::tests</automated>
  </verify>
  <acceptance_criteria>
    - `rg "fn known_directories_default_role_matches_type" crates/tome/src/wizard.rs` returns 1 hit
    - `rg "fn known_directories_default_role_is_in_valid_roles" crates/tome/src/wizard.rs` returns 1 hit
    - `rg "fn find_known_directories_in_discovers_every_registry_entry" crates/tome/src/wizard.rs` returns 1 hit
    - `rg "fn find_known_directories_in_discovers_multiple_entries" crates/tome/src/wizard.rs` returns 1 hit
    - `rg "fn find_known_directories_in_mixed_dir_and_file" crates/tome/src/wizard.rs` returns 1 hit
    - `rg "fn assemble_config_empty_inputs_produces_empty_config" crates/tome/src/wizard.rs` returns 1 hit
    - `rg "fn assemble_config_single_entry_is_preserved" crates/tome/src/wizard.rs` returns 1 hit
    - `rg "fn assemble_config_multi_entry_preserves_all" crates/tome/src/wizard.rs` returns 1 hit
    - `rg "fn assemble_config_custom_entry_alongside_known" crates/tome/src/wizard.rs` returns 1 hit
    - `rg "fn assemble_config_exclusions_preserved" crates/tome/src/wizard.rs` returns 1 hit
    - `rg "fn assemble_config_library_dir_passed_through_verbatim" crates/tome/src/wizard.rs` returns 1 hit
    - `cargo test -p tome --lib wizard::tests::known_directories_default_role_matches_type` exits 0
    - `cargo test -p tome --lib wizard::tests::known_directories_default_role_is_in_valid_roles` exits 0
    - `cargo test -p tome --lib wizard::tests::find_known_directories_in_discovers_every_registry_entry` exits 0
    - `cargo test -p tome --lib wizard::tests::find_known_directories_in_discovers_multiple_entries` exits 0
    - `cargo test -p tome --lib wizard::tests::find_known_directories_in_mixed_dir_and_file` exits 0
    - `cargo test -p tome --lib wizard::tests::assemble_config_empty_inputs_produces_empty_config` exits 0
    - `cargo test -p tome --lib wizard::tests::assemble_config_single_entry_is_preserved` exits 0
    - `cargo test -p tome --lib wizard::tests::assemble_config_multi_entry_preserves_all` exits 0
    - `cargo test -p tome --lib wizard::tests::assemble_config_custom_entry_alongside_known` exits 0
    - `cargo test -p tome --lib wizard::tests::assemble_config_exclusions_preserved` exits 0
    - `cargo test -p tome --lib wizard::tests::assemble_config_library_dir_passed_through_verbatim` exits 0
    - `cargo test -p tome --lib wizard::tests::claude_plugins_always_managed` exits 0 (no regression)
    - `cargo test -p tome --lib wizard::tests::find_known_directories_in_discovers_existing_dirs` exits 0 (no regression)
    - `cargo clippy --all-targets -- -D warnings` exits 0
    - `cargo fmt -- --check` exits 0
  </acceptance_criteria>
  <done>
    `wizard.rs::tests` gains 11 new test functions: 2 registry-invariant tests, 3 `find_known_directories_in` extensions, 6 `assemble_config` tests. All existing tests still pass. `make ci` clean. No production code changed in this plan.
  </done>
</task>

</tasks>

<verification>
Phase-exit checks for Plan 05-02:

1. `cd /Users/martin/dev/opensource/tome && cargo fmt -- --check` exits 0
2. `cd /Users/martin/dev/opensource/tome && cargo clippy --all-targets -- -D warnings` exits 0
3. `cd /Users/martin/dev/opensource/tome && cargo test -p tome --lib wizard::tests` exits 0 (old 6 + new 11 = 17 tests pass)
4. `rg "fn assemble_config_" crates/tome/src/wizard.rs` returns 6 hits (6 assemble_config_* tests)
5. `rg "fn find_known_directories_in_" crates/tome/src/wizard.rs` returns 5 hits (3 old + 3 new test fns — wait, the count: empty_home_returns_empty, discovers_existing_dirs, skips_files_with_same_name (3 existing) + discovers_every_registry_entry, discovers_multiple_entries, mixed_dir_and_file (3 new) = 6. Expect 6 hits)
6. `rg "fn known_directories_" crates/tome/src/wizard.rs` returns 4 hits (has_no_duplicate_names, all_have_valid_names — existing — plus default_role_matches_type and default_role_is_in_valid_roles — new)
</verification>

<success_criteria>
- `cargo test -p tome --lib wizard::tests` runs 17 tests (6 old + 11 new) and passes on ubuntu + macos.
- `KNOWN_DIRECTORIES` now has an in-repo invariant test that ties its entries to `DirectoryType::default_role()` and `valid_roles()` — future additions that drift from the type-system truth fail CI immediately.
- `find_known_directories_in` is exercised across three HOME shapes (existing 3 + new 3 = 6 tests total).
- `assemble_config` is covered for 6 shapes: empty, single, multi, custom-added, exclusions, library_dir pass-through — sufficient to detect any regression that silently drops a field.
- No production code changed — this plan is pure test expansion.
</success_criteria>

<output>
After completion, create `.planning/phases/05-wizard-test-coverage/05-02-pure-helper-unit-tests-SUMMARY.md`.
</output>
