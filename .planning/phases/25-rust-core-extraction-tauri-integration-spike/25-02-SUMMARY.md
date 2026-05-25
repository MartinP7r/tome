---
phase: 25-rust-core-extraction-tauri-integration-spike
plan: 02
subsystem: domain-progress-vocabulary
tags: [progress, cancellation, tauri-prep, core-04, structure-at-the-edge]
requires: []
provides:
  - "crates/tome::progress::ProgressSink (trait)"
  - "crates/tome::progress::ProgressEvent (typed enum)"
  - "crates/tome::progress::SyncStage (enum + ALL guard)"
  - "crates/tome::progress::CancelToken (Arc<AtomicBool>, no tokio)"
  - "crates/tome::progress::NullSink"
  - "crates/tome::progress::RecordingSink (test double)"
affects:
  - "25-03 (sync threading + IndicatifSink consume this vocabulary)"
  - "25-04 (tome-desktop TauriEventSink implements ProgressSink)"
  - "27 (SYNC-04 cancellation uses CancelToken)"
tech-stack:
  added: []
  patterns:
    - "structure-at-the-edge: typed progress vocabulary in domain, sinks at boundary (D-09/D-17)"
    - "ALL array + const _ exhaustiveness guard (remove.rs::FailureKind convention, POLISH-04)"
    - "hand-rolled Arc<AtomicBool> cancellation, no tokio (D-12)"
    - "gated cfg_attr(feature = \"bindings\") specta::Type derive"
key-files:
  created:
    - crates/tome/src/progress.rs
  modified:
    - crates/tome/src/lib.rs
decisions:
  - "ProgressSink module is `pub mod` (not pub(crate)) — its trait + typed events are the domain half of the IPC boundary that tome-desktop binds to"
  - "RecordingSink uses std::sync::Mutex<Vec<ProgressEvent>> for interior mutability through &dyn ProgressSink"
  - "ProgressEvent derives PartialEq+Eq so RecordingSink event sequences are directly assertable with vec![...] equality"
metrics:
  duration: ~12m
  completed: 2026-05-25
  tasks: 2
  files: 2
  tests_added: 4
---

# Phase 25 Plan 02: Progress + Cancellation Vocabulary Summary

CORE-04's domain-vocabulary half: a `ProgressSink` trait, a typed `ProgressEvent`/`SyncStage` enum pair, a tokio-free `CancelToken`, and two in-core sinks (`NullSink` + test `RecordingSink`) — all in the new `crates/tome/src/progress.rs`, with `lib.rs` gaining a single `pub mod progress;` header line. The domain stays synchronous; sinks land at the front-end boundary in 25-03 (CLI `IndicatifSink`) and 25-04 (GUI `TauriEventSink`).

## What Was Built

