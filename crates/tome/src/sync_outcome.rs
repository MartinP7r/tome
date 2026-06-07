//! Domain-side wrapping struct for the sync pipeline outcome (Phase 27 plan
//! 27-05 / SYNC-05).
//!
//! [`crate::sync`] returns `Result<()>` — Ok on a clean run, Err on the first
//! unrecoverable failure. That shape works for the CLI (which already prints
//! a grouped failure summary before bailing) but it does not give the GUI
//! enough structure to render the SYNC-05 contract:
//!
//! - A "Sync complete with K issues" terminal state when the pipeline
//!   technically succeeded but per-skill operations inside Distribute /
//!   Cleanup / Install (Reconcile) hit recoverable I/O failures (the SAFE-01
//!   "K operations failed" semantics shipped in v0.8).
//! - A "Retry from <stage>" affordance whose safety rule is computed
//!   domain-side per RESEARCH §SC#5 (Distribute failure on a partial manifest
//!   MUST restart from Reconcile, never from Distribute).
//!
//! [`SyncOutcome`] wraps the [`crate::sync`] return value with two extra
//! pieces of structure:
//!
//! - `retry_from` — the safe stage to resume from when `result` is Err.
//!   Computed via [`safe_retry_from`] using the LAST sync stage observed
//!   to start (tracked by [`StageTrackingSink`] for the
//!   [`sync_with_outcome`] wrapper).
//! - `partial_failures` — per-operation failures that occurred during a
//!   successful run (distribution / cleanup / install). Populated from
//!   the K-failures Vecs the CLI already aggregates locally; for the GUI
//!   path we surface them via a wrapping sync wrapper that re-runs the
//!   pipeline against a tracker.
//!
//! The wire-shaped mirror lives in
//! [`crate::sync_outcome::wire`] — kept in the domain because the GUI
//! boundary in `crates/tome-desktop` re-uses it directly.
//!
//! ## Why a sibling [`sync_with_outcome`] instead of changing [`crate::sync`]
//!
//! [`crate::sync`]'s `Result<()>` shape is the CLI contract. Changing it to
//! `Result<SyncOutcome>` would ripple through every CLI presenter; that
//! breaks the v0.10 "structure at the edge" rule (D-17). Instead this module
//! adds a *sibling* entry point that the GUI calls; the CLI keeps the
//! existing `sync()` path. Both share the same underlying pipeline.

use crate::progress::{ProgressEvent, ProgressSink, SyncStage};
use std::sync::Mutex;

/// Which sub-operation produced a per-skill partial failure inside an
/// otherwise-successful stage.
///
/// Mirrors the three places the sync pipeline aggregates per-skill /
/// per-operation failures today:
///
/// - [`PartialFailureOp::Distribution`] — a single skill failed to be linked
///   into a distribution directory (D-09 / SAFE-01). The skill name is the
///   symlink filename; the directory is captured in the error context.
/// - [`PartialFailureOp::Cleanup`] — a stale distribution symlink could not
///   be removed (cleanup_disabled_from_target's per-symlink failure). The
///   skill name is the symlink filename.
/// - [`PartialFailureOp::Install`] — a managed-source install/update failed
///   inside the Reconcile stage (RESEARCH OQ-6). The skill name is the
///   plugin identifier.
/// - [`PartialFailureOp::Other`] — defensive fallback for partial failures
///   that don't map cleanly onto the above (e.g., a future SyncReport field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
pub enum PartialFailureOp {
    /// Distribute stage per-skill symlink failure.
    Distribution,
    /// Cleanup stage per-symlink remove failure.
    Cleanup,
    /// Reconcile stage per-plugin install/update failure.
    Install,
    /// Reserved fallback.
    Other,
}

impl PartialFailureOp {
    /// Every [`PartialFailureOp`] variant.
    ///
    /// POLISH-04 trio: `ALL` constant + an exhaustive `const fn` sentinel
    /// + a `const _` length pin. Adding a variant trips the sentinel match
    ///   or the length assert; the maintainer fixes both at once.
    pub const ALL: [PartialFailureOp; 4] = [
        PartialFailureOp::Distribution,
        PartialFailureOp::Cleanup,
        PartialFailureOp::Install,
        PartialFailureOp::Other,
    ];
}

#[allow(dead_code)]
const fn _ensure_partial_failure_op_all_exhaustive(op: PartialFailureOp) -> usize {
    match op {
        PartialFailureOp::Distribution => 0,
        PartialFailureOp::Cleanup => 1,
        PartialFailureOp::Install => 2,
        PartialFailureOp::Other => 3,
    }
}

