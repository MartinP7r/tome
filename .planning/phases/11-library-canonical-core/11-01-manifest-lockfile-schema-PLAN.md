---
phase: 11-library-canonical-core
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/tome/src/manifest.rs
  - crates/tome/src/lockfile.rs
autonomous: true
requirements:
  - LIB-03
must_haves:
  truths:
    - "Old-shape manifests with `source_name: \"foo\"` deserialize as `Some(DirectoryName::new(\"foo\")?)` without errors."
    - "New-shape manifests with `source_name: null` or missing `source_name` deserialize as `None`."
    - "Old-shape lockfiles with `source_name: \"foo\"` deserialize as `Some(DirectoryName::new(\"foo\")?)` without errors."
    - "Round-tripping a manifest entry with `source_name = None` writes JSON without a `source_name` key (skip_serializing_if)."
    - "Round-tripping a manifest entry with `source_name = Some(...)` writes the same string shape as today (`\"source_name\": \"foo\"`)."
    - "All existing call-sites continue to work via the existing `SkillEntry::new(...)` constructor (still takes owned `DirectoryName`)."
    - "A new `SkillEntry::new_unowned(source_path, content_hash, managed)` constructor produces an entry with `source_name = None`."
  artifacts:
    - path: "crates/tome/src/manifest.rs"
      provides: "SkillEntry with `source_name: Option<DirectoryName>`, `new_unowned` constructor"
      contains: "Option<DirectoryName>"
    - path: "crates/tome/src/lockfile.rs"
      provides: "LockEntry with `source_name: Option<DirectoryName>`"
      contains: "Option<DirectoryName>"
  key_links:
    - from: "manifest.rs::SkillEntry"
      to: "Option<DirectoryName> serde defaults"
      via: "#[serde(default, skip_serializing_if = \"Option::is_none\")]"
      pattern: "skip_serializing_if = \"Option::is_none\""
    - from: "lockfile.rs::LockEntry"
      to: "Option<DirectoryName> serde defaults"
      via: "#[serde(default, skip_serializing_if = \"Option::is_none\")]"
      pattern: "skip_serializing_if = \"Option::is_none\""
---

<objective>
Lift the schema for manifest and lockfile to make `source_name` optional, representing
the new Unowned state introduced in v0.10. This is the foundational change all later
Phase 11 plans build on.

Implements LIB-03 (per CONTEXT.md decisions D-12, D-13, D-14).

Purpose: Wave 1 — independent foundation. Until this lands, no other Phase 11 plan
can compile-meaningfully (call-sites in library.rs, cleanup.rs, remove.rs all touch
SkillEntry/LockEntry).

Output: `manifest.rs` and `lockfile.rs` updated; tests cover old-shape compatibility
and new Unowned round-trip. All existing call-sites unchanged (still pass owned
`DirectoryName` via `SkillEntry::new`).
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/phases/11-library-canonical-core/11-CONTEXT.md
@CLAUDE.md
@crates/tome/src/manifest.rs
@crates/tome/src/lockfile.rs

<interfaces>
<!-- Current `SkillEntry` shape (will be modified): -->
```rust
// crates/tome/src/manifest.rs
pub struct SkillEntry {
    pub source_path: PathBuf,
    pub source_name: DirectoryName,           // <-- becomes Option<DirectoryName>
    pub content_hash: ContentHash,
    pub synced_at: String,
    #[serde(default)]
    pub managed: bool,
}

impl SkillEntry {
    pub fn new(
        source_path: PathBuf,
        source_name: DirectoryName,            // <-- KEEP — this is the owned constructor
        content_hash: ContentHash,
        managed: bool,
    ) -> Self { ... }
}
```

<!-- Current `LockEntry` shape (will be modified): -->
```rust
// crates/tome/src/lockfile.rs
pub struct LockEntry {
    pub source_name: DirectoryName,           // <-- becomes Option<DirectoryName>
    pub content_hash: ContentHash,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit_sha: Option<String>,
}
```

