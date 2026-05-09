---
phase: 14-unowned-library-lifecycle
plan: 02
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/tome/src/summary.rs
  - crates/tome/src/lib.rs
autonomous: true
requirements:
  - UNOWN-03

must_haves:
  truths:
    - "A shared `SkillSummary` type exists that both `tome status` JSON and `tome doctor` JSON serialise as their `unowned: [...]` array element."
    - "The type carries the data points D-D3 specifies: name, previous_source, source_path_display (collapse_home rendering), synced_at, managed."
    - "It can be constructed from a `(SkillName, &SkillEntry)` pair without re-reading the filesystem."
  artifacts:
    - path: "crates/tome/src/summary.rs"
      provides: "SkillSummary public struct with serde::Serialize, plus from_entry constructor."
      contains: "pub struct SkillSummary"
      min_lines: 40
    - path: "crates/tome/src/lib.rs"
      provides: "module declaration `pub(crate) mod summary;`"
  key_links:
    - from: "summary::SkillSummary"
      to: "manifest::SkillEntry"
      via: "from_entry constructor reads SkillEntry fields"
      pattern: "SkillSummary::from_entry\\("
---

<objective>
Introduce a new `summary.rs` module exporting a public `SkillSummary` struct
that captures the data both `tome status` and `tome doctor` need to render
the Unowned section (D-D1, D-D3). One concrete type, one place to evolve.

Purpose: 14-06 (status) and 14-07 (doctor) both consume this type for their
`unowned: Vec<SkillSummary>` field; landing it in Wave 1 lets them proceed
in parallel during Wave 3 without coordinating struct shape changes.

Output: `crates/tome/src/summary.rs` with `SkillSummary` + `from_entry`
constructor + unit tests covering serialisation shape and previous_source
fallback to source_path_display.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/phases/14-unowned-library-lifecycle/14-CONTEXT.md

# Source-of-truth pattern files:
@crates/tome/src/manifest.rs
@crates/tome/src/discover.rs
@crates/tome/src/paths.rs
@crates/tome/src/lib.rs

<interfaces>
<!-- The exact struct shape required is specified in CONTEXT.md D-D3. -->
<!-- Reproduced verbatim here so the executor doesn't need to navigate -->
<!-- back to CONTEXT.md to find it. -->

D-D3 SkillSummary (verbatim from 14-CONTEXT.md):
```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct SkillSummary {
    pub name: String,
    pub previous_source: Option<String>, // DirectoryName as string
    pub source_path_display: String,     // collapse_home rendering
    pub synced_at: String,
    pub managed: bool,
}
```

Existing `SkillEntry` shape (after 14-01 lands):
```rust
pub struct SkillEntry {
    pub source_path: PathBuf,
    pub source_name: Option<DirectoryName>,
    pub previous_source: Option<DirectoryName>,  // added by 14-01
    pub content_hash: ContentHash,
    pub synced_at: String,
    pub managed: bool,
}
```

Existing `paths::collapse_home` signature (paths.rs:150):
```rust
pub(crate) fn collapse_home(path: &Path) -> String;
```

Existing `discover::SkillName` shape: newtype wrapper exposing `as_str() -> &str`.

