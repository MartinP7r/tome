---
phase: 19-doctor-status-surface-bugfix-bundle
plan: 03
type: execute
wave: 2
depends_on: [01]
files_modified:
  - crates/tome/src/manifest.rs
  - crates/tome/src/status.rs
  - crates/tome/src/lib.rs
  - crates/tome/tests/cli_status.rs
autonomous: true
requirements: [OBS-07]
requirements_addressed: [OBS-07]

must_haves:
  truths:
    - "tome status text output prints a top-line `Last sync: <RFC-3339 timestamp>` (or `Last sync: never` when manifest doesn't exist or last_synced_at is None)"
    - "tome status --json adds a `last_sync: Option<String>` field at top level — `null` for never, RFC-3339 string otherwise"
    - "tome status text Directories table includes a SKILLS column rendered as `✓ N` or `✗ ?` matching the CountOrError pattern; existing (override) annotation from PORT-05 preserved"
    - "tome sync stamps manifest.last_synced_at via stamp_last_synced_at() immediately before manifest::save in the existing `if !dry_run && paths.config_dir().is_dir()` block — after distribute + cleanup succeed, before lockfile.save"
    - "Pre-v0.11 manifests deserialize cleanly with last_synced_at: None (additive schema; no migration required)"
    - "Mid-sync panic or early bail leaves the previous last_synced_at value unchanged (D-LSYNC-3 honest reporting)"
  artifacts:
    - path: "crates/tome/src/manifest.rs"
      provides: "last_synced_at: Option<String> field on Manifest + last_synced_at() accessor + stamp_last_synced_at() method; serde additive-compat via #[serde(default)] + #[serde(skip_serializing_if = \"Option::is_none\")]"
      contains: "last_synced_at"
    - path: "crates/tome/src/status.rs"
      provides: "StatusReport.last_sync: Option<String>; Last sync: line in render_status; SKILLS column in Directories table"
      contains: "last_sync"
    - path: "crates/tome/src/lib.rs"
      provides: "stamp_last_synced_at() call in sync() between cleanup and existing manifest::save call"
      contains: "stamp_last_synced_at"
    - path: "crates/tome/tests/cli_status.rs"
      provides: "Integration tests: pre-v0.11 manifest deserializes with None; stamp round-trip via sync; Last sync: never rendering; JSON last_sync: null shape; SKILLS column appears"
      contains: "status_last_sync"
  key_links:
    - from: "crates/tome/src/lib.rs::sync()"
      to: "manifest.last_synced_at"
      via: "manifest.stamp_last_synced_at() immediately before manifest::save"
      pattern: "manifest\\.stamp_last_synced_at"
    - from: "crates/tome/src/status.rs::gather()"
      to: "manifest.last_synced_at()"
      via: "thread accessor result into StatusReport.last_sync"
      pattern: "\\.last_synced_at\\(\\)"
    - from: "crates/tome/src/status.rs::render_status"
      to: "Directories table"
      via: "5-column tabled::Table: NAME / TYPE / ROLE / PATH / SKILLS"
      pattern: "SKILLS"
---

<objective>
Deliver OBS-07: `tome status` gains a top-line `Last sync: <RFC-3339 timestamp>` and a per-directory SKILLS column in the Directories table; JSON shape parity (`last_sync` at top level, `skill_count` already present in JSON since v0.9 — surfaced in text now). Manifest gains a top-level `last_synced_at: Option<String>` field stamped at the end of every successful `sync()` (D-LSYNC-3 — after distribute + cleanup, before lockfile.save).

Purpose: Close OBS-07. Gives users an at-a-glance signal of when sync last completed cleanly + per-directory skill density without running `tome doctor` or `tome list`.
Output: Manifest schema additive lift (`Option<String>` + `#[serde(default)]`); `StatusReport.last_sync` field; `render_status` Last-sync line + SKILLS column; sync() stamp call; 5+ integration tests pinning the shape.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/phases/19-doctor-status-surface-bugfix-bundle/19-CONTEXT.md
@.planning/phases/19-doctor-status-surface-bugfix-bundle/19-RESEARCH.md
@crates/tome/src/manifest.rs
@crates/tome/src/status.rs
@crates/tome/src/lib.rs

<interfaces>
<!-- Existing types/functions extracted from the codebase via RESEARCH.md
     code anchors. Use directly — no codebase exploration needed. -->

From `crates/tome/src/manifest.rs:22-26` (current shape):
```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Manifest {
    skills: BTreeMap<SkillName, SkillEntry>,  // private field
}
```
Already exposes: `pub fn iter(&self) -> ...`, `pub fn get(&self, name: &SkillName) -> Option<&SkillEntry>`, `pub fn insert(...)`, `pub fn remove(...)`, etc. The `skills` field stays private — the new `last_synced_at` also stays private with accessor + mutator methods.

