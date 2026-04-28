---
phase: 09-cross-machine-path-overrides
plan: 03
type: execute
wave: 2
depends_on: [09-01]
files_modified:
  - crates/tome/src/status.rs
  - crates/tome/src/doctor.rs
  - crates/tome/tests/cli.rs
autonomous: true
requirements: [PORT-05]
issue: "https://github.com/MartinP7r/tome/issues/458"

must_haves:
  truths:
    - "`tome status` (text mode) marks each directory whose `path` was rewritten by a machine.toml override with an `(override)` annotation in the directory table"
    - "`tome status --json` includes a per-directory boolean `override_applied: true|false` field so machine-readable consumers (future tome-desktop GUI, scripts) can render the same information"
    - "`tome doctor` (text mode) marks override-affected directories in the per-directory section so the user sees the same context when running `tome doctor`"
    - "`tome doctor --json` includes the same per-directory `override_applied` boolean for consistency with status"
    - "When NO overrides are applied, behavior is byte-identical to v0.8.1 — the `(override)` marker / `override_applied: true` field never appears spuriously"
  artifacts:
    - path: "crates/tome/src/status.rs"
      provides: "`DirectoryStatus.override_applied: bool` field (serialized via serde). `gather()` populates it from `dir_config.override_applied`. `render_status` text path appends ` (override)` to the `PATH` column for entries where `override_applied == true`."
      contains: "override_applied"
    - path: "crates/tome/src/doctor.rs"
      provides: "`DiagnosticIssue` is unchanged (per-issue, not per-directory). The `DoctorReport.directory_issues: Vec<(String, Vec<DiagnosticIssue>)>` is replaced or augmented with `Vec<DirectoryDiagnostic>` carrying `name`, `override_applied`, and `issues`. `render_issues_for_directory` includes the override marker when present. `--json` output includes `override_applied` per directory."
      contains: "override_applied"
    - path: "crates/tome/tests/cli.rs"
      provides: "End-to-end integration test covering the full I2 invariant: tome.toml + machine.toml override → `tome sync` succeeds → `tome status --json` reports `override_applied: true` AND text mode shows `(override)` AND `tome doctor --json` reports `override_applied: true`."
      contains: "machine_override_appears_in_status_and_doctor"
  key_links:
    - from: "crates/tome/src/status.rs DirectoryStatus"
      to: "crates/tome/src/config.rs DirectoryConfig.override_applied"
      via: "gather() reads dir_config.override_applied during the directory iteration loop"
      pattern: "override_applied"
    - from: "crates/tome/src/doctor.rs DoctorReport per-directory entries"
      to: "crates/tome/src/config.rs DirectoryConfig.override_applied"
      via: "check() reads dir_config.override_applied when building the per-directory diagnostic shape"
      pattern: "override_applied"
    - from: "crates/tome/src/status.rs render_status (text mode)"
      to: "DirectoryStatus.override_applied"
      via: "PATH column gets ` (override)` suffix when the flag is true"
      pattern: "\\(override\\)"
---

<objective>
Surface `[directory_overrides.<name>]` activations in the two read-only commands a user reaches for when asking "why is this path different on this machine?": `tome status` and `tome doctor`. Both commands already render per-directory information; this plan adds the `override_applied` signal to their data structures (so `--json` consumers see it too) and adds an `(override)` annotation to the text output paths.

Plan 01 set the `DirectoryConfig.override_applied: bool` flag during config load. This plan reads that flag in `status::gather()` and `doctor::check()` and renders it.

**Closes:** PORT-05.

**Design choice (option a — `override_applied` flag on DirectoryConfig vs option b — diff against pre-override snapshot):** I picked option (a) in Plan 01 and use it here. Justification:

1. **Single source of truth.** The flag is set once in `Config::apply_machine_overrides` (Plan 01) — the same place that owns the I2 invariant. status/doctor never re-resolve overrides; they read the same merged config every other consumer reads.
2. **No second snapshot.** Option (b) would require status/doctor to load `MachinePrefs` themselves, build a pre-override `Config` snapshot, and diff paths — duplicating logic from `apply_machine_overrides` in two more places. That's exactly the kind of "second code path that observes pre-override paths" the I2 invariant forbids (success criterion 2).
3. **`#[serde(skip)]` keeps tome.toml clean.** The flag is invisible to TOML serialization (verified in Plan 01 Task 2), so no existing round-trip test breaks and no spurious field appears in user configs.
4. **Costs one bool per DirectoryConfig.** Effectively zero memory overhead.

The trade-off: any future code that constructs a `Config` in-memory (e.g., the wizard, tests) needs the field default-initialized to `false`. That's already handled by `#[serde(skip, default)]` in Plan 01.