<!-- `DirectoryName` is already a transparent newtype with validating Deserialize, so
     `Option<DirectoryName>` parsing comes "for free" from serde's natural Option handling.
     No custom deserializer is needed. -->
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Lift `SkillEntry.source_name` to `Option<DirectoryName>` and add `new_unowned` constructor</name>
  <files>crates/tome/src/manifest.rs</files>
  <read_first>
    - crates/tome/src/manifest.rs (current SkillEntry struct, `new` constructor, `update_source_name`, all `mod tests`)
    - .planning/phases/11-library-canonical-core/11-CONTEXT.md (D-12, D-13 for exact serde attributes and constructor shape)
  </read_first>
  <behavior>
    - Test 1 (old-shape compat): `serde_json::from_str(r#"{"source_path":"/tmp/x","source_name":"foo","content_hash":"<64-hex>","synced_at":"2024-01-01T00:00:00Z","managed":false}"#)` deserializes to `SkillEntry { source_name: Some(DirectoryName::new("foo").unwrap()), ... }`.
    - Test 2 (new-shape Unowned, null): `serde_json::from_str(r#"{"source_path":"/tmp/x","source_name":null,"content_hash":"<64-hex>","synced_at":"2024-01-01T00:00:00Z","managed":false}"#)` deserializes to `SkillEntry { source_name: None, ... }`.
    - Test 3 (new-shape Unowned, missing key): `serde_json::from_str(r#"{"source_path":"/tmp/x","content_hash":"<64-hex>","synced_at":"2024-01-01T00:00:00Z","managed":false}"#)` deserializes to `SkillEntry { source_name: None, ... }`.
    - Test 4 (round-trip Unowned omits key): an entry constructed with `SkillEntry::new_unowned(...)` serialized to JSON has NO `"source_name"` key (because of `skip_serializing_if`).
    - Test 5 (round-trip owned preserves shape): `SkillEntry::new(path, DirectoryName::new("foo")?, hash, false)` serializes to JSON containing `"source_name": "foo"` (no `Some(...)` wrapping in JSON).
    - Test 6 (`new_unowned` constructor): `SkillEntry::new_unowned(PathBuf::from("/tmp/x"), test_hash("h"), false)` returns an entry with `source_name == None`, `source_path == "/tmp/x"`, `content_hash == test_hash("h")`, `managed == false`, and a non-empty `synced_at`.
    - Test 7 (`update_source_name` still works for owned entries): existing `update_source_name("name", &new_dir)` test must still pass (entry must already have `source_name = Some(...)` to be updated; behavior change documented below).
  </behavior>
  <action>
