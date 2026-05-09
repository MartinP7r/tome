---
phase: 15-cli-hardening
plan: 04
subsystem: testing+safety
tags: [anyhow, downcast, atomic-write, rename, foreign-symlink, canonicalize, doctor, machine-toml, directory-overrides, integration-tests]

# Dependency graph
requires:
  - phase: 15-cli-hardening
    provides: 15-01 cmd_<name> dispatch helpers (cmd_lint, cmd_migrate_library, cmd_remove_dir); per-domain tests/cli_*.rs split + tests/common/mod.rs
  - phase: 15-cli-hardening
    provides: 15-02 config/{mod,types,overrides,validate}.rs split; Config::save_checked at config/mod.rs (S3 grep target locked)
  - phase: 15-cli-hardening
    provides: 15-03 anyhow::Result conversion (HARD-01) — LintFailed/MigrationPartialOrFailed compose cleanly with the migrated skill::parse error path
  - phase: 14-unowned-library-lifecycle
    provides: D-API-2 `tome remove dir <name>` shape; D-C1 previous_source schema; LIB-04 Owned→Unowned transition in remove.rs
  - phase: 13-lockfile-authoritative-sync
    provides: D-22 atomic end-of-loop save invariant for tome.lock — HARD-08 atomic-save tests pin this contract
  - phase: 09-cross-machine-path-overrides
    provides: PORT-01..05 [directory_overrides.<name>] schema + apply timing + machine.toml-named error wrapper
provides:
  - LintFailed typed error in lint.rs (downcastable through anyhow)
  - MigrationPartialOrFailed sibling typed error in migration_v010.rs
  - main.rs top-level downcast → ExitCode::FAILURE mapping for both
  - Config::save / Config::save_checked promoted to atomic temp+rename via shared atomic_write_toml helper
  - HARD-08 regression tests pinning all four save() impls preserve previous content on rename failure
  - is_foreign_symlink predicate in distribute.rs (canonicalize+lexical 2x2 matrix)
  - distribute warn-and-skip + force-clobber for foreign symlinks (D-DIST-1)
  - DiagnosticIssueKind::ForeignSymlink + ALL-array + POLISH-04 sentinel in doctor.rs (D-DIST-2)
  - Hostile-input rejection in apply_machine_overrides (.. traversal, NUL bytes, symlink loops, duplicate target paths)
  - tests/cli_overrides.rs (3 hostile-input scenarios)
  - tests/cli_remove.rs HARD-11 e2e (git + claude-plugins flavours of `tome remove dir`)
affects:
  - 15-05 (browse UI): no overlap — different module surface
  - 15-06 (polish + older bugs): manifest.rs HARD-20 epoch-0 timestamp warning sits alongside HARD-08 regression tests; keep docstrings tight so 15-06 layers on top cleanly
  - 16 (UX rewrite): UX-01 cleanup-message rewrite intersects HARD-09 distribute warning text (foreign-symlink wording) — Phase 16 may iterate on shared phrasing

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "POLISH-04 ALL-array + compile-time exhaustiveness sentinel applied to a third enum (DiagnosticIssueKind)"
    - "Typed error + main.rs downcast pattern (LintFailed, MigrationPartialOrFailed) — replaces in-library process::exit(1)"
    - "Atomic temp+rename helper (atomic_write_toml) factored out of three pre-existing implementations and a fourth fixed-up site"
    - "Foreign-symlink detection via 2x2 canonicalize+lexical prefix matrix"
    - "Per-machine-config hostile-input rejection (.. traversal, NUL bytes, symlink loops, duplicate target paths)"

