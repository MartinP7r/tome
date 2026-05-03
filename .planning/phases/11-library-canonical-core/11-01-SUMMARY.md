---
phase: 11-library-canonical-core
plan: 01
subsystem: persistence
tags: [manifest, lockfile, serde, schema, option, libcanonical]

# Dependency graph
requires:
  - phase: 10-phase-8-review-tail
    provides: source_name typed as DirectoryName (PR #504); FailureKind ALL exhaustive-match invariant; arboard patch-pin
provides:
  - SkillEntry.source_name lifted to Option<DirectoryName> with serde default + skip_serializing_if
  - SkillEntry::new_unowned constructor (LIB-04 Unowned construction)
  - Manifest::skills_get_mut (pub(crate)) accessor for downstream Plan 11-03 transitions
  - LockEntry.source_name lifted to Option<DirectoryName> mirroring manifest schema
  - resolved_paths_from_lockfile_cache safely skips Unowned entries
  - 8 new manifest serde + accessor tests
  - 4 new lockfile Option-shape serde tests
  - LIB-02 doc shift on Manifest.managed (now: "update channel" indicator)
affects: [11-02, 11-03, 11-04, 11-05, 12, 13, 14]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Option<DirectoryName> with serde default + skip_serializing_if for backwards-compatible field widening"
    - "Twin constructors (new + new_unowned) preserve every existing call-site signature when widening domain types"

key-files:
  created: []
  modified:
    - crates/tome/src/manifest.rs
    - crates/tome/src/lockfile.rs
    - crates/tome/src/cleanup.rs
    - crates/tome/src/distribute.rs
    - crates/tome/src/doctor.rs
    - crates/tome/src/install.rs
    - crates/tome/src/library.rs
    - crates/tome/src/reassign.rs
    - crates/tome/src/remove.rs
    - crates/tome/src/status.rs
    - crates/tome/src/update.rs

key-decisions:
  - "Lift Manifest::skills_get_mut into Plan 11-01 (rather than 11-03) so all manifest.rs touches stay contained to one plan"
  - "Reassign on an Unowned skill returns an explicit error pointing at Phase 14's tome adopt — avoids silent fallback to a synthetic source name"
  - "Cleanup/update group-by-source labels None as 'unknown' / 'unowned' — keeps existing UX shape; Phase 16 UX-01 will refine the wording"

patterns-established:
  - "Schema widening via Option<T> + #[serde(default, skip_serializing_if = \"Option::is_none\")]: old data with the field present parses as Some(...), new data omitting the key parses as None"
  - "Constructor split (new + new_*): keep the original signature for the common case so call-sites don't churn when adding a new domain state"

requirements-completed: [LIB-03]

# Metrics
duration: 30min
completed: 2026-05-03
---

# Phase 11 Plan 01: Manifest + Lockfile Schema Summary

**`SkillEntry.source_name` and `LockEntry.source_name` widened to `Option<DirectoryName>` (serde default + skip_serializing_if) so the v0.10 Unowned state is representable end-to-end; old manifests/lockfiles parse unchanged via serde's natural Option handling.**

## Performance

- **Duration:** ~30 min
- **Started:** 2026-05-03T13:08:36Z
- **Completed:** 2026-05-03T13:20:08Z
- **Tasks:** 2 (Task 1 manifest, Task 2 lockfile)
- **Files modified:** 11

## Accomplishments

- `SkillEntry.source_name` widened from `DirectoryName` to `Option<DirectoryName>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`; old `"source_name": "foo"` and new `null`/missing both round-trip correctly.
- `SkillEntry::new(...)` signature unchanged (wraps in `Some` internally) so every existing owned-construction call-site keeps compiling without modification.
- New `SkillEntry::new_unowned(source_path, content_hash, managed)` constructor for the Unowned state (LIB-04, D-13).
- New `Manifest::skills_get_mut(name) -> Option<&mut SkillEntry>` accessor (`pub(crate)`) lifted into Plan 11-01 so Plan 11-03's `cleanup_library` Case 1 transition can happen without re-touching `manifest.rs`.
- `LockEntry.source_name` mirror lift (D-14). `resolved_paths_from_lockfile_cache` now skips Unowned entries with an inline rationale comment.
- `Manifest.managed` doc updated for LIB-02's "update channel" semantic shift (field semantics, no code change).
- 8 new manifest tests + 4 new lockfile tests pin both old-shape compatibility and new Unowned round-trip behavior.
- All 538 unit + 136 integration tests pass on macOS.

## Task Commits

1. **Task 1: Lift `SkillEntry`/`LockEntry` `source_name` to `Option<DirectoryName>`** — `f869e03` (feat). Includes the manifest schema lift, the matching lockfile schema lift (required for the crate to compile), the new `SkillEntry::new_unowned` constructor, the new `Manifest::skills_get_mut` accessor, the `Manifest.managed` doc update, all blocking call-site adjustments (Rule 3 auto-fix), and the 8 new manifest tests.
2. **Task 2: Add LockEntry Option<DirectoryName> serde tests** — `cbcc0dd` (test). Adds the 4 new lockfile tests pinning old-shape parse, null parse, missing-key parse, and Unowned-omits-key serialize.

## Files Created/Modified

- `crates/tome/src/manifest.rs` — schema lift, twin constructors, `skills_get_mut` accessor, `managed` doc, 8 new tests
- `crates/tome/src/lockfile.rs` — mirror schema lift, `resolved_paths_from_lockfile_cache` Unowned-safe loop, assertion-shape updates, 4 new tests, LockEntry test fixtures wrapped in `Some(...)`
- `crates/tome/src/cleanup.rs` — `stale_by_source` Option-aware label ("unknown" for None)
- `crates/tome/src/distribute.rs` — circular-symlink skip-self check uses `Option<DirectoryName>` compare
- `crates/tome/src/doctor.rs` — test fixtures wrapped in `Some(...)` for `SkillEntry` literal constructions
- `crates/tome/src/install.rs` — LockEntry test fixture wrapped in `Some(...)`
- `crates/tome/src/library.rs` — test fixtures wrapped (manifest sites) and `DiscoveredSkill` fixtures left as-is (its `source_name` is unchanged)
- `crates/tome/src/reassign.rs` — `plan()` errors on Unowned skill with hint at Phase 14's `tome adopt`
- `crates/tome/src/remove.rs` — `plan()` filter only matches `Some(name)`
- `crates/tome/src/status.rs` — test fixtures wrapped in `Some(...)`
- `crates/tome/src/update.rs` — diff source-label fallback "unowned" for None; test fixtures wrapped

## Decisions Made

- **Lift `Manifest::skills_get_mut` here (Plan 11-01) rather than Plan 11-03.** Keeps `manifest.rs` touches contained to a single plan and lets Plan 11-03's `cleanup_library` Case 1 transition implementation be the only `cleanup.rs` change in that plan.
- **`reassign.rs::plan` returns an explicit error on Unowned skills** rather than synthesising a fallback source name. Phase 14's `tome adopt` is the right migration path; this surfaces it loudly. Documented in the error message with a phase pointer.
- **`cleanup.rs` keeps the existing `stale_by_source` shape** with "unknown" as the None label. Phase 16 UX-01 will rewrite the cleanup messaging into the 3-bucket partition; not in scope here.
- **`update.rs` diff grouping uses "unowned" for None entries.** Mirrors `cleanup.rs`'s pattern but uses a more semantic label since `update.rs` is about diffing lockfile entries (not stale skills).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Lockfile schema lifted in Task 1 commit (not held for Task 2)**

- **Found during:** Task 1 (compile failure after manifest schema lift)
- **Issue:** `lockfile::generate()` clones `entry.source_name` from a manifest entry into a new `LockEntry`. After the manifest field became `Option<DirectoryName>` but before `LockEntry.source_name` lifted, the assignment failed to compile (type mismatch).
- **Fix:** Bundled `LockEntry`'s schema lift (Task 2 step 1) and the `resolved_paths_from_lockfile_cache` Unowned-safe loop (Task 2 step 6) into the Task 1 commit so the crate compiles atomically. Task 2 commit only adds the 4 new lockfile tests.
- **Files modified:** `crates/tome/src/lockfile.rs`
- **Verification:** `cargo build --package tome` succeeds; full unit suite passes after both commits.
- **Committed in:** `f869e03` (Task 1 commit)

**2. [Rule 3 - Blocking] Call-site adjustments across 9 modules required to keep the crate compiling**

- **Found during:** Task 1 (compile failures across crate)
- **Issue:** Lifting `source_name` to `Option` broke every site that:
  - constructed `SkillEntry { source_name: DirectoryName::new(...), ... }` literally (vs `SkillEntry::new(...)`)
  - constructed `LockEntry { source_name: DirectoryName::new(...), ... }` literally
  - accessed `entry.source_name.as_str()` directly
  - compared `entry.source_name == name` against a `&str`
- **Fix:** Wrapped all literal `source_name` field assignments in `Some(...)`; threaded `Option` through accessors via `as_ref().map(...)` / `is_some_and(...)` patterns; explicit error in `reassign.rs` for the Unowned case.
- **Files modified:** `cleanup.rs`, `distribute.rs`, `doctor.rs`, `install.rs`, `library.rs`, `reassign.rs`, `remove.rs`, `status.rs`, `update.rs`, plus `lockfile.rs` LockEntry test fixtures (all bundled into Task 1 commit per #1 above).
- **Verification:** `cargo test --package tome` passes 538 unit + 136 integration tests.
- **Committed in:** `f869e03` (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 3 — blocking compile fixes triggered by the schema lift)
**Impact on plan:** Mechanical type-system propagation. No scope creep, no architectural changes. The plan's `<interfaces>` block flagged `SkillEntry::new` signature stability as a goal; this was preserved (twin constructors). The plan's "Bundle these together" implication for Task 1 + Task 2 step 1+2+6 was the cleanest atomic-commit path; Task 2 commit kept its plan-prescribed scope (the 4 new tests).

## Issues Encountered

- **Test fixture sweep tool over-wrapped `DiscoveredSkill` literals.** A perl one-liner that wrapped `source_name: DirectoryName::new(...).unwrap()` in `Some(...)` caught 5 `DiscoveredSkill` constructions whose `source_name` field is still `DirectoryName` (not `Option<DirectoryName>`). Manually reverted in `library.rs` (4 sites) and `lockfile.rs::make_discovered` (1 site).
- **`cargo test ... manifest::tests lockfile::tests` syntax.** Cargo no longer accepts multiple positional test patterns directly; needed `--` separator. Workflow command shape: `cargo test --package tome --lib -- manifest::tests lockfile::tests`.

## User Setup Required

None — schema-only change with backward-compatible serde defaults. No on-disk migration needed.

## Next Phase Readiness

Plans 11-02 through 11-05 can now build on the new schema:

- **11-02 (consolidate-managed-as-copy)** uses `SkillEntry::new` (signature unchanged) for the new copy-based managed flow.
- **11-03 (cleanup-case1-unowned)** uses the new `Manifest::skills_get_mut` accessor + `SkillEntry.source_name = None` mutation pattern for Case 1 transitions.
- **11-04 (remove-sets-source-none)** sets `entry.source_name = None` directly via `skills_get_mut` for the explicit `tome remove` path (D-10 trigger 1).
- **11-05 (migration_v010 module)** consumes the lockfile shape that's now Unowned-aware.

Phase 13 drift detection (RECON-01..05) will read `LockEntry.content_hash` (the authoritative drift signal per D-08) — that field shape is unchanged.

Phase 14 surfacing (UNOWN-03 in `tome status` / `tome doctor`) will read `SkillEntry.source_name == None` to detect Unowned entries — the data structure is now in place.

## Self-Check: PASSED

- crates/tome/src/manifest.rs: FOUND
- crates/tome/src/lockfile.rs: FOUND
- .planning/phases/11-library-canonical-core/11-01-SUMMARY.md: FOUND
- Commit f869e03 (Task 1 — manifest+lockfile schema lift): FOUND
- Commit cbcc0dd (Task 2 — LockEntry Option tests): FOUND

---
*Phase: 11-library-canonical-core*
*Completed: 2026-05-03*
