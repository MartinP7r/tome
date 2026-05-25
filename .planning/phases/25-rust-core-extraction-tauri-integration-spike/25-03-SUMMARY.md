---
phase: 25-rust-core-extraction-tauri-integration-spike
plan: 03
subsystem: api
tags: [progress-sink, cancel-token, indicatif, presenter-decomposition, tauri-prep, core-extraction]

# Dependency graph
requires:
  - phase: 25-01
    provides: SkillOwnership enum in manifest.rs + bindings cargo feature
  - phase: 25-02
    provides: progress.rs (ProgressSink, ProgressEvent, SyncStage, CancelToken, NullSink, RecordingSink)
provides:
  - "sync() takes &dyn ProgressSink + &CancelToken; emits a typed ProgressEvent per stage; checks is_cancelled() at every stage boundary"
  - "IndicatifSink (CLI front-end sink) re-homing the spinner()/finish_and_clear() presentation into lib.rs"
  - "git::clone_repo/update_repo + backup::snapshot/restore emit GitCloneProgress/BackupSnapshot and observe cancellation"
  - "list::collect(config) -> ListReport (net-new domain fn) — list logic extracted out of cmd_list"
  - "remove::plan / reassign::plan / relocate::plan / eject::plan promoted pub(crate) -> pub (GUI-callable)"
affects: [26-read-only-views, 27-sync-triage-ui, 29-mutating-operations-ui, 25-04-tauri-spike]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Structure at the edge (D-17): domain stays sync + presentation-free; CLI IndicatifSink / GUI TauriEventSink at the boundary"
    - "Stage-boundary cancellation (D-12): is_cancelled() checked between stages, never mid-write (atomic temp+rename invariant preserved)"
    - "Thin inline presenter over a pub structured-type domain fn (D-GUI-08), mirroring status::gather/show"

key-files:
  created:
    - crates/tome/src/list.rs
  modified:
    - crates/tome/src/lib.rs
    - crates/tome/src/git.rs
    - crates/tome/src/backup.rs
    - crates/tome/src/marketplace.rs
    - crates/tome/src/eject.rs
    - crates/tome/src/reassign.rs
    - crates/tome/src/relocate.rs
    - crates/tome/src/remove.rs

key-decisions:
  - "Reconcile stage drives the 'Resolving git sources...' spinner (plan stage mapping); Discover keeps 'Discovering skills...'. The per-directory 'Distributing to {name}...' message collapses to a single Distribute-stage spinner — all three are TTY-transient (finish_and_clear'd), so absent from captured output."
  - "git long-op family adopted now: clone_repo/update_repo gain (sink, cancel) params; GitAdapter (marketplace.rs) passes NullSink + fresh CancelToken to honor the D-05a verbatim-delegation regression contract."
  - "backup long-op family adopted now: snapshot/restore gain (sink, cancel) params; cmd_backup passes NullSink (the existing println! presentation is unchanged, so CLI stdout stays byte-for-byte)."
  - "list module registered pub(crate) (matching status/doctor/remove siblings); the collect fn is pub. Module-level widening for tome-desktop is a 25-04 concern (build-failure anchored)."

patterns-established:
  - "IndicatifSink: interior-mutable Mutex<Option<ProgressBar>> holds the active spinner between SyncStageStarted and SyncStageFinished; emit(&self) keeps the trait Send + Sync for a GUI AppHandle sink"
  - "CancelToken threaded through sync() + git + backup signatures now, never tripped by the CLI — the API shape is fixed so Phase 27's cancel button doesn't re-sign every fn"

requirements-completed: [CORE-01, CORE-04]

# Metrics
duration: 38min
completed: 2026-05-25
---

# Phase 25 Plan 03: Rust core extraction — ProgressSink/CancelToken threading + presenter decomposition Summary

**sync() is now sink+cancel aware (typed ProgressEvent per stage, is_cancelled() at every boundary) with an IndicatifSink reproducing CLI spinner output exactly; the remaining commands' domain logic is pub structured-type fns behind thin inline presenters — the full CLI integration suite passes byte-for-byte.**

## Performance