Output: per-directory `override_applied` field in `DirectoryStatus` and the doctor report, text-mode `(override)` annotations in both commands' rendering, and one end-to-end integration test that pins the full PORT-05 contract.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/REQUIREMENTS.md

@.planning/phases/09-cross-machine-path-overrides/09-01-SUMMARY.md

@crates/tome/src/status.rs
@crates/tome/src/doctor.rs
@crates/tome/src/config.rs
@crates/tome/src/lib.rs
@crates/tome/tests/cli.rs

<interfaces>
<!-- Key types and contracts the executor needs. After Plan 01. -->

From `crates/tome/src/config.rs` (after Plan 01):
```rust
pub struct DirectoryConfig {
    pub path: PathBuf,
    pub directory_type: DirectoryType,
    pub(crate) role: Option<DirectoryRole>,
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub rev: Option<String>,
    pub subdir: Option<String>,
    #[serde(skip, default)]
    pub(crate) override_applied: bool,   // <-- added by Plan 01
}
```

The `override_applied` field is `pub(crate)` — accessible from `status.rs` and `doctor.rs` since both live in the same crate.

From `crates/tome/src/status.rs::DirectoryStatus`:
```rust
pub struct DirectoryStatus {
    pub name: String,
    pub directory_type: String,
    pub role: String,
    pub path: String,
    pub skill_count: CountOrError,
    pub warnings: Vec<String>,
    // <-- add: pub override_applied: bool,
}
```

From `crates/tome/src/status.rs::gather` (line ~75):
```rust
let directories: Vec<DirectoryStatus> = config
    .directories
    .iter()
    .map(|(name, dir_config)| {
        // ...
        DirectoryStatus {
            name: name.as_str().to_string(),
            directory_type: dir_config.directory_type.to_string(),
            role: role.description().to_string(),
            path: dir_config.path.display().to_string(),
            skill_count: skill_count.into(),
            warnings,
            // <-- add: override_applied: dir_config.override_applied,
        }
    })
    .collect();
```

From `crates/tome/src/status.rs::render_status` (line ~127, text-mode rendering — the table builder):
```rust
for dir in &report.directories {
    let count = match (&dir.skill_count.count, &dir.skill_count.error) {
        // ...
    };
    rows.push([
        dir.name.clone(),
        dir.directory_type.clone(),
        dir.role.clone(),
        crate::paths::collapse_home(std::path::Path::new(&dir.path)),  // <-- modify this column
        count,
    ]);
}
```

From `crates/tome/src/doctor.rs::DoctorReport`:
```rust
pub struct DoctorReport {
    pub configured: bool,
    pub library_issues: Vec<DiagnosticIssue>,
    pub directory_issues: Vec<(String, Vec<DiagnosticIssue>)>,   // <-- replace with new shape
    pub config_issues: Vec<DiagnosticIssue>,
}
```

The `(String, Vec<DiagnosticIssue>)` tuple is just `(name, issues)`. To carry `override_applied` we have two options:
- (a) Replace with `Vec<DirectoryDiagnostic>` where `DirectoryDiagnostic { name, override_applied, issues }`. Cleaner shape; minor JSON-schema break.
- (b) Add a parallel `Vec<String> override_applied_directory_names` next to `directory_issues`. Avoids the schema break but creates a second-source-of-truth lookup.

**Choose (a)** — `tome doctor --json` is consumed only by humans grep-ing JSON or by the future tome-desktop GUI (drafted but not yet implemented). The schema break is acceptable in v0.9, and the wrapped shape is what we'd want for the GUI anyway. Document the JSON shape change in the v0.9 CHANGELOG entry (out of scope for this plan; track via the existing changelog convention).

From `crates/tome/src/doctor.rs::check` (line ~57):
```rust
pub fn check(config: &Config, paths: &TomePaths) -> Result<DoctorReport> {
    // ...
    let mut directory_issues = Vec::new();
    for (name, dir_config) in config.distribution_dirs() {
        let issues = check_distribution_dir(name.as_str(), &dir_config.path, paths.library_dir())?;
        directory_issues.push((name.as_str().to_string(), issues));
        // <-- becomes: directory_issues.push(DirectoryDiagnostic {
        //                  name: name.as_str().to_string(),
        //                  override_applied: dir_config.override_applied,
        //                  issues,
        //              });
    }
    // ...
}
```

From `crates/tome/src/doctor.rs::render_issues_for_directory` (line ~383):
```rust
fn render_issues_for_directory(name: &str, issues: &[DiagnosticIssue]) {
    if issues.is_empty() {
        println!("  {} {}: OK", style("ok").green(), name);
    } else {
        for issue in issues {
            // ...
            println!("  {} {}: {}", marker, name, issue.message);
        }
    }
}
```
This signature stays; pass `override_applied: bool` as a third argument and append `" (override)"` to the name string when true.

