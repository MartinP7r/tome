---
phase: 16-cleanup-message-ux-docs
plan: 02
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/tome/src/migration_v010.rs
  - crates/tome/src/cli.rs
  - crates/tome/src/lib.rs
autonomous: true
requirements:
  - UX-02

must_haves:
  truths:
    - "`tome migrate-library` (non-dry-run, no `--yes`) prompts via `dialoguer::Confirm` defaulting to false; pressing anything other than `y` aborts cleanly without mutating the filesystem."
    - "`tome migrate-library --yes` skips the prompt and proceeds; `tome migrate-library --no-input` (without `--yes`) bails with a Conflict/Why/Suggestion error mentioning `--yes` (Phase 7 D-10 shape); `tome migrate-library --dry-run` always skips the prompt."
    - "Above the per-skill plan listing, `render_plan` emits a bold inline summary line of the form `Will convert N symlinks → real directories (~X.Y MB additional disk).` followed by a `tabled::Table` styled with `Style::rounded()` and four columns: SKILL, SOURCE, SIZE, STATUS."
    - "`MigrationEntry` carries a new `byte_size: Option<u64>` field — `Some(bytes)` when the source is reachable, `None` when broken — populated by a `walkdir` + `metadata().len()` walk during `migration_v010::plan` using `follow_links(false)`."
    - "Aborted migrations (user answers `n` to the prompt) leave the library state byte-for-byte unchanged: no symlinks removed, no new directories created. Verified by an integration test."
    - "`make ci` passes."
  artifacts:
    - path: "crates/tome/src/migration_v010.rs"
      provides: "`MigrationEntry.byte_size: Option<u64>`; `prompt_confirmation(yes, no_input) -> Result<bool>`; rewritten `render_plan` with `Style::rounded()` table; `humanize_bytes` helper; `render_result` lifted to `pub(crate)`."
      contains: "byte_size: Option<u64>"
    - path: "crates/tome/src/cli.rs"
      provides: "`Command::MigrateLibrary { dry_run: bool, yes: bool }` with `#[arg(long, short = 'y')]`."
      contains: "yes: bool"
    - path: "crates/tome/src/lib.rs"
      provides: "`cmd_migrate_library` rewritten: load manifest → plan → render_plan → confirm gate (unless dry_run) → execute → render_result."
  key_links:
    - from: "Command::MigrateLibrary in cli.rs"
      to: "cmd_migrate_library in lib.rs (around line 417)"
      via: "match arm dispatch"
      pattern: "Command::MigrateLibrary \\{ dry_run, yes \\}"
    - from: "cmd_migrate_library in lib.rs"
      to: "migration_v010::prompt_confirmation"
      via: "function call between render_plan and execute"
      pattern: "prompt_confirmation"
    - from: "migration_v010::plan"
      to: "byte_size walk via walkdir + metadata().len()"
      via: "per-entry size accumulation; follow_links(false)"
      pattern: "metadata\\(\\).*len\\(\\)"
---

<objective>
Add a confirm-or-abort gate to `tome migrate-library` per UX-02 and CONTEXT.md D-UX02-1..-4. Today `cmd_migrate_library` (lib.rs:982) calls `migration_v010::run_migrate_library` which renders the plan and immediately executes — there is no human gate. After this plan: `render_plan` is rewritten to surface a bold summary line + `tabled::Style::rounded()` summary table (SKILL | SOURCE | SIZE | STATUS), then `cmd_migrate_library` runs a `dialoguer::Confirm::default(false)` prompt before invoking `execute`. A `--yes` / `-y` flag bypasses the prompt (Phase 14 D-B3 pattern). `--no-input` without `--yes` bails with a Conflict/Why/Suggestion error (Phase 7 D-10). `--dry-run` always skips the prompt.

A new `byte_size: Option<u64>` field on `MigrationEntry` captures per-skill source size via a `walkdir::WalkDir::new(source).follow_links(false)` + `metadata().len()` walk during `plan()`. Total disk estimate goes into the summary line; per-skill values into the table SIZE column.

Purpose: closes UX-02. Migration is destructive and irreversible (per Phase 11 D-04 there is no `--undo-migrate`); the user deserves a deliberate confirmation step with full visibility into what will change before any conversion runs.

