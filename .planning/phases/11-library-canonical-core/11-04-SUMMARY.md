---
phase: 11-library-canonical-core
plan: 04
subsystem: migration
tags: [migration, libcanonical, lib-05, d-01, d-02, d-03, d-04, d-05, d-06, cli, sync]

# Dependency graph
requires:
  - phase: 11-01
    provides: SkillEntry/LockEntry source_name lifted to Option<DirectoryName>; manifest schema with twin constructors
  - phase: 11-02
    provides: consolidate_managed copy-only semantics; LIB-01 invariant (no symlinks for managed entries)
provides:
  - migration_v010 transitional module — detection (D-03), plan/render/execute, SAFE-01 failure aggregation
  - tome migrate-library CLI subcommand with --dry-run and one-shot semantics
  - sync v0.9-shape refuse-with-hint check (D-02) — bail with Conflict/Why/Suggestion before consolidate
  - MigrationFailureKind enum with compile-time exhaustive guard (BrokenSource, IoError)
  - copy_dir_recursive_resolving — recursive copy that follows symlinks (opposite of relocate.rs::copy_library)
  - 13 new unit tests covering detection variations, broken-symlink preservation, dry-run, idempotent re-run
affects: [11-05, 17]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Transitional module shape: module-level doc marks file for v0.11+ removal; entire module + the one sync-side hook delete cleanly together"
    - "Manifest-anchored detection: only entries with (a) symlink path AND (b) manifest entry exists AND (c) managed=true qualify — never touches user-created symlinks"
    - "Filesystem-only conversion (D-06): manifest source_path/content_hash/managed all stay correct after symlink→copy; idempotent re-runs from fresh detection"

key-files:
  created:
    - crates/tome/src/migration_v010.rs
  modified:
    - crates/tome/src/cli.rs
    - crates/tome/src/lib.rs

key-decisions:
  - "Migration is filesystem-only — manifest is never mutated during conversion. source_path/content_hash/managed all stay correct after symlink→copy, so re-runs pick up where they left off without consistency-recovery code (D-06)"
  - "Broken symlinks are PRESERVED in place rather than deleted — the symlink target carries metadata about where the original source lived, giving the user a chance to manually recover (D-04)"
  - "copy_dir_recursive_resolving is a separate helper from library.rs::copy_dir_recursive (which preserves symlinks); migration's copy must materialize the source content (follow_links(true)) so the library has zero symlinks post-conversion"
  - "Sync's v0.9-shape detection loads the manifest a second time (consolidate also loads it) to keep the check isolated as a single deletable block — the modest perf cost (one JSON parse per sync) is worth the v0.11+ cleanup ergonomics"

patterns-established:
  - "Plan/render/execute for one-shot maintenance commands: the same shape that add/remove/reassign use; dry-run is free with this pattern"
  - "Compile-time enum exhaustiveness: const fn match + const _ assertion on ALL.len() — direct copy of remove.rs::FailureKind pattern (POLISH-04)"
  - "Conflict/Why/Suggestion error template (Phase 7 D-10) reused for the sync refuse-with-hint message — three lines, terminal-friendly"

requirements-completed: [LIB-05]

# Metrics
duration: 5min
completed: 2026-05-03
---

# Phase 11 Plan 04: Migrate-Library Command Summary

**`tome migrate-library` is a one-shot CLI command that converts v0.9-shape libraries (managed skills as symlinks) to v0.10-shape (real directory copies) with manifest-anchored detection (D-03), broken-symlink preservation (D-04), SAFE-01 failure aggregation (D-05), and idempotent re-runs (D-06); `tome sync` refuses to run on a v0.9-shape library and points the user at the new command (D-02).**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-05-03T13:40:05Z
- **Completed:** 2026-05-03T13:44:27Z
- **Tasks:** 2 (Task 1 — module + tests; Task 2 — CLI wiring + sync gate)
- **Files modified:** 3 (1 created, 2 modified)
- **Tests added:** 13 (all passing)
- **Total unit tests:** 558 passing

## Accomplishments

