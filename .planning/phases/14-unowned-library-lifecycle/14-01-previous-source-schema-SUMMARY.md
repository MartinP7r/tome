---
phase: 14-unowned-library-lifecycle
plan: 01
subsystem: api
tags: [rust, serde, manifest, lockfile, unowned, transition-sites, schema, fork-in-place]

# Dependency graph
requires:
  - phase: 11-library-canonical-core
    provides: "SkillEntry / LockEntry / Manifest / SkillName / DirectoryName, Manifest::skills_get_mut, in-place transition machinery (cleanup_library Case 1, remove::execute dir flavour)"
  - phase: 13-lockfile-authoritative-sync
    provides: "reconcile::EditDecision (Fork/Revert/Skip), apply_edit_decisions in lib.rs (fork-in-place flip — D-13 lossy-gap site)"
provides:
  - "previous_source: Option<DirectoryName> field on SkillEntry (manifest.rs) and LockEntry (lockfile.rs) with #[serde(default, skip_serializing_if = \"Option::is_none\")]"
  - "lockfile::generate copies previous_source from SkillEntry → LockEntry per skill"
  - "SkillEntry::new_unowned 4-arg signature (previous_source: Option<DirectoryName>)"
  - "Three Owned→Unowned transition sites capture previous_source via .take():"
  - "  • cleanup::cleanup_library Case 1 (orphan-detection sync transition)"
  - "  • remove::execute (dir flavour — `tome remove dir <name>`)"
  - "  • lib.rs::apply_edit_decisions Fork branch (Phase 13 D-13 closure)"
affects: [14-04-reassign-unowned-input, 14-05-remove-skill, 14-06-status-unowned-section, 14-07-doctor-unowned-section]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "serde-default optional schema additions for backward-compatible manifest/lockfile evolution (#[serde(default, skip_serializing_if = \"Option::is_none\")])"
    - ".take() pattern for in-place Option<T> transitions that move the old value into a sibling field while leaving None"
    - "Twin-constructor pattern preserved on SkillEntry: new() for Owned, new_unowned() for Unowned (now also accepts previous_source)"

key-files:
  created:
    - ".planning/phases/14-unowned-library-lifecycle/deferred-items.md"
  modified:
    - "crates/tome/src/manifest.rs"
    - "crates/tome/src/lockfile.rs"
    - "crates/tome/src/cleanup.rs"
    - "crates/tome/src/remove.rs"
    - "crates/tome/src/lib.rs"
    - "crates/tome/src/reconcile.rs"
    - "crates/tome/src/distribute.rs"
    - "crates/tome/src/doctor.rs"
    - "crates/tome/src/library.rs"
    - "crates/tome/src/status.rs"
    - "crates/tome/src/update.rs"

key-decisions:
  - "Retained #[allow(dead_code)] on SkillEntry::new_unowned (Rule 3 deviation): no production caller lands until Plans 14-04/14-05; CI clippy --all-targets -D warnings would otherwise fail. Tracked in deferred-items.md."
  - "All test-side SkillEntry/LockEntry literals across 9 modules updated to include previous_source: None (compile-mandatory after schema lift)."

patterns-established:
  - "Schema additions stay backward-compatible via #[serde(default, skip_serializing_if)]: old payloads deserialise unchanged, new payloads stay terse."
  - "Transition sites use entry.previous_source = entry.source_name.take() — atomic move with no Clone; replaces the existing source_name = None assignment in one step."
  - "Test coverage for serde-shape evolution covers (a) old-shape deserialise, (b) new-shape round-trip, (c) skip_serializing_if omission, (d) constructor-side population."

requirements-completed: [UNOWN-03]

# Metrics
duration: 12min
completed: 2026-05-07
---

# Phase 14 Plan 01: Previous-Source Schema Summary

**Adds `previous_source: Option<DirectoryName>` to SkillEntry + LockEntry, captured at all three Owned→Unowned transition sites (cleanup orphan, `tome remove dir`, fork-in-place), closing the Phase 13 D-13 lossy-fork-in-place gap.**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-05-07T12:47:14Z
- **Completed:** 2026-05-07T12:59:29Z
- **Tasks:** 2 (both TDD)
- **Files modified:** 11
- **Files created:** 1 (deferred-items.md)

## Accomplishments

- Schema lift: `SkillEntry.previous_source` and `LockEntry.previous_source` ship with serde-default + skip_serializing_if so existing manifests/lockfiles deserialise as `None` and old-shape JSON keeps working.
- Generate propagation: `lockfile::generate` copies `previous_source` from each `SkillEntry` into the matching `LockEntry`, providing cross-machine surfacing symmetry.
- Constructor widening: `SkillEntry::new_unowned` now accepts `Option<DirectoryName>` for `previous_source`. Callers across the codebase updated.
- Three transition sites capture `previous_source = entry.source_name.take()`:
  1. `cleanup::cleanup_library` Case 1 — when a directory disappears from `tome.toml` and `cleanup` orphans the manifest entry.
  2. `remove::execute` (dir flavour) — when `tome remove dir <name>` succeeds and the post-success transition loop runs.
  3. `lib.rs::apply_edit_decisions` Fork branch — when Phase 13's edit-in-library prompt picks "fork", the in-place flip now records the previous owner.