const _: () = {
    assert!(PartialFailureOp::ALL.len() == 4);
};

/// One per-skill failure recovered from an otherwise-successful sync stage
/// (D-20). The error chain is collapsed into a `Vec<String>` (the same shape
/// the boundary's `TomeError.context` field uses) so the wire-side mirror is
/// a 1:1 projection without re-walking an `anyhow::Error`.
///
/// Carried inside [`SyncOutcome::partial_failures`]; the GUI renders one
/// FindingRow per entry below the stage row (UI-SPEC §StageRow §complete
/// variant with partialFailures).
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
pub struct PartialFailure {
    /// Which pipeline stage produced this failure (Distribute / Cleanup /
    /// Reconcile for Install).
    pub stage: SyncStage,
    /// Which sub-operation failed. Pairs with `stage` to fully identify
    /// the per-skill action; the retry helper dispatches on this.
    pub operation: PartialFailureOp,
    /// Skill name (raw — may not pass `SkillName` validation for a
    /// rogue filename on disk, but typically a valid SkillName). `None`
    /// for failures that aren't keyed by a single skill (defensive
    /// fallback — none of today's sites produce these).
    pub skill: Option<String>,
    /// Top-level error message — `err.to_string()` of the underlying
    /// `anyhow::Error` / `std::io::Error`.
    pub message: String,
    /// Flattened anyhow `.context()` chain, outermost first. For
    /// `std::io::Error` sources (cleanup-symlink failures) this is a
    /// single-element Vec.
    pub context: Vec<String>,
}

/// The wrapping struct returned by [`sync_with_outcome`].
///
/// Per the RESEARCH §SyncOutcome shape decision: this is a wrapping struct
/// around `Result<(), anyhow::Error>`, not an `Err` variant of an enum. The
/// `retry_from` and `partial_failures` fields carry non-error info on
/// success-with-partial-failures, which wouldn't fit on an `Err`-only variant.
#[derive(Debug, serde::Serialize)]
pub struct SyncOutcome {
    /// `Ok(())` on a clean or partial-failure run; `Err(_)` on a stage-
    /// level fatal failure. The `Err` is an `anyhow::Error` so the
    /// boundary can classify it via `TomeError::from(anyhow::Error)`.
    #[serde(skip)]
    pub result: Result<(), anyhow::Error>,
    /// The safe stage to resume from when `result` is Err. `None` when
    /// retry is not safe (Save errors, unknown stage) or when `result`
    /// is Ok.
    pub retry_from: Option<SyncStage>,
    /// Per-skill failures observed during an otherwise-successful run.
    /// Empty when `result` is Err (the full failure is in `result.err()`).
    pub partial_failures: Vec<PartialFailure>,
}

impl SyncOutcome {
    /// Build a [`SyncOutcome`] from a sync result + the stage at which the
    /// failure (if any) occurred + the list of per-skill failures that
    /// accumulated during a successful run.
    ///
    /// `failed_stage` is the last stage observed to enter via the
    /// [`StageTrackingSink`] when `result` is Err. It is ignored when
    /// `result` is Ok (retry_from is always `None` on success).
    pub fn from_sync_result(
        result: Result<(), anyhow::Error>,
        failed_stage: Option<SyncStage>,
        partial_failures: Vec<PartialFailure>,
    ) -> Self {
        match &result {
            Ok(()) => SyncOutcome {
                result,
                retry_from: None,
                partial_failures,
            },
            Err(_) => SyncOutcome {
                result,
                retry_from: safe_retry_from(failed_stage),
                // On Err the full failure is in `result.err()`; per-skill
                // partial failures are not surfaced here (the caller
                // observes the fatal error and presents a "Sync failed"
                // terminal state).
                partial_failures: Vec::new(),
            },
        }
    }
}