- **New `crates/tome/src/migration_v010.rs` module** (~650 LOC including tests). Module-level doc comment marks the file for removal in v0.11+ once all known users have migrated.
- **Detection (D-03)** is manifest-anchored: a `library_dir/<name>` qualifies for migration ONLY when ALL of: (a) the path is a symlink, AND (b) `manifest[name].managed == true`, AND (c) `manifest.contains_key(name)`. User-created symlinks never qualify.
- **Plan/render/execute** pattern from add/remove/reassign — `plan()` enumerates qualifying entries, `render_plan()` prints a human-readable summary with broken-source warnings, `execute()` performs the conversion (or counts in dry-run mode).
- **Broken-symlink handling (D-04):** broken symlinks (target gone) are SKIPPED with a stderr warning AND PRESERVED in place — never deleted. The symlink target string carries metadata about where the original source lived; preserving it gives the user a chance to manually recover.
- **SAFE-01 failure aggregation (D-05):** `MigrationFailureKind::{BrokenSource, IoError}` enum with compile-time exhaustive guard (`_ensure_failure_kind_all_exhaustive` const fn + `assert!(MigrationFailureKind::ALL.len() == 2)`). Final summary: `⚠ N converted · K skipped (broken source) · M failed`. Any skip OR failure means non-zero exit.
- **Idempotent re-run (D-06):** the manifest is not mutated by migration — `source_path`, `content_hash`, `managed: true` all stay correct after the filesystem-only conversion. Detection re-runs from scratch each invocation so partial migrations pick up where they left off.
- **`copy_dir_recursive_resolving` helper** uses `walkdir::WalkDir::new(src).follow_links(true)` — the opposite of `relocate.rs::copy_library` which preserves symlinks. Migration must materialize the source content so the library has zero symlinks post-conversion.
- **Inline comment at the `pre_hash` site** documents why `hash_directory` works on a symlink path: `WalkDir` follows the symlink root by default even when `follow_links(false)`, so hashing succeeds against the source content the symlink resolves to.
- **CLI wiring:** `Command::MigrateLibrary { dry_run: bool }` added to `cli.rs` with after_help mentioning "one-shot" and v0.10. clap auto-converts to the kebab-case `tome migrate-library` subcommand.
- **Dispatch arm in `lib.rs::run()`:** calls `migration_v010::run_migrate_library(&paths, dry_run || cli.dry_run)?` (honors the global `--dry-run` flag too) and exits with code 1 on partial-or-failed migration per D-05.
- **`lib.rs::sync` v0.9-shape refuse gate (D-02):** an isolated check before `library::consolidate` loads the manifest, calls `migration_v010::detect_v09_shape`, and bails with a Conflict/Why/Suggestion error pointing at `tome migrate-library`. Whole block deletes cleanly with the rest of `migration_v010` in v0.11+.
- **13 unit tests** cover plan detection variations (managed-symlink, user-created, non-managed, broken), execute (convert, preserve broken per D-04, dry-run, idempotent), `detect_v09_shape` (true/false/already-real-dir), and the `MigrationFailureKind::ALL` compile-time guard.

## Task Commits

1. **Task 1: Create `migration_v010.rs` with detection, plan, render, execute** — `e8ddb42` (feat). Single atomic commit with the new module + lib.rs `pub(crate) mod migration_v010;` declaration. 13 new unit tests, all passing.
2. **Task 2: Wire `Command::MigrateLibrary` + sync v0.9-shape refuse gate** — `8a793f0` (feat). CLI variant declaration in cli.rs, dispatch arm in lib.rs::run(), refuse-with-hint check in lib.rs::sync. Smoke verified via `tome migrate-library --help`.

## Files Created/Modified

- **Created:**
  - `crates/tome/src/migration_v010.rs` — transitional module: `MigrationFailureKind` enum + compile-time guard, `MigrationFailure`, `MigrationEntry`, `MigrationPlan`, `MigrationResult`, `plan()`, `detect_v09_shape()`, `render_plan()`, `execute()`, `copy_dir_recursive_resolving()`, `render_result()`, `run_migrate_library()` top-level entry. 13 unit tests in `#[cfg(test)] mod tests` covering D-03/D-04/D-05/D-06.
- **Modified:**
  - `crates/tome/src/cli.rs` — added `Command::MigrateLibrary { dry_run: bool }` variant between `Lint` and `Browse` with after_help block.
  - `crates/tome/src/lib.rs` — added `pub(crate) mod migration_v010;` (between `manifest` and `paths`); `Command::MigrateLibrary` dispatch arm before `Command::Eject`; v0.9-shape refuse gate in `sync()` immediately before `library::consolidate`.

## Decisions Made

