---
phase: 14-unowned-library-lifecycle
plan: 07
type: execute
wave: 3
depends_on:
  - 14-01
  - 14-02
files_modified:
  - crates/tome/src/doctor.rs
autonomous: true
requirements:
  - UNOWN-03

must_haves:
  truths:
    - "`tome doctor` text output includes a separate `Unowned skills (N):` section for the unowned set when N > 0; omits cleanly when N == 0."
    - "`tome doctor --json` includes an `unowned_skills: [SkillSummary]` array field on `DoctorReport`."
    - "Unowned skills do NOT contribute to `DoctorReport::total_issues` (D-D3 informational severity); `tome doctor` exit code is unaffected by the unowned set."
    - "Unowned section is parallel to library/directory/config issue sections — its own field, its own renderer, no IssueSeverity."
  artifacts:
    - path: "crates/tome/src/doctor.rs"
      provides: "DoctorReport.unowned_skills field; check() populates it; render in diagnose() between existing issue sections; total_issues() unchanged."
      contains: "unowned_skills: Vec<SkillSummary>"
  key_links:
    - from: "doctor::check"
      to: "summary::SkillSummary::from_entry"
      via: "iterate manifest, filter source_name.is_none()"
    - from: "doctor::DoctorReport::total_issues"
      to: "report fields"
      via: "MUST sum library_issues + directory_issues + config_issues; MUST NOT include unowned_skills (D-D3)"
---

<objective>
Add an `unowned_skills: Vec<SkillSummary>` field to `DoctorReport`, populate
in `check()`, render as a parallel section in `diagnose()`. Per D-D3, the
section is INFORMATIONAL: unowned skills do NOT count toward
`total_issues`, and `tome doctor` exit code is unaffected.

Mirror the status renderer (D-D1: NAME / LAST-KNOWN SOURCE / SYNCED columns),
but use a separate render helper since DoctorReport's section is parallel
(no severity, no per-issue grouping).

Purpose: completes UNOWN-03 — both `tome status` (14-06) and `tome doctor`
(this plan) surface the unowned set with the same data shape (`SkillSummary`).
The parallel-field-not-issue treatment per D-D3 prevents conflating
intentional state (user removed a directory) with actionable malfunctions.

Output: `tome doctor` shows the Unowned section without affecting issue
counts or exit code; JSON consumers see `unowned_skills: [SkillSummary]`.
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
@.planning/phases/14-unowned-library-lifecycle/14-06-status-unowned-section-PLAN.md

# Source-of-truth pattern files:
@crates/tome/src/doctor.rs
@crates/tome/src/manifest.rs
@crates/tome/src/summary.rs
@crates/tome/src/status.rs

<interfaces>
<!-- Existing DoctorReport (doctor.rs:48-66): -->
```rust
pub struct DoctorReport {
    pub configured: bool,
    pub library_issues: Vec<DiagnosticIssue>,
    pub directory_issues: Vec<DirectoryDiagnostic>,
    pub config_issues: Vec<DiagnosticIssue>,
}

impl DoctorReport {
    pub fn total_issues(&self) -> usize {
        self.library_issues.len()
            + self.directory_issues.iter().map(|d| d.issues.len()).sum::<usize>()
            + self.config_issues.len()
    }
}
```

<!-- Existing check() shape (doctor.rs:71-103) returns DoctorReport. -->

<!-- Existing diagnose() rendering pattern (doctor.rs:106-318): -->
```rust
println!("{}", style("Checking library...").bold());
render_issues(&report.library_issues, "library");

println!("{}", style("Checking directories...").bold());
for d in &report.directory_issues {
    render_issues_for_directory(&d.name, &d.issues, d.override_applied);
}

println!("{}", style("Checking config...").bold());
render_issues(&report.config_issues, "config");

let total = report.total_issues();
```

<!-- D-D3 verbatim: -->
- Unowned is intentional state. Conflating with broken-symlinks/missing-from-disk
  would be noisy. New parallel field on DoctorReport: unowned_skills: Vec<SkillSummary>.