**Annotation format:** `(override)` (lowercase, parens, no color in JSON, optional dim styling in text). Picked over alternatives:
- `[override]` — visually heavier
- `*` suffix — too cryptic
- A separate column — overkill for one bit; the table is already 5 cols wide

Apply the dim/cyan styling pattern that's already used elsewhere (`style("...").dim()` or `style("...").cyan()`). Match the existing console::style use in the file (status.rs uses `.cyan()` for paths; doctor.rs uses `.dim()` for "skip" text). Use `style("(override)").cyan()` to match status.rs convention.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Add `override_applied` to `DirectoryStatus` + render `(override)` in `tome status`</name>
  <files>crates/tome/src/status.rs</files>
  <read_first>
    - crates/tome/src/status.rs full file (673 lines — small enough to absorb)
    - crates/tome/src/config.rs lines 196–235 (DirectoryConfig.override_applied — verify field visibility is `pub(crate)`)
    - crates/tome/src/paths.rs (`collapse_home` — used in the PATH column)
  </read_first>
  <behavior>
    - Test 1 (`gather_with_no_overrides_sets_flag_false`): Config with one directory, `override_applied = false` → `gather()` produces `report.directories[0].override_applied == false`.
    - Test 2 (`gather_with_override_applied_sets_flag_true`): Config with one directory, `override_applied = true` (set manually in test) → `report.directories[0].override_applied == true`.
    - Test 3 (`render_status_appends_override_marker_to_path`): Build a `StatusReport` with one directory, `override_applied = true`, `path = "/foo"`. Capture the output of `render_status` (or test the column composition logic in isolation) and assert the PATH column contains `(override)` AND `/foo`. (If `render_status` writes directly to stdout, factor the per-row formatting into a small helper `format_dir_path_column(path: &str, override_applied: bool) -> String` and unit-test that.)
    - Test 4 (`render_status_no_override_omits_marker`): Same as Test 3 but `override_applied = false` → output does NOT contain `(override)`.
    - Test 5 (`status_json_includes_override_applied_field`): Serialize a `DirectoryStatus` with `override_applied = true` to JSON via `serde_json::to_string` → resulting string contains `"override_applied":true`. (Confirms the field is exposed in machine-readable output.)
  </behavior>
  <action>
**Step 1 — Add field to `DirectoryStatus`:**

In `crates/tome/src/status.rs` lines 38–49, add the new field:
```rust
#[derive(serde::Serialize)]
pub struct DirectoryStatus {
    pub name: String,
    pub directory_type: String,
    pub role: String,
    pub path: String,
    pub skill_count: CountOrError,
    pub warnings: Vec<String>,
    /// True iff `directories.<name>.path` was rewritten by a machine.toml
    /// `[directory_overrides.<name>]` entry during config load (PORT-05).
    /// JSON consumers can use this to render the same context that text-mode
    /// `tome status` shows via the `(override)` annotation.
    pub override_applied: bool,
}
```

**Step 2 — Populate field in `gather()`:**

In the existing `gather()` function (line ~75), inside the `.map(|(name, dir_config)| { ... })` closure, add `override_applied` to the struct literal:
```rust
DirectoryStatus {
    name: name.as_str().to_string(),
    directory_type: dir_config.directory_type.to_string(),
    role: role.description().to_string(),
    path: dir_config.path.display().to_string(),
    skill_count: skill_count.into(),
    warnings,
    override_applied: dir_config.override_applied,
}
```

**Step 3 — Render `(override)` in text mode:**

Extract a small helper near the top of the file (above `render_status`):
```rust
/// Format the PATH column for the directories table. When `override_applied`
/// is true, append a styled ` (override)` annotation so the user can see
/// which entries were rewritten by a machine.toml override (PORT-05).
fn format_dir_path_column(path: &str, override_applied: bool) -> String {
    let collapsed = crate::paths::collapse_home(std::path::Path::new(path));
    if override_applied {
        format!("{} {}", collapsed, style("(override)").cyan())
    } else {
        collapsed
    }
}
```

Then update the table-building loop in `render_status` (~line 168):
```rust
for dir in &report.directories {
    let count = match (&dir.skill_count.count, &dir.skill_count.error) {
        // ... unchanged ...
    };
    rows.push([
        dir.name.clone(),
        dir.directory_type.clone(),
        dir.role.clone(),
        format_dir_path_column(&dir.path, dir.override_applied),  // <-- changed line
        count,
    ]);
}
```