key-files:
  created:
    - "crates/tome/tests/cli_overrides.rs (HARD-10 hostile inputs)"
  modified:
    - "crates/tome/src/lint.rs — LintFailed type + 3 unit tests"
    - "crates/tome/src/migration_v010.rs — MigrationPartialOrFailed sibling type"
    - "crates/tome/src/lib.rs — bail-with-typed-error replaces process::exit(1) in cmd_lint and cmd_migrate_library; LintFailed + MigrationPartialOrFailed re-exported at crate root"
    - "crates/tome/src/main.rs — downcast_ref::<LintFailed> + downcast_ref::<MigrationPartialOrFailed> branches in the top-level error handler"
    - "crates/tome/src/config/mod.rs — atomic_write_toml helper; Config::save and Config::save_checked routed through it; HARD-08 regression test"
    - "crates/tome/src/manifest.rs — HARD-08 atomic-save test + Owned-state previous_source round-trip pin"
    - "crates/tome/src/lockfile.rs — HARD-08 atomic-save test + Owned-state previous_source round-trip pin"
    - "crates/tome/src/machine.rs — HARD-08 atomic-save test"
    - "crates/tome/src/distribute.rs — is_foreign_symlink predicate + warn-and-skip block (D-DIST-1) + 5 unit tests; existing distribute_updates_stale_link fixture relaxed to in-library staleness"
    - "crates/tome/src/doctor.rs — DiagnosticIssueKind enum + ALL-array + POLISH-04 sentinel + Optional kind on DiagnosticIssue + check_distribution_dir foreign-symlink emit + 5 unit tests; refactored 12 emit sites from struct-literal to DiagnosticIssue::untyped helper"
    - "crates/tome/src/config/overrides.rs — reject_hostile_override_path (empty/NUL/.. traversal/symlink-loop) + duplicate-path post-apply check"
    - "crates/tome/tests/cli_lint.rs — lint_failure_exit_code_via_lint_failed_downcast e2e"
    - "crates/tome/tests/cli_sync.rs — sync_warns_and_skips_foreign_symlink_in_distribution_dir e2e"
    - "crates/tome/tests/cli_remove.rs — tome_remove_dir_cleans_git_cache + tome_remove_dir_cleans_claude_plugins"

key-decisions:
  - "MigrationPartialOrFailed introduced alongside LintFailed (Rule 2 deviation): the plan only mandates HARD-04 for lint, but the acceptance criterion `grep -E 'process::exit\\(1\\)' crates/tome/src/lib.rs returns NOTHING` was literally violated by a SECOND process::exit(1) site in cmd_migrate_library. Rather than redefine the criterion, route the migrate-library failure through the same anyhow downcast pattern so lib.rs is genuinely process::exit-free."
  - "DiagnosticIssue kept as `{ severity, message, kind: Option<DiagnosticIssueKind> }` instead of converted to an enum: the plan's <interfaces> describes `DiagnosticIssue` as if it were already enum-shaped (it is not — it is a struct). Converting all 12 existing emit sites to enum variants would have churned the JSON shape and forced every doctor consumer to rebuild. Adding a typed `kind` field with `skip_serializing_if = Option::is_none` extends the shape backward-compatibly while still giving D-DIST-2 the typed dispatch the plan calls for. POLISH-04 ALL-array + sentinel still apply, just at the kind level."
  - "Config::save and Config::save_checked were NOT atomic before this plan — both used direct `std::fs::write`. The plan flagged this as 'unlikely scenario … this is a bug — fix it to use temp+rename FIRST, then add the regression test.' Promoted both to a shared `atomic_write_toml` helper so the four canonical save sites (manifest, lockfile, machine.toml, tome.toml) all use the same temp+rename shape now."
  - "Foreign-symlink detection uses a 2x2 canonicalize+lexical prefix matrix instead of the simpler 'canonicalize both sides + starts_with' the plan suggested. The 2x2 covers (a) macOS symlinks-in-the-middle (/var → /private/var), (b) targets that don't exist on disk (canonicalize fails — must fall back to lexical), and (c) library_dir spelled as a symlink itself. A naïve canonicalize-both broke two pre-existing in-library staleness tests on macOS; the matrix fixes the false-foreign-positive without weakening the true-foreign detection."
  - "Hostile-input rejection in apply_machine_overrides covers .. traversal, NUL bytes, broken-symlink loops, and duplicate target paths. The plan's case 1 (..) and case 3 (duplicate target) were unrejected before this patch — discover would silently scan ../etc with predictable failure noise. The plan's case 2 (symlink loop) was previously surfaced as a confusing 'path does not exist' warning during config-check; we now reject up-front with a stable Conflict / Why / Suggestion error message that names machine.toml (PORT-04 wrapper convention)."
  - "HARD-11 tests use the `remove_test_env` helper (config at <tome_home>/tome.toml + --tome-home flag) instead of TestEnvBuilder (config at tmp/config.toml). The remove command saves back to `paths.config_path()` = `<tome_home>/tome.toml`, NOT the `--config` argument. TestEnvBuilder's layout means remove writes to a NEW tome.toml file alongside the existing config.toml; the remove_test_env layout matches existing test_remove_local_directory and round-trips correctly."

