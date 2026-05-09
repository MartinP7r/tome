---
phase: 14-unowned-library-lifecycle
plan: 06
type: execute
wave: 3
depends_on:
  - 14-01
  - 14-02
files_modified:
  - crates/tome/src/status.rs
autonomous: true
requirements:
  - UNOWN-03

must_haves:
  truths:
    - "`tome status` text output includes an `Unowned skills (N):` section between Directories and Health when N > 0; the section omits cleanly (no header, no blank line) when N == 0."
    - "`tome status --json` output includes an `unowned` array of `SkillSummary` items."
    - "Each row shows NAME, LAST-KNOWN SOURCE (previous_source if set, source_path collapsed if not), SYNCED."
  artifacts:
    - path: "crates/tome/src/status.rs"
      provides: "StatusReport.unowned: Vec<SkillSummary>; gather populates it; render_status renders the table; json output includes the field."
      contains: "unowned: Vec<SkillSummary>"
  key_links:
    - from: "status::gather"
      to: "summary::SkillSummary::from_entry"
      via: "iterate manifest, filter source_name.is_none(), build SkillSummary per entry"
      pattern: "SkillSummary::from_entry"
    - from: "status::render_status"
      to: "report.unowned"
      via: "tabled::Table with NAME / LAST-KNOWN SOURCE / SYNCED columns"
---

<objective>
Add an `unowned: Vec<SkillSummary>` field to `StatusReport`. Populate it in
`gather()` by reading the manifest at `paths.config_dir()` and filtering for
entries with `source_name.is_none()`. Render it in `render_status` between
the Directories table and the Health line per D-D2, using the same
`tabled::Table::from_iter` + `Style::blank()` + bold-header pattern. JSON
output gets the field for free via `#[derive(Serialize)]`.

Empty-set behaviour: section omits cleanly (no `Unowned skills (0):` header,
no blank line). JSON includes `"unowned": []` for stable shape.

Purpose: half of UNOWN-03 (status); 14-07 lands the doctor side. Both
consume the `SkillSummary` type from 14-02 and the `previous_source` data
from 14-01.

Output: `tome status` shows the Unowned section per D-D1/D-D2; JSON
consumers (other tools, future GUI) see `unowned: [...]`.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/phases/14-unowned-library-lifecycle/14-CONTEXT.md
@.planning/phases/14-unowned-library-lifecycle/14-01-previous-source-schema-PLAN.md
@.planning/phases/14-unowned-library-lifecycle/14-02-skill-summary-type-PLAN.md

# Source-of-truth pattern files:
@crates/tome/src/status.rs
@crates/tome/src/manifest.rs
@crates/tome/src/summary.rs
@crates/tome/src/paths.rs

<interfaces>
<!-- Existing StatusReport struct (status.rs:57-66): -->
```rust
pub struct StatusReport {
    pub configured: bool,
    pub library_dir: PathBuf,
    pub library_count: CountOrError,
    pub directories: Vec<DirectoryStatus>,
    pub health: CountOrError,
}
```

<!-- After 14-02, SkillSummary lives at crate::summary::SkillSummary with -->
<!-- from_entry(name: &SkillName, entry: &SkillEntry) -> Self. -->

<!-- The existing tabled rendering pattern in render_status (status.rs:204-211): -->
```rust
let table = tabled::Table::from_iter(rows)
    .with(Style::blank())
    .with(
        Modify::new(Rows::first()).with(tabled::settings::Format::content(|s| {
            style(s).bold().to_string()
        })),
    )
    .to_string();
```

<!-- D-D2 placement (verbatim): -->
1. Library: <path> + count
2. Directories: <table>
3. **Unowned skills (N): <table>**  (this section)
4. Health: <summary>