Already available: `now_iso8601()` helper (returns RFC-3339 string for the current UTC time). Used by per-entry `synced_at` already.

From `crates/tome/src/status.rs:40-50` (current `DirectoryStatus`):
```rust
pub struct DirectoryStatus {
    pub name: String,
    pub directory_type: String,
    pub role: String,
    pub path: PathBuf,
    pub override_applied: bool,        // PORT-05
    pub skill_count: CountOrError,     // already in JSON; D-DIR-1 surfaces in text
}
```

From `crates/tome/src/status.rs` (`StatusReport`, around :30-40):
```rust
pub struct StatusReport {
    pub configured: bool,
    pub library_dir: PathBuf,
    pub library_count: CountOrError,
    pub directories: Vec<DirectoryStatus>,
    pub unowned: Vec<crate::summary::SkillSummary>,  // UNOWN-03
    pub health: CountOrError,
}
```

From `crates/tome/src/lib.rs:1779-1789` (current sync save block — RESEARCH-verified):
```rust
// 7. Save manifest, gitignore, and lockfile
if !dry_run && paths.config_dir().is_dir() {
    manifest::save(&manifest, paths.config_dir())?;  // <-- existing save
    // ... gitignore + lockfile saves follow
}
```

CountOrError glyph pattern (from existing `status.rs:246-253`):
```rust
let count = match (&dir.skill_count.count, &dir.skill_count.error) {
    (Some(n), _) => format!("✓ {}", n),
    (None, Some(e)) => { eprintln!(...); "✗ ?".to_string() }
    (None, None) => "✓ 0".to_string(),
};
```
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Add last_synced_at to Manifest with additive serde compat + accessor/mutator</name>
  <files>crates/tome/src/manifest.rs</files>
  <read_first>
    - crates/tome/src/manifest.rs (full file — Manifest struct ~lines 22-26, now_iso8601 helper, existing test patterns at the bottom of the file)
    - .planning/phases/19-doctor-status-surface-bugfix-bundle/19-CONTEXT.md (D-LSYNC-1, D-LSYNC-2, D-LSYNC-3 locked decisions)
    - .planning/phases/19-doctor-status-surface-bugfix-bundle/19-RESEARCH.md "OBS-07 Rendering Specifics" → "Manifest header placement" (lines 308-360)
  </read_first>
  <behavior>
    - Test 1 (additive-compat): A manifest JSON written by v0.10 (lacking `last_synced_at` key) deserializes successfully with `last_synced_at == None`.
    - Test 2 (round-trip): `let mut m = Manifest::default(); m.stamp_last_synced_at(); let s = serde_json::to_string(&m).unwrap(); let m2: Manifest = serde_json::from_str(&s).unwrap();` — `m2.last_synced_at()` returns Some(s) where s is parseable as RFC-3339.
    - Test 3 (skip_serializing_if): A default Manifest (last_synced_at: None) serializes WITHOUT the `last_synced_at` key in the output JSON.
    - Test 4 (accessor returns &str-shaped Option): `Manifest::default().last_synced_at()` returns None; after `stamp_last_synced_at()`, returns Some(non-empty str).
  </behavior>
  <action>
    Extend the `Manifest` struct at `crates/tome/src/manifest.rs:22-26`:

    ```rust
    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    pub struct Manifest {
        skills: BTreeMap<SkillName, SkillEntry>,
        /// Timestamp of last successful `tome sync` completion (post-cleanup).
        /// Stamped by `sync()` after distribute + cleanup succeed (D-LSYNC-3).
        /// `None` for pre-v0.11 manifests; renders as "never" in `tome status`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        last_synced_at: Option<String>,
    }
    ```

    Add accessor + mutator methods in the existing `impl Manifest` block:

    ```rust
    impl Manifest {
        /// RFC-3339 timestamp of last successful sync; `None` for pre-v0.11
        /// manifests or before any sync has completed.
        pub fn last_synced_at(&self) -> Option<&str> {
            self.last_synced_at.as_deref()
        }

        /// Stamps `last_synced_at` with the current UTC time in RFC-3339 form.
        /// Called by `sync()` after distribute + cleanup succeed (D-LSYNC-3).
        /// Crate-visible only — external mutation must go through sync.
        pub(crate) fn stamp_last_synced_at(&mut self) {
            self.last_synced_at = Some(now_iso8601());
        }
    }
    ```

    Add four unit tests in the existing `#[cfg(test)] mod tests` block:

    1. `manifest_pre_v011_json_deserializes_with_none_last_synced_at` — feed `{"skills": {}}` into `serde_json::from_str::<Manifest>` and assert `last_synced_at` is None.
    2. `manifest_stamp_round_trip_preserves_timestamp` — default, stamp, serialize, deserialize, assert Some(non-empty) survives.
    3. `manifest_default_skips_last_synced_at_in_json` — default manifest serializes without the `last_synced_at` key (assert via `!json.contains("last_synced_at")`).
    4. `manifest_last_synced_at_accessor_shape` — default returns None; after stamp returns Some(&str).

    HARD-20 epoch-zero edge case: RESEARCH.md OQ-3 notes the existing `epoch_zero_warning` pattern at `manifest.rs:198-218`. For Phase 19, treat the epoch-zero case as a non-issue (the system clock cannot reasonably be at epoch during a running `tome sync`). Do NOT thread epoch_zero_warning over the header field — that's a future polish if it ever surfaces.

    The `skills` field stays private. The new field also stays private — accessor + mutator are the only public API.
  </action>
  <verify>
    <automated>cargo test -p tome --lib manifest::tests::manifest_pre_v011 manifest::tests::manifest_stamp manifest::tests::manifest_default_skips manifest::tests::manifest_last_synced_at</automated>
  </verify>
  <acceptance_criteria>
    - `rg "last_synced_at: Option<String>" crates/tome/src/manifest.rs` returns 1 match
    - `rg 'fn last_synced_at\(&self\) -> Option<&str>' crates/tome/src/manifest.rs` returns 1 match
    - `rg "fn stamp_last_synced_at" crates/tome/src/manifest.rs` returns 1 match
    - `rg "manifest_pre_v011_json_deserializes_with_none_last_synced_at" crates/tome/src/manifest.rs` returns 1 match
    - `rg "skip_serializing_if = \"Option::is_none\"" crates/tome/src/manifest.rs` returns at least 1 match (the new field)
    - `cargo test -p tome --lib manifest::tests` exits 0 with all four new tests passing
    - `cargo clippy --all-targets -- -D warnings` exits 0
  </acceptance_criteria>
  <done>Manifest carries the new field + accessor/mutator; four unit tests pass; pre-v0.11 manifests deserialize cleanly.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Stamp manifest.last_synced_at in sync() + plumb to StatusReport.last_sync + render `Last sync:` line + 5-column SKILLS table</name>
  <files>crates/tome/src/lib.rs, crates/tome/src/status.rs</files>
  <read_first>
    - crates/tome/src/lib.rs lines 1450-1900 (the full sync() pipeline — especially the existing manifest::save block around :1779-1789)
    - crates/tome/src/status.rs (full file — gather() at :123, render_status at :204-300, current 4-column Directories table render)
    - .planning/phases/19-doctor-status-surface-bugfix-bundle/19-CONTEXT.md (D-LSYNC-1/2/3, D-DIR-1)
    - .planning/phases/19-doctor-status-surface-bugfix-bundle/19-RESEARCH.md "OBS-07 Rendering Specifics" (lines 308-484)
  </read_first>
  <behavior>
    - Test 1 (D-LSYNC-3 stamp ordering): `sync()` calls `manifest.stamp_last_synced_at()` immediately before the existing `manifest::save(...)` call inside the `if !dry_run && paths.config_dir().is_dir()` block. A dry-run sync does NOT stamp.
    - Test 2 (D-LSYNC-2 never rendering): `tome status` on a fresh TempDir without a manifest prints "Last sync: never" and JSON shape has `"last_sync": null`.
    - Test 3 (D-LSYNC-2 stamped rendering): after a successful `tome sync`, `tome status` prints "Last sync: <RFC-3339>" matching the manifest value; JSON `last_sync` is the same RFC-3339 string.
    - Test 4 (D-DIR-1 SKILLS column): `tome status` text Directories table has 5 columns with header "SKILLS"; the cell values match the `✓ N` or `✗ ?` pattern.
    - Test 5 (JSON parity preserved): `tome status --json` shape still includes `directories[].skill_count` (it already did pre-Phase-19) AND now includes top-level `last_sync` field.
  </behavior>
  <action>
    **Part A — `crates/tome/src/lib.rs::sync()` stamp call:**

    Locate the existing manifest save block (RESEARCH verified line 1779-1789, executor confirms by content):
    ```rust
    // 7. Save manifest, gitignore, and lockfile
    if !dry_run && paths.config_dir().is_dir() {
        manifest::save(&manifest, paths.config_dir())?;
        // ... gitignore + lockfile saves follow
    }
    ```

    Insert the stamp call immediately before `manifest::save`:
    ```rust
    // 7. Save manifest, gitignore, and lockfile
    if !dry_run && paths.config_dir().is_dir() {
        // D-LSYNC-3: stamp after distribute + cleanup succeed, before persist.
        manifest.stamp_last_synced_at();
        manifest::save(&manifest, paths.config_dir())?;
        // ... gitignore + lockfile saves follow
    }
    ```

    The stamp is INSIDE the `!dry_run` guard — dry-run does NOT update last_synced_at (correct per D-LSYNC-3). The stamp lands AFTER distribute (line 1710) + target cleanup (line 1755-1777) but BEFORE lockfile.save and the post-sync doctor health check.

    **Edge case decision per RESEARCH OQ-3 (`reconcile_install_failures` bail at :1878-1884):** The stamp fires AT the manifest save, which is BEFORE the install-failure bail. This is acceptable per D-LSYNC-3 wording ("after distribute + cleanup succeed"). Document this in a comment:
    ```rust
    // D-LSYNC-3: stamp after distribute + cleanup succeed, before persist.
    // Note: a subsequent reconcile-install-failure bail (`bail!` at the end
    // of sync()) still treats `last_synced_at` as stamped — the user-facing
    // semantics are "cleanup completed; install-failure exit is downstream."
    manifest.stamp_last_synced_at();
    ```

    **Part B — `crates/tome/src/status.rs::StatusReport` + gather() plumbing:**

    Add `last_sync` field to `StatusReport`:
    ```rust
    pub struct StatusReport {
        pub configured: bool,
        pub library_dir: PathBuf,
        pub library_count: CountOrError,
        /// RFC-3339 timestamp of last successful sync; `null` if never synced
        /// or pre-v0.11 manifest. Per D-LSYNC-2: "never" in text; null in JSON.
        pub last_sync: Option<String>,
        pub directories: Vec<DirectoryStatus>,
        pub unowned: Vec<crate::summary::SkillSummary>,
        pub health: CountOrError,
    }
    ```

    Per RESEARCH.md (line 469), do NOT add `#[serde(skip_serializing_if = "Option::is_none")]` on `last_sync` — emit `"last_sync": null` for stable-shape JSON consumers. This matches the `unowned: []` always-present pattern at status.rs:946-970.

    In `status::gather()`, load the manifest (already happens at line ~123 per RESEARCH) and thread `last_synced_at`:
    ```rust
    let last_sync = match manifest::load(paths.config_dir()) {
        Ok(m) => m.last_synced_at().map(String::from),
        Err(_) => None,
    };
    // ... build StatusReport { ..., last_sync, ... }
    ```

    **Part C — `render_status` text output: `Last sync:` line + 5-column Directories table:**

    After the existing `Library:` block (typically two `println!` calls — path + count line), add:
    ```rust
    // D-LSYNC-2: Last sync line. Reads from StatusReport.last_sync.
    let last_sync_str = match &report.last_sync {
        Some(ts) => ts.clone(),
        None => "never".to_string(),
    };
    println!("  {} {}", style("Last sync:").bold(), style(last_sync_str).cyan());
    println!();
    ```

    For the Directories table, change from 4 columns to 5. The current header is `["NAME", "TYPE", "ROLE", "PATH"]`; the new header is `["NAME", "TYPE", "ROLE", "PATH", "SKILLS"]`. For each row, append the existing CountOrError glyph rendering (which already exists in the JSON path):

    ```rust
    let mut rows: Vec<[String; 5]> = Vec::with_capacity(report.directories.len() + 1);
    rows.push([
        "NAME".to_string(),
        "TYPE".to_string(),
        "ROLE".to_string(),
        "PATH".to_string(),
        "SKILLS".to_string(),
    ]);
    for dir in &report.directories {
        let count = match (&dir.skill_count.count, &dir.skill_count.error) {
            (Some(n), _) => format!("✓ {}", n),
            (None, Some(_)) => "✗ ?".to_string(),
            (None, None) => "✓ 0".to_string(),
        };
        rows.push([
            dir.name.clone(),
            dir.directory_type.clone(),
            dir.role.clone(),
            format_dir_path_column(&dir.path, dir.override_applied),  // existing helper, preserved
            count,
        ]);
    }
    ```

    Per D-DIR-1 contract: the existing `(override)` annotation from PORT-05 is preserved (handled by the existing `format_dir_path_column` helper or equivalent). Column-width policy: NO `Width::*` setting added — same `Style::blank()` + header-bold pattern as today.

    Header is bare `SKILLS` (matching the brevity of NAME/TYPE/ROLE/PATH).

    **Part D — Integration tests in `crates/tome/tests/cli_status.rs`:**

    Create or extend `crates/tome/tests/cli_status.rs` (verify existence with `fd cli_status crates/tome/tests`; if exists, append to it; if not, create it). Add these tests:

    1. `status_last_sync_never_for_fresh_manifest` — TempDir, no manifest file. Run `tome status`. Assert stdout contains "Last sync: never".
    2. `status_last_sync_renders_after_sync` — TempDir, run `tome sync` first (with empty config to make sync trivially succeed), then `tome status`. Assert stdout contains "Last sync: 2026-" (year prefix — exact timestamp varies).
    3. `status_json_last_sync_null_for_fresh` — Run `tome status --json` on a fresh TempDir; parse via serde_json; assert `obj["last_sync"]` is `null`.
    4. `status_json_last_sync_string_after_sync` — Run sync then status --json; assert `obj["last_sync"]` is a String parseable as RFC-3339.
    5. `status_skills_column_present_in_text` — Add one directory to a tome.toml in TempDir, run sync, then status. Assert stdout contains "SKILLS" (the column header).

    These tests follow the pattern in existing `crates/tome/tests/cli.rs` (use `assert_cmd::Command::cargo_bin`, `tempfile::TempDir`, `std::fs::write` for seeding).
  </action>
  <verify>
    <automated>cargo test -p tome --lib status:: && cargo test -p tome --test cli_status</automated>
  </verify>
  <acceptance_criteria>
    - `rg "manifest\\.stamp_last_synced_at\\(\\)" crates/tome/src/lib.rs` returns 1 match
    - `rg "pub last_sync: Option<String>" crates/tome/src/status.rs` returns 1 match
    - `rg 'style\("Last sync:"\)' crates/tome/src/status.rs` returns 1 match (the bold styled text)
    - `rg '"SKILLS"' crates/tome/src/status.rs` returns at least 1 match (the new column header literal)
    - `rg '\[String; 5\]' crates/tome/src/status.rs` returns at least 1 match (the 5-column rows vector)
    - `crates/tome/tests/cli_status.rs` exists
    - `rg "status_last_sync_never_for_fresh_manifest" crates/tome/tests/cli_status.rs` returns 1 match
    - `rg "status_skills_column_present_in_text" crates/tome/tests/cli_status.rs` returns 1 match
    - `cargo test -p tome --lib status::` exits 0
    - `cargo test -p tome --test cli_status` exits 0 (5 new tests pass)
    - `cargo clippy --all-targets -- -D warnings` exits 0
    - Manual smoke: `cargo run -p tome -- status` (in repo root) prints a "Last sync:" line and a "SKILLS" column
  </acceptance_criteria>
  <done>sync() stamps last_synced_at; StatusReport has last_sync field; render_status prints Last sync: line + 5-column Directories table; 5 new integration tests pass.</done>