patterns-established:
  - "Library-typed-error → main.rs downcast pattern: the binary owns the exit-code mapping, the library bubbles typed errors. Phase 16/17 can layer differentiated exit codes (e.g. lint = 1, migrate = 2) without churning every call site."
  - "DiagnosticIssue::untyped(severity, message) helper: legacy emit sites use this; ::typed(severity, kind, message) for new categorical sites. Keeps the field-addition mechanical and avoids enum-conversion churn."
  - "atomic_write_toml(path, content): single source of truth for the temp+rename pattern across both Config::save and Config::save_checked. Future config save points reuse it directly."
  - "is_foreign_symlink(link_path, library_dir): pub(crate) predicate shared between distribute and doctor; canonicalize+lexical 2x2 matrix handles the macOS symlinked-/var edge case AND missing-target-leaf staleness."
  - "reject_hostile_override_path(name, path): hostile-input gatekeeper for any future per-machine override schema. Pattern (empty + NUL + .. traversal + symlink loop + post-apply collision check) extends naturally to other override-style configs."

requirements-completed:
  - HARD-04
  - HARD-08
  - HARD-09
  - HARD-10
  - HARD-11

# Metrics
duration: 35min
completed: 2026-05-08
---

# Phase 15 Plan 04: Safety Guards + Integration Tests Summary

**LintFailed/MigrationPartialOrFailed downcast through anyhow replaces in-library process::exit(1); all four save() impls now atomic with regression coverage; distribute warns-and-skips foreign symlinks (D-DIST-1) and doctor surfaces them as typed ForeignSymlink Warning (D-DIST-2); [directory_overrides] hostile inputs (`..`, NUL, loops, duplicates) rejected with machine.toml-named errors; tome remove dir end-to-end coverage for git + claude-plugins.**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-05-08T06:13:41Z
- **Completed:** 2026-05-08T06:42:05Z
- **Tasks:** 5
- **Files modified:** 14 (1 created, 13 modified)

## Accomplishments

- HARD-04: `process::exit(1)` is gone from lib.rs (both lint AND migrate-library sites). LintFailed + MigrationPartialOrFailed bubble through `anyhow::Result`; main.rs downcasts and exits 1.
- HARD-08: All four save sites (manifest, lockfile, machine.toml, tome.toml) have a regression test pinning the temp+rename invariant. Two of the four (`Config::save`, `Config::save_checked`) were NOT atomic before this plan — fixed via a shared `atomic_write_toml` helper.
- HARD-09: distribute warns-and-skips pre-existing symlinks pointing OUTSIDE library_dir; reuses the existing `force` parameter as opt-in clobber. doctor surfaces the same condition persistently as `DiagnosticIssueKind::ForeignSymlink` (Warning severity, contributes to total_issues, POLISH-04 ALL-array + sentinel).
- HARD-10: 3 hostile-input integration tests for `[directory_overrides.<name>]` covering `..` traversal, symlink loops, and duplicate-target paths. All 3 reject with stable Conflict / Why / Suggestion error wording naming `machine.toml`.
- HARD-11: 2 e2e integration tests for `tome remove dir <name>` covering git (cache cleanup) and claude-plugins (distribution-symlink + Unowned transition).
- 25 net new tests across the suite (3 unit lint + 4 atomic-save + 2 round-trip pin + 5 distribute + 5 doctor + 1 cli_lint + 1 cli_sync + 3 cli_overrides + 2 cli_remove). Total: 743 unit + 145 integration tests passing in `make test`.

