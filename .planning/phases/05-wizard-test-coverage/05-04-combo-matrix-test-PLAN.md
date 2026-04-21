---
phase: 5
plan: 4
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/tome/src/config.rs
requirements:
  - WHARD-06
autonomous: true
must_haves:
  truths:
    - "`cargo test -p tome --lib config::tests::combo_matrix_*` exercises all 12 `(DirectoryType, DirectoryRole)` combinations — 3 types × 4 roles"
    - "Expected outcome per combo is decided by `DirectoryType::valid_roles().contains(&role)` — not a hand-written allow/deny list (D-08)"
    - "Valid combos (`valid_roles().contains(&role)` is true) pass `Config::save_checked` and re-load to an equivalent `Config` via `Config::load`"
    - "Invalid combos fail `Config::validate()` with an error message containing the role's `description()` substring (D-09)"
    - "Git fields (branch/tag/rev) and `subdir` are ONLY set for the `DirectoryType::Git` combos — otherwise validation rejects them for a different reason than the role/type conflict, which the test explicitly avoids"
  artifacts:
    - path: "crates/tome/src/config.rs"
      provides: "Table-driven test `combo_matrix_all_type_role_pairs` iterating cross-product of DirectoryType × DirectoryRole"
      contains: "combo_matrix_all_type_role_pairs"
    - path: "crates/tome/src/config.rs"
      provides: "Sibling test `combo_matrix_invalid_error_mentions_role_description` asserting error messages include the role's description() for all invalid combos"
      contains: "combo_matrix_invalid_error_mentions_role_description"
    - path: "crates/tome/src/config.rs"
      provides: "Test helper `build_single_entry_config` constructing a one-entry `Config` from (type, role)"
      contains: "fn build_single_entry_config"
  key_links:
    - from: "crates/tome/src/config.rs::tests::combo_matrix_*"
      to: "crates/tome/src/config.rs::DirectoryType::valid_roles"
      via: "decides expected pass/fail per combo"
      pattern: "\\.valid_roles\\(\\)\\.contains\\("
    - from: "crates/tome/src/config.rs::tests::combo_matrix_all_type_role_pairs"
      to: "crates/tome/src/config.rs::Config::save_checked"
      via: "positive-path integration — valid combos must save + reload"
      pattern: "save_checked"
    - from: "crates/tome/src/config.rs::tests::combo_matrix_invalid_error_mentions_role_description"
      to: "crates/tome/src/config.rs::DirectoryRole::description"
      via: "substring assertion on error message"
      pattern: "\\.description\\(\\)"
---

<objective>
Close WHARD-06 with a pair of table-driven tests in `crates/tome/src/config.rs::tests` that cover every `(DirectoryType, DirectoryRole)` combination — 3 types × 4 roles = 12 combos:

| Type          | Role       | Valid? | Why                                                   |
|---------------|------------|--------|-------------------------------------------------------|
| ClaudePlugins | Managed    | YES    | `valid_roles()` for ClaudePlugins == `[Managed]`      |
| ClaudePlugins | Synced     | NO     | Managed-only type, anything else is role/type conflict|
| ClaudePlugins | Source     | NO     | Managed-only type                                     |
| ClaudePlugins | Target     | NO     | Managed-only type                                     |
| Directory     | Managed    | NO     | Managed requires ClaudePlugins type (config.rs:344)   |
| Directory     | Synced     | YES    | Directory default role                                |
| Directory     | Source     | YES    | Directory accepts Source                              |
| Directory     | Target     | YES    | Directory accepts Target                              |
| Git           | Managed    | NO     | Managed requires ClaudePlugins type                   |
| Git           | Synced     | NO     | Git only accepts Source (`valid_roles()`)             |
| Git           | Source     | YES    | Git default role                                      |
| Git           | Target     | NO     | Target explicitly rejected for Git (config.rs:359)    |

Per D-08, the test does NOT hardcode the pass/fail list above — it iterates the cross-product and
asks `DirectoryType::valid_roles().contains(&role)` at runtime. If `valid_roles()` is ever
updated, expectations update automatically. The matrix above is informational only; the code
below does not depend on it.