- **Migration is filesystem-only — manifest is never mutated during conversion.** `source_path`, `content_hash`, and `managed` all stay correct after symlink→copy because the source content is identical to what the symlink resolved to. This means partial migrations leave a clean, recoverable state — re-running picks up where it left off without any consistency-recovery code (D-06).
- **Broken symlinks are PRESERVED rather than deleted.** Even though the target is gone, the symlink target string carries metadata about where the original source lived (e.g. `~/.claude/plugins/cache/claude-plugins-official/superpowers/5.0.7/skills/...`). Preserving it gives the user the only available clue for manual recovery. Library stays partially-migrated; subsequent `tome sync` keeps refusing per D-02 until the user resolves manually.
- **`copy_dir_recursive_resolving` is a separate helper from `library.rs::copy_dir_recursive`.** The library's copy preserves symlinks (via `follow_links(false)` + an explicit "skip symlink" branch); migration's copy must materialize source content (via `follow_links(true)`) so the library has zero symlinks post-conversion. Two helpers, opposite intents — naming makes the difference explicit.
- **Sync's v0.9-shape check loads the manifest a second time.** `library::consolidate` also loads the manifest, but the check is intentionally isolated as a single deletable block. The modest perf cost (one JSON parse per sync) is worth the v0.11+ cleanup ergonomics — when migration_v010 deletes, the entire `{ ... }` block in sync deletes with it without disturbing the consolidate flow.
- **Dispatch honors the global `--dry-run` flag too** (`dry_run || cli.dry_run`). A user typing `tome --dry-run migrate-library` would otherwise be surprised when the global flag is silently ignored. The `--dry-run` flag on the subcommand is also kept for explicit per-command invocation.

## Deviations from Plan

### Auto-fixed Issues

None. The plan executed exactly as written. Both commits land with all acceptance criteria met (with one note below).

### Acceptance Criteria Notes

- **Task 2 acceptance criterion `rg -n "MigrateLibrary" crates/tome/src/cli.rs` returns at least 2 matches**: only 1 match exists (the `Command` enum variant declaration). The criterion's "at least 2" expectation seems to assume the after_help string would mention "MigrateLibrary" but it actually contains the kebab-case `migrate-library` (which is what clap auto-generates from the variant name). The functional intent — `Command::MigrateLibrary` exists, dispatch works, `tome migrate-library --help` exits 0 with "one-shot" and "--dry-run" in the output — is fully met. This is a planner-text artifact, not a real failure.

## Issues Encountered

- **Pre-existing dead-code warning on `SkillEntry::new_unowned`.** The `new_unowned` constructor was lifted in Plan 11-01 for use by Phase 14's `tome adopt`/`tome forget` commands. It's not consumed in Phase 11. The warning is expected and not introduced by this plan.

## User Setup Required

None — code-only change. The new `tome migrate-library` command is available immediately after build. On a v0.10 fresh install (no managed symlinks in the library), the command reports "no v0.9-shape entries detected — library is already in v0.10 shape" and exits cleanly.

## Next Phase Readiness

- **Plan 11-05 (integration tests)** can now exercise the full migration end-to-end via `tome migrate-library` against a synthetic v0.9 library fixture. The boundary defense from Plan 11-02 (`consolidate_managed` Symlink branch) ensures that even if the sync gate is bypassed, the user gets a stderr warning pointing at `tome migrate-library`.
- **Phase 14 (UNOWN-01..03)** can read the now-Unowned manifest state via `entry.source_name.is_none()`. The Phase 11 schema work (Plan 11-01) + the migration tooling (this plan) give Phase 14 a clean starting point.
- **Phase 17 (REL-01..05)** should add release-notes guidance for v0.10 users: "after upgrading, run `tome migrate-library` once to convert your library; commit your library to git first."

## v0.11+ Follow-up Reminder

When v0.11 ships and migration is no longer needed:

1. Delete `crates/tome/src/migration_v010.rs` entirely.
2. Remove `pub(crate) mod migration_v010;` from `crates/tome/src/lib.rs`.
3. Delete the v0.9-shape refuse-with-hint block in `lib.rs::sync` (the entire `{ let manifest_for_detection = ... }` scope before consolidate).
4. Delete the `Command::MigrateLibrary` variant from `cli.rs` and its dispatch arm in `lib.rs::run()`.
5. Delete the boundary-defense `Symlink` arm in `library.rs::consolidate_managed` (Plan 11-02 added it as a defense against bypass; v0.11 no longer needs it).

File a v0.11 issue at v0.10 ship time so this isn't forgotten.

## Self-Check: PASSED

- crates/tome/src/migration_v010.rs: FOUND
- crates/tome/src/cli.rs: FOUND (modified)
- crates/tome/src/lib.rs: FOUND (modified)
- .planning/phases/11-library-canonical-core/11-04-SUMMARY.md: FOUND
- Commit e8ddb42 (Task 1 — migration_v010 module + tests): FOUND
- Commit 8a793f0 (Task 2 — CLI + sync gate wiring): FOUND
- `cargo test --package tome --lib migration_v010::tests` passes 13/13
- `cargo test --package tome --lib` passes 558/558
- `cargo build --package tome` exits 0
- `./target/debug/tome migrate-library --help | grep -q "one-shot"` exits 0

---
*Phase: 11-library-canonical-core*
*Completed: 2026-05-03*
