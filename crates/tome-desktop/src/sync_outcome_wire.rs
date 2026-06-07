//! IPC wire-type mirrors for [`tome::SyncOutcome`] + [`tome::PartialFailure`]
//! (Phase 27 plan 27-05 / SYNC-05).
//!
//! The domain ships its outcome shape with `anyhow::Error` payloads —
//! [`tome::SyncOutcome`] holds `Result<(), anyhow::Error>` and the inner
//! per-skill `PartialFailure` carries `message`/`context` strings collapsed
//! from anyhow chains. Across the Tauri IPC boundary we substitute the
//! [`crate::error::TomeError`] classified shape so the React side gets the
//! same `code` discriminant it already pattern-matches on for `FindingRow`
//! and the doctor-report error chain.
//!
//! The conversion runs at the boundary inside `start_sync` /
//! `retry_sync_from` / `retry_failed_items` (`commands.rs`) — the domain
//! types stay anyhow-shaped and the wire types stay TomeError-shaped, so
//! the classification responsibility lives in one place
//! (`TomeError::from(anyhow::Error)`).
//!
//! ## Why a wrapping struct (not Result<SyncOutcomeWire, TomeError>)
//!
//! Per the RESEARCH §Pitfall-6 SyncOutcome shape decision (and per
//! RESEARCH §3 recommendation), the IPC return type is a wrapping struct
//! with an `Option<TomeError>` "fatal error" slot plus `retry_from` +
//! `partial_failures`. A `Result<…>` envelope would force the partial-
//! failure path to live on `Ok` only, but `retry_from` is information the
//! React side reads regardless of the Ok/Err split. A single wrapping
//! struct surfaces both terminal states (success-with-issues, failed-with-
//! retry) through one shape; the React side reads `result === null`
//! (success or partial) vs `result !== null` (failed) to discriminate.
//!
//! The Tauri command itself still returns `Result<SyncOutcomeWire,
//! TomeError>` — the outer Err carries setup / spawn_blocking JoinError
//! failures (Pitfall 5) that happen BEFORE the sync pipeline could
//! produce a structured outcome.

use serde::{Deserialize, Serialize};
use specta::Type;
use tome::progress::SyncStage;
use tome::{PartialFailure, PartialFailureOp, SyncOutcome};

use crate::error::TomeError;

/// The IPC wire-shape of a sync outcome (Phase 27 plan 27-05 / SYNC-05).
///
/// `result: Option<TomeError>` — `None` on a clean or partial-success run;
/// `Some(_)` on a stage-level fatal failure. The React side reads this as
/// the discriminator for the SyncView terminal-state branches:
///
/// - `result === null && partial_failures.length === 0` → "Sync complete"
/// - `result === null && partial_failures.length > 0`  → "Sync complete
///   with K issues"
/// - `result !== null && retry_from !== null`          → "Sync failed —
///   Retry from <stage>"
/// - `result !== null && retry_from === null`          → "Sync failed —
///   Dismiss only" (Save errors)
#[derive(Debug, Clone, Serialize, Type)]
// SyncOutcomeWire only flows webview-bound (it's a return type, never an
// argument), so Deserialize is not required on this struct.
pub struct SyncOutcomeWire {
    /// Fatal stage-level error; `None` on Ok runs (even with K partial
    /// failures). The Tauri tagged-Result projection would normally split
    /// the success/error states; we collapse them into one field so the
    /// React side reads ONE shape regardless of which terminal branch
    /// renders.
    pub result: Option<TomeError>,
    /// Safe stage to resume from when `result` is Some. `None` when retry
    /// is not safe (Save errors, unknown stage) or when `result` is None.
    pub retry_from: Option<SyncStage>,
    /// Per-skill failures observed during an otherwise-successful run.
    /// Empty when `result` is Some (the full failure is in `result`).
    pub partial_failures: Vec<PartialFailureWire>,
}

/// IPC wire-shape mirror of [`tome::PartialFailure`] (Phase 27 plan 27-05).
///
/// Same fields; the `message`/`context` already carry the projected error
/// payload from the domain side, so this is a structural mirror — no
/// reclassification at the boundary. The `code` field is fixed to
/// [`crate::error::ErrorCode::Internal`] because per-skill cleanup /
/// install failures don't currently carry domain sentinels through the
/// SAFE-01 aggregation pipeline; a future plan that threads sentinels
/// onto per-skill error sites will narrow this.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
// PartialFailureWire flows both webview-bound (carried inside
// SyncOutcomeWire) AND boundary-inbound (as an argument to
// retry_failed_items), so it needs both Serialize and Deserialize.
pub struct PartialFailureWire {
    /// Which pipeline stage produced this failure.
    pub stage: SyncStage,
    /// Which sub-operation failed.
    pub operation: PartialFailureOp,
    /// Skill name (raw — may not pass `SkillName` validation for a
    /// rogue filename on disk). `None` for failures not keyed by a skill.
    pub skill: Option<String>,
    /// Structured error payload (matches `FindingRow`'s shape).
    pub error: TomeError,
}

