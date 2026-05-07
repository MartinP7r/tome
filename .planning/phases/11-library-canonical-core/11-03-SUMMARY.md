---
phase: 11-library-canonical-core
plan: 03
subsystem: persistence
tags: [cleanup, remove, unowned, manifest, libcanonical, lib-04, d-09, d-10]

# Dependency graph
requires:
  - phase: 11-01
    provides: SkillEntry.source_name lifted to Option<DirectoryName>; Manifest::skills_get_mut accessor; SkillEntry::new_unowned constructor
provides:
  - cleanup_library partitions stale candidates by D-09 case (Case 1 transition vs Case 2 delete)
  - CleanupResult.transitioned_to_unowned counter for visibility into Case 1 transitions
  - Already-Unowned manifest entries are preserved by definition (filtered out of stale set)
  - tome remove transitions owned manifest entries to Unowned BEFORE removing config entry (D-10 trigger 1)
  - Library content for owned skills is preserved on tome remove (LIB-04)
  - RemoveResult.library_entries_transitioned_to_unowned replaces library_entries_removed
  - FailureKind reduced to 2 variants (DistributionSymlink, GitCache) since execute() no longer touches library files
  - render_plan reports library content as "kept as Unowned" with Phase 14 forget-command hint
  - Integration tests updated to reflect the new contract (test_remove_local_directory, remove_retry_succeeds_after_failure_resolved)
affects: [11-04, 11-05, 14, 16]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Two-trigger transition: explicit (tome remove) + implicit safety net (sync cleanup) — both deletable in one place each if the model evolves"
    - "Stale-set partition by config.directories.contains_key — Case 1 vs Case 2 dispatch via single contains_key check"
    - "FailureKind shrink with const-fn drift guard updated together so the compile-time invariant stays in lockstep with the enum"

key-files:
  created:
    - .planning/phases/11-library-canonical-core/deferred-items.md
  modified:
    - crates/tome/src/cleanup.rs
    - crates/tome/src/remove.rs
    - crates/tome/src/lib.rs
    - crates/tome/tests/cli.rs

key-decisions:
  - "Case 1 transition messaging is silent (info-level eprintln), Case 2 keeps today's interactive confirm flow — preserves UX expectations because Case 1 doesn't lose data"
  - "Replace partial_failure_aggregates_multiple_kinds with failure_kind_label_coverage + failure_kind_all_pinned_size_two — the LibraryDir variant is gone so multi-kind aggregation can't be reproduced via filesystem permission denial"
  - "Integration tests test_remove_local_directory and remove_retry_succeeds_after_failure_resolved updated in this plan's commit (Rule 3 — direct consequences of the contract change in remove.rs)"

patterns-established:
  - "Stale-entry partition: filter out None-source entries first (preserved by definition), then dispatch remaining stale by config.directories.contains_key"
  - "Step ordering in destructive commands: filesystem removals (steps 1+2) THEN gated state transitions (step 3 on full-success) — preserves config+manifest for retry on partial failure (Phase 8 SAFE-01 retention)"

requirements-completed: [LIB-04]

# Metrics
duration: 12min
completed: 2026-05-03
---

# Phase 11 Plan 03: Source-Removal Unowned Transition Summary

**`tome remove` transitions owned manifest entries to `source_name = None` and preserves library content; `cleanup_library` adds the same safety-net transition for users who manually edit `tome.toml` outside `tome remove` — D-10 hybrid triggers for LIB-04.**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-05-03T13:23:40Z
- **Completed:** 2026-05-03T13:35:15Z
- **Tasks:** 2 (Task 1 cleanup partition, Task 2 remove transition)
- **Files modified:** 4 (cleanup.rs, remove.rs, lib.rs, tests/cli.rs)

## Accomplishments