- **Duration:** ~38 min
- **Started:** 2026-05-25T13:48:56Z (Phase 25 execution started)
- **Completed:** 2026-05-25
- **Tasks:** 2
- **Files modified:** 8 (1 created, 7 modified)

## Accomplishments

- Threaded `&dyn ProgressSink` + `&CancelToken` through `sync()`, replacing the four inline `spinner()` + `finish_and_clear()` call sites with typed `SyncStageStarted`/`SyncStageFinished` emits (Reconcile → Discover → Consolidate → Distribute → Cleanup → Save) plus per-directory `SyncStageProgress` in distribute.
- Added `IndicatifSink` in `lib.rs` (D-11) — the CLI front-end sink that owns the spinner machinery; `cmd_sync` and the post-init sync select it for interactive runs and `NullSink` under `--quiet`/`--verbose`, exactly matching the prior `show_progress = !quiet && !verbose` gate.
- Adopted the git + backup long-op families per D-11: `git::clone_repo`/`update_repo` emit `GitCloneProgress` and check cancellation; `backup::snapshot`/`restore` emit `BackupSnapshot` and check cancellation.
- Extracted `list::collect(config) -> ListReport` (net-new module + pub fn); promoted `remove::plan`, `reassign::plan`, `relocate::plan`, `eject::plan` from `pub(crate)` to `pub` — the GUI-callable domain surface for Phases 26/29.
- Added the CORE-04 `RecordingSink` harness test driving a real `sync()` and asserting ≥1 `SyncStageStarted` per stage.

## Task Commits

1. **Task 1: IndicatifSink + thread sink/cancel through sync()** — `93cffd5` (feat)
2. **Task 2: Finish presenter decomposition for remaining commands** — `f46436c` (feat)

## Files Created/Modified

- `crates/tome/src/list.rs` — **created**: `list::collect(config) -> ListReport` domain fn + `ListReport` struct (skills + warnings).
- `crates/tome/src/lib.rs` — `IndicatifSink` struct; extended `sync()` signature; per-stage `sink.emit` + `is_cancelled()` checks; `resolve_git_directories` threads sink/cancel; `cmd_sync` + post-init sync construct the sink/token; `cmd_backup` passes NullSink; `list()` now formats `list::collect`'s report; CORE-04 RecordingSink test.
- `crates/tome/src/git.rs` — `clone_repo`/`update_repo` gain `(sink, cancel)`; emit `GitCloneProgress`; bail on pre-start cancellation.
- `crates/tome/src/backup.rs` — `snapshot`/`restore` gain `(sink, cancel)`; emit `BackupSnapshot`; pre-start cancel check; test wrappers `snap`/`rest` pass NullSink.
- `crates/tome/src/marketplace.rs` — `GitAdapter::install/update` pass `NullSink` + fresh `CancelToken` to the new `clone_repo`/`update_repo` signatures (D-05a contract preserved).
- `crates/tome/src/eject.rs`, `reassign.rs`, `relocate.rs`, `remove.rs` — `plan()` promoted `pub(crate)` → `pub`.

## Decisions Made

- **Stage→spinner mapping:** Reconcile reuses the "Resolving git sources..." spinner message and Discover keeps "Discovering skills...", per the plan's explicit mapping. The per-directory "Distributing to {name}..." message collapses to one stage-level Distribute spinner. All spinners are `finish_and_clear()`'d (TTY-transient) and never reach piped stdout, so the `insta`/`assert_cmd` regression snapshots are unaffected — verified byte-for-byte.
- **git/backup signature threading vs. wrappers:** added `(sink, cancel)` directly to `clone_repo`/`update_repo`/`snapshot`/`restore` (rather than parallel `*_with_progress` wrappers) for a single source of truth; non-sync callers (`GitAdapter`, `cmd_backup`, internal `restore`→`snapshot`) pass `NullSink` + a never-tripped token. The D-05a "install delegates verbatim to git::clone_repo" anchor test still passes.
- **`list` module visibility:** registered `pub(crate)` to match `status`/`doctor`/`remove` siblings; `collect` is `pub`. Crate-external module widening for `tome-desktop` is left to 25-04 where the actual cross-crate calls land and a missing/misnamed fn would surface as a compile error.

## Name Corrections vs. Plan Assumptions