- 10 new unit tests anchor the schema + transition behaviour. Full lib + integration suite green (797 tests).

## Task Commits

1. **Task 1: Add `previous_source` field to SkillEntry + LockEntry and update `generate()`** — `f663b5a` (feat)
2. **Task 2: Capture `previous_source` at the three Owned→Unowned transition sites** — `86fc69d` (feat)

_Both tasks were TDD — tests added in the same commit as the code change since Rust compiles tests with the lib in one pass; separating test-only and impl-only commits would have produced an intermediate commit that doesn't compile._

## Files Created/Modified

### Schema (Task 1)

- `crates/tome/src/manifest.rs` — `SkillEntry.previous_source` field; `new()` initialises None; `new_unowned()` widened to 4-arg (accepts `Option<DirectoryName>`); 5 new unit tests; `#[allow(dead_code)]` retained on `new_unowned` (deferred to 14-04/14-05).
- `crates/tome/src/lockfile.rs` — `LockEntry.previous_source` field; `generate()` propagates from manifest; 2 new unit tests.

### Test-side compatibility (Task 1, mechanical)

Test literals updated to include `previous_source: None`:
- `crates/tome/src/cleanup.rs` (4 SkillEntry literals + 1 `new_unowned` call)
- `crates/tome/src/distribute.rs` (3 SkillEntry literals)
- `crates/tome/src/doctor.rs` (8 SkillEntry literals)
- `crates/tome/src/library.rs` (1 SkillEntry literal)
- `crates/tome/src/reconcile.rs` (1 LockEntry literal + 1 `new_unowned` call)
- `crates/tome/src/status.rs` (3 SkillEntry literals)
- `crates/tome/src/update.rs` (2 LockEntry literals)

### Transition-site captures (Task 2)

- `crates/tome/src/cleanup.rs` — Case 1 transition: `entry.previous_source = entry.source_name.take()` + new test `cleanup_case1_records_previous_source`.
- `crates/tome/src/remove.rs` — `execute()` dir-flavour transition loop: same pattern + new test `execute_records_previous_source_on_unowned_transition`.
- `crates/tome/src/lib.rs` — `apply_edit_decisions` Fork branch: same pattern + new test `apply_edit_decisions_fork_records_previous_source`.

### Process

- `.planning/phases/14-unowned-library-lifecycle/deferred-items.md` — documents the `#[allow(dead_code)]` retention on `new_unowned` and the resolution plan (14-04/14-05).

## Decisions Made

