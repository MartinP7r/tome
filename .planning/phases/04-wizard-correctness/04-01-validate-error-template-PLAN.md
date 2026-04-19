---
phase: 4
plan: 1
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/tome/src/config.rs
requirements:
  - WHARD-01
autonomous: true
must_haves:
  truths:
    - "Every existing validate() error message follows the Conflict + Why + Suggestion template"
    - "Every role named in a validation error carries the plain-english parenthetical from DirectoryRole::description()"
    - "A successful tome init still round-trips: the written config passes Config::validate() and reloads without changes (criterion 4 — no regression from rewording)"
  artifacts:
    - path: "crates/tome/src/config.rs"
      provides: "Updated error strings in existing bail! calls inside Config::validate()"
      contains: "Conflict:"
    - path: "crates/tome/src/config.rs"
      provides: "Updated validate_rejects_* unit tests asserting on new substrings"
      contains: "Synced (skills discovered here AND distributed here)"
  key_links:
    - from: "crates/tome/src/config.rs::Config::validate()"
      to: "DirectoryRole::description()"
      via: "direct call in bail! formatting"
      pattern: "\\.description\\(\\)"
---

<objective>
Upgrade the four existing `bail!` calls inside `Config::validate()` to the D-10 Conflict + Why + Suggestion template, and make every role mentioned in an error carry the D-11 plain-english parenthetical (via `DirectoryRole::description()`). Also updates the four corresponding `validate_rejects_*` unit tests so they continue to pass against the new substrings.

Purpose: establish a single, consistent voice for `Config::validate()` errors before Plan 04-02 appends new overlap checks using the same template. Executing the template rewrite first keeps the new code in Plan 04-02 focused on semantics, not style.

Output: `crates/tome/src/config.rs` with updated error messages and updated test assertions. `make ci` green.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/phases/04-wizard-correctness/04-CONTEXT.md

<interfaces>
<!-- Error message template (D-10) is authoritative context. Do not invent alternative phrasing. -->
<!-- Role names MUST use DirectoryRole::description() (D-11) not their Display impl. -->

The D-10 "Conflict + Why + Suggestion" template (VERBATIM):
  - **Conflict:** which fields/directories collide, with paths
  - **Why:** plain-english explanation of the consequence (circular symlinks, self-loop, etc.)
  - **Suggestion:** a concrete alternative

Existing `DirectoryRole::description()` outputs (from config.rs:156 — USE THESE EXACT STRINGS):
  - DirectoryRole::Managed   → "Managed (read-only, owned by package manager)"
  - DirectoryRole::Synced    → "Synced (skills discovered here AND distributed here)"
  - DirectoryRole::Source    → "Source (skills discovered here, not distributed here)"
  - DirectoryRole::Target    → "Target (skills distributed here, not discovered here)"

Existing DirectoryType Display outputs (use these verbatim in "type" references):
  - DirectoryType::ClaudePlugins → "claude-plugins"
  - DirectoryType::Directory     → "directory"
  - DirectoryType::Git           → "git"

Existing four error sites being rewritten (config.rs):
  1. lines 333-337 — library_dir-is-a-file check  (leave this one UNCHANGED — no role/type collision to re-template; see action for exact scope)
  2. lines 346-350 — Managed role with non-ClaudePlugins type
  3. lines 353-358 — Target role with Git type
  4. lines 361-367 — branch/tag/rev on non-Git type
  5. lines 369-375 — subdir on non-Git type

Tests being updated (config.rs:799+):
  - validate_rejects_managed_with_directory_type     (line 799)
  - validate_rejects_target_with_git_type            (line 824)
  - validate_rejects_git_fields_with_non_git_type    (line 849)
  - validate_rejects_library_dir_that_is_a_file      (line 1135) — NOT being re-templated, leave its assertion unchanged
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Rewrite existing validate() error messages to D-10 template</name>
  <files>crates/tome/src/config.rs</files>
  <read_first>
    - crates/tome/src/config.rs (entire file; focus on lines 331-379 `validate()` body and lines 153-163 `DirectoryRole::description()`)
    - .planning/phases/04-wizard-correctness/04-CONTEXT.md (D-10, D-11, D-12 — canonical wording authority)
  </read_first>
  <behavior>
    Each of the four non-library-file checks in `Config::validate()` produces an error that:
    - Names the conflict (field / directory / path) — **Conflict:** component
    - Explains the consequence in plain english — **Why:** component
    - Offers a concrete alternative — **Suggestion:** component (with a `hint:` prefix on a trailing line per Phase 1 pattern)
    - Uses `DirectoryRole::description()` whenever a role is named (D-11)
    - Uses `DirectoryType` `Display` (e.g., "git", "claude-plugins") whenever a type is named

    The library-is-a-file check at config.rs:333-337 is left as-is (no type/role collision; scope decision — D-12 says upgrade the "Managed-only-for-ClaudePlugins, Git-fields-only-for-Git, etc." errors).

    The existing fail-fast behaviour of `bail!` is preserved — these are string changes only, not control-flow changes.

    Test 1 (existing, renamed-or-updated assertion): `validate_rejects_managed_with_directory_type` asserts the new message contains the exact substring `Managed (read-only, owned by package manager)` AND the substring `directory` (type name in plain-english) AND `hint:`.
    Test 2 (existing): `validate_rejects_target_with_git_type` asserts the new message contains `Target (skills distributed here, not discovered here)` AND `git` AND `hint:`.
    Test 3 (existing): `validate_rejects_git_fields_with_non_git_type` asserts the new message contains one of `branch`/`tag`/`rev` AND `git` AND `hint:`.
    Test 4 (new, subdir check has no test today): `validate_rejects_subdir_with_non_git_type` asserts the new message contains `subdir` AND `git` AND `hint:`.
  </behavior>
  <action>
