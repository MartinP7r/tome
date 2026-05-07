---
phase: 11-library-canonical-core
plan: 05
subsystem: testing
tags: [integration-tests, cli, migrate-library, sync-gate, libcanonical, lib-01, lib-04, lib-05, d-02, d-03, d-04, d-05, d-09, d-10]

# Dependency graph
requires:
  - phase: 11-01
    provides: SkillEntry/LockEntry source_name lifted to Option<DirectoryName>; manifest schema with twin constructors
  - phase: 11-02
    provides: consolidate_managed copy-only semantics; LIB-01 invariant (no symlinks for managed entries); v0.9-shape boundary defense
  - phase: 11-03
    provides: cleanup_library Case 1/Case 2 partition; tome remove explicit Unowned transition
  - phase: 11-04
    provides: tome migrate-library CLI command; migration_v010 module with detection (D-03), broken-symlink preservation (D-04), SAFE-01 failure aggregation (D-05); sync v0.9-shape refuse-with-hint gate (D-02)
provides:
  - tome::hash_directory crate-root re-export (single canonical hashing implementation, no parallel SHA-256 helper in tests)
  - build_v09_fixture() — synthetic v0.9 library helper mirroring CONTEXT.md <specifics>
  - migrate_library_converts_managed_symlinks_to_real_dirs (LIB-05 happy path; D-03/D-04/D-05 boundary defenses)
  - migrate_library_dry_run_makes_no_changes (LIB-05 dry-run safety)
  - sync_refuses_on_v09_shape_library_with_hint (D-02 binary-level pin)
  - sync_succeeds_after_migrate_library (post-migration idempotency)
  - sync_preserves_library_when_source_removed_from_config (LIB-04 / D-09 Case 1 / D-10 trigger 2)
  - Updated symlink_chain_managed_skill (cli.rs:1775) — resolves Plan 11-03 deferred-items.md entry
affects: [12, 13, 14, 17]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Crate-root re-export of pure deterministic helpers for integration-test reuse — single canonical implementation, no risk of parallel-helper drift"
    - "Synthetic-fixture builder mirroring user's real library shape — high-confidence transfer to Phase 17 / REL-04 production smoke test"

key-files:
  created:
    - .planning/phases/11-library-canonical-core/11-05-SUMMARY.md
  modified:
    - crates/tome/src/lib.rs
    - crates/tome/tests/cli.rs
    - .planning/phases/11-library-canonical-core/deferred-items.md

key-decisions:
  - "Use the production tome::hash_directory via crate-root re-export instead of a parallel sha256_dir_simple helper — guarantees byte-for-byte identity with manifest hashing, eliminates drift risk"
  - "Update the symlink_chain_managed_skill deferred test as part of this plan (Task 1 commit) since it depends on the new re-export and is the natural home for the v0.10-shape rewrite"
  - "Add a filler skill in `other` source for the source-removal preservation test to avoid sync's CFG-06 / `skills.is_empty()` early-exit guards — comment in test documents this"

patterns-established:
  - "Re-export pure helpers at the crate root when integration tests need them: zero production-code impact, eliminates parallel-implementation drift"
  - "Synthetic v0.9 fixtures use the production hash function via re-export: hashes are guaranteed identical to what production code would compute"
  - "Cleanup-flow integration tests must thread past the sync early-exit guards (CFG-06 empty-config, skills.is_empty()) — keep at least one configured directory with at least one discoverable skill"

requirements-completed: [LIB-01, LIB-04, LIB-05]

# Metrics
duration: ~10min
completed: 2026-05-03
---

# Phase 11 Plan 05: Integration Tests Summary

**Five end-to-end CLI tests anchor the v0.10 library-canonical-core success criteria at the binary surface — `tome migrate-library` (happy path + dry-run + boundary defenses for D-03/D-04/D-05), `tome sync` v0.9-shape refuse-with-hint (D-02), source-removal Unowned preservation (LIB-04 / D-09 Case 1 / D-10 trigger 2), and post-migration idempotent sync — all reusing the production `tome::hash_directory` via a new crate-root re-export so synthetic-fixture hashes are byte-for-byte identical to production hashes.**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-05-03T13:46:35Z
- **Completed:** 2026-05-03T13:56:12Z
- **Tasks:** 3 (Task 0 re-export; Task 1 fixture + 2 migrate-library tests + symlink_chain_managed_skill update; Task 2 sync gate + post-migrate + source-removal preservation tests)
- **Files modified:** 3 (1 src, 1 test, 1 planning doc)
- **Tests added/updated:** 5 new + 1 deferred test resolved
- **Total tests:** 558 unit + 141 integration (was 558 + 136) — all passing