## Task Commits

1. **Task 1: HARD-04 LintFailed + main.rs exit-code mapping** — `ad04979` (feat)
2. **Task 2: HARD-08 atomic-save preservation regression tests** — `0c06e38` (test)
3. **Task 3: HARD-09 distribute foreign-symlink + doctor surface** — `98735a4` (feat)
4. **Task 4: HARD-10 hostile-input rejection for [directory_overrides]** — `cd4b1d7` (feat)
5. **Task 5: HARD-11 tome remove dir e2e tests + cargo fmt cleanup** — `0e990d8` (test)

## LintFailed type signature + main.rs downcast pattern

```rust
// crates/tome/src/lint.rs
pub struct LintFailed { pub violations: usize }
impl std::fmt::Display for LintFailed { /* "lint failed: N violation(s)" */ }
impl std::error::Error for LintFailed {}

// crates/tome/src/lib.rs (cmd_lint)
if report.has_errors() {
    anyhow::bail!(lint::LintFailed { violations: report.error_count() });
}

// crates/tome/src/main.rs
if let Some(lint_failed) = e.downcast_ref::<tome::LintFailed>() {
    eprintln!("error: {lint_failed}");
    return ExitCode::FAILURE;
}
if let Some(m) = e.downcast_ref::<tome::MigrationPartialOrFailed>() { ... }
```

## Atomic-save mechanism (real fs, no mock layer)

Each of the four regression tests:
1. Writes content A via the canonical happy-path save.
2. `chmod 0o500` on the parent directory (read+exec, no write) so `fs::rename(tmp, path)` returns EACCES.
3. Attempts to save content B; asserts `is_err()`.
4. Restores permissions BEFORE asserting (so TempDir Drop cleanup works on assertion panic).
5. Re-reads the file; byte-identical to A.

`Config::save` and `Config::save_checked` were NOT atomic before this plan — they used direct `std::fs::write(path, ...)`. Promoted to `atomic_write_toml(path, content)` (a shared helper that mirrors manifest::save / lockfile::save / machine::save).

## Foreign-symlink warning text (verbatim)

```
warning: <link> is a foreign symlink
         (→ <actual-target>); skipping.
         Pass --force to overwrite, or remove manually.
```

## DiagnosticIssue::ForeignSymlink JSON shape

The legacy free-form `{ severity, message }` shape is preserved. New optional `kind` field is omitted when None:

```json
// Pre-HARD-09 untyped issue (kind absent)
{ "severity": "Warning", "message": "stale symlink ..." }

// New typed ForeignSymlink issue
{ "severity": "Warning", "message": "foreign symlink: <link> -> <target> ...",
  "kind": "ForeignSymlink" }
```

## HARD-10 + HARD-11 test counts

- **tests/cli_overrides.rs:** 3 `#[test]` fns (`cli_overrides_hostile_dotdot_traversal_rejected`, `cli_overrides_hostile_symlink_loop_rejected`, `cli_overrides_hostile_duplicate_target_rejected`).
- **tests/cli_remove.rs:** 2 new `#[test]` fns (`tome_remove_dir_cleans_git_cache`, `tome_remove_dir_cleans_claude_plugins`) added alongside the existing 13 (15 total in the file).

## Decisions Made

See key-decisions in frontmatter. The substantive ones:

1. Bundle the `cmd_migrate_library` site with HARD-04's lint refactor: literally satisfying the `grep returns NOTHING` acceptance criterion required treating both `process::exit(1)` sites the same way. The migrate path now bubbles a `MigrationPartialOrFailed` typed error and `main.rs` downcasts it the same way as `LintFailed`.

2. Keep `DiagnosticIssue` as a struct (with new optional `kind` field) instead of converting to an enum. The plan's <interfaces> assumed enum shape; converting would have rewritten 12 emit sites + the JSON output shape consumers depend on.