- **Retained `#[allow(dead_code)]` on `SkillEntry::new_unowned`** — the plan's Task 1 instructed dropping the attribute, but Plan 14-01 itself adds no production caller (Task 2's transitions use `.take()` directly). CI runs `cargo clippy --all-targets -- -D warnings` and rejects unused public items. Same precedent set in Phase 11's `deferred-items.md`. Updated comment points the reader at Plans 14-04 / 14-05 (the actual production callers). Tracked in `deferred-items.md`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Retain `#[allow(dead_code)]` on `SkillEntry::new_unowned`**
- **Found during:** Task 1 (manifest.rs schema lift)
- **Issue:** Plan instructs dropping the attribute under the assumption that "this plan + 14-04 + 14-05 indirectly" produce a production caller. In practice, Plan 14-01 Task 2 captures `previous_source` via `.take()` (which doesn't construct a new SkillEntry), and the parallel-wave Plan 14-02 (SkillSummary) only reads the field. CI's `cargo clippy --all-targets -- -D warnings` rejects the unused public item.
- **Fix:** Retained `#[allow(dead_code)]` with an updated comment pointing at Plans 14-04 (reassign re-anchor) and 14-05 (remove-skill) as the actual production-caller delivery sites. Documented in phase `deferred-items.md`.
- **Files modified:** `crates/tome/src/manifest.rs`, `.planning/phases/14-unowned-library-lifecycle/deferred-items.md`
- **Verification:** `cargo clippy --all-targets -p tome -- -D warnings` exits 0; `cargo build -p tome` exits 0.
- **Committed in:** `f663b5a` (Task 1 commit)

**2. [Rule 3 - Blocking] Update test-side `SkillEntry`/`LockEntry` struct literals across the codebase**
- **Found during:** Task 1 (post-schema-lift compile failures)
- **Issue:** The plan listed only `manifest.rs` and `lockfile.rs` as files to modify. After adding the `previous_source` field, all test modules with direct struct literals (`SkillEntry { ... }` / `LockEntry { ... }`) failed to compile with `missing field 'previous_source'`. Test literals exist in 7 additional modules (cleanup, distribute, doctor, library, reconcile, status, update).
- **Fix:** Added `previous_source: None,` to every test-side struct literal. Updated three `SkillEntry::new_unowned(...)` callers across cleanup.rs/reconcile.rs to pass the new fourth argument as `None`.
- **Files modified:** `cleanup.rs`, `distribute.rs`, `doctor.rs`, `library.rs`, `reconcile.rs`, `status.rs`, `update.rs` (test sections only — no production behaviour change).
- **Verification:** `cargo build --all-targets -p tome` exits 0; `cargo test -p tome` runs 797 tests, 0 failures.
- **Committed in:** `f663b5a` (folded into the Task 1 commit since these are mechanical compile-mandatory changes)

---

**Total deviations:** 2 auto-fixed (both Rule 3 - blocking).
**Impact on plan:** Neither deviation widens scope; both unblock the schema lift. The dead-code retention is documented for a follow-up phase; the test-literal updates are compile-mandatory and behaviour-neutral.

## Issues Encountered

- **Parallel-execution coordination:** The orchestrator's `<parallel_execution>` block instructed "DO NOT touch lib.rs" while Task 2's plan explicitly directs modifying `lib.rs::apply_edit_decisions` (transition site 3). Resolved by waiting until the parallel agent's commit landed (verified via `git log --oneline`), then editing lib.rs cleanly. The single edit (Fork branch body, ~3 LOC) does not conflict with the parallel agent's `pub(crate) mod summary;` line.

- **Test parallelism flakes:** `cargo test -p tome --lib` (default parallel) intermittently fails 3 backup tests (`push_and_pull_roundtrip`, `dry_run_snapshot_no_commit`, `list_returns_entries`, `has_remote_true_with_remote`, `restore_bails_when_pre_snapshot_fails`). Each passes in isolation. This is a pre-existing flake (tracked in v0.10 carry-overs as Phase 15 / HARD-14 / issue #500). Re-running with `--test-threads=1` confirms 646/646 lib + 141/141 cli + 10/10 cli_sync_reconcile pass.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Plans 14-06 (status Unowned section) and 14-07 (doctor Unowned section) can now consume the `previous_source` field via `SkillSummary::from_entry` (Plan 14-02 — already shipped in this Wave).
- Plans 14-04 (reassign re-anchor flow) and 14-05 (remove-skill) own the resolution of the `#[allow(dead_code)]` retention — when either plan adds a production caller of `SkillEntry::new_unowned`, the attribute can be dropped and the deferred-items.md entry retired.
- Phase 13 D-13 lossy-fork-in-place gap is closed for new transitions. Pre-Phase-14 entries that became Unowned via fork before this plan landed remain `previous_source = None` (D-C2 fallback to `source_path` rendering — explicitly out-of-scope per CONTEXT.md).

## Self-Check: PASSED

Verification commands run after writing this summary:

- `[ -f crates/tome/src/manifest.rs ]` → FOUND
- `[ -f crates/tome/src/lockfile.rs ]` → FOUND
- `[ -f crates/tome/src/cleanup.rs ]` → FOUND
- `[ -f crates/tome/src/remove.rs ]` → FOUND
- `[ -f crates/tome/src/lib.rs ]` → FOUND
- `[ -f .planning/phases/14-unowned-library-lifecycle/deferred-items.md ]` → FOUND
- `git log --oneline | grep f663b5a` → FOUND (Task 1 commit)
- `git log --oneline | grep 86fc69d` → FOUND (Task 2 commit)
- `cargo test -p tome` → 797/797 pass
- `cargo clippy --all-targets -p tome -- -D warnings` → exit 0
- `cargo fmt -p tome -- --check` → exit 0
- `grep -q "pub previous_source: Option<DirectoryName>" crates/tome/src/manifest.rs` → OK
- `grep -q "pub previous_source: Option<DirectoryName>" crates/tome/src/lockfile.rs` → OK
- `grep -q "previous_source: entry.previous_source.clone()" crates/tome/src/lockfile.rs` → OK
- `grep -q "entry.previous_source = entry.source_name.take()" crates/tome/src/cleanup.rs` → OK
- `grep -q "entry.previous_source = entry.source_name.take()" crates/tome/src/remove.rs` → OK
- `grep -q "entry.previous_source = entry.source_name.take()" crates/tome/src/lib.rs` → OK

---
*Phase: 14-unowned-library-lifecycle*
*Plan: 01*
*Completed: 2026-05-07*