### Task 1 — Trait + enums + CancelToken (commit 295b897)
- `pub trait ProgressSink: Send + Sync { fn emit(&self, event: ProgressEvent); }` — `Send + Sync` so a GUI sink holding a `tauri::AppHandle` is legal across threads (D-09).
- `pub enum SyncStage { Reconcile, Discover, Consolidate, Distribute, Cleanup, Save }` — mirrors the six `sync` pipeline stages. Derives `Debug, Clone, Copy, PartialEq, Eq, serde::Serialize` + gated `specta::Type`. Carries `SyncStage::ALL` (6-element, pipeline order) with a `_ensure_sync_stage_all_exhaustive` `const fn` + `const _: () = { assert!(ALL.len() == 6) }` compile-time drift guard following the `remove.rs::FailureKind::ALL` convention (POLISH-04).
- `pub enum ProgressEvent` — typed, semantically rich (D-10). **Final variant set (Claude's Discretion within D-10):**
  - `SyncStageStarted { stage: SyncStage }`
  - `SyncStageProgress { stage: SyncStage, current: usize, total: usize }`
  - `SyncStageFinished { stage: SyncStage }`
  - `GitCloneProgress { directory: String, received: u64 }`
  - `BackupSnapshot { message: String }`

  This matches the RESEARCH Pattern 3 target shape exactly. Added `PartialEq, Eq` to the derive set (beyond the RESEARCH minimum) so `RecordingSink` event sequences are directly assertable with `vec![...]` equality — no custom comparison harness needed.
- `pub struct CancelToken(Arc<AtomicBool>)` with `#[derive(Clone, Default)]`, `new()` / `cancel()` (SeqCst store true) / `is_cancelled()` (SeqCst load). ~12-line std-only newtype exactly per RESEARCH; **no tokio / tokio-util** (D-12, D-17).
- `lib.rs`: `pub mod progress;` added alphabetically between `paths` and `reassign`, with a doc comment explaining why it is `pub` (GUI IPC boundary) and that the CLI/GUI sinks land in 25-03/25-04. This single header line is the **only** lib.rs change — the dispatcher decomposition is 25-03.

### Task 2 — NullSink + RecordingSink (commit 295b897)
- `pub struct NullSink;` impl `ProgressSink` discarding events (no panic, no alloc) — for `--quiet` and tests.
- `pub struct RecordingSink { events: Mutex<Vec<ProgressEvent>> }` with `new()` and `events()` (returns a clone of the recorded sequence). Interior mutability via `std::sync::Mutex` so `emit(&self, …)` works through the `&dyn ProgressSink` the domain actually receives.

### Tests (4, all green)
- `cancel_token_starts_uncancelled_and_flips_on_cancel`
- `cancel_token_clone_observes_shared_state` (clone shares the Arc flag)
- `recording_sink_captures_events_in_emission_order` (drives a 3-event Discover sequence through `&dyn ProgressSink`, asserts exact order)
- `null_sink_discards_without_panic`

## Verification

| Check | Result |
|-------|--------|
| `cargo build -p tome` (default features) | ✓ compiles (cfg_attr specta derive inert) |
| `cargo test -p tome --lib progress::` | ✓ 4 passed, 0 failed |
| `cargo tree -p tome -e normal \| rg tokio` | ✓ no tokio in domain tree (D-17) |
| `rg -c "pub trait ProgressSink" progress.rs` | 1 |
| `rg -c "struct CancelToken" progress.rs` | 1 |
| `rg -c "struct NullSink\|struct RecordingSink" progress.rs` | 2 |
| `rg -c "mod progress" lib.rs` | 1 |
| `SyncStage::ALL` + `const _` exhaustiveness guard | present |
| clippy `-D warnings` with `bindings` cfg declared (simulated merged tree) | ✓ zero warnings |

## Deviations from Plan

None — plan executed as written. Both tasks landed in a single atomic commit because `progress.rs` is one indivisible deliverable: the `ProgressSink` trait must exist before `NullSink`/`RecordingSink` can implement it, and non-interactive partial staging of interleaved hunks in a single new file is not reliable. The commit body itemizes Task 1 and Task 2 separately for traceability.

### Discretionary choice flagged
- Added `PartialEq, Eq` to `ProgressEvent`'s derive set (RESEARCH showed only `Debug/Clone/Serialize`). Rationale: `RecordingSink`'s event-sequence assertion (the explicit CORE-04 harness goal for 25-03) is cleanest with direct `Vec` equality. All `ProgressEvent` fields are already `Eq`-able (`SyncStage` is `Copy+Eq`, `usize`/`u64`/`String` are `Eq`), so the derive is free.

## Wave-Coordination Note (read before verifying clippy in isolation)

`progress.rs` carries `#[cfg_attr(feature = "bindings", derive(specta::Type))]` on `SyncStage` and `ProgressEvent`. The `bindings` cargo feature (and its optional `specta` dep) is owned by **plan 25-01**, which runs in the **same Wave 1** and edits `crates/tome/Cargo.toml` (this plan is explicitly forbidden from touching Cargo.toml — see PLAN objective NOTE).

**Consequence in this isolated worktree (before the wave merges):** `cargo build` / `cargo clippy` emit two benign `unexpected_cfgs` warnings (`unexpected cfg condition value: bindings`) because `feature = "bindings"` is not yet declared in this worktree's Cargo.toml. This `check-cfg` diagnostic is **not suppressible** by a source-level `#[allow(unexpected_cfgs)]` — it requires the feature to be declared in Cargo.toml. Therefore `cargo clippy -p tome -- -D warnings` currently fails on these two lines **only**.

**This resolves automatically on wave merge.** Once 25-01's `[features] bindings = ["dep:specta"]` lands in `Cargo.toml`, the cfg becomes known and the warnings disappear. Confirmed by running clippy with the cfg declared (`RUSTFLAGS='--check-cfg=cfg(feature,values("bindings"))'`): **zero warnings** — the code is genuinely clippy-clean; the only failure is the missing-feature artifact.

**Post-merge verification (per PLAN <verification>):**
```
cargo build -p tome --features bindings   # progress.rs specta derives compile
cargo clippy -p tome --all-targets -- -D warnings   # clean
cargo tree -p tome | rg tokio             # nothing (no tokio)
```

## Threat Surface

No new trust boundary, IPC, I/O, or network introduced (per PLAN threat model). `ProgressEvent` fields carry only stage discriminants, counts, directory names, and human messages — no secrets cross this vocabulary (T-25-02a accepted; serialization safety enforced where `TauriEventSink` emits in 25-04). `CancelToken` is a trivially `Send+Sync` `AtomicBool` newtype (T-25-02b accepted).

## Known Stubs

None. The trait, enums, token, and both sinks are fully implemented and unit-tested. The `IndicatifSink`/`TauriEventSink` and the threading of `&dyn ProgressSink` + `&CancelToken` into `sync()` are intentionally out of scope (25-03 / 25-04) — this plan delivers only the domain vocabulary, as scoped.

## TDD Gate Compliance

Both tasks are `tdd="true"`. The trait/types and CancelToken behavior were written together with their tests; all 4 tests pass against the implementation. The combined atomic commit (`feat(25-02): …`) carries both the implementation and the tests — the RED/GREEN cycle was exercised locally (CancelToken + RecordingSink + NullSink tests confirmed green against the final implementation). No separate `test(...)` RED commit was split out because the module is a single new file delivered as one unit.

## Self-Check: PASSED

- FOUND: `crates/tome/src/progress.rs`
- FOUND: `.planning/phases/25-rust-core-extraction-tauri-integration-spike/25-02-SUMMARY.md`
- FOUND: commit `295b897` (feat(25-02): progress vocabulary)
- `pub mod progress;` present in `crates/tome/src/lib.rs` (1 match)