3. Promote `Config::save` and `Config::save_checked` to atomic temp+rename. The plan called this out as the fix-first-then-test scenario. Both now use a shared `atomic_write_toml` helper.

4. Foreign-symlink detection uses a 2x2 (raw vs canonicalised) prefix matrix instead of canonicalize-only. Naïve canonicalize broke two pre-existing in-library staleness tests on macOS; the matrix fixes the false positive without weakening true-foreign detection.

5. Hostile-input rejection added in `apply_machine_overrides` (vs `validate()`) so the rejection happens at the closest layer to the override source. Errors use the existing PORT-04 wrapper convention (mention `machine.toml`, not `tome.toml`).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Closed second `process::exit(1)` site in `cmd_migrate_library`**

- **Found during:** Task 1 (HARD-04)
- **Issue:** The plan's HARD-04 acceptance criteria require `grep -E "process::exit\\(1\\)" crates/tome/src/lib.rs` to return NOTHING. The plan only describes the lint site explicitly, but lib.rs had a second process::exit(1) at line 934 inside cmd_migrate_library. Leaving it would have left the criterion only partially satisfied.
- **Fix:** Introduced a sibling `MigrationPartialOrFailed` typed error in migration_v010.rs alongside `LintFailed`; migrate-library now bails through anyhow with that type, and main.rs downcasts both. Same pattern, same exit code.
- **Files modified:** crates/tome/src/migration_v010.rs, crates/tome/src/lib.rs, crates/tome/src/main.rs
- **Verification:** `rg 'process::exit\(1\)' crates/tome/src/lib.rs` returns 0 matches in code (the only matches are in comments referencing the historical removal).
- **Committed in:** ad04979 (Task 1)

**2. [Rule 2 - Missing Critical] Promoted `Config::save` + `Config::save_checked` to atomic temp+rename**

- **Found during:** Task 2 (HARD-08)
- **Issue:** Both `Config::save` (line 85) and `Config::save_checked` (line 322) used `std::fs::write(path, ...)` directly — NOT the atomic temp+rename pattern documented for the other three save() sites (manifest, lockfile, machine.toml). A crash mid-write would leave a half-written tome.toml. The plan's <action> Step B explicitly anticipated this scenario: "If reading the existing impl reveals NON-atomic behaviour … this is a bug — fix it to use temp+rename FIRST, then add the regression test."
- **Fix:** Factored a shared `atomic_write_toml(path, content)` helper inside config/mod.rs and routed both `save` and `save_checked` through it. Pattern matches manifest::save / lockfile::save / machine::save (write tmp, fs::rename, best-effort cleanup on failure).
- **Files modified:** crates/tome/src/config/mod.rs
- **Verification:** New `save_checked_preserves_previous_on_rename_failure` test passes.
- **Committed in:** 0c06e38 (Task 2)

**3. [Rule 1 - Bug] Foreign-symlink detection canonicalisation asymmetry**

- **Found during:** Task 3 (HARD-09)
- **Issue:** First-cut implementation of `is_foreign_symlink` used `canonicalize(library_dir).starts_with(canonicalize(link))` only. This false-positived in two pre-existing tests on macOS:
  - `distribute_updates_stale_link`: stale link points at a missing in-library leaf; `canonicalize(link_path)` fails because the target doesn't exist.
  - `check_distribution_dir_stale_symlink`: same issue. Fallback to raw vs canonical comparison reported "foreign" because `/var/...` (raw) doesn't start_with `/private/var/...` (canonical).
- **Fix:** Replaced with a 2x2 prefix matrix — try (raw library, raw target), (raw library, canonical target), (canonical library, raw target), (canonical library, canonical target). Any match → not-foreign. The matrix handles both macOS symlinks-in-the-middle AND missing-leaf staleness.
- **Files modified:** crates/tome/src/distribute.rs (is_foreign_symlink helper)
- **Verification:** All 36 doctor::tests + 18 distribute::tests pass.
- **Committed in:** 98735a4 (Task 3)

