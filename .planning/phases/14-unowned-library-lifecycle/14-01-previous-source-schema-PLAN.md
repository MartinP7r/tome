---
phase: 14-unowned-library-lifecycle
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/tome/src/manifest.rs
  - crates/tome/src/lockfile.rs
  - crates/tome/src/cleanup.rs
  - crates/tome/src/reconcile.rs
  - crates/tome/src/remove.rs
autonomous: true
requirements:
  - UNOWN-03
gap_closure: false

must_haves:
  truths:
    - "An Unowned `SkillEntry` can record the directory that previously owned it, surviving manifest round-trip."
    - "An Unowned `LockEntry` can record the same value, surviving lockfile round-trip."
    - "All three Owned→Unowned transition sites (cleanup orphan, `tome remove dir`, fork-in-place) capture the previous owner before clearing `source_name`."
  artifacts:
    - path: "crates/tome/src/manifest.rs"
      provides: "previous_source field on SkillEntry; new_unowned signature accepts previous_source; #[allow(dead_code)] removed."
      contains: "previous_source: Option<DirectoryName>"
    - path: "crates/tome/src/lockfile.rs"
      provides: "previous_source field on LockEntry, mirroring SkillEntry."
      contains: "previous_source: Option<DirectoryName>"
    - path: "crates/tome/src/cleanup.rs"
      provides: "Case 1 transition captures previous_source = old source_name before flipping to None."
    - path: "crates/tome/src/reconcile.rs"
      provides: "apply_edit_decisions captures previous_source on fork-in-place flip."
    - path: "crates/tome/src/remove.rs"
      provides: "execute() captures previous_source for each Owned skill before flipping to Unowned."
  key_links:
    - from: "manifest::SkillEntry"
      to: "lockfile::LockEntry"
      via: "lockfile::generate copies previous_source from SkillEntry into LockEntry"
      pattern: "previous_source: entry.previous_source.clone()"
    - from: "cleanup::cleanup_library Case 1"
      to: "manifest::SkillEntry::previous_source"
      via: "skills_get_mut + previous_source = entry.source_name.take()"
    - from: "reconcile::apply_edit_decisions"
      to: "manifest::SkillEntry::previous_source"
      via: "entry.previous_source = entry.source_name.take() before clearing"
    - from: "remove::execute"
      to: "manifest::SkillEntry::previous_source"
      via: "entry.previous_source = entry.source_name.take() in dir-flavour transition loop"
---

<objective>
Add the `previous_source: Option<DirectoryName>` field to `SkillEntry` (manifest)
and `LockEntry` (lockfile), with `#[serde(default, skip_serializing_if =
"Option::is_none")]` so old payloads continue to deserialise. Capture this
field at all three Owned→Unowned transition sites: `cleanup_library` Case 1,
`remove::execute` (dir flavour), and `reconcile::apply_edit_decisions`
(fork-in-place flip). Drop the `#[allow(dead_code)]` from
`SkillEntry::new_unowned` and update its signature to optionally accept the
previous owner.

Purpose: closes the Phase 13 D-13 lossy-fork-in-place gap (and the same gap
in cleanup orphan detection / `tome remove`) so Phase 14's `tome status` and
`tome doctor` Unowned section can render a clean directory name (D-D1, D-C1)
instead of falling back to `source_path`. This is the data-plumbing
prerequisite for plans 14-06 and 14-07.

Output: schema field + 3 transition-site captures + 1 clear-on-re-anchor
hook (the `reassign` clear is delivered in 14-04). All round-trip + transition
behaviours covered by unit tests.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/phases/14-unowned-library-lifecycle/14-CONTEXT.md

# Source-of-truth pattern files (read these to understand what already exists):
@crates/tome/src/manifest.rs
@crates/tome/src/lockfile.rs
@crates/tome/src/cleanup.rs
@crates/tome/src/reconcile.rs
@crates/tome/src/remove.rs

<interfaces>
<!-- The exact serde shape required for backward compatibility — see -->
<!-- D-C1 in 14-CONTEXT.md and the existing source_name pattern. -->

Existing `SkillEntry` shape (manifest.rs):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEntry {
    pub source_path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_name: Option<DirectoryName>,
    pub content_hash: ContentHash,
    pub synced_at: String,
    #[serde(default)]
    pub managed: bool,
}
```

Existing `LockEntry` shape (lockfile.rs):
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LockEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_name: Option<DirectoryName>,
    pub content_hash: ContentHash,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit_sha: Option<String>,
}
```

