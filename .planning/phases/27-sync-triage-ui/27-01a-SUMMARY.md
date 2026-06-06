---
phase: 27-sync-triage-ui
plan: 01a
subsystem: rust-domain
tags: [rust, progress, manifest, sink, discover, sync-pipeline, d-08, d-09, d-16, tauri]

# Dependency graph
requires:
  - phase: 25-rust-core-extraction-tauri-integration-spike
    provides: "ProgressEvent + ProgressSink + SyncStage trait vocabulary (D-09/D-10/D-11), TauriEventSink stub, CancelToken (D-12), bindings feature gate"
  - phase: 26-read-only-views-alpha-cut
    provides: "ListReport + DiscoveredSkill serde shape (consumed by Skills view), VIEW-02 carryover #2 (Recent sort) plumbing-only delta"
provides:
  - "ProgressEvent::SyncStageProgress.item: Option<String> field (D-08) with per-stage assignment semantics documented in the type"
  - "DiscoveredSkill.synced_at: Option<String> field (D-16) populated from the manifest at the post-discover boundary of sync()"
  - "join_synced_at_from_manifest(skills, manifest) helper in lib.rs — directly unit-testable"
  - "SyncProgress mirror struct gains item: Option<String> + PartialEq/Eq for testability"
  - "event_to_sync_progress(event) -> SyncProgress pure conversion in tome-desktop::sink (extracted from TauriEventSink::emit for unit-testability)"
  - "D-09 sink-side fold-in: GitCloneProgress → Reconcile + Some(\"git: <dir> (<bytes>)\"); BackupSnapshot → Save + Some(message)"
  - "format_bytes(u64) -> String IEC-prefixed byte formatter (sink-private, 1 decimal, B/KiB/MiB/GiB/TiB)"
  - "RecordingSink event-order test pinning Pitfall 4 / Assumption A4 (Reconcile-started precedes first GitCloneProgress at the sink-input level)"
affects: [27-01b, 27-02, 27-02b, 27-03, 27-04, 27-05]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pure conversion functions extracted for testability — event_to_sync_progress + format_bytes mirror the join_synced_at_from_manifest pattern: take the side effect out of the trait impl, leave a pure fn behind, test that fn directly."
    - "Per-stage subtitle semantics documented as a doc comment on the field itself, not in a separate spec. The struct's doc carries the per-stage table (Discover → dir name, Consolidate/Distribute → skill name, Cleanup → path, Save → filename, Reconcile → None)."

key-files:
  created: []
  modified:
    - "crates/tome/src/progress.rs"
    - "crates/tome/src/discover.rs"
    - "crates/tome/src/list.rs"
    - "crates/tome/src/lib.rs"
    - "crates/tome/src/library.rs"
    - "crates/tome/src/lockfile.rs"
    - "crates/tome-desktop/src/sink.rs"

key-decisions:
  - "Per-stage emission interpretation: only update the ONE existing SyncStageProgress emission site (Distribute, lib.rs:2100) with item: Some(name.to_string()) using the DirectoryName already in scope. The plan's per-stage assignment list (Consolidate → skill name, Cleanup → path, Save → filename) documents the SEMANTIC for future plans that add per-skill emission inside library::consolidate / distribute::distribute_to_directory / cleanup::cleanup_* — those sites do NOT exist today, and threading sinks into those submodules would violate the 5-file scope cap. The future-state assignment is captured in the doc comment on the field itself so the next plan adding a per-skill emission has the contract in front of it."
  - "Pitfall 4 RecordingSink test is fixture-only (emits two events directly into a RecordingSink). The plan explicitly notes the real-pipeline ordering check belongs in 27-04's sync_cancel.rs. The fixture test pins the sink-input contract; an end-to-end pipeline test is out of scope for 27-01a."
  - "join_synced_at_from_manifest extracted into a pub(crate) helper rather than inlined inside the sync() Discover span block — makes the join directly unit-testable against an in-memory Manifest fixture without spinning a full TempDir+config+sync roundtrip."
  - "event_to_sync_progress extracted from TauriEventSink::emit for the same reason — pure function, no AppHandle, unit-testable in isolation. emit() now delegates to it (single statement)."
  - "format_bytes uses IEC binary prefixes (KiB/MiB/GiB/TiB) with 1 decimal place. Pathological u64::MAX clamps to TiB (the largest unit), not panics. Tests pin the small-value (B), KiB-promotion (1024 → 1.0 KiB), and large-value (MiB exact + GiB approximate + TiB pathological) shapes separately."
  - "Skipped CLI snapshot regeneration. tome list --json now serializes synced_at on every entry (null for unstamped skills), but the existing snapshots in crates/tome/tests/snapshots/ don't include any list --json output, so no insta review was needed. Verified by running cargo test -p tome --test cli_list (5/5 pass) and grepping snapshots for synced_at."