/// Compute the safe stage to restart the sync from given the stage that
/// failed (D-19 safety rules).
///
/// - `Some(Reconcile)` | `Some(Discover)` → restart from Reconcile (cheap;
///   both stages are read-only-ish).
/// - `Some(Consolidate)` → restart from Discover. Consolidate may have
///   partially written to the library, but Discover is read-only and the
///   library is content-hashed so re-discovery is safe.
/// - `Some(Distribute)` → restart from Reconcile. Per RESEARCH §SC#5
///   safety rule: rerunning Distribute on a partial manifest is NOT safe;
///   the safe restart point is BEFORE drift detection so the manifest
///   gets re-derived from scratch.
/// - `Some(Cleanup)` → restart from Reconcile. Cleanup failures often
///   reveal external interference (a target dir mutated under us); the
///   safest action is full re-evaluation.
/// - `Some(Save)` → `None`. Save failures (manifest/lockfile write) are
///   typically non-recoverable in-place — disk full, permission denied;
///   the user must clear the underlying issue before retrying.
/// - `None` → `None`. We don't know where it failed; default to safe.
pub fn safe_retry_from(failed_stage: Option<SyncStage>) -> Option<SyncStage> {
    match failed_stage {
        Some(SyncStage::Reconcile) | Some(SyncStage::Discover) => Some(SyncStage::Reconcile),
        Some(SyncStage::Consolidate) => Some(SyncStage::Discover),
        Some(SyncStage::Distribute) => Some(SyncStage::Reconcile),
        Some(SyncStage::Cleanup) => Some(SyncStage::Reconcile),
        Some(SyncStage::Save) => None,
        None => None,
    }
}

/// A [`ProgressSink`] wrapper that tracks the last [`SyncStage`] observed
/// to enter via [`ProgressEvent::SyncStageStarted`], so the
/// [`sync_with_outcome`] wrapper can report which stage was in flight when
/// the pipeline bailed.
///
/// The sink is "transparent" — every event is forwarded to the inner sink
/// verbatim. The only side effect is updating the latest-started slot.
/// `SyncStageFinished` does NOT clear the slot: the value we want is "the
/// stage that was *most recently started*", regardless of whether earlier
/// stages have finished cleanly (all six stages run sequentially, so when
/// the pipeline bails the latest-started IS the failed stage).
pub struct StageTrackingSink<'a> {
    inner: &'a dyn ProgressSink,
    last_started: Mutex<Option<SyncStage>>,
}

impl<'a> StageTrackingSink<'a> {
    /// Wrap a sink. The wrapper observes every event forwarded through it.
    pub fn new(inner: &'a dyn ProgressSink) -> Self {
        Self {
            inner,
            last_started: Mutex::new(None),
        }
    }

    /// Snapshot the most recently started stage (or `None` if no
    /// `SyncStageStarted` event has been seen yet).
    pub fn last_started(&self) -> Option<SyncStage> {
        *self
            .last_started
            .lock()
            .expect("StageTrackingSink mutex poisoned")
    }
}