**Implementation notes:**
- `style` is already imported at the top of `status.rs` (`use console::style;` — verify at line 4).
- The helper `collapse_home` is reused inside `format_dir_path_column`; do NOT call it twice in the table loop.
- The annotation uses `.cyan()` to match the existing `style(... ).cyan()` pattern in `render_status` (e.g., the library count line at line 151). Avoid `.bold()` — it would dominate the row visually.
- The annotation is appended AFTER the path with one space separator (no parentheses around the path itself).
- For the JSON schema, `override_applied` is a public field of a `pub struct` with `#[derive(serde::Serialize)]` — no extra attributes needed; it'll appear in `tome status --json`.
- Since `DirectoryStatus` does NOT derive `Default`, all existing test code that builds a `DirectoryStatus` literal must add the new field. Find these with `rg -n "DirectoryStatus \\{" crates/tome/`. Update each to set `override_applied: false`.

Add the 5 unit tests in the existing `#[cfg(test)] mod tests` of `status.rs`. Reuse existing config-builder patterns from the file's existing tests (`gather_with_directories_marks_configured`, etc.).

For Test 3/4, test the `format_dir_path_column` helper directly. To avoid stripping ANSI codes, set `console::set_colors_enabled(false)` at the start of the test, OR strip codes via a regex/substring check. Match the file's existing approach: search for `set_colors_enabled` in `status.rs`; if not used, prefer a substring assertion that doesn't depend on ANSI:
```rust
let s = format_dir_path_column("/foo", true);
assert!(s.contains("/foo"));
assert!(s.contains("(override)"));
```
The `console::style` produces strings that contain the literal `(override)` even with ANSI escapes — substring assertion works either way.