patterns-established:
  - "Boundary-pure conversion + thin emit() shell: when the sink implementation requires an AppHandle (untestable in isolation), extract a pure event_to_sync_progress(event) fn that does ALL the routing/formatting logic, then have emit() just call it and dispatch the result. Tests exercise the pure fn; emit() reduces to a single line."
  - "Layering rule: discover scans the filesystem and produces DiscoveredSkill; the manifest is owned by sync() and joined in at the post-discover boundary. The synced_at field is initialized to None at the construction site (discover.rs scan_for_skills) and populated by the orchestrator (lib.rs join_synced_at_from_manifest). Pinned by discover_all_leaves_synced_at_none."

requirements-completed:
  - SYNC-01  # Rust-side substrate only. 27-01b ships the Tauri boundary + React skeleton that closes SYNC-01 user-visible.

# Metrics
duration: 35min
completed: 2026-06-06
---

# Phase 27 Plan 01a: Rust domain foundational scaffolding for SYNC-01 Summary

**Typed Rust substrate for Phase 27 — D-08 ProgressEvent.item + D-16 DiscoveredSkill.synced_at + D-09 sink-side fold-in for GitCloneProgress (Reconcile) and BackupSnapshot (Save) + Pitfall 4 ordering pin via RecordingSink. Zero dependencies added, zero CLI regression.**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-06-06T11:41:00Z (approx)
- **Completed:** 2026-06-06T12:16:38Z
- **Tasks:** 3 (all atomic, TDD-style)
- **Files modified:** 7

## Accomplishments

- D-08 substrate landed: `ProgressEvent::SyncStageProgress` carries an optional per-unit subtitle (`item: Option<String>`); the single existing emission site in `lib.rs::sync` (Distribute) passes the current `DirectoryName`; `RecordingSink` round-trips the field byte-for-byte; Pitfall 4 / Assumption A4 ordering pinned (Reconcile-start precedes the first `GitCloneProgress` at the sink-input level).
- D-16 plumbing landed: `DiscoveredSkill` carries `synced_at: Option<String>`; `lib.rs::sync` joins it in from the in-memory `manifest_for_reconcile` immediately after `discover_all` returns; `ListReport` surfaces the field through (no wrapper struct needed — list re-uses `DiscoveredSkill` directly); `join_synced_at_from_manifest` extracted for direct unit-testing.
- D-09 sink-side fold-in implemented: `SyncProgress` mirror gains `item: Option<String>` + `PartialEq/Eq`; `TauriEventSink::emit` now delegates to a pure `event_to_sync_progress(event) -> SyncProgress` function that routes `GitCloneProgress` → `SyncStage::Reconcile` with `Some(format!("git: <dir> (<bytes>)"))` and `BackupSnapshot` → `SyncStage::Save` with the message verbatim. New `format_bytes` helper renders bytes as IEC strings (B/KiB/MiB/GiB/TiB, 1 decimal).
- 14 new tests across the three crates (3 progress, 1 discover, 2 list, 1 lib helper, 7 sink); 1114 total workspace tests pass; clippy clean across the entire workspace (`-D warnings`); no `bindings.ts` regeneration (27-01b owns that).

## Task Commits

Each task was committed atomically:

1. **Task 1: D-08 — add `item: Option<String>` to `SyncStageProgress`; update emission/match sites; RecordingSink tests for round-trip (Some/None) + Pitfall 4 ordering.** — `7697ced` (feat) — `crates/tome/src/progress.rs`, `crates/tome/src/lib.rs`.
2. **Task 2: D-16 — add `DiscoveredSkill.synced_at`; populate from manifest in `sync()` via extracted `join_synced_at_from_manifest` helper; surface on `ListReport`; serde round-trip tests + layering invariant.** — `94ac4ec` (feat) — `crates/tome/src/discover.rs`, `crates/tome/src/lib.rs`, `crates/tome/src/library.rs` (test fixtures), `crates/tome/src/list.rs`, `crates/tome/src/lockfile.rs` (test fixtures).
3. **Task 3: D-09 — extend `SyncProgress` mirror with `item`; implement D-09 sink-side fold-in for `GitCloneProgress` + `BackupSnapshot`; extract pure `event_to_sync_progress`; add `format_bytes` helper.** — `0aa6550` (feat) — `crates/tome-desktop/src/sink.rs`.

_Note: tasks were marked `tdd="true"` but the change is mechanically additive on the type level (adding an `Option<String>` field). Compile errors at the consumer sites act as the "RED" signal; the value semantics tests pin the contract. No separate `test(…)` → `feat(…)` split is meaningful for additive-Option deltas, so each task is a single `feat(…)` commit that includes both the test and the implementation in the same diff._

## Files Created/Modified

- `crates/tome/src/progress.rs` — `ProgressEvent::SyncStageProgress.item: Option<String>` (D-08); three new `RecordingSink` tests (round-trip Some, round-trip None, Pitfall 4 ordering). Per-stage assignment table documented on the field itself.
- `crates/tome/src/discover.rs` — `DiscoveredSkill.synced_at: Option<String>` (D-16, `#[serde(default)]`); initialize `None` at the `scan_for_skills` constructor; `discover_all_leaves_synced_at_none` invariant test.
- `crates/tome/src/list.rs` — adds a `mod tests` block (file previously had none); `collect_leaves_synced_at_none_for_unstamped_skills` + `list_report_serializes_synced_at_in_json` (Some + None JSON serialization round-trip).
- `crates/tome/src/lib.rs` — `IndicatifSink::emit` binds `item: _` on `SyncStageProgress`; Distribute emission site passes `item: Some(name.to_string())`; extracted `join_synced_at_from_manifest` helper; sync()'s Discover block calls the helper; `join_synced_at_populates_known_skills_and_leaves_others_none` test fixture.
- `crates/tome/src/library.rs` — test-fixture `DiscoveredSkill` literals updated to set `synced_at: None` (4 sites).
- `crates/tome/src/lockfile.rs` — test-fixture `make_discovered` helper updated to set `synced_at: None`.
- `crates/tome-desktop/src/sink.rs` — `SyncProgress` mirror gains `item: Option<String>` + `PartialEq+Eq`; extracted pure `event_to_sync_progress`; new `format_bytes` helper; `emit()` delegates to the pure fn; 7 new unit tests (3 `format_bytes` + 4 conversion).

## Decisions Made

See `key-decisions` in the frontmatter for full rationale. Quick index:

1. **Per-stage emission scope:** only the one existing `SyncStageProgress` site (Distribute) is updated; per-skill emissions inside Consolidate/Cleanup/Save are deferred to future plans (would require threading sinks into submodules — out of the 5-file scope cap). Semantics for the future sites are documented on the field's doc comment.
2. **Fixture-only Pitfall 4 test:** the RecordingSink ordering test emits two events directly; the real-pipeline ordering check is owned by 27-04 per the plan's own `<behavior>` note.
3. **`join_synced_at_from_manifest` extraction:** join semantic is extracted into a `pub(crate) fn` so it can be unit-tested against an in-memory `Manifest` fixture without spinning a full sync TempDir.
4. **`event_to_sync_progress` extraction:** mirror of the same pattern on the sink side — pure conversion factored out of `emit()` so tests don't need an `AppHandle`.
5. **`format_bytes` shape:** IEC binary prefixes (KiB/MiB/GiB/TiB) with 1 decimal place; `u64::MAX` clamps to TiB (no panic). Tests pin small/medium/large ranges separately.