## Accomplishments

- **Crate-root re-export of `manifest::hash_directory`** added to `crates/tome/src/lib.rs` with a doc comment explaining its purpose. Production code paths unchanged; integration tests now have one canonical hashing implementation to consume.
- **Synthetic v0.9 fixture builder** (`build_v09_fixture`) in `crates/tome/tests/cli.rs` mirrors the user's real library shape per CONTEXT.md `<specifics>`: 2 managed symlinks (p1, p2), 1 local real-dir (l1, already v0.10-shape), 1 broken symlink (D-04 boundary), 1 user-created symlink NOT in manifest (D-03 boundary). Manifest entries computed using the new `tome::hash_directory` re-export.
- **5 new integration tests** added to `crates/tome/tests/cli.rs`:
  - `migrate_library_converts_managed_symlinks_to_real_dirs` — anchors LIB-01 success criterion 1 (zero symlinks after sync for managed); checks D-03 (user-symlink preserved), D-04 (broken-symlink preserved + warned), D-05 (non-zero exit on partial), SAFE-01 banner format.
  - `migrate_library_dry_run_makes_no_changes` — `--dry-run` performs zero filesystem mutation; output mentions dry-run.
  - `sync_refuses_on_v09_shape_library_with_hint` — anchors D-02; sync exits non-zero, stderr contains both `v0.9 shape` and `tome migrate-library`, library not modified.
  - `sync_succeeds_after_migrate_library` — anchors success criterion 4 (idempotency / refuse-with-hint workflow); after migrate, sync's v0.9-shape gate no longer fires.
  - `sync_preserves_library_when_source_removed_from_config` — anchors LIB-04 / D-09 Case 1 / D-10 trigger 2 (success criterion 2); cleanup phase transitions orphan to Unowned (`source_name -> null`) and preserves library content + content_hash.
- **Resolved deferred-items.md entry**: `symlink_chain_managed_skill` (cli.rs:1775) updated from v0.9 shape (asserted `is_symlink()`) to v0.10 shape (`is_dir() && !is_symlink()` + `tome::hash_directory(library) == tome::hash_directory(source)`). Test passes; deferred-items.md entry annotated as RESOLVED.
- **No parallel SHA-256 helper** in the test file — `rg -n "fn sha256_dir_simple" crates/tome/tests/cli.rs` returns 0 matches. The plan's earlier draft included one; replaced with the production re-export per the checker's option (a) recommendation.
- **Full test suite green**: 558 unit + 141 integration tests pass. `cargo build --package tome` exits 0. No regressions introduced.

## Task Commits

1. **Task 0: Re-export `manifest::hash_directory` at crate root** — `e5bf045` (feat). One-line `pub use` in `lib.rs` with explanatory doc comment.
2. **Task 1: Add v0.9 fixture + migrate-library happy-path/dry-run tests + resolve symlink_chain_managed_skill** — `5e70031` (test). Adds the new test section, the fixture builder, the two migrate-library tests, and updates the deferred test in-place to assert the v0.10 shape.
3. **Task 2: Add sync refuse-with-hint, post-migrate, and source-removal preservation tests** — `078c5ec` (test). Adds the three remaining tests; documents the CFG-06 / `skills.is_empty()` workaround in the source-removal test comment.

## Files Created/Modified

- `crates/tome/src/lib.rs` — added `pub use manifest::hash_directory;` with doc comment (1 file changed, 7 insertions).
- `crates/tome/tests/cli.rs` — updated `symlink_chain_managed_skill` to v0.10 shape (Task 1); appended new v0.10 test section with `build_v09_fixture` + 5 new tests (Task 1 + Task 2). Net: +549 lines, -13 lines across both Task 1 and Task 2 commits.
- `.planning/phases/11-library-canonical-core/deferred-items.md` — `symlink_chain_managed_skill` annotated RESOLVED with commit pointer; new entry logging the pre-existing `SkillEntry::new_unowned` dead-code warning as Phase 14 territory.