Existing `cleanup_library` Case 1 transition (cleanup.rs:114-119):
```rust
if !dry_run {
    if let Some(entry) = manifest.skills_get_mut(name.as_str()) {
        entry.source_name = None;
    }
}
```

Existing `apply_edit_decisions` Fork branch (lib.rs:978-984, NOT modified
here — same logic now lives in lib.rs but is reachable; transition site 3
target is reconcile.rs's `apply_edit_decisions`. Verify via grep — if
`apply_edit_decisions` actually lives in `lib.rs::apply_edit_decisions`
(it does, per grep), then this plan modifies lib.rs at that helper, NOT
reconcile.rs. Update files_modified accordingly during execution if needed.

Existing remove::execute Owned→Unowned loop (remove.rs:359-364):
```rust
for skill_name in &plan.skills {
    if let Some(entry) = manifest.skills_get_mut(skill_name) {
        entry.source_name = None;
        library_entries_transitioned_to_unowned += 1;
    }
}
```
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Add `previous_source` field to `SkillEntry` + `LockEntry` and update generate()</name>
  <read_first>
    - crates/tome/src/manifest.rs (entire file — particularly SkillEntry struct around line 100, SkillEntry::new and SkillEntry::new_unowned around line 129-168, and the manifest test module from line 303)
    - crates/tome/src/lockfile.rs (entire file — particularly LockEntry struct around line 30 and the generate() function at line 59-94, plus the lockfile test module from line 233)
    - crates/tome/src/config.rs (DirectoryName newtype — to confirm Option<DirectoryName> derives Serialize/Deserialize cleanly)
    - .planning/phases/14-unowned-library-lifecycle/14-CONTEXT.md (D-C1 — the exact serde attribute shape required)
  </read_first>
  <behavior>
    - Test 1: Round-trip a Manifest with one Owned and one Unowned (with previous_source = Some) entry — both fields preserve.
    - Test 2: Old-shape JSON (no `previous_source` key) deserialises with `previous_source = None`.
    - Test 3: An Unowned entry with `previous_source = None` serialises WITHOUT a `"previous_source"` key (skip_serializing_if).
    - Test 4: `LockEntry` round-trip with `previous_source = Some(...)`.
    - Test 5: `lockfile::generate` copies `previous_source` from each `SkillEntry` into the corresponding `LockEntry`.
    - Test 6: `SkillEntry::new_unowned(source_path, content_hash, managed, previous_source)` records `previous_source` and sets `source_name = None`.
  </behavior>
  <action>
    1. **manifest.rs — `SkillEntry` struct.** After the existing `source_name` field (currently lines 113-114), insert:

    ```rust
    /// Last directory that owned this skill before transition to Unowned.
    /// Surfaced in `tome status`/`tome doctor` Unowned section. Cleared
    /// (set to None) when an Unowned skill is re-anchored via
    /// `tome reassign`. Per D-C1 (14-CONTEXT.md).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_source: Option<DirectoryName>,
    ```

    2. **manifest.rs — `SkillEntry::new`.** Update the constructor body to include
       `previous_source: None,` in the struct literal (Owned skills have no previous).
       The function signature is unchanged.

    3. **manifest.rs — `SkillEntry::new_unowned`.**
       - Change signature from
         `pub fn new_unowned(source_path: PathBuf, content_hash: ContentHash, managed: bool) -> Self`
         to
         `pub fn new_unowned(source_path: PathBuf, content_hash: ContentHash, managed: bool, previous_source: Option<DirectoryName>) -> Self`.
       - Set `previous_source` in the struct literal from the argument.
       - **Drop the `#[allow(dead_code)]` attribute and its 4-line doc-comment justification** (lines 151-159 in current code) — Phase 14 has callers (this plan + 14-04 + 14-05 indirectly).

    4. **lockfile.rs — `LockEntry` struct.** After the existing `source_name` field (currently lines 40-41), insert:

    ```rust
    /// Last directory that owned this skill before transition to Unowned.
    /// Mirrors `SkillEntry.previous_source` (D-C1) for cross-machine
    /// surfacing in `tome status` / `tome doctor` Unowned section.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_source: Option<DirectoryName>,
    ```

    5. **lockfile.rs — `generate()`.** Inside the loop body at lines 65-88, when constructing the `LockEntry` (currently at lines 79-87), add `previous_source: entry.previous_source.clone(),` immediately after the `source_name: entry.source_name.clone(),` line.

    6. **manifest.rs — tests module.** Update existing test bodies that build a `SkillEntry` literal directly (search for `SkillEntry {` in `#[cfg(test)] mod tests` — there are several — and add `previous_source: None,` to each literal). DO NOT touch `SkillEntry::new(...)` call sites (those keep working unchanged). Update the existing `serialize_unowned_entry_omits_source_name_key` and `new_unowned_constructor_sets_source_name_none` tests to pass the new `previous_source` argument as `None`.

    7. **manifest.rs — add new tests** (in `#[cfg(test)] mod tests`):

    ```rust
    #[test]
    fn deserialize_old_shape_without_previous_source_key() {
        let valid_hash = "a".repeat(64);
        let json = format!(
            r#"{{"source_path":"/tmp/x","source_name":"foo","content_hash":"{valid_hash}","synced_at":"2024-01-01T00:00:00Z","managed":false}}"#
        );
        let entry: SkillEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.previous_source, None);
    }

    #[test]
    fn serialize_owned_entry_omits_previous_source_when_none() {
        let entry = SkillEntry::new(
            PathBuf::from("/tmp/x"),
            DirectoryName::new("foo").unwrap(),
            test_hash("h"),
            false,
        );
        let json = serde_json::to_string(&entry).unwrap();
        assert!(
            !json.contains("previous_source"),
            "Owned entry with previous_source=None must omit key, got: {json}"
        );
    }

    #[test]
    fn serialize_unowned_entry_with_previous_source_includes_key() {
        let entry = SkillEntry::new_unowned(
            PathBuf::from("/tmp/x"),
            test_hash("h"),
            false,
            Some(DirectoryName::new("old-dir").unwrap()),
        );
        let json = serde_json::to_string(&entry).unwrap();
        assert!(
            json.contains("\"previous_source\":\"old-dir\""),
            "Unowned entry with previous_source=Some must include key, got: {json}"
        );
    }

    #[test]
    fn new_unowned_records_previous_source() {
        let entry = SkillEntry::new_unowned(
            PathBuf::from("/tmp/x"),
            test_hash("h"),
            false,
            Some(DirectoryName::new("old").unwrap()),
        );
        assert_eq!(entry.source_name, None);
        assert_eq!(entry.previous_source, Some(DirectoryName::new("old").unwrap()));
    }
    ```

    8. **lockfile.rs — tests module.** Locate `LockEntry { ... }` literal constructions in tests and add `previous_source: None,` to each. Update `make_discovered`-using tests as needed (they go through `generate()` which now propagates `previous_source` from the manifest entry).

    9. **lockfile.rs — add new tests:**

    ```rust
    #[test]
    fn lockentry_round_trip_with_previous_source() {
        use crate::manifest::SkillEntry;
        let mut manifest = Manifest::default();
        manifest.insert(
            SkillName::new("orphan").unwrap(),
            SkillEntry::new_unowned(
                PathBuf::from("/tmp/orphan"),
                test_hash("h"),
                false,
                Some(DirectoryName::new("old-source").unwrap()),
            ),
        );
        let lf = generate(&manifest, &[]);
        let key = SkillName::new("orphan").unwrap();
        assert_eq!(
            lf.skills[&key].previous_source,
            Some(DirectoryName::new("old-source").unwrap()),
            "generate() must copy previous_source from manifest entry"
        );

        // Round-trip through JSON.
        let json = serde_json::to_string_pretty(&lf).unwrap();
        let parsed: Lockfile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, lf);
    }

    #[test]
    fn deserialize_old_shape_lockfile_without_previous_source() {
        let valid_hash = "a".repeat(64);
        let json = format!(r#"{{"source_name":"foo","content_hash":"{valid_hash}"}}"#);
        let entry: LockEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.previous_source, None);
    }
    ```

    10. **Update other call sites** — search the codebase for all callers of `SkillEntry::new_unowned(` and update each invocation to pass `None` (or `Some(...)` where appropriate) as the new fourth argument. Likely call sites: tests in cleanup.rs, tests in lockfile.rs, tests in remove.rs, tests in reconcile.rs, tests in lib.rs / cli.rs integration. Use `rg "new_unowned\(" crates/tome` to find them.
  </action>
  <verify>
    <automated>cargo test -p tome --lib manifest::tests::deserialize_old_shape_without_previous_source_key manifest::tests::serialize_owned_entry_omits_previous_source_when_none manifest::tests::serialize_unowned_entry_with_previous_source_includes_key manifest::tests::new_unowned_records_previous_source lockfile::tests::lockentry_round_trip_with_previous_source lockfile::tests::deserialize_old_shape_lockfile_without_previous_source</automated>
  </verify>
  <acceptance_criteria>
    - `grep -q "pub previous_source: Option<DirectoryName>" crates/tome/src/manifest.rs` succeeds
    - `grep -q "pub previous_source: Option<DirectoryName>" crates/tome/src/lockfile.rs` succeeds
    - `grep -q "previous_source: entry.previous_source.clone()" crates/tome/src/lockfile.rs` succeeds (generate() propagates the field)
    - `grep -q "#\[allow(dead_code)\]" crates/tome/src/manifest.rs` returns NO output for `new_unowned` (the attribute and its multi-line justification doc comment must be gone) — `grep -B1 -A4 "pub fn new_unowned" crates/tome/src/manifest.rs | grep -q "allow(dead_code)"` exits 1
    - `grep -q "previous_source: Option<DirectoryName>" crates/tome/src/manifest.rs` shows the new_unowned signature has the new parameter — verify via `grep -A3 "pub fn new_unowned" crates/tome/src/manifest.rs | grep -q "previous_source: Option<DirectoryName>"`
    - `cargo test -p tome --lib manifest::tests::deserialize_old_shape_without_previous_source_key` exits 0
    - `cargo test -p tome --lib manifest::tests::serialize_owned_entry_omits_previous_source_when_none` exits 0
    - `cargo test -p tome --lib manifest::tests::serialize_unowned_entry_with_previous_source_includes_key` exits 0
    - `cargo test -p tome --lib lockfile::tests::lockentry_round_trip_with_previous_source` exits 0
    - `cargo test -p tome --lib lockfile::tests::deserialize_old_shape_lockfile_without_previous_source` exits 0
  </acceptance_criteria>
  <done>
    `SkillEntry` and `LockEntry` carry a `previous_source` field that survives manifest+lockfile round-trip; old-shape payloads still deserialise (returning `None`); `Option::is_none` skip means already-shipped manifests don't grow the key. `SkillEntry::new_unowned` accepts the field and is no longer marked dead code.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Capture `previous_source` at the three Owned→Unowned transition sites</name>
  <read_first>
    - crates/tome/src/cleanup.rs (lines 102-122 — the Case 1 transition loop)
    - crates/tome/src/remove.rs (lines 358-365 — the dir-flavour Owned→Unowned loop in `execute`)
    - crates/tome/src/lib.rs (lines 963-1003 — the actual home of `apply_edit_decisions`; verify with `grep -n "fn apply_edit_decisions" crates/tome/src/lib.rs` — CONTEXT.md says reconcile.rs but it's actually lib.rs in current code; use whichever the grep confirms)
    - .planning/phases/14-unowned-library-lifecycle/14-CONTEXT.md (D-C1 — the three transition sites)
    - crates/tome/src/manifest.rs (after Task 1 lands — to confirm `previous_source` field exists)
  </read_first>
  <behavior>
    - Test 1 (cleanup): Case 1 transition records `previous_source = Some(<old source_name>)` AND clears `source_name = None` for each transitioned skill.
    - Test 2 (remove::execute): the dir-flavour Owned→Unowned loop captures `previous_source = Some(<old source_name>)` for each skill on full-success path.
    - Test 3 (apply_edit_decisions Fork branch): records `previous_source = Some(<old source_name>)` before clearing source_name (only on the Fork branch — Revert and Skip are unchanged).
  </behavior>
  <action>
    1. **cleanup.rs — Case 1 transition.** Locate the block currently at cleanup.rs lines 113-119 (after the `eprintln!("info: skill ...")` line and before the `result.transitioned_to_unowned += 1;` line) — specifically the inner `if !dry_run { if let Some(entry) = manifest.skills_get_mut(name.as_str()) { entry.source_name = None; } }`. Replace the inner mutation with:

    ```rust
    if !dry_run {
        // Per D-C1 (Phase 14): capture previous_source before clearing
        // source_name so tome status / tome doctor can render a clean
        // directory name in the Unowned section instead of falling back
        // to source_path. The .take() pattern atomically moves the old
        // value into previous_source and leaves source_name = None.
        if let Some(entry) = manifest.skills_get_mut(name.as_str()) {
            entry.previous_source = entry.source_name.take();
        }
    }
    ```

    2. **remove.rs — dir-flavour transition.** Locate the loop at lines 359-364 and update the body of the `if let Some(entry) = ...` block:

    ```rust
    for skill_name in &plan.skills {
        if let Some(entry) = manifest.skills_get_mut(skill_name) {
            // Per D-C1 (Phase 14, transition site 2): capture
            // previous_source before flipping to Unowned so the user can
            // see the original owner name in `tome status` after this
            // directory is gone from config.
            entry.previous_source = entry.source_name.take();
            library_entries_transitioned_to_unowned += 1;
        }
    }
    ```

    Note: `entry.source_name.take()` returns the old `Some(DirectoryName)` and replaces with `None` in one step — semantically identical to the existing `entry.source_name = None;` for the source_name effect, plus also stores the old value in `previous_source`.

    3. **lib.rs OR reconcile.rs — `apply_edit_decisions` fork branch.** Verify via `grep -n "fn apply_edit_decisions" crates/tome/src/`. The current home is `crates/tome/src/lib.rs` (line ~963). Locate the `EditDecision::Fork` arm (lib.rs:978-984):

    ```rust
    reconcile::EditDecision::Fork => {
        if let Some(entry) = manifest.skills_get_mut(edit.name.as_str()) {
            entry.managed = false;
            entry.source_name = None;
            mutated = true;
        }
    }
    ```

    Replace the body of the inner `if let Some(entry) = ...` block with:

    ```rust
    reconcile::EditDecision::Fork => {
        if let Some(entry) = manifest.skills_get_mut(edit.name.as_str()) {
            entry.managed = false;
            // Per D-C1 (Phase 14, transition site 3): capture
            // previous_source before clearing source_name. Closes the
            // Phase 13 D-13 lossy fork-in-place gap.
            entry.previous_source = entry.source_name.take();
            mutated = true;
        }
    }
    ```

    4. **cleanup.rs — add new test:**

    ```rust
    #[test]
    fn cleanup_case1_records_previous_source() {
        let library = TempDir::new().unwrap();
        let skill_dir = library.path().join("orphan");
        std::fs::create_dir_all(&skill_dir).unwrap();

        let mut manifest = Manifest::default();
        manifest.insert(
            crate::discover::SkillName::new("orphan").unwrap(),
            crate::manifest::SkillEntry::new(
                std::path::PathBuf::from("/tmp/removed/orphan"),
                crate::config::DirectoryName::new("removed-source").unwrap(),
                crate::validation::test_hash("h"),
                false,
            ),
        );

        let config = empty_config();
        let discovered: HashSet<String> = HashSet::new();
        let result = cleanup_library(
            library.path(),
            &discovered,
            &mut manifest,
            &config,
            false,
            false,
            true,
        )
        .unwrap();
        assert_eq!(result.transitioned_to_unowned, 1);
        let entry = manifest.get("orphan").unwrap();
        assert_eq!(entry.source_name, None, "source_name cleared");
        assert_eq!(
            entry.previous_source,
            Some(crate::config::DirectoryName::new("removed-source").unwrap()),
            "previous_source must record the original owner per D-C1"
        );
    }
    ```

    5. **remove.rs — add new test** (in `#[cfg(test)] mod tests`):

    ```rust
    #[test]
    fn execute_records_previous_source_on_unowned_transition() {
        let (_tmp, mut config, paths, mut manifest) = make_test_setup();
        let p = plan("test-source", &config, &paths, &manifest).unwrap();

        let result = execute(&p, &mut config, &mut manifest, false).unwrap();
        assert_eq!(result.library_entries_transitioned_to_unowned, 1);

        let entry = manifest.get("my-skill").unwrap();
        assert_eq!(entry.source_name, None);
        assert_eq!(
            entry.previous_source,
            Some(DirectoryName::new("test-source").unwrap()),
            "previous_source must record the original owner per D-C1"
        );
    }
    ```

    6. **lib.rs — add new test for apply_edit_decisions Fork branch.** Place this in the existing test module that exercises `apply_edit_decisions` (search via `rg "fn .*apply_edit" crates/tome/src/lib.rs`; if no test exists, add one inside `#[cfg(test)] mod tests` near the function):

    ```rust
    #[test]
    fn apply_edit_decisions_fork_records_previous_source() {
        // Build a minimal manifest with one Owned managed skill.
        let tmp = tempfile::TempDir::new().unwrap();
        let paths = TomePaths::new(tmp.path().to_path_buf(), tmp.path().join("library")).unwrap();
        std::fs::create_dir_all(paths.library_dir()).unwrap();

        let mut manifest = manifest::Manifest::default();
        manifest.insert(
            discover::SkillName::new("plug").unwrap(),
            manifest::SkillEntry::new(
                std::path::PathBuf::from("/tmp/plug"),
                config::DirectoryName::new("claude-plugins").unwrap(),
                validation::test_hash("h"),
                true, // managed
            ),
        );
        manifest::save(&manifest, paths.config_dir()).unwrap();

        // Build a ReconcileReport with one Edited entry and Fork decision.
        let report = reconcile::ReconcileReport {
            edited: vec![reconcile::EditedSkill {
                name: discover::SkillName::new("plug").unwrap(),
            }],
            edit_decisions: vec![reconcile::EditDecision::Fork],
            ..Default::default()
        };

        apply_edit_decisions(&report, &paths, false).unwrap();

        let reloaded = manifest::load(paths.config_dir()).unwrap();
        let entry = reloaded.get("plug").unwrap();
        assert_eq!(entry.source_name, None, "fork-in-place clears source_name");
        assert!(!entry.managed, "fork-in-place clears managed");
        assert_eq!(
            entry.previous_source,
            Some(config::DirectoryName::new("claude-plugins").unwrap()),
            "fork-in-place must record previous_source per D-C1 / Phase 13 D-13 closure"
        );
    }
    ```

    Note: the exact `EditedSkill` shape may differ — read `reconcile.rs` to find the actual constructor and adapt. If there is no public constructor, use `..Default::default()` or whichever pattern reconcile.rs uses elsewhere (e.g., the existing `report_default_edit_decisions_empty` test referenced at lib.rs:1858-1863 may show the pattern). Adjust the test scaffold to compile.

    7. Run `cargo build -p tome` after each step to catch compile errors early.
  </action>
  <verify>
    <automated>cargo test -p tome --lib cleanup::tests::cleanup_case1_records_previous_source remove::tests::execute_records_previous_source_on_unowned_transition</automated>
  </verify>
  <acceptance_criteria>
    - `grep -q "entry.previous_source = entry.source_name.take()" crates/tome/src/cleanup.rs` succeeds
    - `grep -q "entry.previous_source = entry.source_name.take()" crates/tome/src/remove.rs` succeeds
    - `grep -q "entry.previous_source = entry.source_name.take()" crates/tome/src/lib.rs` succeeds (or `crates/tome/src/reconcile.rs` if `apply_edit_decisions` was moved — check via `rg "fn apply_edit_decisions" crates/tome/src` first)
    - `cargo test -p tome --lib cleanup::tests::cleanup_case1_records_previous_source` exits 0
    - `cargo test -p tome --lib remove::tests::execute_records_previous_source_on_unowned_transition` exits 0
    - All pre-existing tests still pass: `cargo test -p tome --lib cleanup::tests` exits 0
    - All pre-existing tests still pass: `cargo test -p tome --lib remove::tests` exits 0
    - `cargo clippy --all-targets -p tome -- -D warnings` exits 0
  </acceptance_criteria>
  <done>
    All three Owned→Unowned transitions capture the previous owner. Tests cover each site. Existing tests still pass. Clippy clean.
  </done>
</task>

</tasks>

<verification>
- `cargo test -p tome` exits 0 (full test suite passes)
- `cargo clippy --all-targets -p tome -- -D warnings` exits 0
- `cargo fmt -p tome -- --check` exits 0
- Old manifest payloads on disk still load (verified by the `deserialize_old_shape_without_previous_source_key` test which round-trips JSON without the new key)
</verification>

<success_criteria>
- `SkillEntry::previous_source` and `LockEntry::previous_source` exist with serde defaults.
- `SkillEntry::new_unowned` signature includes `previous_source: Option<DirectoryName>`; `#[allow(dead_code)]` is gone.
- All three transition sites (`cleanup_library` Case 1, `remove::execute` dir flavour, `apply_edit_decisions` Fork branch) capture `previous_source = entry.source_name.take()` before clearing.
- 6+ new unit tests pass (3 manifest, 2 lockfile, 1 cleanup, 1 remove, 1 lib.rs/reconcile fork).
- Full test suite green; clippy -D warnings clean.
</success_criteria>

<output>
After completion, create `.planning/phases/14-unowned-library-lifecycle/14-01-SUMMARY.md`
</output>