Per D-09, invalid-combo assertions check that the error message contains the role's
`description()` substring (e.g., `"Managed (read-only, owned by package manager)"`) — stable,
wording-insensitive check that confirms the error fired for the right reason.

Per D-07, valid combos are exercised end-to-end via `Config::save_checked` to a TempDir (so the
test also confirms Phase 4 save_checked handles every wizard-producible combo without
surprises) and the saved file is re-loaded via `Config::load` and compared to the input for
semantic equality on the wizard-relevant fields.

Purpose: close WHARD-06. This is a coverage gate — once landed, any change to `DirectoryType`,
`DirectoryRole`, `valid_roles()`, or `Config::validate()` that regresses any combo fails CI
with a clear error pointing at the offending pair.

Output: two new test functions + one test helper in `config.rs::tests`. No production code
changes. The existing `validate_rejects_managed_with_directory_type` and
`validate_rejects_target_with_git_type` tests (config.rs:927 and 953) stay as focused smoke
tests; this plan ADDS the exhaustive matrix alongside them, it does not replace them.
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
@.planning/phases/04-wizard-correctness/04-CONTEXT.md

<interfaces>
<!-- Load-bearing signatures and values. Test depends on these exactly. -->

From `crates/tome/src/config.rs:92-136` (authoritative enum definitions):
```rust
pub enum DirectoryType {
    ClaudePlugins,
    Directory,   // <-- default
    Git,
}

impl DirectoryType {
    pub fn default_role(&self) -> DirectoryRole { /* ... */ }
    pub fn valid_roles(&self) -> Vec<DirectoryRole> {
        match self {
            DirectoryType::ClaudePlugins => vec![DirectoryRole::Managed],
            DirectoryType::Directory     => vec![DirectoryRole::Synced, DirectoryRole::Source, DirectoryRole::Target],
            DirectoryType::Git           => vec![DirectoryRole::Source],
        }
    }
}
```

From `crates/tome/src/config.rs:140-163`:
```rust
pub enum DirectoryRole {
    Managed, Synced, Source, Target,
}

impl DirectoryRole {
    pub fn description(&self) -> &'static str { /* ... */ }
}
```

From `crates/tome/src/config.rs:485-517` (Phase 4):
```rust
pub fn save_checked(&self, path: &Path) -> Result<()>
```
Runs expand → validate → TOML round-trip → write. Returns Err without writing on any failure.

From `crates/tome/src/config.rs:331-444` — `Config::validate()` rejects:
- Managed role when type != ClaudePlugins (line 344)
- Target role when type == Git (line 359)
- branch/tag/rev set when type != Git (line 372)
- subdir set when type != Git (line 383)

Critical test design point: to isolate role/type failures, the test must NOT set branch/tag/rev
OR subdir for any combo (all `None`). Otherwise the "Git field on non-git type" branch at
line 372 fires FIRST for non-Git types and the test would pass the wrong reason.

Critical test design point: `Config::save_checked` also runs the library-overlap check at
config.rs:394-441 — so the TempDir-based path set must not overlap. The test uses distinct
library_dir and directory.path under different subdirs of the same TempDir to avoid overlap.

Constants the test uses (to iterate the cross-product):
```rust
const ALL_TYPES: [DirectoryType; 3] = [
    DirectoryType::ClaudePlugins,
    DirectoryType::Directory,
    DirectoryType::Git,
];
const ALL_ROLES: [DirectoryRole; 4] = [
    DirectoryRole::Managed,
    DirectoryRole::Synced,
    DirectoryRole::Source,
    DirectoryRole::Target,
];
```

Existing tests that MUST continue to pass (regression surface for this plan):
- `config::tests::validate_rejects_managed_with_directory_type` (config.rs:927)
- `config::tests::validate_rejects_target_with_git_type` (config.rs:953)
- `config::tests::directory_type_valid_roles` (config.rs:724)
- `config::tests::save_checked_rejects_role_type_conflict` (config.rs added by Plan 04-03)
- `config::tests::save_checked_writes_valid_config_and_reloads_unchanged` (config.rs, Plan 04-03)