</task>

</tasks>

<verification>
- `cargo test -p tome --lib manifest::tests status::` — all unit tests pass
- `cargo test -p tome --test cli_status` — integration tests pass
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo fmt -- --check` — clean
- Manual smoke: `cargo run -p tome -- status --json | jq '.last_sync, .directories[0].skill_count'` returns valid JSON
- Schema-compat smoke: parse a v0.10-shape manifest JSON (`{"skills": {}}`) via `serde_json::from_str::<Manifest>` and confirm no error
</verification>

<success_criteria>
- OBS-07: `tome status` text has `Last sync: <RFC-3339>` (or `never`) + 5-column Directories table with SKILLS column
- OBS-07 JSON: `last_sync` at top level (`null` for never, RFC-3339 string otherwise); `directories[].skill_count` preserved
- Additive schema: pre-v0.11 manifests deserialize cleanly with `last_synced_at: None`
- Stamp ordering: occurs AFTER distribute + cleanup, BEFORE lockfile.save; dry-run does NOT stamp
- Test count delta: +9 (4 manifest unit tests + 5 integration tests)
</success_criteria>

<output>
After completion, create `.planning/phases/19-doctor-status-surface-bugfix-bundle/19-03-SUMMARY.md` documenting:
- Final placement of stamp_last_synced_at() in sync() (line number)
- Whether reconcile_install_failure bail ordering required additional handling beyond the comment
- Final Directories table column widths under realistic data (smoke output snippet)
- Confirmation that JSON `last_sync` emits as literal `null` for fresh manifests (not omitted)
</output>