Rewrite the four `bail!` bodies inside `Config::validate()` at config.rs:340-375 to follow the D-10 Conflict + Why + Suggestion template. The file structure (`for (name, dir) in &self.directories { ... }` loop, fail-fast on first error, `anyhow::bail!` call shape) is unchanged.

Use these EXACT message bodies (wording is authoritative; test assertions are locked to these substrings). Line breaks are literal `\n` inside the format string — `bail!` accepts multi-line messages.

---

**Site 1 — Managed role with non-ClaudePlugins type (config.rs:344-350):**
Replace:
```rust
anyhow::bail!(
    "directory '{}': Managed role is only valid with claude-plugins type",
    name
);
```
with:
```rust
anyhow::bail!(
    "directory '{name}': role/type conflict\n\
     Conflict: role is {} but type is '{}'\n\
     Why: the Managed role means skills are owned by a package manager; only the claude-plugins type is known to behave this way, so any other type with Managed would be sync'd incorrectly.\n\
     hint: either change type to 'claude-plugins', or change role to {} or {}.",
    DirectoryRole::Managed.description(),
    dir.directory_type,
    DirectoryRole::Synced.description(),
    DirectoryRole::Source.description(),
);
```

**Site 2 — Target role with Git type (config.rs:353-358):**
Replace:
```rust
anyhow::bail!(
    "directory '{}': Target role is not valid with git type",
    name
);
```
with:
```rust
anyhow::bail!(
    "directory '{name}': role/type conflict\n\
     Conflict: role is {} but type is 'git'\n\
     Why: the Target role means skills are distributed into this directory, but git-type directories are remote clones that tome must not write skills into — pushing symlinks into a git clone would clash with the working tree.\n\
     hint: change role to {} (git repos are read-only skill sources).",
    DirectoryRole::Target.description(),
    DirectoryRole::Source.description(),
);
```

**Site 3 — branch/tag/rev on non-Git type (config.rs:361-367):**
Replace:
```rust
anyhow::bail!(
    "directory '{}': branch/tag/rev fields are only valid with git type",
    name
);
```
with:
```rust
anyhow::bail!(
    "directory '{name}': git ref fields on non-git directory\n\
     Conflict: branch/tag/rev is set but type is '{}'\n\
     Why: branch, tag, and rev pin a remote git clone to a specific commit; they have no meaning for a local directory or a claude-plugins cache.\n\
     hint: either change type to 'git', or remove the branch/tag/rev fields from this directory.",
    dir.directory_type,
);
```

**Site 4 — subdir on non-Git type (config.rs:369-375):**
Replace:
```rust
anyhow::bail!(
    "directory '{}': 'subdir' is only valid for git-type directories",
    name
);
```
with:
```rust
anyhow::bail!(
    "directory '{name}': subdir on non-git directory\n\
     Conflict: subdir is set but type is '{}'\n\
     Why: subdir scopes skill discovery to a sub-path within a remote git clone; for a plain directory you can just point 'path' at the sub-path directly.\n\
     hint: either change type to 'git', or remove 'subdir' and adjust 'path' to point where skills actually live.",
    dir.directory_type,
);
```

Do NOT touch the library-is-a-file check at config.rs:333-337. Leave it exactly as-is.

Then update the three existing tests (config.rs:799, 824, 849) and ADD one new subdir test:

**config.rs:815-820** — `validate_rejects_managed_with_directory_type`:
Replace the `assert!` body with:
```rust
let msg = err.to_string();
assert!(msg.contains("Managed (read-only, owned by package manager)"), "missing role description: {msg}");
assert!(msg.contains("directory"), "missing type name: {msg}");
assert!(msg.contains("hint:"), "missing hint line: {msg}");
```

**config.rs:840-845** — `validate_rejects_target_with_git_type`:
Replace the `assert!` body with:
```rust
let msg = err.to_string();
assert!(msg.contains("Target (skills distributed here, not discovered here)"), "missing role description: {msg}");
assert!(msg.contains("git"), "missing type name: {msg}");
assert!(msg.contains("hint:"), "missing hint line: {msg}");
```