## Deviations from Plan

None - plan executed exactly as written, modulo the deliberate interpretation of "per-stage assignment" recorded in Decision 1 above (the per-Save / per-Cleanup / per-Consolidate emission sites do not exist in `tome::sync`, so the documented per-stage `item` values are an aspirational contract this plan pins via the field's doc comment for future plans to honor).

## Issues Encountered

- **Clippy `doc_lazy_continuation` lint** on the new `SyncProgress.item` doc comment (a bulleted list directly followed by a paragraph line without a blank-line separator). Fixed by inserting the required blank lines around the bullet list. Caught by `cargo clippy -p tome-desktop --all-targets -- -D warnings`; re-ran clean on the fix.
- **Intermittent flake (`backup::tests::push_and_pull_roundtrip`)** showed up once during the cross-task verification chain and passed on a re-run — same flake already documented in `CLAUDE.md` as a known open item. Not caused by this plan.
- **Format-on-save drift** on the first `progress.rs` test (the multi-line `matches!()` macro got collapsed to a single line by `cargo fmt`). The auto-format was applied; the test still passes. Documented here only because the formatter modified a file mid-task — no semantic change.

## User Setup Required

None — Rust-side substrate only. No external services, no env vars, no dashboard configuration. The Tauri boundary commands + React skeleton ship in 27-01b.

## Next Phase Readiness

- **27-01b (Wave 2, depends_on: [01a]):** can now wire `start_sync` / `cancel_sync` Tauri commands + `MenuAction::JumpSync` + the React `SyncView` / `useSync` / Sidebar Sync NavItem + `bindings.ts` regen + axe scan against the typed types this plan ships. Both `ProgressEvent::SyncStageProgress.item` and `SyncProgress.item` are in place; the D-09 fold-in already produces a Reconcile-stage payload from `GitCloneProgress` so the GUI's per-stage rows will pattern-match correctly out of the box.
- **27-02b (Skills view Recent sort):** can read `DiscoveredSkill.synced_at` from the JSON payload produced by the existing `list_skills` Tauri command — no domain change needed, just a comparator wiring on the React side.
- **27-04 (real-pipeline sync_cancel test):** the fixture-only Pitfall 4 pin lives in `tome::progress`; the end-to-end pipeline assertion (sync() against a real git-source fixture emits `SyncStageStarted{Reconcile}` before any `GitCloneProgress`) is the next acceptance test on the path.
- **No blockers carried forward.**

## Verification Summary

- `cargo test -p tome --lib progress::tests`: 7/7 pass (4 pre-existing + 3 new D-08 tests).
- `cargo test -p tome --lib discover::tests`: 34/34 pass (33 pre-existing + 1 new D-16 layering invariant).
- `cargo test -p tome --lib list::tests`: 2/2 pass (new module).
- `cargo test -p tome --lib tests::join_synced_at_populates_known_skills_and_leaves_others_none`: pass.
- `cargo test -p tome-desktop --lib sink::tests`: 7/7 pass (new module).
- `cargo test --workspace --exclude tome-desktop`: 1114/1114 pass; 0 fail (after one transient `backup::tests::push_and_pull_roundtrip` flake that resolved on retry — documented intermittent issue, unrelated to this plan).
- `cargo test -p tome --test cli_list`: 5/5 pass — no `tome list --json` regression.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo fmt --check` (across all touched files): clean.

## Self-Check: PASSED

All claimed artifacts verified:
- `.planning/phases/27-sync-triage-ui/27-01a-SUMMARY.md` exists.
- `crates/tome/src/progress.rs`, `crates/tome/src/discover.rs`, `crates/tome/src/list.rs`, `crates/tome/src/lib.rs`, `crates/tome/src/library.rs`, `crates/tome/src/lockfile.rs`, `crates/tome-desktop/src/sink.rs` exist.
- Commits `7697ced`, `94ac4ec`, `0aa6550` present in `git log --oneline --all`.

---
*Phase: 27-sync-triage-ui*
*Completed: 2026-06-06*