If this plan's matrix duplicates a specific case, that's fine — the focused smoke tests are
easier to read as documentation; the matrix is the exhaustive guard.

`DirectoryRole` does NOT derive `Copy`. It does derive `Clone + PartialEq`. The cross-product
iterations therefore use `.clone()` when storing the role in the config and `&role` when
comparing — both patterns match existing test style (see config.rs:934).

`DirectoryType` also only derives `Clone + PartialEq`, no `Copy`. Same clone/ref pattern.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="false">
  <name>Task 1: Add cross-product `(DirectoryType, DirectoryRole)` matrix tests to config::tests</name>
  <files>
    crates/tome/src/config.rs
  </files>
  <read_first>
    - crates/tome/src/config.rs (focus on enum definitions at lines 92-136, DirectoryRole at 140-163, Config::validate at 331-444, Config::save_checked at 485-517, the test module starting at line 658, the existing validate_rejects_* tests at 927-999, and the save_checked_* tests added by Plan 04-03)
    - .planning/phases/05-wizard-test-coverage/05-CONTEXT.md (D-07, D-08, D-09 — authoritative)
    - .planning/phases/04-wizard-correctness/04-CONTEXT.md (D-10/D-11 role parenthetical — the description() substring is the match target)
  </read_first>
  <action>

### Part A — Append the matrix tests at the end of the existing `#[cfg(test)] mod tests` block in `crates/tome/src/config.rs`

The `#[cfg(test)] mod tests` block begins at `config.rs:658`. Append the new helper + two new
tests at the end of the block, after the `save_checked_*` tests added by Plan 04-03. Do NOT
reorder or modify any existing test.

Imports: `use super::*;` at config.rs:660 already brings `Config`, `DirectoryName`,
`DirectoryConfig`, `DirectoryRole`, `DirectoryType`, `BTreeMap`, `PathBuf`, `Path`, `SkillName`
into scope. `tempfile::TempDir` is already used inside the test module (e.g. in
`config_load_adds_migration_hint_for_old_sources` at line 868) — it's available via
`tempfile::TempDir::new()`. No new imports needed.

Step A.1 — Add the cross-product constants and a helper that builds a single-entry Config.
Insert BEFORE the first matrix test.

```rust
    // --- Cross-product (DirectoryType, DirectoryRole) matrix (WHARD-06) ---
    //
    // Per D-07/D-08/D-09:
    //   - Every combination is tested, no exclusions.
    //   - Expected pass/fail is derived from `DirectoryType::valid_roles().contains(&role)`,
    //     not a hand-written table — so drift between the wizard's role picker
    //     and the validator is impossible.
    //   - Invalid combos must produce an error message containing the role's
    //     description() substring (Phase 4 D-10/D-11 Conflict+Why+Suggestion format).

    const ALL_TYPES_FOR_MATRIX: [DirectoryType; 3] = [
        DirectoryType::ClaudePlugins,
        DirectoryType::Directory,
        DirectoryType::Git,
    ];
    const ALL_ROLES_FOR_MATRIX: [DirectoryRole; 4] = [
        DirectoryRole::Managed,
        DirectoryRole::Synced,
        DirectoryRole::Source,
        DirectoryRole::Target,
    ];

    /// Build a Config containing exactly one directory entry with the given
    /// (type, role) pair. library_dir and the entry path are placed under
    /// different subdirs of `tmp` to avoid triggering the library-overlap
    /// check in Config::validate — we want role/type failures to surface cleanly.
    ///
    /// The helper deliberately leaves branch/tag/rev/subdir as None for ALL
    /// types (including Git) because those fields have their own validation
    /// paths; this matrix isolates role/type conflicts only.
    fn build_single_entry_config(
        tmp: &std::path::Path,
        dir_type: DirectoryType,
        role: DirectoryRole,
    ) -> Config {
        let library_dir = tmp.join("lib");
        let entry_path = tmp.join("entry");
        let mut directories = BTreeMap::new();
        directories.insert(
            DirectoryName::new("combo").unwrap(),
            DirectoryConfig {
                path: entry_path,
                directory_type: dir_type,
                role: Some(role),
                branch: None,
                tag: None,
                rev: None,
                subdir: None,
            },
        );
        Config {
            library_dir,
            directories,
            ..Default::default()
        }
    }
```