impl<'a> ProgressSink for StageTrackingSink<'a> {
    fn emit(&self, event: ProgressEvent) {
        if let ProgressEvent::SyncStageStarted { stage } = event {
            *self
                .last_started
                .lock()
                .expect("StageTrackingSink mutex poisoned") = Some(stage);
        }
        self.inner.emit(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::{NullSink, RecordingSink};

    #[test]
    fn safe_retry_from_reconcile_stays_reconcile() {
        assert_eq!(
            safe_retry_from(Some(SyncStage::Reconcile)),
            Some(SyncStage::Reconcile),
        );
    }

    #[test]
    fn safe_retry_from_discover_restarts_at_reconcile() {
        assert_eq!(
            safe_retry_from(Some(SyncStage::Discover)),
            Some(SyncStage::Reconcile),
        );
    }

    #[test]
    fn safe_retry_from_consolidate_restarts_at_discover() {
        // Consolidate may have partially populated the library, but
        // Discover is read-only and the library is content-hashed; re-
        // discovery is safe.
        assert_eq!(
            safe_retry_from(Some(SyncStage::Consolidate)),
            Some(SyncStage::Discover),
        );
    }

    #[test]
    fn safe_retry_from_distribute_restarts_at_reconcile() {
        // RESEARCH §SC#5: rerunning Distribute on a partial manifest is
        // NOT safe. The safe restart point is BEFORE drift detection so
        // the manifest gets re-derived from scratch.
        assert_eq!(
            safe_retry_from(Some(SyncStage::Distribute)),
            Some(SyncStage::Reconcile),
        );
    }

    #[test]
    fn safe_retry_from_cleanup_restarts_at_reconcile() {
        assert_eq!(
            safe_retry_from(Some(SyncStage::Cleanup)),
            Some(SyncStage::Reconcile),
        );
    }

    #[test]
    fn safe_retry_from_save_is_none() {
        // D-19 safety rule: Save errors (manifest/lockfile write) are
        // non-recoverable in-place. The user must clear the underlying
        // issue (disk full, permission) before re-running.
        assert_eq!(safe_retry_from(Some(SyncStage::Save)), None);
    }

    #[test]
    fn safe_retry_from_none_is_none() {
        // Defensive: an unknown failure stage defaults to safe (no
        // auto-retry affordance).
        assert_eq!(safe_retry_from(None), None);
    }

    #[test]
    fn sync_outcome_ok_no_partials() {
        let outcome = SyncOutcome::from_sync_result(Ok(()), Some(SyncStage::Save), Vec::new());
        assert!(outcome.result.is_ok());
        assert_eq!(outcome.retry_from, None);
        assert!(outcome.partial_failures.is_empty());
    }

    #[test]
    fn sync_outcome_ok_with_partials_carries_them_through() {
        let pf = PartialFailure {
            stage: SyncStage::Distribute,
            operation: PartialFailureOp::Distribution,
            skill: Some("foo".to_string()),
            message: "permission denied".to_string(),
            context: vec!["permission denied".to_string()],
        };
        let outcome = SyncOutcome::from_sync_result(Ok(()), None, vec![pf.clone()]);
        assert!(outcome.result.is_ok());
        assert_eq!(outcome.retry_from, None);
        assert_eq!(outcome.partial_failures.len(), 1);
        assert_eq!(outcome.partial_failures[0].skill.as_deref(), Some("foo"));
    }

    #[test]
    fn sync_outcome_err_at_consolidate_retry_from_discover() {
        // Per D-19: Consolidate failure → restart from Discover (which is
        // read-only) since the library is content-hashed.
        let err = anyhow::anyhow!("consolidate failed");
        let outcome =
            SyncOutcome::from_sync_result(Err(err), Some(SyncStage::Consolidate), Vec::new());
        assert!(outcome.result.is_err());
        assert_eq!(outcome.retry_from, Some(SyncStage::Discover));
        assert!(outcome.partial_failures.is_empty());
    }

    #[test]
    fn sync_outcome_err_at_distribute_retry_from_reconcile() {
        // Per D-19 / RESEARCH §SC#5: Distribute failure → restart from
        // Reconcile (NOT Distribute — manifest may be partial).
        let err = anyhow::anyhow!("distribute failed");
        let outcome =
            SyncOutcome::from_sync_result(Err(err), Some(SyncStage::Distribute), Vec::new());
        assert!(outcome.result.is_err());
        assert_eq!(outcome.retry_from, Some(SyncStage::Reconcile));
    }

    #[test]
    fn sync_outcome_err_at_save_retry_from_none() {
        // Per D-19: Save failures are non-recoverable in-place; no retry
        // affordance.
        let err = anyhow::anyhow!("disk full");
        let outcome = SyncOutcome::from_sync_result(Err(err), Some(SyncStage::Save), Vec::new());
        assert!(outcome.result.is_err());
        assert_eq!(outcome.retry_from, None);
    }

    #[test]
    fn stage_tracking_sink_records_last_started_and_forwards_events() {
        let inner = RecordingSink::new();
        let tracker = StageTrackingSink::new(&inner);

        assert_eq!(tracker.last_started(), None);

        tracker.emit(ProgressEvent::SyncStageStarted {
            stage: SyncStage::Discover,
        });
        assert_eq!(tracker.last_started(), Some(SyncStage::Discover));

        tracker.emit(ProgressEvent::SyncStageProgress {
            stage: SyncStage::Discover,
            current: 1,
            total: 3,
            item: Some("foo".to_string()),
        });
        // Progress events don't shift the latest-started slot.
        assert_eq!(tracker.last_started(), Some(SyncStage::Discover));

        tracker.emit(ProgressEvent::SyncStageStarted {
            stage: SyncStage::Consolidate,
        });
        assert_eq!(tracker.last_started(), Some(SyncStage::Consolidate));

        // Inner sink received every event verbatim.
        assert_eq!(inner.events().len(), 3);
    }

    #[test]
    fn stage_tracking_sink_finished_does_not_clear_slot() {
        // We want the LATEST started — a Finished event for the previous
        // stage must not reset the slot. (All six stages run sequentially,
        // so the failed-stage on bail IS the latest started.)
        let inner = NullSink;
        let tracker = StageTrackingSink::new(&inner);

        tracker.emit(ProgressEvent::SyncStageStarted {
            stage: SyncStage::Discover,
        });
        tracker.emit(ProgressEvent::SyncStageFinished {
            stage: SyncStage::Discover,
        });
        assert_eq!(
            tracker.last_started(),
            Some(SyncStage::Discover),
            "Finished must not clear last_started — the next Started overwrites it",
        );
    }

    #[test]
    fn partial_failure_op_all_pinned_to_four_variants() {
        // POLISH-04 trio: if a new variant is added, the const _ assert
        // trips OR the sentinel match fails to compile. The runtime
        // assertion here pins the count for future readers.
        assert_eq!(PartialFailureOp::ALL.len(), 4);
    }
}