**config.rs:865-870** — `validate_rejects_git_fields_with_non_git_type`:
Replace the `assert!` body with:
```rust
let msg = err.to_string();
assert!(msg.contains("branch") || msg.contains("tag") || msg.contains("rev"), "missing git-field mention: {msg}");
assert!(msg.contains("git"), "missing type name: {msg}");
assert!(msg.contains("hint:"), "missing hint line: {msg}");
```

**ADD a new test** immediately after `validate_rejects_git_fields_with_non_git_type` (i.e., around config.rs:872):
```rust
#[test]
fn validate_rejects_subdir_with_non_git_type() {
    let config = Config {
        directories: BTreeMap::from([(
            DirectoryName::new("bad").unwrap(),
            DirectoryConfig {
                path: PathBuf::from("/tmp"),
                directory_type: DirectoryType::Directory,
                role: None,
                branch: None,
                tag: None,
                rev: None,
                subdir: Some("nested".to_string()),
            },
        )]),
        ..Default::default()
    };
    let err = config.validate().unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("subdir"), "missing 'subdir': {msg}");
    assert!(msg.contains("git"), "missing type name: {msg}");
    assert!(msg.contains("hint:"), "missing hint line: {msg}");
}
```

Do not update `validate_rejects_library_dir_that_is_a_file` (config.rs:1135) — its check is out of scope for D-12.

Finally, run `make ci` locally-equivalent to verify:
```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test -p tome
```
  </action>
  <verify>
    <automated>cd /Users/martin/dev/opensource/tome && cargo test -p tome --lib -- config::tests::validate_rejects_managed_with_directory_type config::tests::validate_rejects_target_with_git_type config::tests::validate_rejects_git_fields_with_non_git_type config::tests::validate_rejects_subdir_with_non_git_type</automated>
  </verify>
  <acceptance_criteria>
    - `rg "Conflict: role is" crates/tome/src/config.rs` returns at least 2 hits (sites 1 and 2)
    - `rg "Conflict: branch/tag/rev" crates/tome/src/config.rs` returns 1 hit (site 3)
    - `rg "Conflict: subdir is set" crates/tome/src/config.rs` returns 1 hit (site 4)
    - `rg "DirectoryRole::Managed.description\(\)" crates/tome/src/config.rs` returns at least 1 hit (site 1 uses it)
    - `rg "DirectoryRole::Target.description\(\)" crates/tome/src/config.rs` returns at least 1 hit (site 2 uses it)
    - `rg "DirectoryRole::Source.description\(\)" crates/tome/src/config.rs` returns at least 2 hits (sites 1 and 2 both suggest Source)
    - `rg "validate_rejects_subdir_with_non_git_type" crates/tome/src/config.rs` returns 1 hit (new test added)
    - `cargo test -p tome --lib config::tests::validate_rejects_managed_with_directory_type` exits 0
    - `cargo test -p tome --lib config::tests::validate_rejects_target_with_git_type` exits 0
    - `cargo test -p tome --lib config::tests::validate_rejects_git_fields_with_non_git_type` exits 0
    - `cargo test -p tome --lib config::tests::validate_rejects_subdir_with_non_git_type` exits 0
    - `cargo test -p tome --lib config::tests::validate_rejects_library_dir_that_is_a_file` exits 0 (regression: we did not touch it)
    - `cargo clippy --all-targets -- -D warnings` exits 0
  </acceptance_criteria>
  <done>
    The four existing validate() bail! bodies follow the Conflict + Why + Suggestion template; role mentions use DirectoryRole::description(); tests assert on the new substrings; `make ci` passes.
  </done>
</task>

</tasks>

<verification>
Phase-exit checks for Plan 04-01:

1. `cd /Users/martin/dev/opensource/tome && cargo fmt -- --check` exits 0
2. `cd /Users/martin/dev/opensource/tome && cargo clippy --all-targets -- -D warnings` exits 0
3. `cd /Users/martin/dev/opensource/tome && cargo test -p tome` exits 0
4. `rg "Conflict:" crates/tome/src/config.rs` returns ≥ 4 hits (one per re-templated `bail!`)
5. `rg "Synced \(skills discovered here AND distributed here\)" crates/tome/src/config.rs` returns ≥ 1 hit (D-11 parenthetical present — comes from site 1's Source suggestion or site 2's fallback)
</verification>

<success_criteria>
- The four `bail!` call bodies in `Config::validate()` (non-library-file checks) follow the D-10 template.
- Every role in an error message is rendered via `DirectoryRole::description()` (D-11).
- No regression: existing tests that didn't target rewritten strings still pass.
- One new test added for the previously-untested subdir-on-non-git case.
- `make ci` clean.
</success_criteria>

<output>
After completion, create `.planning/phases/04-wizard-correctness/04-01-SUMMARY.md`.
</output>