Run: `cargo test -p tome --lib status::tests`
  </action>
  <verify>
    <automated>cargo test -p tome --lib status::tests 2>&1 | tail -20 && cargo clippy -p tome --all-targets -- -D warnings 2>&1 | tail -5</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "pub override_applied: bool" crates/tome/src/status.rs` returns exactly 1 match.
    - `rg -n "fn format_dir_path_column" crates/tome/src/status.rs` returns exactly 1 match.
    - `rg -n "override_applied: dir_config\\.override_applied" crates/tome/src/status.rs` returns exactly 1 match (the `gather()` populator).
    - `cargo test -p tome --lib status::tests::gather_with_no_overrides_sets_flag_false` passes.
    - `cargo test -p tome --lib status::tests::gather_with_override_applied_sets_flag_true` passes.
    - `cargo test -p tome --lib status::tests::render_status_appends_override_marker_to_path` passes.
    - `cargo test -p tome --lib status::tests::render_status_no_override_omits_marker` passes.
    - `cargo test -p tome --lib status::tests::status_json_includes_override_applied_field` passes.
    - All pre-existing status::tests still pass (regression — `DirectoryStatus { ... }` literals updated everywhere).
    - `cargo clippy -p tome --all-targets -- -D warnings` is clean.
  </acceptance_criteria>
  <done>
    `DirectoryStatus.override_applied` field exists, `gather()` populates it from `DirectoryConfig.override_applied`, `format_dir_path_column` helper appends ` (override)` when true, and 5 unit tests pin the contract.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Add `override_applied` to `DoctorReport` per-directory entries + render annotation</name>
  <files>crates/tome/src/doctor.rs</files>
  <read_first>
    - crates/tome/src/doctor.rs lines 26–85 (DiagnosticIssue, DoctorReport, check())
    - crates/tome/src/doctor.rs lines 116–127 (the per-directory render loop in `diagnose()`)
    - crates/tome/src/doctor.rs lines 383–395 (render_issues_for_directory)
    - crates/tome/src/doctor.rs lines 335–365 (render_repair_plan_auto — also iterates directory_issues)
    - status.rs Task 1 result (mirror the field naming and styling)
  </read_first>
  <behavior>
    - Test 1 (`check_with_no_overrides_sets_flags_false`): Config with one distribution directory, `override_applied = false` → `report.directory_issues[0].override_applied == false`.
    - Test 2 (`check_with_override_applied_sets_flag_true`): Config with one distribution directory, `override_applied = true` → `report.directory_issues[0].override_applied == true`.
    - Test 3 (`render_issues_for_directory_appends_override_marker_when_set`): Capture `render_issues_for_directory("work", &[issue], true)` output → contains `(override)` in the line that names `work`.
    - Test 4 (`render_issues_for_directory_omits_marker_when_unset`): Same as Test 3 but `override_applied = false` → output does NOT contain `(override)`.
    - Test 5 (`doctor_json_includes_override_applied_per_directory`): Serialize a `DoctorReport` (with one entry, `override_applied = true`) → JSON contains `"override_applied":true` inside the directory entry.
    - Test 6 (`total_issues_unchanged_by_directory_diagnostic_shape`): The existing `total_issues()` accounting still works after the shape change — count is correct regardless of how many directories have overrides.
  </behavior>
  <action>
**Step 1 — Replace `(String, Vec<DiagnosticIssue>)` tuple with `DirectoryDiagnostic` struct:**

In `crates/tome/src/doctor.rs` line ~26 (above `DoctorReport`):
```rust
/// Per-directory diagnostic entry. Aggregates issues for one configured
/// directory and notes whether its `path` was rewritten by a machine.toml
/// `[directory_overrides.<name>]` entry (PORT-05).
#[derive(Debug, Clone, serde::Serialize)]
pub struct DirectoryDiagnostic {
    pub name: String,
    pub issues: Vec<DiagnosticIssue>,
    /// True iff `directories.<name>.path` was rewritten by a machine.toml
    /// override during config load. Renders as ` (override)` after the
    /// directory name in text mode; appears as `override_applied: true` in
    /// `tome doctor --json`.
    pub override_applied: bool,
}
```

Update `DoctorReport` (line ~35):
```rust
#[derive(Debug, serde::Serialize)]
pub struct DoctorReport {
    pub configured: bool,
    pub library_issues: Vec<DiagnosticIssue>,
    pub directory_issues: Vec<DirectoryDiagnostic>,   // <-- was Vec<(String, Vec<DiagnosticIssue>)>
    pub config_issues: Vec<DiagnosticIssue>,
}
```

Update `total_issues()` (line ~42):
```rust
impl DoctorReport {
    pub fn total_issues(&self) -> usize {
        self.library_issues.len()
            + self
                .directory_issues
                .iter()
                .map(|d| d.issues.len())   // <-- was .map(|(_, v)| v.len())
                .sum::<usize>()
            + self.config_issues.len()
    }
}
```

**Step 2 — Update `check()` (line ~57) to populate the new field:**

```rust
let mut directory_issues = Vec::new();
for (name, dir_config) in config.distribution_dirs() {
    let issues = check_distribution_dir(name.as_str(), &dir_config.path, paths.library_dir())?;
    directory_issues.push(DirectoryDiagnostic {
        name: name.as_str().to_string(),
        issues,
        override_applied: dir_config.override_applied,
    });
}
```

**Step 3 — Update all consumers of `directory_issues`:**

Find them with `rg -n "directory_issues" crates/tome/src/doctor.rs`. Expected sites:
- Line ~121 (in `diagnose()`): change `for (name, issues) in &report.directory_issues` → `for d in &report.directory_issues` and update body to `render_issues_for_directory(&d.name, &d.issues, d.override_applied)`.
- Line ~350 (in `render_repair_plan_auto`): change `for (name, issues) in &report.directory_issues` → `for d in &report.directory_issues` and update body to use `&d.name` / `&d.issues`.

**Step 4 — Update `render_issues_for_directory` signature** (line ~383) to accept `override_applied`:

```rust
fn render_issues_for_directory(name: &str, issues: &[DiagnosticIssue], override_applied: bool) {
    let display_name = if override_applied {
        format!("{} {}", name, style("(override)").cyan())
    } else {
        name.to_string()
    };
    if issues.is_empty() {
        println!("  {} {}: OK", style("ok").green(), display_name);
    } else {
        for issue in issues {
            let marker = match issue.severity {
                IssueSeverity::Error => style("x").red(),
                IssueSeverity::Warning => style("!").yellow(),
            };
            println!("  {} {}: {}", marker, display_name, issue.message);
        }
    }
}
```

**Implementation notes:**
- `console::style` is already imported in `doctor.rs` (`use console::style;` — verify at line 4 or thereabouts).
- The `(override)` annotation uses `.cyan()` to match the status.rs Task 1 styling exactly. (Consistency — not a strong requirement, but good for the user.)
- ALL existing tests in `doctor.rs` that reference `directory_issues` (search with `rg -n "directory_issues" crates/tome/src/doctor.rs`) need updating to use the new shape:
  - Construction: `directory_issues: vec![DirectoryDiagnostic { name: "...".to_string(), issues: vec![...], override_applied: false }]` instead of `vec![("...".to_string(), vec![...])]`.
  - Field access: `report.directory_issues[0].name` / `.issues` instead of `.0` / `.1` tuple access.
- Add the 6 new tests at the end of the existing `#[cfg(test)] mod tests` block.
- Test 3/4 capture stdout. The existing `doctor.rs` tests do not capture stdout (verify with grep). Two options:
  - (a) Refactor `render_issues_for_directory` to take an `&mut impl std::io::Write` instead of `println!` — bigger change but enables clean unit testing.
  - (b) Use `cargo test --captured` reliance + a substring check on the test thread's stdout via the `gag` crate (NOT in the deps).
  - (c) Just exercise the helper indirectly: call it (it prints to stdout — `cargo test` swallows that), and instead test the **string-building** logic by extracting `format_dir_diagnostic_header(name: &str, override_applied: bool) -> String` and testing that helper directly. Use the same pattern as Task 1's `format_dir_path_column`.
  
  **Choose (c)** — minimum invasion, parallel to Task 1, avoids a captured-stdout dependency. Refactor `render_issues_for_directory` to call:
  ```rust
  fn format_dir_diagnostic_header(name: &str, override_applied: bool) -> String {
      if override_applied {
          format!("{} {}", name, style("(override)").cyan())
      } else {
          name.to_string()
      }
  }
  ```
  And test that helper directly:
  ```rust
  let s = format_dir_diagnostic_header("work", true);
  assert!(s.contains("work") && s.contains("(override)"));
  let s = format_dir_diagnostic_header("work", false);
  assert!(s.contains("work") && !s.contains("(override)"));
  ```