- `cleanup_library` partitions stale manifest entries into D-09 Case 1 (source removed from config → transition to Unowned, preserve library content) and Case 2 (source still configured but file vanished from disk → today's delete behavior).
- Already-Unowned entries (`source_name == None`) are filtered out of the stale set entirely — preserved by definition per LIB-04. They were skipped from discover too.
- New `CleanupResult.transitioned_to_unowned` counter exposes Case 1 activity; existing `removed_from_library` continues to track Case 2.
- `tome remove <dir>` (D-10 trigger 1) now explicitly sets `source_name = None` on every manifest entry it owns via `Manifest::skills_get_mut` (lifted in Plan 11-01) instead of deleting the manifest entry and library directory.
- `RemoveResult` exposes `library_entries_transitioned_to_unowned` (replaces `library_entries_removed`); `render_plan` reports library content as "preserved as Unowned" with a Phase 14 `tome forget` pointer.
- `FailureKind` reduced from 4 variants to 2 (`DistributionSymlink`, `GitCache`) since `execute()` no longer touches library files. The const-fn drift guard, the `ALL.len() == 2` static assert, and the ordering test were all updated in lockstep.
- Partial-failure semantics preserved: if any cleanup step fails, the transition is NOT applied — config and manifest stay unchanged so `tome remove` can be re-run after addressing the underlying cause (Phase 8 SAFE-01 retention).
- Integration tests `test_remove_local_directory` and `remove_retry_succeeds_after_failure_resolved` updated to assert the new contract (library content preserved, manifest entry retained as Unowned, config entry removed).
- 24 tests pass in scope (12 cleanup + 12 remove); full unit suite (545 tests) green; touched integration tests green.

## Task Commits

1. **Task 1: cleanup_library Case 1/Case 2 partition** — `c18a7da` (feat). Adds the D-09 partition logic, the `transitioned_to_unowned` counter, and 6 new/renamed tests covering Case 1, Case 2, already-Unowned, mixed, dry-run, and the legacy managed-symlink shape.
2. **Task 2: tome remove explicit Unowned transition** — `fe13960` (feat). Rewrites `execute` to transition instead of delete, shrinks `FailureKind`, updates `render_plan` and `lib.rs` success banner, adds `execute_transitions_multiple_owned_skills_to_unowned`, replaces `partial_failure_aggregates_multiple_kinds` with two narrower tests, updates 2 integration tests to the new contract.

## Files Created/Modified

- `crates/tome/src/cleanup.rs` — added `&Config` parameter to `cleanup_library`; partitioned stale entries into Case 1 (transition) and Case 2 (delete); filtered already-Unowned entries; new `transitioned_to_unowned` counter; 6 new/renamed tests + helpers (`empty_config`, `config_with_dir`).
- `crates/tome/src/remove.rs` — `execute()` now transitions owned entries to Unowned via `manifest.skills_get_mut()` instead of deleting library files; `RemoveResult` gains `library_entries_transitioned_to_unowned`; `FailureKind` shrunk to 2 variants; const drift guard + `ALL.len() == 2` assertion + ordering test updated; `render_plan` reports preserved library content; doc comments updated for v0.10 step ordering.
- `crates/tome/src/lib.rs` — `cleanup::cleanup_library` call site passes `config`; `Command::Remove` success banner updated to "library entries kept as Unowned".
- `crates/tome/tests/cli.rs` — `test_remove_local_directory` and `remove_retry_succeeds_after_failure_resolved` updated to assert library/manifest preservation under the new contract.
- `.planning/phases/11-library-canonical-core/deferred-items.md` — logs `symlink_chain_managed_skill` as a Plan 11-02 / Plan 11-05 follow-up.

## Decisions Made

- **Case 1 transition messaging is silent (info-level `eprintln`).** The current Case 2 path keeps today's `dialoguer::Confirm` flow because deletion is destructive. Case 1 transitions don't lose data — the library copy is preserved and the manifest entry is retained as Unowned — so an interactive confirmation would be ceremony without value. Phase 16 UX-01 will rewrite the cleanup messaging into the 3-bucket partition; this plan keeps the existing UX shape.
- **Replace `partial_failure_aggregates_multiple_kinds` with `failure_kind_label_coverage` + `failure_kind_all_pinned_size_two`.** The original test relied on an `EACCES` chmod trick to provoke a `LibraryDir` failure alongside `DistributionSymlink`. With LibraryDir gone, the multi-kind aggregation pattern can't be reproduced via filesystem permission denial. The two narrower tests cover the same compile-enforcement boundary (every variant has a label, ALL has the right length) without the chmod ceremony.
- **Update the two affected integration tests in this plan's commit (Rule 3 — blocking).** `test_remove_local_directory` and `remove_retry_succeeds_after_failure_resolved` assert v0.9 behavior that contradicts the new contract. Updating them is a direct consequence of the `remove.rs` change, not a separate piece of work — keeping them red across the wave would block CI.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Update two integration tests for the new tome remove contract**

- **Found during:** Task 2 (post-commit verification with `cargo test --package tome --test cli`)
- **Issue:** `test_remove_local_directory` and `remove_retry_succeeds_after_failure_resolved` in `tests/cli.rs` asserted v0.9 behavior — that `tome remove` deletes the library skill and removes the manifest entry. Per LIB-04 / D-10 trigger 1 the new contract preserves the library entry and transitions the manifest entry to Unowned. The plan's `<files_modified>` only lists `cleanup.rs` and `remove.rs`, but the integration tests fail without an update.
- **Fix:** Updated both tests to assert the new contract: library content preserved, manifest entry retained, source_name omitted from the JSON (per `skip_serializing_if = "Option::is_none"` on the field shape lifted in Plan 11-01).
- **Files modified:** `crates/tome/tests/cli.rs` (2 test bodies)
- **Verification:** `cargo test --package tome --test cli -- test_remove_local_directory remove_retry_succeeds_after_failure_resolved remove_preserves_git_lockfile_entries` passes 3/3.
- **Committed in:** `fe13960` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 3 — blocking integration-test update direct from contract change)
**Impact on plan:** Mechanical update; no scope creep, no architectural change. The plan's intent (`tome remove` preserves library) is exactly what the test now asserts.