**4. [Rule 1 - Bug] Two pre-existing tests pinned the OLD (pre-HARD-09) behaviour**

- **Found during:** Task 3 (HARD-09)
- **Issue:** `distribute_updates_stale_link` originally created a stale link pointing OUTSIDE library_dir and expected sync to silently clobber it. With HARD-09 / D-DIST-1 that's now a foreign-symlink case → warn-and-skip (correct new behaviour, but breaks the test). Similarly, `doctor::tests::check_distribution_dir_ignores_external_symlinks` asserted external symlinks were silently ignored — D-DIST-2 now surfaces them as ForeignSymlink Warnings (correct, but breaks the test).
- **Fix:** Updated both fixtures to pin the new contract:
  - `distribute_updates_stale_link`: stale link now points at an in-library missing leaf (still stale, no longer foreign — auto-recreate path is exercised).
  - `check_distribution_dir_ignores_external_symlinks` → renamed `check_distribution_dir_surfaces_external_symlinks_as_foreign`: asserts the new ForeignSymlink Warning emission.
- **Files modified:** crates/tome/src/distribute.rs, crates/tome/src/doctor.rs
- **Verification:** Both tests now pass; the renamed doctor test pins the new contract.
- **Committed in:** 98735a4 (Task 3)

**5. [Rule 2 - Missing Critical] Override hostile-input rejection added to `apply_machine_overrides`**

- **Found during:** Task 4 (HARD-10)
- **Issue:** The plan's hostile-input cases (`..` traversal, symlink loop, duplicate target) were NOT rejected at the apply layer pre-this-patch. `..` paths reached discover (and silently failed). Symlink loops surfaced as "path does not exist" warnings during config-check (unhelpful — the path DOES exist as a symlink). Duplicate target paths weren't rejected at all — distribute would silently distribute conflicting symlinks driven by BTreeMap iteration order. Without an apply-layer guard the integration tests would either need to test downstream failure modes (brittle) or document the gap as deferred.
- **Fix:** Added `reject_hostile_override_path(name, path)` helper covering empty paths, NUL bytes, `..` components, and broken/looping symlinks (canonicalize fail). Added a post-loop duplicate-path check that enumerates colliding directory names. All errors follow the Phase 7 D-10 Conflict / Why / Suggestion template AND the PORT-04 message-content convention (mention `machine.toml`).
- **Files modified:** crates/tome/src/config/overrides.rs
- **Verification:** All 3 cli_overrides integration tests pass; existing 17 config::overrides::tests still pass.
- **Committed in:** cd4b1d7 (Task 4)

---

**Total deviations:** 5 auto-fixed (3 Rule 2 missing-critical + 2 Rule 1 bug-fix)
**Impact on plan:** All deviations were essential for correctness, security, or literal acceptance-criteria compliance. No scope creep — every fix sits squarely within the plan's stated objectives.

## Issues Encountered

None — the plan executed cleanly modulo the deviations above (which are by design auto-fixed without user intervention).

Pre-existing flake observed: `backup::tests::push_and_pull_roundtrip` and `backup::tests::restore_reverts_changes` are intermittent in the full unit-test run (pass cleanly in isolation). Tracked in #500 as HARD-14 in plan 15-06; outside this plan's scope.

## User Setup Required

None — no external service configuration touched.

## Next Phase Readiness

- 15-05 (browse UI) is unblocked — no shared module surface.
- 15-06 (polish + older bugs) is the natural follower; HARD-14 backup-flake fix lives there. The atomic-save tests and HARD-20 epoch-0 timestamp warning will share `manifest.rs` — the docstrings around the new test are tight so 15-06 can layer on top cleanly.

## Self-Check: PASSED

All 15 tracked files (10 source + 4 test + 1 summary) confirmed present on disk.
All 5 task commit hashes (ad04979, 0c06e38, 98735a4, cd4b1d7, 0e990d8) confirmed in `git log --oneline --all`.

---
*Phase: 15-cli-hardening*
*Completed: 2026-05-08*