<!-- D-D1 columns: NAME | LAST-KNOWN SOURCE | SYNCED -->
<!-- LAST-KNOWN SOURCE = previous_source if set, else source_path collapsed via paths::collapse_home (D-C2 fallback). -->
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Add `unowned` field to `StatusReport`, populate in `gather`, render in `render_status`</name>
  <read_first>
    - crates/tome/src/status.rs (entire file — particularly StatusReport struct at 57-66, gather() at 71-118, render_status() at 146-236, and the existing test module from line 330)
    - crates/tome/src/summary.rs (SkillSummary + from_entry from 14-02)
    - crates/tome/src/manifest.rs (Manifest::iter, SkillEntry shape after 14-01)
    - .planning/phases/14-unowned-library-lifecycle/14-CONTEXT.md (D-D1 columns, D-D2 placement, D-D3 JSON shape)
  </read_first>
  <behavior>
    - Test 1: `gather` on a manifest with one Owned and one Unowned skill returns `unowned.len() == 1` containing the Unowned entry.
    - Test 2: `gather` on a manifest with no Unowned entries returns `unowned.is_empty() == true`.
    - Test 3: JSON serialisation of `StatusReport` includes the `unowned` key (always — even when empty array).
    - Test 4: An `Unowned` skill with `previous_source = Some("removed-dir")` round-trips into `SkillSummary { previous_source: Some("removed-dir".into()), ... }`.
    - Test 5: Empty unowned set produces no "Unowned skills" header in render output (capture stdout — see existing render tests for the pattern; if stdout capture is not used in this file, instead verify by extracting the rendering helper into a pure formatter that returns String, then assert `!output.contains("Unowned skills")`).
  </behavior>
  <action>
    1. **Add the field to `StatusReport`** (status.rs:57-66):

    ```rust
    #[derive(serde::Serialize)]
    pub struct StatusReport {
        pub configured: bool,
        pub library_dir: PathBuf,
        pub library_count: CountOrError,
        pub directories: Vec<DirectoryStatus>,
        /// Skills in the library whose source was removed from `tome.toml`
        /// (Unowned per LIB-04). Surfaces in text rendering between the
        /// Directories table and the Health line (D-D2). Always present in
        /// JSON output for stable shape; empty array when no Unowned skills.
        pub unowned: Vec<crate::summary::SkillSummary>,
        pub health: CountOrError,
    }
    ```

    2. **Update `gather()` to populate `unowned`** (status.rs:71-118). After the existing `library_count` calculation and before the `directories` Vec construction, OR right before the final `Ok(StatusReport { ... })`, add the unowned population.

    Inside `gather()`, after the existing `let library_count = ...;` block:

    ```rust
    // Populate the Unowned set per UNOWN-03. Read the manifest from
    // paths.config_dir() and project entries with source_name.is_none()
    // through SkillSummary::from_entry. Sorted ascending by name (D-D1
    // discretion choice — matches the BTreeMap natural order of Manifest).
    let unowned: Vec<crate::summary::SkillSummary> = match crate::manifest::load(paths.config_dir()) {
        Ok(manifest) => manifest
            .iter()
            .filter(|(_, entry)| entry.source_name.is_none())
            .map(|(name, entry)| crate::summary::SkillSummary::from_entry(name, entry))
            .collect(),
        Err(_) => {
            // Manifest read errors are surfaced via library_count.error;
            // the Unowned section degrades gracefully to empty.
            Vec::new()
        }
    };
    ```

    3. **Update the `Ok(StatusReport { ... })` literal** at the bottom of `gather()` to include `unowned`:

    ```rust
    Ok(StatusReport {
        configured,
        library_dir: paths.library_dir().to_path_buf(),
        library_count: library_count.into(),
        directories,
        unowned,
        health: health.into(),
    })
    ```

    4. **Update `render_status` to print the Unowned section** between the Directories block and the Health line (D-D2). The existing layout (status.rs:174-235):

    ```
    println!();  // after Directories
    [...] // Health line
    ```

    Insert the Unowned section between Directories and Health. Locate the line `println!();` after the Directories rendering loop (status.rs:219). The next block is the Health line (status.rs:221-235). Insert the Unowned rendering BEFORE the Health line.

    Add this block — concrete code:

    ```rust
    // Unowned skills (UNOWN-03 / D-D1, D-D2). Section omits cleanly when empty.
    if !report.unowned.is_empty() {
        println!(
            "{} ({}):",
            style("Unowned skills").bold(),
            report.unowned.len()
        );
        let mut rows: Vec<[String; 3]> = Vec::with_capacity(report.unowned.len() + 1);
        rows.push([
            "NAME".to_string(),
            "LAST-KNOWN SOURCE".to_string(),
            "SYNCED".to_string(),
        ]);
        for s in &report.unowned {
            // D-C1 / D-C2 fallback: render previous_source when present;
            // fall back to source_path_display (already collapse_home-rendered
            // by SkillSummary::from_entry).
            let last_known = s
                .previous_source
                .clone()
                .unwrap_or_else(|| s.source_path_display.clone());
            rows.push([s.name.clone(), last_known, s.synced_at.clone()]);
        }
        let table = tabled::Table::from_iter(rows)
            .with(Style::blank())
            .with(
                Modify::new(Rows::first()).with(tabled::settings::Format::content(|s| {
                    style(s).bold().to_string()
                })),
            )
            .to_string();
        println!("{table}");
        println!();
    }
    ```

    Make sure `Style`, `Modify`, `Rows` imports are already at the top of status.rs (they are — see line 6).

    5. **Add unit tests** in `#[cfg(test)] mod tests`:

    ```rust
    #[test]
    fn gather_populates_unowned_for_entries_with_no_source_name() {
        let tome_home = tempfile::TempDir::new().unwrap();
        let library = tome_home.path().join("library");
        std::fs::create_dir_all(&library).unwrap();
        std::fs::create_dir_all(library.join("orphan")).unwrap();
        std::fs::create_dir_all(library.join("kept")).unwrap();

        // Build manifest with one Owned + one Unowned (with previous_source).
        let mut manifest = manifest::Manifest::default();
        manifest.insert(
            crate::discover::SkillName::new("kept").unwrap(),
            manifest::SkillEntry::new(
                std::path::PathBuf::from("/tmp/src/kept"),
                DirectoryName::new("active").unwrap(),
                crate::validation::test_hash("h"),
                false,
            ),
        );
        manifest.insert(
            crate::discover::SkillName::new("orphan").unwrap(),
            manifest::SkillEntry::new_unowned(
                std::path::PathBuf::from("/tmp/old/orphan"),
                crate::validation::test_hash("o"),
                false,
                Some(DirectoryName::new("removed-dir").unwrap()),
            ),
        );
        manifest::save(&manifest, tome_home.path()).unwrap();

        let config = Config {
            library_dir: library.clone(),
            ..Config::default()
        };
        let paths = TomePaths::new(tome_home.path().to_path_buf(), library).unwrap();

        let report = gather(&config, &paths).unwrap();
        assert_eq!(report.unowned.len(), 1);
        assert_eq!(report.unowned[0].name, "orphan");
        assert_eq!(report.unowned[0].previous_source, Some("removed-dir".to_string()));
    }

    #[test]
    fn gather_returns_empty_unowned_when_all_entries_are_owned() {
        let tome_home = tempfile::TempDir::new().unwrap();
        let library = tome_home.path().join("library");
        std::fs::create_dir_all(&library).unwrap();
        std::fs::create_dir_all(library.join("kept")).unwrap();

        let mut manifest = manifest::Manifest::default();
        manifest.insert(
            crate::discover::SkillName::new("kept").unwrap(),
            manifest::SkillEntry::new(
                std::path::PathBuf::from("/tmp/src/kept"),
                DirectoryName::new("active").unwrap(),
                crate::validation::test_hash("h"),
                false,
            ),
        );
        manifest::save(&manifest, tome_home.path()).unwrap();

        let config = Config {
            library_dir: library.clone(),
            ..Config::default()
        };
        let paths = TomePaths::new(tome_home.path().to_path_buf(), library).unwrap();

        let report = gather(&config, &paths).unwrap();
        assert!(report.unowned.is_empty(), "expected no Unowned entries, got {:?}", report.unowned);
    }

    #[test]
    fn json_status_always_includes_unowned_field() {
        let report = StatusReport {
            configured: false,
            library_dir: PathBuf::from("/tmp/lib"),
            library_count: CountOrError { count: Some(0), error: None },
            directories: Vec::new(),
            unowned: Vec::new(),
            health: CountOrError { count: Some(0), error: None },
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(
            json.contains("\"unowned\""),
            "JSON must include 'unowned' key for stable shape: {json}"
        );
    }
    ```

    6. **Update existing tests** that construct `StatusReport { ... }` literals — the only one is the `status_json_includes_override_applied_field` test (status.rs:771-790) which builds a `DirectoryStatus`, not StatusReport, so likely no change required. Run all status tests: `cargo test -p tome --lib status::tests`. Add `unowned: Vec::new(),` to any literal that fails to compile.

    7. **Verify the rendering manually** by building the binary and running `tome status` against a test manifest with Unowned entries. (Or rely on the existing CLI integration tests in 14-08 to anchor end-to-end.)
  </action>
  <verify>
    <automated>cargo test -p tome --lib status::tests</automated>
  </verify>
  <acceptance_criteria>
    - `grep -q "pub unowned: Vec<crate::summary::SkillSummary>" crates/tome/src/status.rs` succeeds
    - `grep -q "SkillSummary::from_entry" crates/tome/src/status.rs` succeeds
    - `grep -q "Unowned skills" crates/tome/src/status.rs` succeeds (the heading literal)
    - `grep -q "LAST-KNOWN SOURCE" crates/tome/src/status.rs` succeeds (D-D1 column header)
    - `cargo test -p tome --lib status::tests::gather_populates_unowned_for_entries_with_no_source_name` exits 0
    - `cargo test -p tome --lib status::tests::gather_returns_empty_unowned_when_all_entries_are_owned` exits 0
    - `cargo test -p tome --lib status::tests::json_status_always_includes_unowned_field` exits 0
    - All pre-existing status tests still pass: `cargo test -p tome --lib status::tests` exits 0
    - `cargo clippy --all-targets -p tome -- -D warnings` exits 0
  </acceptance_criteria>
  <done>
    `StatusReport.unowned` populated from manifest. Text rendering shows the Unowned section between Directories and Health when non-empty; omits cleanly when empty. JSON always includes the field. Tests pass.
  </done>
</task>

</tasks>

<verification>
- `cargo test -p tome` exits 0
- `cargo clippy --all-targets -p tome -- -D warnings` exits 0
- Manual smoke test (or 14-08 integration): manifest with Unowned entries shows the section in `tome status`; manifest without them omits cleanly
</verification>

<success_criteria>
- UNOWN-03 status side delivered: text shows `Unowned skills (N):` table; JSON includes `unowned: [SkillSummary]`.
- Empty-set behaviour correct: text omits header; JSON has empty array.
- Last-known source falls back to `source_path_display` when `previous_source` is None (D-C2 fallback).
- 3 new unit tests + all existing status tests pass.
</success_criteria>

<output>
After completion, create `.planning/phases/14-unowned-library-lifecycle/14-06-SUMMARY.md`
</output>