## Issues Encountered

- **Cross-plan test failure: `symlink_chain_managed_skill` (cli.rs:1775).** Failing on the working tree because Plan 11-02 (`consolidate_managed` rewrite to copy) changed managed-skill library shape from symlink to real directory. The integration test still asserts the v0.9 symlink shape. Out of scope for Plan 11-03 (cleanup.rs + remove.rs only); logged in `.planning/phases/11-library-canonical-core/deferred-items.md` for Plan 11-05 (integration tests) or a Plan 11-02 follow-up. Did NOT modify `library.rs` or that test (parallel-wave scope safety).
- **Edit tool reverted Task 2 changes mid-stream.** After applying ~5 edits to `remove.rs`, a subsequent Edit returned "File has been modified since last read." A re-read confirmed earlier edits were undone. Re-applied all edits sequentially. Suspected concurrent linter or post-write reformatting in the parallel wave; no impact on final state.

## User Setup Required

None — code-only change. The new contract is observable on the next `tome remove` invocation: instead of deleting library skills, they remain on disk with `source_name = None` in the manifest. Phase 14 (`tome adopt` / `tome forget`) will provide explicit lifecycle commands.

## Next Phase Readiness

- **Plan 11-04 (`tome migrate-library`)** is independent of this plan — migration is filesystem-only (D-06) and doesn't touch the cleanup/remove paths.
- **Plan 11-05 (integration tests)** should add a synthetic-fixture test exercising the manual `tome.toml` edit → `tome sync` → cleanup-Case-1 transition path (D-10 trigger 2 end-to-end) and pick up the deferred `symlink_chain_managed_skill` v0.10-shape rewrite.
- **Phase 13 (RECON-01..05)** drift detection consumes the now-Unowned manifest state correctly: `LockEntry.source_name` (lifted to `Option<DirectoryName>` in Plan 11-01) round-trips Unowned entries; `resolved_paths_from_lockfile_cache` skips them; the drift signal is `content_hash` per D-08, unchanged.
- **Phase 14 (UNOWN-01..03)** can build on the now-reachable Unowned manifest state. `tome status` and `tome doctor` will detect entries via `entry.source_name.is_none()`. `tome adopt <skill> <dir>` flips an entry from Unowned → owned (using `update_source_name` from Plan 11-01). `tome forget <skill>` removes the manifest entry AND the library directory.

## Self-Check: PASSED

- crates/tome/src/cleanup.rs: FOUND
- crates/tome/src/remove.rs: FOUND
- crates/tome/src/lib.rs: FOUND
- crates/tome/tests/cli.rs: FOUND
- .planning/phases/11-library-canonical-core/11-03-SUMMARY.md: FOUND
- .planning/phases/11-library-canonical-core/deferred-items.md: FOUND
- Commit c18a7da (Task 1 — cleanup_library D-09 partition): FOUND
- Commit fe13960 (Task 2 — tome remove Unowned transition): FOUND

---
*Phase: 11-library-canonical-core*
*Completed: 2026-05-03*