- Does NOT contribute to total_issues. exit code unaffected.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Add `unowned_skills` field, populate in `check`, render parallel section, leave `total_issues` unchanged</name>
  <read_first>
    - crates/tome/src/doctor.rs (entire file — particularly DoctorReport at 48-54, total_issues at 56-65, check at 71-103, diagnose at 108-onwards including the rendering block 134-158)
    - crates/tome/src/summary.rs (SkillSummary)
    - crates/tome/src/manifest.rs (Manifest::iter)
    - .planning/phases/14-unowned-library-lifecycle/14-CONTEXT.md (D-D3 — informational severity contract; D-D1 columns; D-D2 placement isn't enforced for doctor — discretion)
    - crates/tome/src/status.rs (the analogous Unowned-section renderer added in 14-06 — match column layout for consistency)
  </read_first>
  <behavior>
    - Test 1: `check()` populates `unowned_skills` from manifest entries with `source_name.is_none()`.
    - Test 2: `total_issues()` does NOT count unowned skills (10 unowned + 0 issues → total_issues == 0).
    - Test 3: JSON serialisation of `DoctorReport` always includes `unowned_skills` key.
    - Test 4: `unowned_skills` is preserved through serde round-trip.
  </behavior>
  <action>
    1. **Add the field to `DoctorReport`** (doctor.rs:48-54):

    ```rust
    #[derive(Debug, serde::Serialize)]
    pub struct DoctorReport {
        pub configured: bool,
        pub library_issues: Vec<DiagnosticIssue>,
        pub directory_issues: Vec<DirectoryDiagnostic>,
        pub config_issues: Vec<DiagnosticIssue>,
        /// Unowned skills (UNOWN-03 / D-D3). INFORMATIONAL section — these
        /// entries do NOT contribute to `total_issues` and do NOT affect
        /// `tome doctor` exit code. They surface in text rendering as a
        /// parallel "Unowned skills" section after the issue checks.
        pub unowned_skills: Vec<crate::summary::SkillSummary>,
    }
    ```

    2. **`total_issues` MUST remain unchanged.** D-D3 contract. Leave the existing implementation as-is. Add a doc comment to make the intent explicit:

    ```rust
    impl DoctorReport {
        /// Sum of actionable diagnostic issues. Per D-D3, `unowned_skills`
        /// is INTENTIONALLY excluded — Unowned is an informational state
        /// (the user removed a directory), not a malfunction.
        pub fn total_issues(&self) -> usize {
            self.library_issues.len()
                + self.directory_issues.iter().map(|d| d.issues.len()).sum::<usize>()
                + self.config_issues.len()
        }
    }
    ```

    3. **Update `check()`** to populate `unowned_skills`. The current shape (doctor.rs:71-103) loads the manifest implicitly through `check_library`. Add a separate manifest read for the Unowned set, or thread it through. Simplest: add the read at the end of `check()`:

    ```rust
    pub fn check(config: &Config, paths: &TomePaths) -> Result<DoctorReport> {
        let configured = paths.library_dir().is_dir() || !config.directories.is_empty();

        if !configured {
            return Ok(DoctorReport {
                configured: false,
                library_issues: Vec::new(),
                directory_issues: Vec::new(),
                config_issues: Vec::new(),
                unowned_skills: Vec::new(),
            });
        }

        let library_issues = check_library(paths)?;

        let mut directory_issues = Vec::new();
        for (name, dir_config) in config.distribution_dirs() {
            let issues = check_distribution_dir(name.as_str(), &dir_config.path, paths.library_dir())?;
            directory_issues.push(DirectoryDiagnostic {
                name: name.as_str().to_string(),
                issues,
                override_applied: dir_config.override_applied,
            });
        }

        let config_issues = check_config(config)?;

        // UNOWN-03 / D-D3: collect Unowned skills from the manifest.
        // Manifest read errors degrade gracefully to an empty Vec — the
        // separate library_issues section reports the underlying read
        // failure if there is one.
        let unowned_skills = match crate::manifest::load(paths.config_dir()) {
            Ok(m) => m
                .iter()
                .filter(|(_, e)| e.source_name.is_none())
                .map(|(n, e)| crate::summary::SkillSummary::from_entry(n, e))
                .collect(),
            Err(_) => Vec::new(),
        };

        Ok(DoctorReport {
            configured: true,
            library_issues,
            directory_issues,
            config_issues,
            unowned_skills,
        })
    }
    ```

    4. **Update `diagnose()` rendering** to print the Unowned section. Locate the block in diagnose() that prints `Checking library...` / `Checking directories...` / `Checking config...` (doctor.rs:135-145). Insert the Unowned section AFTER `Checking config...` and BEFORE the `let total = report.total_issues();` line.

    Add this block:

    ```rust
    // UNOWN-03 / D-D3: parallel informational section. Does NOT affect
    // total_issues or exit code. Section omits cleanly when empty.
    if !report.unowned_skills.is_empty() {
        use tabled::settings::{Modify, Style, object::Rows};
        println!();
        println!(
            "{} ({}):",
            style("Unowned skills").bold(),
            report.unowned_skills.len()
        );
        let mut rows: Vec<[String; 3]> = Vec::with_capacity(report.unowned_skills.len() + 1);
        rows.push([
            "NAME".to_string(),
            "LAST-KNOWN SOURCE".to_string(),
            "SYNCED".to_string(),
        ]);
        for s in &report.unowned_skills {
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
    }
    ```

    5. **Add unit tests** in `#[cfg(test)] mod tests`:

    ```rust
    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::config::{Config, DirectoryName};
        use std::path::PathBuf;
        use tempfile::TempDir;

        fn write_manifest_with(entries: Vec<(&str, Option<&str>)>) -> TempDir {
            let tome_home = TempDir::new().unwrap();
            let library = tome_home.path().join("library");
            std::fs::create_dir_all(&library).unwrap();
            let mut manifest = crate::manifest::Manifest::default();
            for (name, source_opt) in entries {
                std::fs::create_dir_all(library.join(name)).unwrap();
                let entry = match source_opt {
                    Some(src) => crate::manifest::SkillEntry::new(
                        std::path::PathBuf::from(format!("/tmp/src/{name}")),
                        DirectoryName::new(src).unwrap(),
                        crate::validation::test_hash(name),
                        false,
                    ),
                    None => crate::manifest::SkillEntry::new_unowned(
                        std::path::PathBuf::from(format!("/tmp/old/{name}")),
                        crate::validation::test_hash(name),
                        false,
                        Some(DirectoryName::new("removed").unwrap()),
                    ),
                };
                manifest.insert(crate::discover::SkillName::new(name).unwrap(), entry);
            }
            crate::manifest::save(&manifest, tome_home.path()).unwrap();
            tome_home
        }

        #[test]
        fn check_populates_unowned_skills() {
            let tome_home = write_manifest_with(vec![
                ("kept", Some("active")),
                ("orphan", None),
            ]);
            let library = tome_home.path().join("library");
            let config = Config {
                library_dir: library.clone(),
                ..Config::default()
            };
            let paths = TomePaths::new(tome_home.path().to_path_buf(), library).unwrap();

            let report = check(&config, &paths).unwrap();
            assert_eq!(report.unowned_skills.len(), 1);
            assert_eq!(report.unowned_skills[0].name, "orphan");
        }

        #[test]
        fn unowned_skills_do_not_contribute_to_total_issues() {
            let tome_home = write_manifest_with(vec![("orphan-1", None), ("orphan-2", None)]);
            let library = tome_home.path().join("library");
            let config = Config {
                library_dir: library.clone(),
                ..Config::default()
            };
            let paths = TomePaths::new(tome_home.path().to_path_buf(), library).unwrap();

            let report = check(&config, &paths).unwrap();
            assert_eq!(report.unowned_skills.len(), 2, "fixture sanity");
            assert_eq!(
                report.total_issues(),
                0,
                "unowned skills must NOT contribute to total_issues per D-D3"
            );
        }

        #[test]
        fn check_empty_unowned_skills_when_all_owned() {
            let tome_home = write_manifest_with(vec![("kept", Some("active"))]);
            let library = tome_home.path().join("library");
            let config = Config {
                library_dir: library.clone(),
                ..Config::default()
            };
            let paths = TomePaths::new(tome_home.path().to_path_buf(), library).unwrap();

            let report = check(&config, &paths).unwrap();
            assert!(report.unowned_skills.is_empty());
        }

        #[test]
        fn json_doctor_always_includes_unowned_skills_field() {
            let report = DoctorReport {
                configured: false,
                library_issues: Vec::new(),
                directory_issues: Vec::new(),
                config_issues: Vec::new(),
                unowned_skills: Vec::new(),
            };
            let json = serde_json::to_string(&report).unwrap();
            assert!(
                json.contains("\"unowned_skills\""),
                "JSON must include 'unowned_skills' key for stable shape: {json}"
            );
        }
    }
    ```

    Note: `crate::validation::test_hash(name)` — verify this helper accepts a name argument (it does — see lockfile tests using `test_hash("hash_a")`). If the existing test_hash signature differs, adapt to whichever the existing doctor tests use.

    6. **Verify the existing doctor tests still pass** by running `cargo test -p tome --lib doctor::tests`. If existing tests fail to compile because they construct `DoctorReport` literals without the new field, add `unowned_skills: Vec::new(),` to each.
  </action>
  <verify>
    <automated>cargo test -p tome --lib doctor::tests</automated>
  </verify>
  <acceptance_criteria>
    - `grep -q "pub unowned_skills: Vec<crate::summary::SkillSummary>" crates/tome/src/doctor.rs` succeeds
    - `grep -q "Unowned skills" crates/tome/src/doctor.rs` succeeds (the heading)
    - `grep -q "LAST-KNOWN SOURCE" crates/tome/src/doctor.rs` succeeds (D-D1 column header)
    - `total_issues()` body unchanged: `grep -A4 "pub fn total_issues" crates/tome/src/doctor.rs | grep -v "unowned"` — verify the function body does NOT reference `unowned_skills`. Concrete check: `! grep -A6 "pub fn total_issues" crates/tome/src/doctor.rs | grep -q "unowned"`
    - `cargo test -p tome --lib doctor::tests::check_populates_unowned_skills` exits 0
    - `cargo test -p tome --lib doctor::tests::unowned_skills_do_not_contribute_to_total_issues` exits 0
    - `cargo test -p tome --lib doctor::tests::check_empty_unowned_skills_when_all_owned` exits 0
    - `cargo test -p tome --lib doctor::tests::json_doctor_always_includes_unowned_skills_field` exits 0
    - All pre-existing doctor tests still pass: `cargo test -p tome --lib doctor::tests` exits 0
    - `cargo clippy --all-targets -p tome -- -D warnings` exits 0
  </acceptance_criteria>
  <done>
    `DoctorReport.unowned_skills` populated. `total_issues()` unchanged (D-D3 contract). Text rendering shows the Unowned section parallel to issue sections. JSON always includes the field. Tests pass.
  </done>
</task>

</tasks>

<verification>
- `cargo test -p tome` exits 0
- `cargo clippy --all-targets -p tome -- -D warnings` exits 0
- Manual smoke test: a manifest with Unowned entries shows the section in `tome doctor`; `tome doctor` exit code remains 0 if no library/directory/config issues; `tome doctor --json` includes `unowned_skills`.
</verification>

<success_criteria>
- UNOWN-03 doctor side delivered: text shows `Unowned skills (N):` table; JSON includes `unowned_skills: [SkillSummary]`.
- D-D3 informational severity: `total_issues()` unchanged; exit code unaffected by Unowned set.
- Empty-set behaviour correct: text omits header; JSON has empty array.
- 4 new unit tests + all existing doctor tests pass.
</success_criteria>

<output>
After completion, create `.planning/phases/14-unowned-library-lifecycle/14-07-SUMMARY.md`
</output>