Step A.2 — Add the main matrix test. Valid combos pass save_checked + reload; invalid combos
fail validate() with the role description in the error.

```rust
    #[test]
    fn combo_matrix_all_type_role_pairs() {
        // Iterate the full 3×4 cross-product. Track every combo we touch so
        // the final assertion proves exhaustiveness.
        let mut tested = Vec::new();

        for dir_type in &ALL_TYPES_FOR_MATRIX {
            let valid = dir_type.valid_roles();
            for role in &ALL_ROLES_FOR_MATRIX {
                let combo = (dir_type.clone(), role.clone());
                tested.push(combo.clone());
                let should_pass = valid.contains(role);

                if should_pass {
                    // Valid combo: save_checked to a fresh TempDir, reload,
                    // and confirm the entry's type + role survived the round-trip.
                    let tmp = tempfile::TempDir::new().unwrap();
                    let path = tmp.path().join("tome.toml");
                    let config = build_single_entry_config(
                        tmp.path(),
                        dir_type.clone(),
                        role.clone(),
                    );

                    config.save_checked(&path).unwrap_or_else(|e| {
                        panic!(
                            "expected VALID combo ({:?}, {:?}) to save, but got: {e:#}",
                            dir_type, role,
                        )
                    });
                    assert!(
                        path.exists(),
                        "save_checked reported success but file missing for combo ({:?}, {:?})",
                        dir_type,
                        role,
                    );

                    let reloaded = Config::load(&path).unwrap_or_else(|e| {
                        panic!(
                            "saved VALID combo ({:?}, {:?}) failed to reload: {e:#}",
                            dir_type, role,
                        )
                    });
                    let entry = reloaded
                        .directories
                        .get("combo")
                        .expect("reloaded Config missing 'combo' entry");
                    assert_eq!(
                        &entry.directory_type,
                        dir_type,
                        "reloaded type drifted for combo ({:?}, {:?})",
                        dir_type,
                        role,
                    );
                    assert_eq!(
                        entry.role(),
                        role.clone(),
                        "reloaded role drifted for combo ({:?}, {:?})",
                        dir_type,
                        role,
                    );
                } else {
                    // Invalid combo: validate() must return Err.
                    // We call validate() directly (no TempDir needed) because the
                    // library-overlap check is path-based and we want to isolate
                    // the role/type rejection.
                    //
                    // Idiomatic pattern matching the sibling test below:
                    // `.err().unwrap_or_else(|| panic!(...))` — no custom extension
                    // trait needed. Prior revision of this plan introduced a
                    // `UnwrapErrOrElsePanic` trait; it was redundant with the std
                    // idiom and has been removed for consistency.
                    let tmp_unused =
                        std::path::PathBuf::from(format!("/tmp/combo-{:?}-{:?}", dir_type, role));
                    let config = build_single_entry_config(
                        &tmp_unused,
                        dir_type.clone(),
                        role.clone(),
                    );
                    let _err = config.validate().err().unwrap_or_else(|| {
                        panic!(
                            "expected INVALID combo ({:?}, {:?}) to fail validate(), but it succeeded",
                            dir_type, role,
                        )
                    });
                    // The sibling test `combo_matrix_invalid_error_mentions_role_description`
                    // asserts the error's contents; here we only care that validate()
                    // produced Err for every invalid combo.
                }
            }
        }

        // Exhaustiveness guard: we touched every cell of the 3×4 grid.
        assert_eq!(
            tested.len(),
            ALL_TYPES_FOR_MATRIX.len() * ALL_ROLES_FOR_MATRIX.len(),
            "matrix should test exactly {} combos, got {}",
            ALL_TYPES_FOR_MATRIX.len() * ALL_ROLES_FOR_MATRIX.len(),
            tested.len(),
        );
    }
```