1. **Change `SkillEntry.source_name` field declaration.**
   Replace exactly:
   ```rust
       /// Which directory config entry contributed this skill.
       /// In v0.6+, this is the directory name from `[directories.*]` in `tome.toml`.
       /// On-disk JSON representation is unchanged (`DirectoryName` is `#[serde(transparent)]`
       /// over `String`); the type lift to `DirectoryName` (closes #489) tightens validation
       /// at deserialize time so a corrupted manifest with an invalid identifier fails fast.
       pub source_name: DirectoryName,
   ```
   with:
   ```rust
       /// Which directory config entry contributed this skill, or `None` if the
       /// skill is **Unowned** (its source was removed from `tome.toml` but the
       /// library copy is preserved per LIB-04).
       ///
       /// Old manifests with `"source_name": "foo"` parse as `Some(DirectoryName::new("foo")?)`
       /// via serde's natural `Option` handling + `DirectoryName`'s transparent
       /// validating `Deserialize`. New Unowned entries serialize without the key
       /// (per `skip_serializing_if`) and read back as `None`.
       #[serde(default, skip_serializing_if = "Option::is_none")]
       pub source_name: Option<DirectoryName>,
   ```

2. **Update `SkillEntry::new` to wrap in `Some(...)`** (constructor signature unchanged so call-sites don't need to change):
   ```rust
       /// Create a new `SkillEntry` for an **owned** skill (source_name known).
       /// Records the current timestamp automatically.
       pub fn new(
           source_path: PathBuf,
           source_name: DirectoryName,
           content_hash: ContentHash,
           managed: bool,
       ) -> Self {
           Self {
               source_path,
               source_name: Some(source_name),
               content_hash,
               synced_at: now_iso8601(),
               managed,
           }
       }
   ```

3. **Add `SkillEntry::new_unowned` constructor** (per D-13). Add this method to the `impl SkillEntry` block, right after `new`:
   ```rust
       /// Create a new `SkillEntry` for an **Unowned** skill — its source was
       /// removed from `tome.toml` but the library copy is preserved (per LIB-04).
       /// Records the current timestamp automatically.
       pub fn new_unowned(
           source_path: PathBuf,
           content_hash: ContentHash,
           managed: bool,
       ) -> Self {
           Self {
               source_path,
               source_name: None,
               content_hash,
               synced_at: now_iso8601(),
               managed,
           }
       }
   ```

4. **Update `Manifest::update_source_name`** so it works against the new `Option` shape. Replace the body so a missing or `None` `source_name` returns `false` (skill not found OR not currently owned), and a `Some` value updates in place:
   ```rust
       /// Update the source_name for an existing **owned** skill entry.
       ///
       /// Returns `true` if the skill was found AND was owned (source_name = Some),
       /// `false` if missing or already Unowned. Preserves `content_hash`,
       /// `synced_at`, and other fields. Does NOT transition Unowned → Owned —
       /// callers wanting that semantic should re-insert with `SkillEntry::new`.
       pub fn update_source_name(&mut self, skill_name: &str, new_source: &DirectoryName) -> bool {
           if let Some(entry) = self.skills.get_mut(skill_name)
               && entry.source_name.is_some()
           {
               entry.source_name = Some(new_source.clone());
               true
           } else {
               false
           }
       }
   ```

5. **Update existing test fixtures in this file** so they keep compiling with the new shape:
   - In `manifest_roundtrip` (line ~294): the literal struct `SkillEntry { source_name: DirectoryName::new("test").unwrap(), ... }` becomes `source_name: Some(DirectoryName::new("test").unwrap())`.
   - In `update_source_name_existing_skill` and `update_source_name_missing_skill`: still use `SkillEntry::new(...)` (no change needed — the constructor wraps in `Some` internally).

6. **Add tests for the new behavior** (place at end of `mod tests`, before the closing `}`):
   - `deserialize_old_shape_with_source_name_string` — covers Test 1
   - `deserialize_new_shape_with_null_source_name` — covers Test 2
   - `deserialize_new_shape_missing_source_name` — covers Test 3
   - `serialize_unowned_entry_omits_source_name_key` — covers Test 4 (assert `!json.contains("source_name")`)
   - `serialize_owned_entry_preserves_string_shape` — covers Test 5 (assert `json.contains("\"source_name\": \"foo\"")`)
   - `new_unowned_constructor_sets_source_name_none` — covers Test 6

   Use `crate::validation::test_hash("...")` for content_hash values. Use the existing `tempfile::TempDir` import where needed; the new tests are pure JSON-parsing and don't need filesystem.

   For Test 4 example:
   ```rust
   #[test]
   fn serialize_unowned_entry_omits_source_name_key() {
       let entry = SkillEntry::new_unowned(
           PathBuf::from("/tmp/orphan"),
           test_hash("orphan"),
           false,
       );
       let json = serde_json::to_string(&entry).unwrap();
       assert!(!json.contains("source_name"), "Unowned entry must omit source_name key, got: {json}");
       assert!(json.contains("\"managed\":false"));
   }
   ```

   For Test 1 example:
   ```rust
   #[test]
   fn deserialize_old_shape_with_source_name_string() {
       let valid_hash = "a".repeat(64);
       let json = format!(
           r#"{{"source_path":"/tmp/x","source_name":"foo","content_hash":"{valid_hash}","synced_at":"2024-01-01T00:00:00Z","managed":false}}"#
       );
       let entry: SkillEntry = serde_json::from_str(&json).unwrap();
       assert_eq!(entry.source_name, Some(DirectoryName::new("foo").unwrap()));
   }
   ```

7. **Honor LIB-02 documentation update** in this file too. Update the `managed` field's doc comment from:
   ```rust
       /// Whether this skill is managed by a package manager (symlinked, not copied).
       /// Defaults to `false` for backwards compatibility with pre-v0.2.1 manifests.
   ```
   to:
   ```rust
       /// Whether upstream sync feeds updates into this library entry (true =
       /// managed update channel, e.g. claude plugin install/update; false =
       /// local, library is canonical). Per LIB-02, this is now an "update
       /// channel" indicator — both managed and local skills live as real
       /// directory copies in the library after Phase 11. Defaults to `false`
       /// for backwards compatibility with pre-v0.2.1 manifests.
       #[serde(default)]
       pub managed: bool,
   ```
   (The serde attribute and field remain identical — only the doc comment changes.)
  </action>
  <verify>
    <automated>cargo test --package tome --lib manifest::tests</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "pub source_name: Option<DirectoryName>" crates/tome/src/manifest.rs` returns 1 match
    - `rg -n "skip_serializing_if = \"Option::is_none\"" crates/tome/src/manifest.rs` returns at least 1 match (on the `source_name` field)
    - `rg -n "pub fn new_unowned" crates/tome/src/manifest.rs` returns 1 match
    - `rg -n "source_name: Some\\(source_name\\)" crates/tome/src/manifest.rs` returns 1 match (in `SkillEntry::new`)
    - `rg -n "source_name: None" crates/tome/src/manifest.rs` returns at least 1 match (in `new_unowned` body and tests)
    - `rg -n "deserialize_old_shape_with_source_name_string|deserialize_new_shape_with_null_source_name|deserialize_new_shape_missing_source_name|serialize_unowned_entry_omits_source_name_key|serialize_owned_entry_preserves_string_shape|new_unowned_constructor_sets_source_name_none" crates/tome/src/manifest.rs` returns 6 matches
    - `cargo test --package tome --lib manifest::tests` exits 0
    - `cargo build --package tome` exits 0 (compilation across all call-sites still works because `SkillEntry::new` signature unchanged)
  </acceptance_criteria>
  <done>Manifest schema lifted; old-shape and new-shape JSON both round-trip correctly; new `new_unowned` constructor available; managed field doc reflects LIB-02 "update channel" semantics; all existing tests pass; new tests assert the exact behaviors above.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Lift `LockEntry.source_name` to `Option<DirectoryName>` and update lockfile generate</name>
  <files>crates/tome/src/lockfile.rs</files>
  <read_first>
    - crates/tome/src/lockfile.rs (current LockEntry struct, generate, save/load, all `mod tests`)
    - crates/tome/src/manifest.rs (after Task 1) — confirm `SkillEntry.source_name` is now `Option<DirectoryName>` so `generate()` can clone it directly
    - .planning/phases/11-library-canonical-core/11-CONTEXT.md (D-14)
  </read_first>
  <behavior>
    - Test 1 (old-shape lockfile compat): `serde_json::from_str` on a lockfile JSON with `"source_name": "foo"` produces a `LockEntry { source_name: Some(DirectoryName::new("foo")?), ... }`.
    - Test 2 (new-shape lockfile, null): `"source_name": null` deserializes to `LockEntry { source_name: None, ... }`.
    - Test 3 (new-shape lockfile, missing key): a JSON object without `"source_name"` deserializes to `LockEntry { source_name: None, ... }`.
    - Test 4 (Unowned skill omits source_name in serialized lockfile): generating a lockfile from a manifest containing an Unowned `SkillEntry` (built via `SkillEntry::new_unowned`) produces JSON without the `"source_name"` key for that skill (serde `skip_serializing_if`).
    - Test 5 (existing tests still pass): all existing tests (`generate_local_skill_no_provenance`, etc.) continue to pass; their string-shape comparisons work because `Some("foo")` still serializes to `"foo"` (transparent).
    - Test 6 (existing PartialEq test for `entry.source_name, "standalone"`): the `assert_eq!(entry.source_name, "standalone");` style assertions need updating to the `Option` shape, e.g. `assert_eq!(entry.source_name.as_ref().map(|d| d.as_str()), Some("standalone"))`.
  </behavior>
  <action>