impl From<&PartialFailure> for PartialFailureWire {
    fn from(pf: &PartialFailure) -> Self {
        PartialFailureWire {
            stage: pf.stage,
            operation: pf.operation,
            skill: pf.skill.clone(),
            error: TomeError {
                // Per-skill failures from SAFE-01 aggregation don't carry
                // domain sentinels today (the sites wrap std::io::Error
                // without `with_domain_kind`). Default to Internal until
                // a future plan attaches sentinels at those sites.
                code: crate::error::ErrorCode::Internal,
                message: pf.message.clone(),
                context: pf.context.clone(),
            },
        }
    }
}

impl From<SyncOutcome> for SyncOutcomeWire {
    fn from(outcome: SyncOutcome) -> Self {
        let SyncOutcome {
            result,
            retry_from,
            partial_failures,
        } = outcome;
        let wire_result = result.err().map(TomeError::from);
        SyncOutcomeWire {
            result: wire_result,
            retry_from,
            partial_failures: partial_failures
                .iter()
                .map(PartialFailureWire::from)
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use tome::progress::SyncStage;

    #[test]
    fn ok_outcome_wire_has_no_result_no_retry_no_failures() {
        let outcome = SyncOutcome::from_sync_result(Ok(()), None, Vec::new());
        let wire = SyncOutcomeWire::from(outcome);
        assert!(wire.result.is_none());
        assert!(wire.retry_from.is_none());
        assert!(wire.partial_failures.is_empty());
    }

    #[test]
    fn ok_outcome_with_partials_carries_them_to_wire() {
        let pf = PartialFailure {
            stage: SyncStage::Distribute,
            operation: PartialFailureOp::Distribution,
            skill: Some("foo".to_string()),
            message: "permission denied".to_string(),
            context: vec!["permission denied".to_string()],
        };
        let outcome = SyncOutcome::from_sync_result(Ok(()), None, vec![pf]);
        let wire = SyncOutcomeWire::from(outcome);
        assert!(wire.result.is_none());
        assert!(wire.retry_from.is_none());
        assert_eq!(wire.partial_failures.len(), 1);
        let pfw = &wire.partial_failures[0];
        assert_eq!(pfw.stage, SyncStage::Distribute);
        assert_eq!(pfw.operation, PartialFailureOp::Distribution);
        assert_eq!(pfw.skill.as_deref(), Some("foo"));
        assert_eq!(pfw.error.code, ErrorCode::Internal);
        assert_eq!(pfw.error.message, "permission denied");
    }

    #[test]
    fn err_at_distribute_wire_has_retry_from_reconcile() {
        let err = anyhow::anyhow!("distribute failed");
        let outcome =
            SyncOutcome::from_sync_result(Err(err), Some(SyncStage::Distribute), Vec::new());
        let wire = SyncOutcomeWire::from(outcome);
        assert!(wire.result.is_some());
        assert_eq!(wire.retry_from, Some(SyncStage::Reconcile));
        assert!(wire.partial_failures.is_empty());
    }

    #[test]
    fn err_at_save_wire_has_no_retry_from() {
        let err = anyhow::anyhow!("disk full");
        let outcome = SyncOutcome::from_sync_result(Err(err), Some(SyncStage::Save), Vec::new());
        let wire = SyncOutcomeWire::from(outcome);
        assert!(wire.result.is_some());
        assert!(wire.retry_from.is_none());
    }

    #[test]
    fn err_outcome_classifies_anyhow_into_tome_error_code() {
        // A sentinel-tagged error → ErrorCode::Permission. The wire shape
        // surfaces the classified code so the React side renders the same
        // [ErrorCode] message format as FindingRow does.
        use anyhow::Context;
        use tome::DomainErrorKind;
        use tome::errors::WithDomainKind;
        let err: anyhow::Error = Err::<(), _>(anyhow::anyhow!("can't write"))
            .with_domain_kind(DomainErrorKind::Permission)
            .context("during Save")
            .unwrap_err();
        let outcome = SyncOutcome::from_sync_result(Err(err), Some(SyncStage::Save), Vec::new());
        let wire = SyncOutcomeWire::from(outcome);
        let te = wire.result.expect("Err outcome must surface a result");
        assert_eq!(te.code, ErrorCode::Permission);
    }
}