## Decisions Made

- **Crate-root re-export of `tome::hash_directory` over a parallel SHA-256 helper.** Per the plan-checker's option (a): `hash_directory` is a pure deterministic function with no side effects and no internal-type dependencies beyond `ContentHash` (which is already crate-public). Exposing it has zero risk and saves us from maintaining a parallel implementation in tests that could drift from production. The re-export carries a doc comment marking it as test-helper provenance so future maintainers know its purpose.
- **Update `symlink_chain_managed_skill` in this plan's commit (Task 1).** The deferred-items.md entry from Plan 11-03 explicitly suggested Plan 11-05 as the right home. The fix needs the new `tome::hash_directory` re-export to verify content fidelity, so it makes the most sense to land it alongside the new tests. Documented in `<scope_safety>` of the prompt.
- **Source-removal test uses a filler `other` source with one skill.** `lib.rs::sync` has two early-exit guards: CFG-06 (`config.directories.is_empty()`) and `skills.is_empty()` (after discover). Both fire before cleanup runs. Removing the only source from config trips both. The test keeps `[directories.other]` in config with a single skill (`beta`) so sync proceeds past discover and reaches the cleanup phase, which then sees `alpha`'s `source_name = "local"` (no longer in `config.directories`) and applies the D-09 Case 1 transition. The test comment documents this explicitly so future readers don't think it's accidental.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Removed broken `let _ = unix_fs::symlink as fn(_, _) -> _;` no-op in `sync_preserves_library_when_source_removed_from_config`**

- **Found during:** Task 2 (compile failure).
- **Issue:** The plan's draft test included `let _ = unix_fs::symlink as fn(_, _) -> _;` as a "silence unused-import" comment-line. `unix_fs::symlink` is generic (`P: AsRef<Path>`, `Q: AsRef<Path>`) so the cast can't be inferred — `error[E0283]: type annotations needed`. The line was unnecessary anyway because the test never actually uses `unix_fs::symlink`.
- **Fix:** Deleted the `use std::os::unix::fs as unix_fs;` import + the cast line entirely. The test compiles cleanly and uses no symlinks (only real directories).
- **Files modified:** `crates/tome/tests/cli.rs`
- **Verification:** `cargo test --package tome --test cli -- sync_preserves_library_when_source_removed_from_config` passes.
- **Committed in:** `078c5ec` (Task 2 commit)

**2. [Rule 3 - Blocking] Added a filler skill to `other` source so sync reaches cleanup**

- **Found during:** Task 2 (test failure — manifest entry's `source_name` still equals `"local"` after sync).
- **Issue:** The plan's draft test removed the only source from config, expecting cleanup to fire. But `lib.rs::sync` has two early-exits before cleanup: CFG-06 ("warning: no directories configured") on empty config and another on `skills.is_empty()` after discover. With only an empty `other` source, discover returns 0 skills and sync exits before cleanup.
- **Fix:** Added a real `beta` skill in `other_source` so discover finds it, sync proceeds past both guards, and cleanup runs the D-09 Case 1 transition on `alpha`. Test comment documents the workaround so it's not mistaken for accidental complexity.
- **Files modified:** `crates/tome/tests/cli.rs`
- **Verification:** `cargo test --package tome --test cli -- sync_preserves_library_when_source_removed_from_config` passes; the manifest's `source_name` for `alpha` is omitted (per `skip_serializing_if = "Option::is_none"`) after sync.
- **Committed in:** `078c5ec` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 3 — blocking compile/test failures from minor planner-draft issues)
**Impact on plan:** Mechanical corrections to keep the plan's intent intact. The first removed dead syntax that didn't compile against the actual `unix_fs::symlink` signature. The second added a single filler skill to thread past sync's early-exit guards — the plan's narrative claim ("the cleanup phase transitions the orphan") is preserved; only the fixture shape changed to make the assertion reachable. Both auto-fixes are documented inline in the test code so future readers (especially HARD-13 file-split) understand the why.