Output: `crates/tome/src/migration_v010.rs` (new field + new helper + rewritten render), `crates/tome/src/cli.rs` (new `--yes` arg), `crates/tome/src/lib.rs::cmd_migrate_library` (wire prompt + flag + bail).
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
@.planning/phases/16-cleanup-message-ux-docs/16-CONTEXT.md

@crates/tome/src/migration_v010.rs
@crates/tome/src/cli.rs
@crates/tome/src/lib.rs
@crates/tome/src/wizard.rs
@crates/tome/src/cleanup.rs
@crates/tome/src/remove.rs

<interfaces>
From crates/tome/src/migration_v010.rs (TODAY's shape, line 104-128):

```rust
pub(crate) struct MigrationEntry {
    pub skill_name: String,
    pub library_path: PathBuf,
    pub raw_link_target: PathBuf,
    pub source_reachable: bool,
}

pub(crate) struct MigrationPlan { pub entries: Vec<MigrationEntry> }

pub(crate) fn plan(library_dir: &Path, manifest: &Manifest) -> Result<MigrationPlan>;
pub(crate) fn render_plan(plan: &MigrationPlan);
pub(crate) fn execute(plan: &MigrationPlan, dry_run: bool) -> Result<MigrationResult>;
fn render_result(result: &MigrationResult);          // private — must lift to pub(crate)
pub(crate) fn run_migrate_library(paths: &TomePaths, dry_run: bool) -> Result<MigrationResult>;
```

From crates/tome/src/cli.rs (TODAY's shape, line 219-233):

```rust
MigrateLibrary {
    /// Preview changes without modifying filesystem
    #[arg(long)]
    dry_run: bool,
    // ADD: #[arg(long, short = 'y')] yes: bool,
},
```

From crates/tome/src/lib.rs (TODAY's dispatch line 417 + cmd_migrate_library line 982):

```rust
Command::MigrateLibrary { dry_run } => cmd_migrate_library(&paths, dry_run || cli.dry_run),

pub(crate) fn cmd_migrate_library(paths: &TomePaths, dry_run: bool) -> Result<()> {
    let result = migration_v010::run_migrate_library(paths, dry_run)?;
    if result.is_partial_or_failed() {
        anyhow::bail!(migration_v010::MigrationPartialOrFailed { ... });
    }
    Ok(())
}
```

`cli.no_input` is on the Cli root struct (line 102): `pub no_input: bool;`.

Phase 14 mirror — crates/tome/src/cli.rs line 246 (RemoveKind::Skill):

```rust
RemoveKind::Skill {
    name: String,
    #[arg(long, short = 'y')]
    yes: bool,
}
```

WHARD-07 tabled precedent in crates/tome/src/wizard.rs:

```rust
use tabled::{Table, settings::{Style, Width, peaker::PriorityMax}};
let table = Table::new(rows)
    .with(Style::rounded())
    .with(Width::truncate(term_width).priority(PriorityMax::right()))
    .to_string();
```

cleanup.rs:166-169 dialoguer pattern (mirror exactly):

```rust
let confirmed = dialoguer::Confirm::new()
    .with_prompt("Delete these skills from library?")
    .default(false)
    .interact_opt()?;
```

CONTEXT.md illustrative cmd_migrate_library shape after this plan:

```rust
pub(crate) fn cmd_migrate_library(
    paths: &TomePaths,
    dry_run: bool,
    yes: bool,
    no_input: bool,
) -> Result<()> {
    let manifest = manifest::load(paths.config_dir())?;
    let plan = migration_v010::plan(paths.library_dir(), &manifest)?;
    migration_v010::render_plan(&plan);  // emits summary line + tabled table
    if !dry_run {
        if !migration_v010::prompt_confirmation(yes, no_input)? {
            return Ok(());  // user said no; clean exit code 0
        }
    }
    let result = migration_v010::execute(&plan, dry_run)?;
    migration_v010::render_result(&result);  // requires pub(crate) lift
    if result.is_partial_or_failed() {
        anyhow::bail!(migration_v010::MigrationPartialOrFailed {
            skipped_broken_source: result.skipped_broken_source,
            failed: result.failed,
        });
    }
    Ok(())
}
```
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Add byte_size field + walk + tabled summary table to migration_v010</name>
  <files>crates/tome/src/migration_v010.rs</files>
  <read_first>
    - crates/tome/src/migration_v010.rs (entire file — `MigrationEntry` struct line 104-113, `plan()` line 170-201, `render_plan()` line 219-265, all unit tests at the bottom which MUST keep passing)
    - crates/tome/src/wizard.rs (WHARD-07 `Style::rounded()` + `Width::truncate(...).priority(PriorityMax::right())` precedent — mirror this for the migration table)
    - crates/tome/src/paths.rs (`collapse_home` function — reuse for SOURCE column rendering, mirrors today's render_plan line 258)
    - .planning/phases/16-cleanup-message-ux-docs/16-CONTEXT.md (D-UX02-3 summary format; D-UX02-4 byte_size walk semantics — `follow_links(false)` to avoid double-counting symlinked subdirs)
    - root Cargo.toml + crates/tome/Cargo.toml — confirm `tabled` is already a dependency (it is; used by wizard.rs)
  </read_first>
  <behavior>
    - Test 1 (existing must still pass): all 11 existing migration_v010 unit tests (`plan_detects_managed_symlinks_in_manifest`, `plan_handles_broken_symlink`, `execute_*`, `detect_v09_shape_*`, `migration_failure_kind_*`).
    - Test 2 (NEW): `plan_populates_byte_size_for_reachable_sources` — fixture with two managed skills (one with a single 1024-byte SKILL.md, one with SKILL.md + a 2048-byte data.txt). Plan output: entry[0].byte_size == Some(>=1024), entry[1].byte_size == Some(>=3072).
    - Test 3 (NEW): `plan_byte_size_is_none_for_broken_source` — fixture with broken managed symlink. Plan output: entry.byte_size == None.
    - Test 4 (NEW): `render_plan_to_writer_emits_summary_line_with_total_size` — refactor render_plan to take a writer (`pub(crate) fn render_plan_to(plan: &MigrationPlan, w: &mut impl std::io::Write)`) and assert the captured output matches `Will convert \d+ symlink` (regex via `regex` crate already a transitive test dep, OR plain `.contains()`) AND contains a size unit token (`B`, `KB`, `MB`, `GB`, `TB`).
    - Test 5 (NEW): `render_plan_table_has_four_column_headers` — assert the rendered output contains all four column header strings: `SKILL`, `SOURCE`, `SIZE`, `STATUS`.
  </behavior>
  <action>
    **Step 1: Extend `MigrationEntry` (line 104-113) with `byte_size: Option<u64>`:**
    ```rust
    #[derive(Debug, Clone)]
    pub(crate) struct MigrationEntry {
        pub skill_name: String,
        pub library_path: PathBuf,
        pub raw_link_target: PathBuf,
        pub source_reachable: bool,
        /// Sum of `metadata().len()` for every regular file under the resolved
        /// source. `Some(bytes)` when source_reachable; `None` when broken.
        /// Walks with `follow_links(false)` per D-UX02-4 to avoid double-
        /// counting nested symlinked subdirs.
        pub byte_size: Option<u64>,
    }
    ```

    **Step 2: Add a private `fn walk_byte_size(source: &Path) -> u64` helper:**
    ```rust
    fn walk_byte_size(source: &Path) -> u64 {
        let mut total: u64 = 0;
        for entry in walkdir::WalkDir::new(source).follow_links(false).into_iter().flatten() {
            if entry.file_type().is_file() {
                if let Ok(meta) = entry.metadata() {
                    total = total.saturating_add(meta.len());
                }
            }
        }
        total
    }
    ```

    **Step 3: Populate `byte_size` in `plan()` (line 170-201).** After computing `source_reachable` (line 190), call `walk_byte_size(&library_path)` (which follows the symlink to the real source for reachable entries) and gate on `source_reachable`:
    ```rust
    let byte_size = if source_reachable {
        Some(walk_byte_size(&library_path))
    } else {
        None
    };
    entries.push(MigrationEntry {
        skill_name: skill_name.as_str().to_string(),
        library_path,
        raw_link_target: raw_target,
        source_reachable,
        byte_size,
    });
    ```

    **Step 4: Add `fn humanize_bytes(bytes: u64) -> String`** as a private helper near the top of migration_v010.rs:
    ```rust
    fn humanize_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut value = bytes as f64;
        let mut unit_idx = 0;
        while value >= 1024.0 && unit_idx < UNITS.len() - 1 {
            value /= 1024.0;
            unit_idx += 1;
        }
        if unit_idx == 0 {
            format!("{} {}", bytes, UNITS[0])
        } else {
            format!("{:.1} {}", value, UNITS[unit_idx])
        }
    }
    ```
    Inline helper preferred over `humansize` crate per CONTEXT.md `<decisions>` "Claude's Discretion" — minimizes deps.

    **Step 5: Refactor `render_plan` into a writer-based variant for testability:**
    ```rust
    pub(crate) fn render_plan(plan: &MigrationPlan) {
        // Adapter: writes to stdout via println! to keep the existing
        // call-site behavior (interactive UX). render_plan_to is used in
        // tests for output capture.
        let mut buf = Vec::new();
        let _ = render_plan_to(plan, &mut buf);
        // Stream the captured output to stdout — preserves today's user
        // experience.
        if let Ok(s) = std::str::from_utf8(&buf) {
            print!("{}", s);
        }
    }

    pub(crate) fn render_plan_to(
        plan: &MigrationPlan,
        w: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        writeln!(w, "{}", style("v0.9 → v0.10 library migration plan").bold())?;
        writeln!(w)?;
        if plan.entries.is_empty() {
            writeln!(
                w,
                "  {} no v0.9-shape entries detected — library is already in v0.10 shape.",
                style("✓").green()
            )?;
            return Ok(());
        }

        let convertible = plan.entries.iter().filter(|e| e.source_reachable).count();
        let broken = plan.entries.len() - convertible;
        let total_bytes: u64 = plan.entries.iter()
            .filter(|e| e.source_reachable)
            .filter_map(|e| e.byte_size)
            .sum();

        // Bold inline summary line per D-UX02-3.
        writeln!(
            w,
            "  {}",
            style(format!(
                "Will convert {} symlink{} → real director{} (~{} additional disk).",
                convertible,
                if convertible == 1 { "" } else { "s" },
                if convertible == 1 { "y" } else { "ies" },
                humanize_bytes(total_bytes),
            )).bold()
        )?;
        if broken > 0 {
            writeln!(
                w,
                "  {} {} broken symlink{} will be SKIPPED and preserved (manual fix required).",
                style("⚠").yellow(),
                style(broken).bold(),
                if broken == 1 { "" } else { "s" }
            )?;
        }
        writeln!(w)?;

        // Tabled summary table per D-UX02-3 — Style::rounded() per WHARD-07.
        use tabled::{Table, settings::Style};
        #[derive(tabled::Tabled)]
        struct Row {
            #[tabled(rename = "SKILL")] skill: String,
            #[tabled(rename = "SOURCE")] source: String,
            #[tabled(rename = "SIZE")] size: String,
            #[tabled(rename = "STATUS")] status: String,
        }
        let rows: Vec<Row> = plan.entries.iter().map(|e| Row {
            skill: e.skill_name.clone(),
            source: collapse_home(&e.raw_link_target).to_string(),
            size: e.byte_size.map(humanize_bytes).unwrap_or_else(|| "—".into()),
            status: if e.source_reachable { "✓".into() } else { "⚠".into() },
        }).collect();
        let mut t = Table::new(rows);
        t.with(Style::rounded());
        writeln!(w, "{}", t)?;
        writeln!(w)?;
        writeln!(w, "  Note: tome does not snapshot your library before migrating. Commit your")?;
        writeln!(w, "  library directory to git (or back it up some other way) BEFORE proceeding.")?;
        writeln!(w, "  This conversion is one-way — there is no path back to v0.9 shape.")?;
        Ok(())
    }
    ```

    **Step 6: Lift `fn render_result` (line 400) to `pub(crate) fn render_result`** so `cmd_migrate_library` in lib.rs (Task 3) can call it.

    **Step 7: Drop `pub(crate) fn run_migrate_library` (line 453-468).** The wrapper is replaced by lib.rs's rewritten `cmd_migrate_library` doing plan/render/prompt/execute/render explicitly. Remove or update any callers — `rg -n 'run_migrate_library' crates/` should return zero hits after Task 3 lands.
  </action>
  <verify>
    <automated>cargo test -p tome --lib migration_v010</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n 'pub byte_size: Option<u64>' crates/tome/src/migration_v010.rs` outputs at least one match
    - `rg -n 'fn walk_byte_size|fn humanize_bytes' crates/tome/src/migration_v010.rs` outputs at least two matches
    - `rg -n 'Style::rounded' crates/tome/src/migration_v010.rs` outputs at least one match
    - `rg -n 'pub\(crate\) fn render_result' crates/tome/src/migration_v010.rs` outputs one match
    - `rg -n 'pub\(crate\) fn render_plan_to' crates/tome/src/migration_v010.rs` outputs one match
    - `cargo test -p tome --lib migration_v010` exits 0
    - `cargo clippy -p tome -- -D warnings` exits 0
  </acceptance_criteria>
  <done>
    `MigrationEntry.byte_size: Option<u64>` populated by `plan()` via a `follow_links(false)` walk; `render_plan` rewritten as a thin wrapper around `render_plan_to(writer)` for testability; the rendered output now contains a bold summary line plus a `Style::rounded()` table with SKILL/SOURCE/SIZE/STATUS columns; `render_result` lifted to `pub(crate)`. All existing tests pass; new tests pin the summary-line + four-column-header invariants.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Add prompt_confirmation helper + --yes flag in cli.rs</name>
  <files>crates/tome/src/migration_v010.rs, crates/tome/src/cli.rs</files>
  <read_first>
    - crates/tome/src/migration_v010.rs (the file from Task 1; this task adds `prompt_confirmation`)
    - crates/tome/src/cleanup.rs:166-169 (`dialoguer::Confirm::default(false)` precedent — mirror exactly)
    - crates/tome/src/cli.rs (lines 219-233 — current `MigrateLibrary` definition; lines 246 + RemoveKind::Skill — mirror the `--yes` flag wiring)
    - .planning/phases/16-cleanup-message-ux-docs/16-CONTEXT.md (D-UX02-1, D-UX02-2 — confirm-default-false; --yes / -y flag; --no-input without --yes bails per Phase 7 D-10)
  </read_first>
  <behavior>
    - Test 1 (NEW migration_v010.rs unit test): `prompt_confirmation_returns_true_when_yes_flag_set` — `prompt_confirmation(true, false)` returns `Ok(true)` without invoking dialoguer.
    - Test 2 (NEW): `prompt_confirmation_bails_when_no_input_without_yes` — `prompt_confirmation(false, true)` returns `Err(_)`; the error message contains substrings "destructive", "--yes", and "--no-input".
    - Test 3 (NEW cli.rs unit test): `migrate_library_parses_yes_flag` — `Cli::try_parse_from(["tome", "migrate-library", "--yes"])` succeeds with parsed `MigrateLibrary { yes: true, .. }`.
    - Test 4 (NEW): `migrate_library_short_y_alias` — `Cli::try_parse_from(["tome", "migrate-library", "-y"])` succeeds with `yes: true`.
    - Test 5 (NEW): `migrate_library_yes_default_false` — without `--yes`, parsed `MigrateLibrary { yes: false, .. }`.
  </behavior>
  <action>
    **Step 1: Add `pub(crate) fn prompt_confirmation` to migration_v010.rs.** Place it directly above `render_result`. Implementation:
    ```rust
    /// Confirm-or-abort gate before destructive migration.
    ///
    /// Returns Ok(true) to proceed, Ok(false) to abort cleanly (user said no).
    /// Returns Err per Phase 7 D-10 Conflict/Why/Suggestion when --no-input is
    /// set without --yes (CI/non-interactive runs MUST opt in explicitly).
    ///
    /// Behavior matrix:
    /// - yes=true, no_input=*    -> Ok(true) (skip prompt; CI-friendly)
    /// - yes=false, no_input=true -> Err(...) (refuses to silently mutate)
    /// - yes=false, no_input=false -> dialoguer::Confirm::default(false)
    pub(crate) fn prompt_confirmation(yes: bool, no_input: bool) -> Result<bool> {
        if yes {
            return Ok(true);
        }
        if no_input {
            anyhow::bail!(
                "tome migrate-library is destructive (converts symlinks to real copies).\n  \
                 Why: --no-input mode skips the confirmation prompt; --yes is required to \
                 confirm.\n  \
                 Suggestion: re-run with `--yes` to proceed, or remove `--no-input` for the \
                 interactive prompt."
            );
        }
        let confirmed = dialoguer::Confirm::new()
            .with_prompt("Proceed with migration?")
            .default(false)
            .interact_opt()?;
        Ok(confirmed.unwrap_or(false))
    }
    ```

    **Step 2: Add unit tests for `prompt_confirmation` in migration_v010.rs `mod tests`.** The yes=true and no_input=true paths don't touch dialoguer so they're fully unit-testable. The interactive path is intentionally NOT tested here per RESEARCH Pitfall 6 (interactive prompts are covered by manual smoke + Task 3's integration test).

    **Step 3: Update `Command::MigrateLibrary` in cli.rs (line 219-233).** Add the `yes` field, mirroring `RemoveKind::Skill` (cli.rs around line 246):
    ```rust
    /// One-shot migration: convert a v0.9-shape library (managed skills as
    /// symlinks) to v0.10 shape (real directory copies). Run once after
    /// upgrading from v0.9.x. Idempotent on re-run.
    ///
    /// Commit your library (or back it up) BEFORE running — there is no
    /// path back to v0.9 shape.
    #[command(
        after_help = "Examples:\n  tome migrate-library --dry-run\n  tome migrate-library\n  tome migrate-library --yes\n\nThis is a one-shot command for migrating from tome v0.9.x to v0.10. \
                       On v0.10 fresh installs it has nothing to do."
    )]
    MigrateLibrary {
        /// Preview changes without modifying filesystem
        #[arg(long)]
        dry_run: bool,
        /// Skip the confirmation prompt and proceed directly. Mirrors
        /// `tome remove skill --yes` (Phase 14 D-B3). Required when running
        /// under `--no-input` to confirm the destructive conversion.
        #[arg(long, short = 'y')]
        yes: bool,
    },
    ```

    **Step 4: Add the cli.rs unit tests (Tests 3-5)** at the bottom of cli.rs. Phase 14 added similar `Cli::try_parse_from` tests for `Remove`/`Reassign` — search `rg -n 'try_parse_from' crates/tome/src/cli.rs` for existing patterns and mirror exactly.
  </action>
  <verify>
    <automated>cargo test -p tome --lib migration_v010 cli</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n 'pub\(crate\) fn prompt_confirmation' crates/tome/src/migration_v010.rs` outputs one match
    - `rg -n 'short = .y.' crates/tome/src/cli.rs | rg -i 'migrate'` shows the new -y short alias on MigrateLibrary (alternatively, search for "MigrateLibrary" then "yes: bool" with `#[arg(long, short = 'y')]`)
    - `rg -n 'yes: bool' crates/tome/src/cli.rs | rg -i 'migrate' || rg -n -A 2 'MigrateLibrary' crates/tome/src/cli.rs | rg 'yes: bool'` confirms the field is on `MigrateLibrary`
    - `cargo test -p tome --lib migration_v010::tests::prompt_confirmation` exits 0 (both yes=true and no_input-bails tests)
    - `cargo test -p tome --lib cli::tests::migrate_library` exits 0 (all three clap-parse tests)
    - `cargo clippy -p tome -- -D warnings` exits 0
  </acceptance_criteria>
  <done>
    `migration_v010::prompt_confirmation` exists with the three-arm behavior matrix (yes / no_input bail / interactive default-false). `Command::MigrateLibrary` carries the new `yes: bool` field with `--yes`/`-y` clap wiring and updated `after_help`. Unit tests pin all four behavioral arms and the clap parsing surface.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Wire prompt + flag through cmd_migrate_library in lib.rs + integration test for abort path</name>
  <files>crates/tome/src/lib.rs, crates/tome/tests/cli_migrate.rs</files>
  <read_first>
    - crates/tome/src/lib.rs (line 417 — match arm for `Command::MigrateLibrary`; line 982-993 — current `cmd_migrate_library` body; line 982's call to `migration_v010::run_migrate_library`)
    - crates/tome/src/migration_v010.rs (after Task 1 + Task 2: `plan`, `render_plan`, `prompt_confirmation`, `execute`, `render_result`, `MigrationPartialOrFailed`)
    - crates/tome/tests/cli_migrate.rs (existing integration tests for migrate-library — search for the file with `fd cli_migrate`; if it doesn't exist, create it and follow the cli_*.rs pattern from Phase 15 HARD-13 split)
    - .planning/phases/16-cleanup-message-ux-docs/16-CONTEXT.md (D-UX02-1, D-UX02-2 behavior matrix; integration test "aborted migrations leave the library state byte-for-byte unchanged" requirement)
  </read_first>
  <behavior>
    - Test 1 (NEW integration test in tests/cli_migrate.rs): `migrate_library_dry_run_does_not_prompt` — `tome migrate-library --dry-run --no-input` against a fixture with one v0.9-shape symlink succeeds (exit 0); fixture symlink is preserved on disk.
    - Test 2 (NEW): `migrate_library_no_input_without_yes_bails` — `tome migrate-library --no-input` (without `--yes`, without `--dry-run`) exits non-zero; stderr contains the substrings "destructive", "--yes", and "--no-input"; fixture symlink is preserved on disk.
    - Test 3 (NEW): `migrate_library_yes_skips_prompt` — `tome migrate-library --yes --no-input` against a v0.9-shape fixture exits 0; the fixture symlink is converted to a real directory.
    - Test 4 (existing must still pass): any pre-existing migrate-library integration test from Phase 11 Plan 11-05 / Phase 15 HARD-13 split (search `rg -n 'migrate' crates/tome/tests/`).
  </behavior>
  <action>
    **Step 1: Update the dispatch arm in lib.rs (around line 417):**
    ```rust
    Command::MigrateLibrary { dry_run, yes } => {
        cmd_migrate_library(&paths, dry_run || cli.dry_run, yes, cli.no_input)
    }
    ```

    **Step 2: Rewrite `cmd_migrate_library` (lib.rs:982-993)** to drive the plan/render/prompt/execute pipeline directly per CONTEXT.md `<code_context>` shape:
    ```rust
    /// `tome migrate-library` — one-shot v0.9 → v0.10 library migration.
    /// Per D-05: any skip or failure means non-zero exit.
    /// Per UX-02 / D-UX02-1..-4: prompts before any conversion unless `yes`
    /// or `dry_run` set; bails under `--no-input` without `--yes`.
    pub(crate) fn cmd_migrate_library(
        paths: &TomePaths,
        dry_run: bool,
        yes: bool,
        no_input: bool,
    ) -> Result<()> {
        if dry_run {
            eprintln!(
                "{}",
                console::style("[dry-run] No changes will be made")
                    .yellow()
                    .bold()
            );
        }
        let manifest = manifest::load(paths.config_dir())?;
        let plan = migration_v010::plan(paths.library_dir(), &manifest)?;
        migration_v010::render_plan(&plan);

        // No conversion to do — render_plan already showed the empty-state line.
        if plan.entries.is_empty() {
            return Ok(());
        }

        if !dry_run {
            // UX-02: confirm-or-abort gate. May Err per Phase 7 D-10 when
            // --no-input is set without --yes.
            if !migration_v010::prompt_confirmation(yes, no_input)? {
                // User said no — clean exit, no mutation.
                return Ok(());
            }
        }

        let result = migration_v010::execute(&plan, dry_run)?;
        migration_v010::render_result(&result);

        // HARD-04 sibling: bubble through anyhow rather than `process::exit(1)`.
        if result.is_partial_or_failed() {
            anyhow::bail!(migration_v010::MigrationPartialOrFailed {
                skipped_broken_source: result.skipped_broken_source,
                failed: result.failed,
            });
        }
        Ok(())
    }
    ```

    **Step 3: Confirm `migration_v010::run_migrate_library` is no longer referenced in lib.rs** (or anywhere else after Task 1's deletion). Run `rg -n 'run_migrate_library' crates/`.

    **Step 4: Add three integration tests to crates/tome/tests/cli_migrate.rs.** If the file doesn't exist, create it with the standard test scaffolding (mirror `tests/cli_remove.rs` structure from Phase 15 HARD-13). Each test builds a v0.9-shape fixture (manifest with `managed: true` + a symlink in the library pointing at a real source dir) and invokes `assert_cmd::Command::cargo_bin("tome")`:

    Test 1 — `migrate_library_dry_run_does_not_prompt`:
    ```rust
    cmd.args(["migrate-library", "--dry-run", "--no-input"])
       .env("HOME", &tmp_home).env("TOME_HOME", &tome_home_path)
       .assert().success();
    assert!(library.join("p1").is_symlink());  // unchanged
    ```

    Test 2 — `migrate_library_no_input_without_yes_bails`:
    ```rust
    cmd.args(["migrate-library", "--no-input"])
       .assert().failure()
       .stderr(predicates::str::contains("destructive"))
       .stderr(predicates::str::contains("--yes"))
       .stderr(predicates::str::contains("--no-input"));
    assert!(library.join("p1").is_symlink());  // unchanged — no mutation on bail
    ```

    Test 3 — `migrate_library_yes_skips_prompt`:
    ```rust
    cmd.args(["migrate-library", "--yes", "--no-input"])
       .assert().success();
    assert!(!library.join("p1").is_symlink());  // converted
    assert!(library.join("p1").is_dir());
    ```

    For fixture setup, mirror `migration_v010::tests::add_managed_entry` (line 497) — that pattern is well-tested. The integration test fixture needs to write a real `.tome-manifest.json` to `tome_home_path/.tome-manifest.json` because `manifest::load` is called from production code (not the test helper that takes an in-memory Manifest).
  </action>
  <verify>
    <automated>cargo test -p tome --test cli_migrate &amp;&amp; cargo build -p tome</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n 'cmd_migrate_library\(' crates/tome/src/lib.rs` shows the four-parameter signature `(paths: &TomePaths, dry_run: bool, yes: bool, no_input: bool)`
    - `rg -n 'prompt_confirmation' crates/tome/src/lib.rs` outputs one match (invoked from cmd_migrate_library)
    - `rg -n 'run_migrate_library' crates/tome/src/` outputs zero matches (wrapper removed)
    - `rg -n 'Command::MigrateLibrary \{ dry_run, yes \}' crates/tome/src/lib.rs` outputs one match (or equivalent destructuring)
    - `cargo test -p tome --test cli_migrate migrate_library_dry_run_does_not_prompt` exits 0
    - `cargo test -p tome --test cli_migrate migrate_library_no_input_without_yes_bails` exits 0
    - `cargo test -p tome --test cli_migrate migrate_library_yes_skips_prompt` exits 0
    - `cargo build -p tome` exits 0
    - `cargo clippy -p tome --all-targets -- -D warnings` exits 0
    - `make ci` exits 0
  </acceptance_criteria>
  <done>
    `cmd_migrate_library` drives plan → render_plan → prompt_confirmation → execute → render_result with full UX-02 semantics. `--dry-run` skips the prompt; `--yes` bypasses; `--no-input` without `--yes` bails with a Conflict/Why/Suggestion error. Three integration tests anchor the three behavioral arms; the abort-leaves-library-untouched invariant is verified. `run_migrate_library` is gone — there's one canonical entry point.
  </done>
</task>

</tasks>

<verification>
- `cargo test -p tome --lib migration_v010` — all 11 existing + 5 new unit tests pass
- `cargo test -p tome --lib cli` — three new clap-parse tests pass
- `cargo test -p tome --test cli_migrate` — three new integration tests pass
- `make ci` (fmt-check + clippy -D warnings + tests) passes
- `cargo run -p tome -- migrate-library --help` shows `--yes` and `-y` listed in help text
- Manual smoke: `cargo run -p tome -- migrate-library` against a v0.9 fixture shows summary line + tabled table + dialoguer prompt; pressing `n` exits clean with no mutation
</verification>

<success_criteria>
- UX-02 satisfied: `tome migrate-library` renders summary table + count + disk estimate before any conversion runs; user confirms or aborts via `dialoguer::Confirm::default(false)` (or bypasses with `--yes`)
- D-UX02-1: confirm defaults to no — pressing Enter or anything other than `y` aborts cleanly
- D-UX02-2: `--yes` flag bypasses; `--no-input` without `--yes` bails with Phase 7 D-10 Conflict/Why/Suggestion shape; `--dry-run` always skips the prompt
- D-UX02-3: bold inline summary line + `tabled::Style::rounded()` table with SKILL/SOURCE/SIZE/STATUS columns
- D-UX02-4: `MigrationEntry.byte_size: Option<u64>` populated via `walkdir + metadata().len()` walk with `follow_links(false)`
- LIB-05 / Phase 11 D-05 partial-or-failed exit semantics preserved: `MigrationPartialOrFailed` still bubbles through `anyhow::bail!`
- Aborted migrations leave the library byte-for-byte unchanged (anchored by integration test)
</success_criteria>

<output>
After completion, create `.planning/phases/16-cleanup-message-ux-docs/16-02-SUMMARY.md` documenting:
- Final wording of the bold summary line ("Will convert N symlinks → real directories (~X.Y MB additional disk).")
- Final wording of the Phase 7 D-10 bail message for `--no-input` without `--yes`
- Whether `humansize` crate or inline `humanize_bytes` was chosen and why
- Whether `run_migrate_library` was deleted or kept (Task 1 Step 7 decision)
- Any deviations from the four-column SKILL/SOURCE/SIZE/STATUS layout (e.g. truncation policy decisions)
- Any DOC-02 wording the executor commits to (e.g. "the migration prompt defaults to no" — DOC-02 in Wave 2 cites this language)
</output>
