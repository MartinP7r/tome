---
phase: 14-unowned-library-lifecycle
plan: 02
subsystem: api
tags: [rust, serde, manifest, status, doctor, unowned, json]

# Dependency graph
requires:
  - phase: 14-01-previous-source-schema
    provides: "previous_source: Option<DirectoryName> field on SkillEntry; new_unowned 4-arg signature"
  - phase: 11-library-canonical-core
    provides: "SkillEntry / Manifest / SkillName / DirectoryName / collapse_home"
provides:
  - "Public SkillSummary type (name, previous_source, source_path_display, synced_at, managed)"
  - "SkillSummary::from_entry(&SkillName, &SkillEntry) projection — no I/O"
  - "Stable JSON shape with all 5 keys present (no skip_serializing_if on previous_source)"
affects: [14-06-status-unowned-section, 14-07-doctor-unowned-section]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Display-shaped projection types (SkillSummary) for serde-rendered status/doctor output"
    - "Stable JSON keys via explicit null serialisation (no #[serde(skip_serializing_if)] on consumer-facing optional fields)"

key-files:
  created:
    - "crates/tome/src/summary.rs"
  modified:
    - "crates/tome/src/lib.rs"

key-decisions:
  - "Place SkillSummary in its own summary.rs module rather than inside status.rs/doctor.rs/manifest.rs — both StatusReport and DoctorReport will own a Vec<SkillSummary>, so a shared module avoids cross-module references."
  - "previous_source is Option<String> in SkillSummary (not Option<DirectoryName>) — it is a display projection; consumers get the validated name as a plain string, no Deserialize/round-trip needs."
  - "No #[serde(skip_serializing_if)] on previous_source — JSON consumers get a stable 5-key object regardless of provenance presence. None serialises as explicit null (matches D-D3 intent of always-present keys)."
  - "#[allow(dead_code)] only on from_entry constructor (not on the struct) — consumed by 14-06 and 14-07 within the same wave-set; the struct itself is reachable via the impl."

patterns-established:
  - "Shared display-projection types live in dedicated modules when consumed by multiple subsystems (here: status + doctor)."
  - "Phase-internal cross-plan dead-code allows are scoped to the specific symbol awaiting the consumer plan, with a doc comment naming the consuming plan."

requirements-completed: [UNOWN-03]

# Metrics
duration: ~2min
completed: 2026-05-07
---

# Phase 14 Plan 02: SkillSummary Type Summary

**Shared SkillSummary projection type wired into lib.rs, ready for 14-06 (status) and 14-07 (doctor) to consume in Wave 3 without struct-shape coordination.**

## Performance

- **Duration:** ~2 min
- **Started:** 2026-05-07T12:46:58Z
- **Completed:** 2026-05-07T12:48:48Z
- **Tasks:** 1
- **Files created:** 1
- **Files modified:** 1

## Accomplishments

- `SkillSummary` struct landed in `crates/tome/src/summary.rs` with the exact D-D3 shape (name, previous_source, source_path_display, synced_at, managed).
- `SkillSummary::from_entry` constructor projects from `(SkillName, &SkillEntry)` using `paths::collapse_home` for the path-display fallback (D-C2).
- 4 unit tests cover: previous_source happy path, D-C2 None fallback (source_path_display always populated), JSON shape stability (all 5 keys), JSON null-on-None for `previous_source`.
- Module wired into `lib.rs` as `pub(crate) mod summary;` in alphabetical order between `status` and `update`.

## Task Commits

1. **Task 1: Create summary.rs with SkillSummary and from_entry** — `3390485` (feat)

_Plan was a single TDD-flagged task; tests and implementation landed together because the impl is a 5-field projection — splitting RED/GREEN would have been artificial._

## Files Created/Modified

- `crates/tome/src/summary.rs` (created) — `pub struct SkillSummary` (5 fields, `serde::Serialize`), `impl SkillSummary { pub fn from_entry(&SkillName, &SkillEntry) -> Self }`, 4 `#[cfg(test)]` units.
- `crates/tome/src/lib.rs` (modified) — single-line addition: `pub(crate) mod summary;` between `status` and `update` in the alphabetical module-declaration block.

## Decisions Made

