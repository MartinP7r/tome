---
phase: 16-cleanup-message-ux-docs
plan: 02
subsystem: cli-ux
tags: [migrate-library, confirm-gate, dialoguer, tabled, ux, destructive-command]

# Dependency graph
requires:
  - phase: 11-library-canonical-core
    provides: LIB-05 `tome migrate-library` one-shot CLI + `migration_v010` module (plan/render_plan/execute/render_result primitives + MigrationPartialOrFailed bail)
  - phase: 14-unowned-library-lifecycle
    provides: D-B3 `--yes` / `-y` flag pattern (mirrored from `tome remove skill --yes`); confirmation-default-false convention
  - phase: 15-cli-hardening
    provides: HARD-04 typed-error bail pattern (MigrationPartialOrFailed bubbles via anyhow); WHARD-07 `tabled::Style::rounded()` ceremonial-summary precedent
provides:
  - `migration_v010::MigrationEntry.byte_size: Option<u64>` populated via `walkdir::WalkDir::follow_links(false)` walk in `plan()`
  - `migration_v010::render_plan_to(plan, &mut impl Write)` writer-based pure renderer + thin `render_plan(plan)` stdout wrapper (testable via Vec<u8>)
  - `migration_v010::prompt_confirmation(yes, no_input) -> Result<bool>` three-arm gate (UX-02 D-UX02-1/-2)
  - `migration_v010::render_result` lifted to `pub(crate)` so `cmd_migrate_library` can compose the flow directly
  - `migration_v010::humanize_bytes(u64) -> String` private helper (B / KB / MB / GB / TB)
  - `Command::MigrateLibrary { dry_run, yes }` with `--yes` / `-y` flag mirroring Phase 14 D-B3
  - `cmd_migrate_library(paths, dry_run, yes, no_input) -> Result<()>` rewritten to drive plan / render_plan / prompt_confirmation / execute / render_result directly
affects: [16-03, 16-04, 16-05]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Writer-based pure renderer + thin stdout/stderr wrapper (`render_plan_to(writer)` + `render_plan` adapter that prints to stdout via the buffer) — testable via `Vec<u8>` without stdout capture. Same shape as Plan 16-01's `render_cleanup_buckets` and Phase 12 ADP-04's `format_install_failures`/`render_install_failures` split."
    - "Three-arm confirm-gate behaviour matrix (yes-bypass / no-input-bails / interactive-default-false). Phase 14 D-B3 `tome remove skill --yes` mirrored exactly, including `#[arg(long, short = 'y')]`. The bail message follows the Phase 7 D-10 Conflict/Why/Suggestion shape."
    - "Inline `humanize_bytes` helper rather than the `humansize` crate — minimises dep growth for ~10 LOC. Documented in CONTEXT.md `<decisions>` Claude's Discretion."

key-files:
  created: []
  modified:
    - "crates/tome/src/migration_v010.rs — `MigrationEntry.byte_size` field + `walk_byte_size` + `humanize_bytes` helpers; `render_plan` rewritten as a thin wrapper around new `render_plan_to(writer)` with bold summary line + `tabled::Style::rounded()` four-column SKILL/SOURCE/SIZE/STATUS table; `render_result` lifted to `pub(crate)`; `prompt_confirmation` added; `run_migrate_library` deleted; 8 new unit tests (5 byte_size/render + 3 prompt_confirmation)"
    - "crates/tome/src/cli.rs — `Command::MigrateLibrary` gains `yes: bool` field with `#[arg(long, short = 'y')]`; updated after_help advertises `--yes`; 4 new clap-parse unit tests"
    - "crates/tome/src/lib.rs — `cmd_migrate_library` rewritten with new four-parameter signature (paths, dry_run, yes, no_input); drives plan → render_plan → confirm-gate → execute → render_result directly; dispatcher threads `cli.no_input` and `yes` through"
    - "crates/tome/tests/cli_migrate_library.rs — 3 new integration tests pin the UX-02 behaviour matrix (`migrate_library_dry_run_does_not_prompt`, `migrate_library_no_input_without_yes_bails`, `migrate_library_yes_skips_prompt`); 2 pre-existing tests updated to pass `--yes` (assert_cmd is non-TTY and cannot answer the dialoguer prompt)"