Run: `cargo test -p tome --lib doctor::tests`
  </action>
  <verify>
    <automated>cargo test -p tome --lib doctor::tests 2>&1 | tail -25 && cargo clippy -p tome --all-targets -- -D warnings 2>&1 | tail -5</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "pub struct DirectoryDiagnostic" crates/tome/src/doctor.rs` returns exactly 1 match.
    - `rg -n "pub override_applied: bool" crates/tome/src/doctor.rs` returns exactly 1 match.
    - `rg -n "Vec<DirectoryDiagnostic>" crates/tome/src/doctor.rs` returns at least 1 match (the `DoctorReport.directory_issues` field).
    - `rg -n "Vec<\\(String, Vec<DiagnosticIssue>\\)>" crates/tome/src/doctor.rs` returns 0 matches (old tuple shape gone).
    - `rg -n "fn format_dir_diagnostic_header" crates/tome/src/doctor.rs` returns exactly 1 match.
    - `cargo test -p tome --lib doctor::tests::check_with_no_overrides_sets_flags_false` passes.
    - `cargo test -p tome --lib doctor::tests::check_with_override_applied_sets_flag_true` passes.
    - `cargo test -p tome --lib doctor::tests::render_issues_for_directory_appends_override_marker_when_set` passes.
    - `cargo test -p tome --lib doctor::tests::render_issues_for_directory_omits_marker_when_unset` passes.
    - `cargo test -p tome --lib doctor::tests::doctor_json_includes_override_applied_per_directory` passes.
    - `cargo test -p tome --lib doctor::tests::total_issues_unchanged_by_directory_diagnostic_shape` passes.
    - All pre-existing doctor::tests still pass (regression — tuple → struct migration didn't break anything).
    - `cargo clippy -p tome --all-targets -- -D warnings` is clean.
  </acceptance_criteria>
  <done>
    `DoctorReport.directory_issues: Vec<DirectoryDiagnostic>` carries `override_applied`, `check()` populates it, `render_issues_for_directory` accepts the flag and renders `(override)`, and 6 unit tests cover the matrix.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: End-to-end integration test — full PORT-05 contract</name>
  <files>crates/tome/tests/cli.rs</files>
  <read_first>
    - crates/tome/tests/cli.rs lines 1–80 (test infrastructure)
    - The smoke test `machine_override_rewrites_directory_path_for_status` (added in Plan 01)
    - The two PORT-03/04 tests (added in Plan 02)
  </read_first>
  <action>
Add one comprehensive integration test in `crates/tome/tests/cli.rs`, after the Plan 02 tests. This test exercises the full I2 invariant + PORT-05 contract end-to-end through the real CLI:

1. Set up `tome.toml` with one synced directory.
2. Set up `machine.toml` with `[directory_overrides.<name>]` rewriting that directory's path.
3. Run `tome sync` — must succeed (covers I2: sync sees the merged result).
4. Run `tome status` (text) — must succeed AND stdout must contain `(override)`.
5. Run `tome status --json` — must include `"override_applied":true` for the overridden directory AND `"override_applied":false` for any non-overridden directory.
6. Run `tome doctor --json` — must include `"override_applied":true` for the overridden directory.

```rust
#[cfg(unix)]
#[test]
fn machine_override_appears_in_status_and_doctor() {
    // PORT-05 (and end-to-end PORT-01/02 confirmation): an override declared
    // in machine.toml causes:
    //   - `tome sync` to operate on the overridden path,
    //   - `tome status` text mode to show `(override)` on the affected row,
    //   - `tome status --json` to include `override_applied: true`,
    //   - `tome doctor --json` to include `override_applied: true` for the overridden directory.
    //
    // The overridden directory `work` uses role = "synced" so it appears in BOTH
    // discovery (skill-a from real_path is consolidated into the library) AND
    // distribution (`tome doctor` diagnoses it). This pins the full PORT-05
    // contract end-to-end on an actually-overridden directory.
    let tmp = TempDir::new().unwrap();
    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();

    // Two directories. `work` is synced (discovery + distribution); `other` is
    // a plain source for the negative-case check in status JSON.
    let dotfiles_path = tmp.path().join("dotfiles-says-here");
    let real_path = tmp.path().join("real-skills");
    create_skill(&real_path, "skill-a");
    // The synced directory must EXIST on disk pre-sync — distribute writes
    // symlinks into it. Create the real path's parent (already done by
    // `create_skill`) and ensure `real_path` itself is a directory.
    assert!(real_path.is_dir(), "real_path must exist for sync to succeed");

    let other_path = tmp.path().join("other-skills");
    create_skill(&other_path, "skill-b");

    let tome_toml = format!(
        "library_dir = \"{}\"\n\
         \n\
         [directories.work]\n\
         path = \"{}\"\n\
         type = \"directory\"\n\
         role = \"synced\"\n\
         \n\
         [directories.other]\n\
         path = \"{}\"\n\
         type = \"directory\"\n\
         role = \"source\"\n",
        library_dir.display(),
        dotfiles_path.display(),
        other_path.display(),
    );
    std::fs::write(tmp.path().join("tome.toml"), tome_toml).unwrap();

    let machine_toml = format!(
        "[directory_overrides.work]\npath = \"{}\"\n",
        real_path.display(),
    );
    let machine_path = tmp.path().join("machine.toml");
    std::fs::write(&machine_path, machine_toml).unwrap();

    // 1. `tome sync` must succeed — sync sees the overridden path.
    let sync_assert = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "sync",
            "--no-triage",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();
    let sync_stdout = String::from_utf8(sync_assert.get_output().stdout.clone()).unwrap();
    let skill_a_in_lib = library_dir.join("skill-a").exists();
    assert!(
        skill_a_in_lib,
        "expected skill-a from overridden path to be consolidated, got sync stdout:\n{sync_stdout}",
    );

    // 2. `tome status` text mode — stdout contains `(override)` exactly once.
    let status_assert = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "status",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();
    let status_stdout = String::from_utf8(status_assert.get_output().stdout.clone()).unwrap();
    assert!(
        status_stdout.contains("(override)"),
        "expected `tome status` text output to contain `(override)`, got:\n{status_stdout}"
    );
    let override_marker_count = status_stdout.matches("(override)").count();
    assert_eq!(
        override_marker_count, 1,
        "expected exactly one `(override)` marker (for `work`), got {override_marker_count} in:\n{status_stdout}"
    );

    // 3. `tome status --json` — `work` has `override_applied: true`, `other` has false.
    let status_json_assert = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "status",
            "--json",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();
    let status_json: serde_json::Value =
        serde_json::from_slice(&status_json_assert.get_output().stdout)
            .expect("status --json output must be valid JSON");
    let dirs = status_json["directories"].as_array().unwrap();
    let work = dirs.iter().find(|d| d["name"] == "work").unwrap();
    let other = dirs.iter().find(|d| d["name"] == "other").unwrap();
    assert_eq!(
        work["override_applied"], serde_json::Value::Bool(true),
        "expected work.override_applied == true, got: {work}"
    );
    assert_eq!(
        other["override_applied"], serde_json::Value::Bool(false),
        "expected other.override_applied == false, got: {other}"
    );

    // 4. `tome doctor --json` — `work` (now synced/distribution) appears in
    // `directory_issues` and carries `override_applied: true`. This is the
    // strongest end-to-end PORT-05 doctor assertion.
    let doctor_json_assert = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "doctor",
            "--json",
        ])
        .env("NO_COLOR", "1")
        .assert();
    // doctor exit may be 0 or non-0 depending on issues found — accept either.
    let doctor_json: serde_json::Value =
        serde_json::from_slice(&doctor_json_assert.get_output().stdout)
            .expect("doctor --json output must be valid JSON");

    let doctor_dirs = doctor_json["directory_issues"]
        .as_array()
        .expect("doctor --json must include directory_issues array");
    let work_entry = doctor_dirs
        .iter()
        .find(|d| d["name"] == "work")
        .expect("work must appear in doctor directory_issues (it has role = synced)");
    assert_eq!(
        work_entry["override_applied"],
        serde_json::Value::Bool(true),
        "expected work.override_applied == true in doctor JSON, got: {work_entry}"
    );

    // Sanity: every entry in directory_issues uses the new DirectoryDiagnostic
    // shape (has `name`, `issues`, and `override_applied`).
    for entry in doctor_dirs {
        assert!(
            entry.get("name").is_some()
                && entry.get("issues").is_some()
                && entry.get("override_applied").is_some(),
            "expected DirectoryDiagnostic shape (name + issues + override_applied), got: {entry}"
        );
    }
}
```

**Implementation notes:**
- `serde_json` is already a workspace dependency (used by other tests). Verify with `rg "serde_json" crates/tome/Cargo.toml`.
- The `role = "synced"` choice for `work` makes it both a discovery dir (skill-a from `real_path` is consolidated) AND a distribution dir (so `tome doctor` will diagnose it and emit a `directory_issues` entry). This pins PORT-05 end-to-end on an actually-overridden directory.
- `real_path` (the override target) MUST exist on disk pre-sync because synced directories receive symlink writes during distribute. `create_skill(&real_path, "skill-a")` ensures both the directory and a SKILL.md exist.
- The `override_marker_count == 1` assertion guards against rendering bugs that would put `(override)` on every row.

Run: `cargo test -p tome --test cli machine_override_appears_in_status_and_doctor`
  </action>
  <verify>
    <automated>cargo test -p tome --test cli machine_override_appears_in_status_and_doctor</automated>
  </verify>
  <acceptance_criteria>
    - `cargo test -p tome --test cli machine_override_appears_in_status_and_doctor` passes.
    - `make ci` passes (no regressions).
    - The test exercises the full chain: sync respects override → status text shows `(override)` exactly once → status JSON has correct booleans for both dirs → doctor JSON's `work` entry exists AND has `override_applied: true`.
    - The `work` entry in `doctor_json["directory_issues"]` is asserted by name (`.find(|d| d["name"] == "work")` returns Some) — pinning the full PORT-05 doctor contract on an actually-overridden distribution directory.
    - Every entry in `doctor_json["directory_issues"]` is asserted to use the new `DirectoryDiagnostic` JSON shape (object with `name`, `issues`, `override_applied`) — guarding against tuple-shape regressions.
  </acceptance_criteria>
  <done>
    PORT-05 is pinned end-to-end by an integration test that covers all four user-visible surfaces (`tome sync`, `tome status` text, `tome status --json`, `tome doctor --json`). The full I2 invariant from PORT-02 is also implicitly verified — sync, status, and doctor all see the same merged config.
  </done>
</task>

</tasks>

<verification>
- `cargo test -p tome --lib status::tests` — all status tests pass (5 new + existing).
- `cargo test -p tome --lib doctor::tests` — all doctor tests pass (6 new + existing tuple → struct migration).
- `cargo test -p tome --test cli machine_override_appears_in_status_and_doctor` — passes.
- `make ci` — clean.
- `rg -n "Vec<\\(String, Vec<DiagnosticIssue>\\)>" crates/tome/src/doctor.rs` — 0 matches (old tuple shape removed).
- `rg -n "DirectoryDiagnostic" crates/tome/src/doctor.rs` — at least 4 matches (struct, field decl, check populator, ≥ 1 test).
</verification>

<success_criteria>
- `DirectoryStatus.override_applied: bool` field is populated from `DirectoryConfig.override_applied` in `status::gather` (PORT-05).
- `tome status` text mode appends ` (override)` to the PATH column for affected directories; non-affected rows show no marker.
- `tome status --json` includes `"override_applied":true|false"` per directory entry.
- `DoctorReport.directory_issues: Vec<DirectoryDiagnostic>` carries `override_applied: bool`; `check()` populates from config; `tome doctor` text and JSON both surface it.
- The annotation marker uses a consistent `(override)` form styled `cyan` in both commands.
- An end-to-end integration test pins all four user-visible surfaces (sync, status text, status JSON, doctor JSON) against a real `tome.toml` + `machine.toml` fixture pair.
</success_criteria>

<output>
After completion, create `.planning/phases/09-cross-machine-path-overrides/09-03-SUMMARY.md` recording:
- New `DirectoryStatus.override_applied: bool` field signature.
- New `format_dir_path_column` helper signature.
- New `DirectoryDiagnostic` struct + the schema break: `DoctorReport.directory_issues: Vec<DirectoryDiagnostic>` (was `Vec<(String, Vec<DiagnosticIssue>)>`).
- Updated `render_issues_for_directory` signature (now takes `override_applied: bool`).
- New `format_dir_diagnostic_header` helper signature.
- Test names added (status.rs ≥ 5, doctor.rs ≥ 6, tests/cli.rs = 1).
- One-line confirmation: PORT-05 closed.
- A note for the v0.9 CHANGELOG entry: `tome doctor --json` schema break — `directory_issues` items are now objects with `name`/`issues`/`override_applied` instead of `[name, issues]` tuples.
- Phase 9 wrap: all 5 PORT requirements (01..05) shipped across plans 01–03.
</output>