The plan warned its FQN line numbers might drift; re-confirmed against the codebase:
- `remove::plan` is at `remove.rs:308` (plan said `:278`). Promoted correctly.
- All other names verified exact: `status::gather`, `doctor::diagnose`, `lint::lint_library`, `lint::lint_skill` already `pub`; `reassign::plan(.., is_fork, force)` is the shared reassign+fork entry (no `fork::plan`); `relocate.rs:46` and `eject.rs:26` `plan` fns promoted.
- `list::collect` chosen as the net-new name (plan's recommendation), confirmed there was no pre-existing `list` domain fn (logic was inline in `lib.rs::list()`).

## Pub Domain Surface Now Available to the GUI (by FQN)

- `tome::status::gather(config, paths) -> Result<StatusReport>`
- `tome::doctor::diagnose(config, paths, dry_run, no_input, json) -> Result<()>`
- `tome::list::collect(config) -> Result<ListReport>` (net-new)
- `tome::lint::lint_library(library_dir) -> LintReport`
- `tome::lint::lint_skill(dir_name, skill_dir) -> Vec<LintIssue>`
- `tome::remove::plan(name, config, paths, manifest) -> Result<RemovePlan>` (promoted)
- `tome::reassign::plan(skill, to, config, paths, manifest, is_fork, force) -> Result<ReassignPlan>` (promoted; serves both reassign and fork)
- `tome::relocate::plan(config, paths, new_library_dir, config_path) -> Result<RelocatePlan>` (promoted)
- `tome::eject::plan(config, paths) -> Result<EjectPlan>` (promoted)

(Note: their owning modules are still `pub(crate)`; 25-04 widens module visibility where `tome-desktop` calls them across the crate boundary.)

## Deviations from Plan

None — plan executed exactly as written. (The plan explicitly authorized correcting any drifted FQNs/line numbers before finalizing; the `remove.rs:308` correction above is that authorized adjustment, not a scope change.)

## Issues Encountered

- Initial `IndicatifSink::emit` used a nested `if let … { if total > 0 { … } }` that tripped clippy's `collapsible_if` under `-D warnings`; collapsed into a single `if let … && total > 0` let-chain.
- The CORE-04 test's `let mut config = Config::default(); config.library_dir = …` tripped clippy `field_reassign_with_default`; rewrote as a `Config { library_dir, ..Config::default() }` struct-update.
- `SyncStage` does not derive `Hash`; the test collects `SyncStageStarted` stages into a `Vec` + `.contains()` instead of a `HashSet` (avoids touching 25-02's progress.rs).
- `NullSink` is not imported at the top of `backup.rs`; added `use crate::progress::NullSink;` inside the backup test module.

All resolved within the relevant task commit.

## Known Stubs

None introduced by this plan. (The pre-existing v0.10 `EditDecision::Revert` warn-and-skip stub in `lib.rs::apply_edit_decisions` is untouched and out of scope.)

## Next Phase Readiness

- The domain is now front-end-agnostic: `sync()` emits typed progress + observes cancellation, and every command's computation is a `pub` structured-type fn behind a thin inline presenter.
- 25-04 (Tauri spike) can: (a) implement a `TauriEventSink: ProgressSink` forwarding `ProgressEvent` over IPC, (b) clone a live `CancelToken` into a cancel command, (c) call the promoted `plan`/`gather`/`collect` fns — and will need to widen the owning modules' visibility from `pub(crate)` to `pub` for the cross-crate calls (build-failure anchored).
- No CLI regression: `make ci` equivalent green (fmt-check + clippy `-D warnings` default & `--features bindings` + 1066 tests); snapshots byte-for-byte unchanged.

## Self-Check: PASSED

- FOUND: `crates/tome/src/list.rs` (created)
- FOUND: `crates/tome/src/lib.rs`, `git.rs`, `backup.rs`, `marketplace.rs`, `eject.rs`, `reassign.rs`, `relocate.rs`, `remove.rs` (modified)
- FOUND commit `93cffd5` (Task 1)
- FOUND commit `f46436c` (Task 2)

---
*Phase: 25-rust-core-extraction-tauri-integration-spike*
*Completed: 2026-05-25*