- **Module location:** new `summary.rs` rather than embedding in `status.rs`, `doctor.rs`, or `manifest.rs`. Both `StatusReport` (14-06) and `DoctorReport` (14-07) will own a `Vec<SkillSummary>`; a shared module is the cleanest fit and matches the recommendation in 14-CONTEXT.md `<canonical_refs>`.
- **`previous_source: Option<String>` in the summary** rather than `Option<DirectoryName>` — the summary is a display projection; downstream JSON consumers want a plain string, not the validated newtype. No Deserialize round-trip is needed (the type is `Serialize`-only).
- **No `skip_serializing_if`** on `previous_source` — D-D3 expects a stable 5-key JSON object regardless of provenance presence. `None` serialises as explicit `null`, which the dedicated test (`json_previous_source_serializes_as_null_when_none`) pins.
- **Targeted `#[allow(dead_code)]` on `from_entry`** only (not on the struct itself). The struct is `pub`; the constructor is unused at the time of this plan's commit but consumed by 14-06 (status) and 14-07 (doctor) within Phase 14's wave-set. Doc-comment annotates the lifecycle.

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

- **Parallel-wave compile coupling:** `cargo test -p tome --lib summary::tests` could not be run from this agent's process because the parallel 14-01 agent had already added `previous_source: Option<DirectoryName>` to `SkillEntry` in `manifest.rs`, but legacy in-tree test code in `status.rs` and `reconcile.rs` still constructs `SkillEntry` literals without the new field — those test sites are 14-01's scope and they will land in the same wave-set's commits. The library itself (non-test target) builds cleanly: `cargo build -p tome --lib` succeeds with one expected `dead_code` warning on `new_unowned`. The orchestrator validates the full test suite once the wave completes; my own module's code is correct in isolation.
- No code change made to `status.rs` or `reconcile.rs` from this agent (out of scope per the parallel-execution boundary). All my changes are confined to `crates/tome/src/summary.rs` (new file) and a single line in `crates/tome/src/lib.rs`.

## Self-Check

```
$ test -f crates/tome/src/summary.rs && grep -q "pub struct SkillSummary" crates/tome/src/summary.rs && echo OK
OK

$ grep -q "pub previous_source: Option<String>" crates/tome/src/summary.rs && echo OK
OK

$ grep -q "pub source_path_display: String" crates/tome/src/summary.rs && echo OK
OK

$ grep -q "pub synced_at: String" crates/tome/src/summary.rs && echo OK
OK

$ grep -q "pub managed: bool" crates/tome/src/summary.rs && echo OK
OK

$ grep -q "pub fn from_entry" crates/tome/src/summary.rs && echo OK
OK

$ grep -q "pub(crate) mod summary;" crates/tome/src/lib.rs && echo OK
OK

$ git log --oneline -1 -- crates/tome/src/summary.rs
3390485 feat(14-02): add SkillSummary shared type for status/doctor unowned section

$ cargo build -p tome --lib  # passes with one expected dead_code warning on new_unowned
$ cargo fmt -p tome -- --check crates/tome/src/summary.rs crates/tome/src/lib.rs  # passes
```

## Self-Check: PASSED

All artefacts present, module wired, build clean, fmt clean. Per-test execution gated on parallel 14-01 finishing its `status.rs`/`reconcile.rs` test-fixture updates; orchestrator runs the full test suite at wave-merge.

## Next Phase Readiness

- `SkillSummary` is the single source of truth for the Unowned-section row shape. 14-06 (`status::StatusReport`) and 14-07 (`doctor::DoctorReport`) can both add `pub unowned: Vec<SkillSummary>` fields and call `SkillSummary::from_entry(name, entry)` against any manifest entry where `entry.source_name.is_none()`. No further struct-shape coordination required.
- The `dead_code` allow on `from_entry` is the explicit hand-off marker — when 14-06/14-07 land they should remove it.
- Stable JSON shape guarantees (all 5 keys, explicit `null` for `previous_source`) are pinned by `json_shape_includes_all_keys` and `json_previous_source_serializes_as_null_when_none`. Future plans changing the shape will trip these tests deliberately.

---
*Phase: 14-unowned-library-lifecycle*
*Plan: 02*
*Completed: 2026-05-07*