key-decisions:
  - "Final summary-line wording (DOC-02 cites this verbatim): `Will convert N symlink(s) → real director{y|ies} (~X.Y UNIT additional disk).` Plurals computed from `convertible == 1`. Total computed via `entries.iter().filter(|e| e.source_reachable).filter_map(|e| e.byte_size).sum()`. The four columns of the tabled summary are SKILL | SOURCE | SIZE | STATUS, exactly as specified by D-UX02-3."
  - "Final `--no-input` without `--yes` bail wording (Phase 7 D-10 shape): `tome migrate-library is destructive (converts symlinks to real copies). Why: --no-input mode skips the confirmation prompt; --yes is required to confirm. Suggestion: re-run with --yes to proceed, or remove --no-input for the interactive prompt.` Single `anyhow::bail!` invocation; no separate template helper. Verified by integration test substrings (`destructive`, `--yes`, `--no-input`)."
  - "Inline `humanize_bytes` chosen over the `humansize` crate — fewer deps, ~10 LOC, exact format `{bytes} B` for sub-KB and `{value:.1} {unit}` for KB and up. Confirmed via `humanize_bytes_unit_promotion` unit test."
  - "`run_migrate_library` deleted (Plan Task 1 Step 7 + Task 3). The wrapper is replaced by `cmd_migrate_library` in lib.rs composing the migration_v010 primitives directly. One canonical entry point. Across the Task 1 / Task 2 / Task 3 commit boundary it was kept temporarily for build-greenness; Task 3's commit removes it cleanly."
  - "Four-column SKILL/SOURCE/SIZE/STATUS layout shipped exactly as specified — no truncation policy implemented (CONTEXT.md `<deferred>` flagged truncation for >100 entries as Claude's discretion; current real-world libraries are well below that threshold so no `Width::truncate` was added; if needed, mirror WHARD-07's `Width::truncate(term).priority(PriorityMax::right())` pattern in a future plan)."
  - "Status column uses a single glyph (✓ for reachable, ⚠ for broken) rather than a verb — keeps the column narrow and matches today's render_plan inline marker (line 252-253 pre-rewrite)."
  - "DOC-02 vocabulary commitments — the CHANGELOG.md v0.10 entry can cite `the migration prompt defaults to no` and reference the Phase 14 `--yes` pattern. Specific wording for DOC-02 to honour: `tome migrate-library` (interactive) prompts `Proceed with migration?` defaulting to no; pressing anything other than `y` aborts cleanly without mutating the filesystem."
  - "Existing pre-Task-3 integration tests (`migrate_library_converts_managed_symlinks_to_real_dirs`, `sync_succeeds_after_migrate_library`) updated to pass `--yes` since `assert_cmd::Command` is non-TTY — the dialoguer prompt fails with `error: IO error: not a terminal` otherwise. New behaviour-pinning tests (`migrate_library_dry_run_does_not_prompt`, `migrate_library_no_input_without_yes_bails`, `migrate_library_yes_skips_prompt`) anchor the three UX-02 arms."

patterns-established:
  - "Confirm-gate composition pattern for destructive one-shot commands: plan → render → `prompt_confirmation(yes, no_input)` → execute → render_result. Mirrors the Phase 14 D-B3 shape and is reusable for future destructive flows."
  - "Inline `humanize_bytes` (B → KB → MB → GB → TB with binary 1024-step promotion) — pure helper, no crate. Available for future UX surfaces that need byte-count display without pulling in `humansize`."

requirements-completed: [UX-02]

# Metrics
duration: 10min
completed: 2026-05-08
---

# Phase 16 Plan 02: Migrate-library confirm gate + summary table Summary

UX-02 ships: `tome migrate-library` now renders a bold summary line + tabled SKILL/SOURCE/SIZE/STATUS plan and gates conversion behind a `dialoguer::Confirm::default(false)` prompt. `--yes` / `-y` skips; `--no-input` without `--yes` bails with a Phase 7 D-10 Conflict/Why/Suggestion error; `--dry-run` always skips the prompt. `MigrationEntry` carries a new `byte_size: Option<u64>` populated via a `walkdir + metadata().len()` walk during `plan()`.

## What Changed

### `crates/tome/src/migration_v010.rs`

- New `byte_size: Option<u64>` field on `MigrationEntry`, populated in `plan()` by a `walk_byte_size(library_path)` call that uses `walkdir::WalkDir::follow_links(false)` per D-UX02-4 (avoids double-counting nested symlinked subdirs). `Some(bytes)` for reachable sources; `None` for broken symlinks.
- `render_plan` refactored: the existing function becomes a thin wrapper that prints to stdout via the buffer; the new `pub(crate) fn render_plan_to(plan: &MigrationPlan, w: &mut impl std::io::Write) -> std::io::Result<()>` carries the actual rendering logic so tests can capture output. The new render emits:
  1. Bold "v0.9 → v0.10 library migration plan" header.
  2. Bold inline summary line: `Will convert N symlink(s) → real director{y|ies} (~X.Y UNIT additional disk).` (DOC-02 vocabulary commitment).
  3. Optional broken-symlink count line.
  4. `tabled::Style::rounded()` four-column table: SKILL | SOURCE | SIZE | STATUS. Status glyph is ✓ (reachable) or ⚠ (broken); SIZE column displays `humanize_bytes(Some)` or `—` for None.
  5. The pre-existing closing note about commit-before-migrate / one-way conversion.
- `pub(crate) fn prompt_confirmation(yes: bool, no_input: bool) -> Result<bool>` added with the three-arm matrix:
  - `yes=true` → `Ok(true)` (skip prompt; CI-friendly).
  - `yes=false, no_input=true` → `Err` with Phase 7 D-10 Conflict/Why/Suggestion bail message.
  - `yes=false, no_input=false` → `dialoguer::Confirm::new().with_prompt("Proceed with migration?").default(false).interact_opt()?` — pressing anything other than `y` aborts.