1. **Change `LockEntry.source_name` field declaration.** Replace exactly:
   ```rust
       /// Directory name (maps to a `[directories.*]` entry in `tome.toml`).
       /// On-disk JSON shape is unchanged (`DirectoryName` is `#[serde(transparent)]`); the
       /// type lift to `DirectoryName` (closes #489) tightens validation at deserialize time.
       pub source_name: DirectoryName,
   ```
   with:
   ```rust
       /// Directory name (maps to a `[directories.*]` entry in `tome.toml`), or
       /// `None` if the skill is **Unowned** (source removed from `tome.toml`,
       /// library copy preserved per LIB-04).
       ///
       /// Mirrors `SkillEntry.source_name` (D-12/D-14): old lockfiles with
       /// `"source_name": "foo"` parse as `Some(DirectoryName::new("foo")?)`;
       /// Unowned entries omit the key on serialize.
       #[serde(default, skip_serializing_if = "Option::is_none")]
       pub source_name: Option<DirectoryName>,
   ```

2. **Update `generate()` to clone the `Option<DirectoryName>` directly.** In the body of `generate`, replace:
   ```rust
                   source_name: entry.source_name.clone(),
   ```
   with the same line — but verify the type: `entry.source_name` is now `Option<DirectoryName>` post-Task 1, so `.clone()` produces `Option<DirectoryName>` which matches the new `LockEntry.source_name` field type. No other change needed in this line.

3. **Update fixture builders in `mod tests`.** Search for every literal struct construction `LockEntry { source_name: DirectoryName::new(...).unwrap(), ... }` and wrap the value in `Some(...)`. There are ~3–4 such sites in this file (in `load_valid_file_returns_some`, in `write_lockfile`, in `make_manifest`'s caller chain). For example:
   ```rust
   LockEntry {
       source_name: Some(DirectoryName::new("test").unwrap()),
       content_hash: test_hash("abc123"),
       registry_id: None,
       version: None,
       git_commit_sha: None,
   }
   ```

4. **Update `assert_eq!(entry.source_name, "standalone");`-style assertions** in tests so they work with `Option`. Example replacement:
   - Was: `assert_eq!(entry.source_name, "standalone");`
   - Becomes: `assert_eq!(entry.source_name.as_ref().map(|d| d.as_str()), Some("standalone"));`

   Also for: `assert_eq!(a.source_name, "src");`, `assert_eq!(b.source_name, "src");`, etc. Use the same `.as_ref().map(|d| d.as_str())` pattern.

5. **Add tests for the new Option shape** (place at end of `mod tests`, before the closing `}`):
   - `deserialize_old_shape_lockfile_source_name_string` — covers Test 1
   - `deserialize_new_shape_lockfile_null_source_name` — covers Test 2
   - `deserialize_new_shape_lockfile_missing_source_name` — covers Test 3
   - `unowned_skill_omits_source_name_in_lockfile_json` — covers Test 4

   Test 4 example:
   ```rust
   #[test]
   fn unowned_skill_omits_source_name_in_lockfile_json() {
       use crate::manifest::SkillEntry;
       let mut manifest = Manifest::default();
       manifest.insert(
           SkillName::new("orphan").unwrap(),
           SkillEntry::new_unowned(PathBuf::from("/tmp/orphan"), test_hash("h"), false),
       );
       let lockfile = generate(&manifest, &[]);
       let json = serde_json::to_string_pretty(&lockfile).unwrap();
       let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
       let orphan = &parsed["skills"]["orphan"];
       assert!(
           orphan.get("source_name").is_none(),
           "Unowned skill must omit source_name in lockfile JSON, got: {json}"
       );
   }
   ```

6. **Update `resolved_paths_from_lockfile_cache`** which currently does `entry.source_name.as_str()` (line ~146 in current file). After the Option lift, this becomes:
   ```rust
   for entry in lf.skills.values() {
       if let Some(source) = &entry.source_name {
           sha_by_dir
               .entry(source.as_str())
               .or_insert_with(|| entry.git_commit_sha.clone());
       }
       // Unowned entries (source_name == None) are skipped — they have no
       // directory in the current config to resolve against.
   }
   ```

   Replace the existing block exactly as shown.
  </action>
  <verify>
    <automated>cargo test --package tome --lib lockfile::tests</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "pub source_name: Option<DirectoryName>" crates/tome/src/lockfile.rs` returns 1 match
    - `rg -n "skip_serializing_if = \"Option::is_none\"" crates/tome/src/lockfile.rs` returns at least 1 match on the source_name field
    - `rg -n "deserialize_old_shape_lockfile_source_name_string|deserialize_new_shape_lockfile_null_source_name|deserialize_new_shape_lockfile_missing_source_name|unowned_skill_omits_source_name_in_lockfile_json" crates/tome/src/lockfile.rs` returns 4 matches
    - `rg -n "if let Some\\(source\\) = &entry.source_name" crates/tome/src/lockfile.rs` returns 1 match (in `resolved_paths_from_lockfile_cache`)
    - `cargo test --package tome --lib lockfile::tests` exits 0
    - `cargo build --package tome` exits 0
  </acceptance_criteria>
  <done>Lockfile schema mirrors manifest schema. Old-shape lockfiles deserialize correctly; Unowned entries omit `source_name` on serialize; resolved_paths_from_lockfile_cache safely handles `None`; existing tests updated for the new `Option` shape and pass.</done>
</task>

</tasks>

<verification>
- `cargo test --package tome --lib manifest::tests lockfile::tests` exits 0
- `cargo build --package tome` exits 0 (every external call-site of `SkillEntry::new` and `LockEntry { source_name: ... }` still compiles because `SkillEntry::new` signature is unchanged and `Option<DirectoryName>` accepts `Some(...)` literals)
- `rg "Option<DirectoryName>" crates/tome/src/manifest.rs crates/tome/src/lockfile.rs` returns at least 2 matches (1 per file)
- `make ci` exits 0
</verification>

<success_criteria>
- LIB-03 fully addressed: manifest and lockfile both accept old (`source_name: "foo"`) and new (Unowned) shapes via serde defaults; new `new_unowned` constructor available.
- Schema is now ready for Phase 13 drift detection (D-08): `content_hash: ContentHash` remains the authoritative drift signal; `version: Option<String>` stays as display-only.
- No call-site changes required outside this plan (the `SkillEntry::new` constructor preserves its signature).
</success_criteria>

<output>
After completion, create `.planning/phases/11-library-canonical-core/11-01-SUMMARY.md`
documenting: schema changes (field type, serde attrs, new constructor), test additions,
backward-compat verification, and any follow-on items for downstream Phase 11 plans.
</output>