Step A.3 — Add the sibling test that asserts the error message shape for every invalid combo.

```rust
    #[test]
    fn combo_matrix_invalid_error_mentions_role_description() {
        // For every INVALID (type, role), Config::validate() must produce an error
        // message containing the role's description() substring (D-09) AND the word
        // "hint:" (Phase 4 D-10 Conflict+Why+Suggestion template).
        // This is stable against wording tweaks that don't remove the role-description
        // parenthetical or the hint line.

        let tmp_unused = std::path::PathBuf::from("/tmp/does-not-need-to-exist");

        for dir_type in &ALL_TYPES_FOR_MATRIX {
            let valid = dir_type.valid_roles();
            for role in &ALL_ROLES_FOR_MATRIX {
                if valid.contains(role) {
                    continue;
                }

                let config = build_single_entry_config(
                    &tmp_unused,
                    dir_type.clone(),
                    role.clone(),
                );
                let err = config.validate().err().unwrap_or_else(|| {
                    panic!(
                        "INVALID combo ({:?}, {:?}) passed validate() — validator bug",
                        dir_type, role,
                    )
                });
                let msg = err.to_string();

                assert!(
                    msg.contains(role.description()),
                    "error for combo ({:?}, {:?}) missing role description {:?}: {msg}",
                    dir_type,
                    role,
                    role.description(),
                );
                assert!(
                    msg.contains("hint:"),
                    "error for combo ({:?}, {:?}) missing 'hint:' line: {msg}",
                    dir_type,
                    role,
                );
            }
        }
    }
```

### Part B — Run CI equivalent

```bash
cd /Users/martin/dev/opensource/tome
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test -p tome --lib config::tests::combo_matrix_all_type_role_pairs
cargo test -p tome --lib config::tests::combo_matrix_invalid_error_mentions_role_description
cargo test -p tome
```

The full test suite (`cargo test -p tome`) must continue to pass — the existing
`validate_rejects_managed_with_directory_type` and `validate_rejects_target_with_git_type`
tests cover subsets of this matrix; they stay as focused regression guards.

Do NOT:
- Set `branch`/`tag`/`rev`/`subdir` on any combo — those trigger a different validation error
  and would mask role/type conflicts.
- Use `insta::assert_snapshot!` — D-09 explicitly rules out snapshots for this matrix.
- Hand-maintain an expected-pass/fail list — D-08 requires deriving it from `valid_roles()`.
- Assert on exact error wording — only the role `description()` substring + "hint:" substring
  are stable targets.