Existing module declarations in lib.rs (around lines 25-52): each module is
declared as `pub(crate) mod <name>;` in alphabetical order.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Create `crates/tome/src/summary.rs` with `SkillSummary` and `from_entry`</name>
  <read_first>
    - .planning/phases/14-unowned-library-lifecycle/14-CONTEXT.md (D-D3 — the exact struct shape; D-C2 — the source_path_display fallback rule)
    - crates/tome/src/manifest.rs (SkillEntry shape after 14-01)
    - crates/tome/src/discover.rs (SkillName shape)
    - crates/tome/src/paths.rs (collapse_home signature)
    - crates/tome/src/lib.rs (lines 25-52 — module declaration alphabetical order)
  </read_first>
  <behavior>
    - Test 1: `SkillSummary::from_entry(name, &entry_with_previous_source_some)` populates `previous_source = Some("dir-name".to_string())`.
    - Test 2: `from_entry` with `previous_source = None` (pre-Phase-14 fallback) leaves the field as `None`; `source_path_display` is always populated via `paths::collapse_home`.
    - Test 3: JSON shape — `serde_json::to_value(&summary)` produces an object with keys `name`, `previous_source`, `source_path_display`, `synced_at`, `managed`. `previous_source: None` serialises as `null` (no skip_serializing_if — JSON consumers expect the key present for stable shape).
    - Test 4: `synced_at` and `managed` flow through unchanged from the SkillEntry.
  </behavior>
  <action>
    1. **Create `crates/tome/src/summary.rs`** with the following content:

    ```rust
    //! Shared skill-summary type for `tome status` and `tome doctor` Unowned
    //! section rendering (UNOWN-03 / D-D3).
    //!
    //! Both `status::StatusReport` and `doctor::DoctorReport` carry
    //! `unowned: Vec<SkillSummary>` fields. The type is intentionally
    //! display-shaped: `previous_source` is the clean directory name when
    //! present (D-C1), `source_path_display` is the always-populated
    //! `collapse_home`-rendered fallback (D-C2). JSON output presents both
    //! so consumers can pick whichever is more informative.

    use crate::discover::SkillName;
    use crate::manifest::SkillEntry;

    /// One row of the Unowned section in `tome status` and `tome doctor`.
    /// Per D-D3 in the Phase 14 CONTEXT.md.
    #[derive(Debug, Clone, serde::Serialize)]
    pub struct SkillSummary {
        /// Skill name as displayed.
        pub name: String,
        /// Last directory that owned this skill, captured at transition time
        /// (D-C1). `None` for entries that became Unowned before Phase 14
        /// landed — consumers fall back to `source_path_display` (D-C2).
        pub previous_source: Option<String>,
        /// `paths::collapse_home`-rendered `source_path` from the manifest.
        /// Always populated; serves as the D-C2 fallback when
        /// `previous_source` is `None`, and as supplementary info otherwise.
        pub source_path_display: String,
        /// ISO 8601 timestamp from the manifest (preserved across Owned→Unowned
        /// transition per Phase 11 manifest semantics).
        pub synced_at: String,
        /// Mirrors `SkillEntry::managed`. Display-only; consumers may want
        /// to surface "originally a managed plugin" for context.
        pub managed: bool,
    }

    impl SkillSummary {
        /// Build a summary from a manifest entry and its name. No filesystem
        /// I/O — purely a projection of `SkillEntry` fields.
        pub fn from_entry(name: &SkillName, entry: &SkillEntry) -> Self {
            Self {
                name: name.as_str().to_string(),
                previous_source: entry
                    .previous_source
                    .as_ref()
                    .map(|d| d.as_str().to_string()),
                source_path_display: crate::paths::collapse_home(&entry.source_path),
                synced_at: entry.synced_at.clone(),
                managed: entry.managed,
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::config::DirectoryName;
        use crate::manifest::SkillEntry;
        use crate::validation::test_hash;
        use std::path::PathBuf;

        fn unowned_entry_with_previous(previous: Option<&str>) -> SkillEntry {
            SkillEntry::new_unowned(
                PathBuf::from("/tmp/orphan-skill"),
                test_hash("h"),
                false,
                previous.map(|s| DirectoryName::new(s).unwrap()),
            )
        }

        #[test]
        fn from_entry_populates_previous_source_when_present() {
            let entry = unowned_entry_with_previous(Some("removed-source"));
            let name = SkillName::new("orphan-skill").unwrap();
            let summary = SkillSummary::from_entry(&name, &entry);
            assert_eq!(summary.name, "orphan-skill");
            assert_eq!(summary.previous_source, Some("removed-source".to_string()));
            assert_eq!(summary.managed, false);
        }

        #[test]
        fn from_entry_falls_back_when_previous_source_missing() {
            // D-C2 fallback case: an Unowned entry that became Unowned
            // before Phase 14 landed has previous_source = None. Consumers
            // render source_path_display.
            let entry = unowned_entry_with_previous(None);
            let name = SkillName::new("legacy-orphan").unwrap();
            let summary = SkillSummary::from_entry(&name, &entry);
            assert_eq!(summary.previous_source, None);
            assert!(
                !summary.source_path_display.is_empty(),
                "source_path_display must always be populated for D-C2 fallback"
            );
        }

        #[test]
        fn json_shape_includes_all_keys() {
            let entry = unowned_entry_with_previous(Some("foo"));
            let name = SkillName::new("bar").unwrap();
            let summary = SkillSummary::from_entry(&name, &entry);
            let value = serde_json::to_value(&summary).unwrap();
            let obj = value.as_object().expect("SkillSummary serializes to JSON object");
            for key in ["name", "previous_source", "source_path_display", "synced_at", "managed"] {
                assert!(
                    obj.contains_key(key),
                    "SkillSummary JSON must contain key '{key}', got: {value}"
                );
            }
            assert_eq!(obj["name"], "bar");
            assert_eq!(obj["previous_source"], "foo");
            assert_eq!(obj["managed"], false);
        }

        #[test]
        fn json_previous_source_serializes_as_null_when_none() {
            // Stable JSON shape: consumers should always see the key with
            // an explicit null when previous_source is absent. NO skip_serializing_if.
            let entry = unowned_entry_with_previous(None);
            let name = SkillName::new("legacy").unwrap();
            let summary = SkillSummary::from_entry(&name, &entry);
            let value = serde_json::to_value(&summary).unwrap();
            assert!(
                value["previous_source"].is_null(),
                "previous_source must serialize as null (not omitted) when None: {value}"
            );
        }
    }
    ```

    2. **Add `pub(crate) mod summary;` to `crates/tome/src/lib.rs`.** Insert it in alphabetical order in the module-declaration block (lines 25-52). It goes between `mod status;` and `mod update;`. Concretely:

    ```rust
    pub(crate) mod status;
    pub(crate) mod summary;  // <-- NEW
    pub(crate) mod update;
    ```

    Verify the alphabetical position with the existing entries before/after.

    3. Run `cargo build -p tome` to catch compile errors. The new module is dead code at this point (no consumers) — that is INTENTIONAL. Do NOT add `#[allow(dead_code)]` because plan 14-06 (status) and 14-07 (doctor) consume `SkillSummary` later in this same wave-set. If clippy fails on dead_code, add a single `#[allow(dead_code)]` on the `pub fn from_entry` only (NOT on the struct itself, which is `pub` and exposed in test code via the `mod tests` block).

    Actually — the tests in this file exercise both `SkillSummary` construction and `from_entry`, so `cargo build --lib` won't complain. `cargo build -p tome` (binary) might. If clippy under `-D warnings` flags `from_entry` as dead-code at the binary level, add `#[allow(dead_code)]` to the `impl SkillSummary` block ONLY for `from_entry`, with a doc comment: `// dead_code allow: consumed in 14-06 (status) and 14-07 (doctor) within this wave-set; remove this attr when those plans land.`
  </action>
  <verify>
    <automated>cargo test -p tome --lib summary::tests</automated>
  </verify>
  <acceptance_criteria>
    - File exists: `test -f crates/tome/src/summary.rs && grep -q "pub struct SkillSummary" crates/tome/src/summary.rs` succeeds
    - `grep -q "pub previous_source: Option<String>" crates/tome/src/summary.rs` succeeds
    - `grep -q "pub source_path_display: String" crates/tome/src/summary.rs` succeeds
    - `grep -q "pub synced_at: String" crates/tome/src/summary.rs` succeeds
    - `grep -q "pub managed: bool" crates/tome/src/summary.rs` succeeds
    - `grep -q "pub fn from_entry" crates/tome/src/summary.rs` succeeds
    - `grep -q "pub(crate) mod summary;" crates/tome/src/lib.rs` succeeds
    - `cargo test -p tome --lib summary::tests::from_entry_populates_previous_source_when_present` exits 0
    - `cargo test -p tome --lib summary::tests::from_entry_falls_back_when_previous_source_missing` exits 0
    - `cargo test -p tome --lib summary::tests::json_shape_includes_all_keys` exits 0
    - `cargo test -p tome --lib summary::tests::json_previous_source_serializes_as_null_when_none` exits 0
    - `cargo clippy --all-targets -p tome -- -D warnings` exits 0
  </acceptance_criteria>
  <done>
    `crates/tome/src/summary.rs` exists with `SkillSummary` struct + `from_entry` + 4 unit tests. Module declared in `lib.rs`. Tests pass. Clippy clean.
  </done>
</task>

</tasks>

<verification>
- `cargo test -p tome --lib summary::tests` exits 0
- `cargo clippy --all-targets -p tome -- -D warnings` exits 0
- `cargo fmt -p tome -- --check` exits 0
</verification>

<success_criteria>
- `SkillSummary` defined exactly per D-D3 (5 fields: name, previous_source, source_path_display, synced_at, managed).
- `from_entry` constructor builds it from a `(SkillName, &SkillEntry)` pair using `paths::collapse_home`.
- 4 unit tests pass: previous_source happy path, D-C2 fallback, JSON shape, JSON null-on-None.
- Module wired into `lib.rs::pub(crate) mod summary;`.
</success_criteria>

<output>
After completion, create `.planning/phases/14-unowned-library-lifecycle/14-02-SUMMARY.md`
</output>