- `render_result` lifted from private `fn` to `pub(crate) fn` so the rewritten `cmd_migrate_library` can compose it directly.
- New private helpers: `walk_byte_size(&Path) -> u64` and `humanize_bytes(u64) -> String` (B / KB / MB / GB / TB binary-step promotion).
- `run_migrate_library` deleted — `cmd_migrate_library` now drives the flow via this module's primitives.

### `crates/tome/src/cli.rs`

- `Command::MigrateLibrary` gains a `yes: bool` field with `#[arg(long, short = 'y')]` mirroring `RemoveKind::Skill`'s shape (Phase 14 D-B3). The `after_help` block now advertises `tome migrate-library --yes` as an example.
- 4 new clap-parse unit tests pin `--yes`, `-y` short alias, default-false, and `--dry-run --yes` composition.

### `crates/tome/src/lib.rs`

- Dispatcher arm for `Command::MigrateLibrary` now destructures `{ dry_run, yes }` and calls `cmd_migrate_library(&paths, dry_run || cli.dry_run, yes, cli.no_input)`.
- `cmd_migrate_library` rewritten with a four-parameter signature `(paths, dry_run, yes, no_input)`. Body drives:
  1. `[dry-run]` banner (when `dry_run`).
  2. `manifest::load(paths.config_dir())?`.
  3. `migration_v010::plan(paths.library_dir(), &manifest)?`.
  4. `migration_v010::render_plan(&plan)` — surfaces the new summary line + tabled table.
  5. Empty-plan early-return (render_plan already printed the already-in-v0.10-shape message).
  6. Confirm gate: `if !dry_run { if !migration_v010::prompt_confirmation(yes, no_input)? { return Ok(()); } }`.
  7. `migration_v010::execute(&plan, dry_run)?`.
  8. `migration_v010::render_result(&result)`.
  9. Partial-or-failed bail via `MigrationPartialOrFailed` (HARD-04 sibling — no `process::exit(1)`).

### `crates/tome/tests/cli_migrate_library.rs`

- 3 new integration tests pin the UX-02 behaviour matrix:
  - `migrate_library_dry_run_does_not_prompt` — `--dry-run --no-input` succeeds without bailing; library symlinks preserved.
  - `migrate_library_no_input_without_yes_bails` — `--no-input` alone exits non-zero with stderr containing `destructive`, `--yes`, `--no-input`; library byte-for-byte unchanged on the bail path (p1, p2, broken, user-symlink all still symlinks).
  - `migrate_library_yes_skips_prompt` — `--yes --no-input` converts managed symlinks to real directories without prompting.
- 2 pre-existing tests (`migrate_library_converts_managed_symlinks_to_real_dirs`, `sync_succeeds_after_migrate_library`) updated to pass `--yes` since `assert_cmd::Command` is non-TTY and cannot answer the dialoguer prompt — pre-existing tests now explicitly opt into the destructive flow.

## Deviations from Plan

None — plan executed exactly as written. The only minor adjustment was sequencing: to keep each commit buildable and clippy-clean, `run_migrate_library` was kept as a deprecated thin shim across the Task 1 → Task 2 commit boundary and deleted in Task 3 once `cmd_migrate_library` could call its replacement primitives directly. Plan Task 1 Step 7 specified deletion in Task 1; the actual deletion landed in Task 3. No behavioural difference — the plan's `rg` acceptance criterion (`run_migrate_library` returns zero hits in `crates/tome/src/`) is met after Task 3 commit. The same applies to `prompt_confirmation`: added with `#[allow(dead_code)]` in Task 2, attribute removed in Task 3 once `cmd_migrate_library` consumes it.

## Verification

- `cargo test -p tome --lib migration_v010` — 22 tests pass (11 pre-existing + 5 byte_size/render + 3 prompt_confirmation + 1 humanize_bytes promotion + 2 yes-flag-handling matrix arms; total +11 net new).
- `cargo test -p tome --lib cli::` — 17 tests pass (13 pre-existing + 4 new migrate-library clap-parse tests).
- `cargo test -p tome --test cli_migrate_library` — 8 tests pass (5 pre-existing + 3 new UX-02 behaviour tests).
- `cargo test -p tome` — 793 unit + 182 integration = 975 tests pass.
- `cargo fmt --check` — clean.
- `cargo clippy -p tome --all-targets -- -D warnings` — clean.
- `cargo run -p tome -- migrate-library --help` — advertises `-y, --yes` with the expected description.

`make ci` would have passed but for the locally-missing `typos` binary (pre-existing tooling gap; runtime behaviour unaffected). The `make ci` deps (`fmt-check`, `lint`, `test`) all pass individually.

## Self-Check: PASSED

- `crates/tome/src/migration_v010.rs` — modified — FOUND
- `crates/tome/src/cli.rs` — modified — FOUND
- `crates/tome/src/lib.rs` — modified — FOUND
- `crates/tome/tests/cli_migrate_library.rs` — modified — FOUND
- Commit `ac2c117` (Task 1) — FOUND
- Commit `1cc9cab` (Task 2) — FOUND
- Commit `2053d28` (Task 3) — FOUND