## Issues Encountered

- **Pre-existing dead-code warning on `SkillEntry::new_unowned`.** `cargo build --package tome` and `cargo test --package tome` both pass with a warning (not an error). `cargo clippy --all-targets -- -D warnings` (and therefore `make ci`) fails on this warning because of the strict-warning policy. The warning is from Plan 11-01 (constructor lifted for use by Phase 14's `tome adopt`/`tome forget`), confirmed in 11-04-SUMMARY.md "Issues Encountered". Out of scope for Plan 11-05 per parallel-wave scope safety; logged in `deferred-items.md` as Phase 14 territory. Not introduced by this plan.

## User Setup Required

None — test-only changes plus a one-line `pub use` re-export. No new commands, configs, or migrations.

## Next Phase Readiness

- **Phase 12 (MarketplaceAdapter, ADP-01..04)** can now build on the proven LIB-01 invariant: managed skills are real-dir copies in the library. The `migrate_library_converts_managed_symlinks_to_real_dirs` test pins this end-to-end at the binary level. ClaudeMarketplaceAdapter's install/update flow can assume the library has zero symlinks for managed entries.
- **Phase 13 (RECON-01..05)** consumes the post-migration library shape; the `sync_succeeds_after_migrate_library` test confirms the v0.9 refuse-with-hint check no longer fires after migration. Drift detection (Phase 13) can run on the now-real-dir library.
- **Phase 14 (UNOWN-01..03)** can build on the now-tested Unowned manifest state: `sync_preserves_library_when_source_removed_from_config` proves the cleanup phase transitions correctly. `tome adopt` flips Unowned → Owned; `tome forget` removes the manifest entry + library directory. The `SkillEntry::new_unowned` dead-code warning will resolve naturally when `tome adopt` consumes it.
- **Phase 15 (HARD-13 file-split)** will eventually split `crates/tome/tests/cli.rs` (now 6128 lines) into multiple test files. The new v0.10 section is delimited with a clear comment banner, so the split can move it as a unit. The `build_v09_fixture` helper would naturally migrate to a shared `tests/common/v09_fixtures.rs` module if HARD-13 introduces one.
- **Phase 17 (REL-01..05)** smoke-test (REL-04) on Martin's real library has high confidence: the synthetic fixture mirrors the real layout per CONTEXT.md `<specifics>`, so behavior verified in synthetic carries to production.

## Self-Check: PASSED

- crates/tome/src/lib.rs: FOUND
- crates/tome/tests/cli.rs: FOUND
- .planning/phases/11-library-canonical-core/11-05-SUMMARY.md: FOUND
- .planning/phases/11-library-canonical-core/deferred-items.md: FOUND
- Commit e5bf045 (Task 0 — re-export hash_directory): FOUND
- Commit 5e70031 (Task 1 — fixture + migrate tests + deferred resolve): FOUND
- Commit 078c5ec (Task 2 — sync gate + preservation tests): FOUND
- `rg -n "pub use manifest::hash_directory" crates/tome/src/lib.rs`: 1 match
- `rg -n "fn build_v09_fixture" crates/tome/tests/cli.rs`: 1 match
- `rg -n "fn migrate_library_converts_managed_symlinks_to_real_dirs|fn migrate_library_dry_run_makes_no_changes|fn sync_refuses_on_v09_shape_library_with_hint|fn sync_succeeds_after_migrate_library|fn sync_preserves_library_when_source_removed_from_config" crates/tome/tests/cli.rs`: 5 matches
- `rg -n "tome::hash_directory" crates/tome/tests/cli.rs`: 6 matches (above the 3-match minimum)
- `rg -n "fn sha256_dir_simple" crates/tome/tests/cli.rs`: 0 matches (no parallel helper)
- `cargo test --package tome --test cli` exits 0 (141 tests pass)
- `cargo test --package tome` exits 0 (558 unit + 141 integration)
- `cargo build --package tome` exits 0

---
*Phase: 11-library-canonical-core*
*Completed: 2026-05-03*