- Skip any combo "because it's covered elsewhere" — the matrix's value is exhaustiveness.
- Reintroduce a `UnwrapErrOrElsePanic` extension trait. The idiomatic
  `.err().unwrap_or_else(|| panic!(...))` is the standard pattern; both invalid-combo sites use
  it. A custom trait for a single call site is non-idiomatic churn.
  </action>
  <verify>
    <automated>cd /Users/martin/dev/opensource/tome && cargo fmt -- --check && cargo clippy --all-targets -- -D warnings && cargo test -p tome --lib config::tests::combo_matrix_all_type_role_pairs config::tests::combo_matrix_invalid_error_mentions_role_description && cargo test -p tome</automated>
  </verify>
  <acceptance_criteria>
    - `rg "fn combo_matrix_all_type_role_pairs" crates/tome/src/config.rs` returns 1 hit
    - `rg "fn combo_matrix_invalid_error_mentions_role_description" crates/tome/src/config.rs` returns 1 hit
    - `rg "fn build_single_entry_config" crates/tome/src/config.rs` returns 1 hit
    - `rg "ALL_TYPES_FOR_MATRIX" crates/tome/src/config.rs -c` returns at least 3 (constant def + two test loops)
    - `rg "ALL_ROLES_FOR_MATRIX" crates/tome/src/config.rs -c` returns at least 3
    - `rg "\\.valid_roles\\(\\)\\.contains\\(" crates/tome/src/config.rs` returns at least 2 hits (one per matrix test using the derivation rule)
    - `rg "role\\.description\\(\\)" crates/tome/src/config.rs` returns at least 1 hit inside the combo_matrix_invalid_error_mentions_role_description test
    - `rg "UnwrapErrOrElsePanic" crates/tome/src/config.rs` returns 0 hits (no extension trait introduced)
    - `rg "unwrap_err_or_else_panic" crates/tome/src/config.rs` returns 0 hits (no method call to the removed trait)
    - `rg "\\.err\\(\\)\\.unwrap_or_else" crates/tome/src/config.rs -c` returns at least 2 (both matrix tests use the idiomatic pattern)
    - `cargo test -p tome --lib config::tests::combo_matrix_all_type_role_pairs` exits 0
    - `cargo test -p tome --lib config::tests::combo_matrix_invalid_error_mentions_role_description` exits 0
    - `cargo test -p tome --lib config::tests::validate_rejects_managed_with_directory_type` exits 0 (regression)
    - `cargo test -p tome --lib config::tests::validate_rejects_target_with_git_type` exits 0 (regression)
    - `cargo test -p tome --lib config::tests::directory_type_valid_roles` exits 0 (regression)
    - `cargo test -p tome` exits 0
    - `cargo clippy --all-targets -- -D warnings` exits 0
    - `cargo fmt -- --check` exits 0
  </acceptance_criteria>
  <done>
    `config.rs::tests` contains two new test functions (`combo_matrix_all_type_role_pairs` and `combo_matrix_invalid_error_mentions_role_description`) plus a `build_single_entry_config` helper. The tests iterate the 12-element cross-product, derive pass/fail from `DirectoryType::valid_roles()` (no hand-written list), run `save_checked` on valid combos and assert reload preserves type+role, and assert invalid combos fail `validate()` with the role description + hint substrings. Both invalid-combo sites use the idiomatic `.err().unwrap_or_else(|| panic!(...))` pattern — no custom extension trait. The existing focused rejection tests remain unchanged. `make ci` clean.
  </done>
</task>

</tasks>

<verification>
Phase-exit checks for Plan 05-04:

1. `cd /Users/martin/dev/opensource/tome && cargo fmt -- --check` exits 0
2. `cd /Users/martin/dev/opensource/tome && cargo clippy --all-targets -- -D warnings` exits 0
3. `cd /Users/martin/dev/opensource/tome && cargo test -p tome` exits 0
4. `rg "combo_matrix_" crates/tome/src/config.rs` returns at least 2 function-definition hits
5. `rg "ALL_TYPES_FOR_MATRIX" crates/tome/src/config.rs` returns at least 3 hits (definition + 2 uses)
6. `rg "UnwrapErrOrElsePanic" crates/tome/src/config.rs` returns 0 hits (extension trait not introduced)
</verification>

<success_criteria>
- WHARD-06 satisfied: all 12 `(DirectoryType, DirectoryRole)` combinations are exercised in a single table-driven test pair.
- Expected outcomes derive from `DirectoryType::valid_roles().contains(&role)` at runtime — no hand-maintained truth table. Changing `valid_roles()` automatically changes which combos are expected to pass.
- Valid combos round-trip: save_checked → load → same type + same role.
- Invalid combos produce D-10-shaped error messages (role `description()` + `hint:` substrings).
- Both invalid-combo sites use the idiomatic `.err().unwrap_or_else(|| panic!(...))` pattern — no extension trait.
- Existing focused rejection tests continue to pass unchanged; the matrix is additive.
- Plan runs in parallel with Plan 05-01 (Wave 1) — no file conflicts, config.rs test additions are independent of wizard.rs changes.
</success_criteria>

<output>
After completion, create `.planning/phases/05-wizard-test-coverage/05-04-combo-matrix-test-SUMMARY.md`.
</output>
</content>
</invoke>