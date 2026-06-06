//! GUI-side [`ProgressSink`] implementation (Rust → webview trust boundary).
//!
//! [`TauriEventSink`] bridges the domain's typed [`ProgressEvent`] vocabulary
//! into the front-end [`SyncProgress`] event over [`tauri::AppHandle::emit`].
//! `AppHandle` is `Clone + Send + Sync` and `emit` is callable from any thread
//! (RESEARCH Pitfall 5), so this sink is sound to share across the worker
//! thread a long-running command runs on.

use tauri_specta::Event;
use tome::progress::{ProgressEvent, ProgressSink, SyncStage};

/// Front-end progress event streamed to the webview.
///
/// Carries the **typed** [`SyncStage`] enum directly (not a stringified label)
/// so the front-end can pattern-match the stage — WARNING 4's typed option.
/// `SyncStage` already derives gated `specta::Type` and is `Copy` (25-02), so
/// it crosses the boundary as a TS string-union discriminant.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, specta::Type, tauri_specta::Event)]
pub struct SyncProgress {
    /// Which pipeline stage this event refers to.
    pub stage: SyncStage,
    /// Units processed so far in this stage (0 when not applicable).
    pub current: u32,
    /// Total units in this stage (0 when unknown / not applicable).
    pub total: u32,
    /// Optional human-readable subtitle for the unit currently in flight
    /// (D-08). Comes from one of three places:
    ///
    /// - `ProgressEvent::SyncStageProgress.item` — pass-through from the
    ///   domain (per-stage assignment owned by the emission site).
    /// - `ProgressEvent::GitCloneProgress` — D-09 sink-side fold-in:
    ///   `Some(format!("git: {dir} ({})", format_bytes(received)))`. The
    ///   domain emits raw bytes; the sink owns the human-readable format.
    /// - `ProgressEvent::BackupSnapshot` — D-09 sink-side fold-in: the
    ///   `message` field becomes the subtitle verbatim.
    ///
    /// `None` for `SyncStageStarted` / `SyncStageFinished` events.
    pub item: Option<String>,
}

/// Saturating `usize`/`u64` → `u32` cast. The domain counts are small in
/// practice (skill counts, byte tallies) but a silent wrap on a pathological
/// value would corrupt a progress bar — clamp to `u32::MAX` instead via
/// `u32::try_from(..).unwrap_or(u32::MAX)`.
fn saturate_usize(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Saturating `u64` → `u32` cast (git-clone byte counts). Same clamp policy as
/// [`saturate_usize`].
fn saturate_u64(n: u64) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Render a byte count as a short human-readable string (D-09 helper).
///
/// One decimal place, IEC binary prefixes (`B`, `KiB`, `MiB`, `GiB`,
/// `TiB`). Pure function: deterministic + side-effect free, so unit
/// tests can pin the exact format without spinning a `TauriEventSink`
/// or an `AppHandle`. The format mirrors what users already see for
/// git-clone progress elsewhere; keep it stable so the GUI subtitle
/// doesn't drift from CLI conventions.
fn format_bytes(received: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    if received < 1024 {
        return format!("{received} B");
    }
    // Find the largest unit where the value is still >= 1.
    let mut value = received as f64;
    let mut idx = 0;
    while value >= 1024.0 && idx < UNITS.len() - 1 {
        value /= 1024.0;
        idx += 1;
    }
    format!("{value:.1} {}", UNITS[idx])
}

/// Pure conversion from a domain [`ProgressEvent`] to a boundary
/// [`SyncProgress`] payload (D-08 pass-through + D-09 fold-in).
///
/// Extracted from `TauriEventSink::emit` so the entire fold-in mapping is
/// testable without an `AppHandle`. `emit` calls this and then dispatches
/// the result to the webview; tests just call this and assert against the
/// returned value.
fn event_to_sync_progress(event: ProgressEvent) -> SyncProgress {
    match event {
        ProgressEvent::SyncStageStarted { stage } | ProgressEvent::SyncStageFinished { stage } => {
            SyncProgress {
                stage,
                current: 0,
                total: 0,
                item: None,
            }
        }
        ProgressEvent::SyncStageProgress {
            stage,
            current,
            total,
            item,
        } => SyncProgress {
            stage,
            current: saturate_usize(current),
            total: saturate_usize(total),
            item,
        },
        // D-09 fold-in: git-clone byte progress folds into the Reconcile
        // stage (the pipeline stage that semantically resolves git sources;
        // see Pitfall 4 / Assumption A4 — verified by RecordingSink test in
        // tome::progress). The sink owns the byte-count → human format so
        // the domain stays presentation-agnostic.
        ProgressEvent::GitCloneProgress {
            directory,
            received,
        } => SyncProgress {
            stage: SyncStage::Reconcile,
            current: saturate_u64(received),
            total: 0,
            item: Some(format!("git: {directory} ({})", format_bytes(received))),
        },
        // D-09 fold-in: backup-snapshot messages have no numeric progress,
        // so they fold into Save with zeroed counts and the message becomes
        // the subtitle verbatim.
        ProgressEvent::BackupSnapshot { message } => SyncProgress {
            stage: SyncStage::Save,
            current: 0,
            total: 0,
            item: Some(message),
        },
    }
}

/// A [`ProgressSink`] that emits [`SyncProgress`] events into the webview.
pub struct TauriEventSink {
    app: tauri::AppHandle,
}

impl TauriEventSink {
    /// Wrap an `AppHandle` (clone it from the command's `app` argument).
    pub fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }
}

impl ProgressSink for TauriEventSink {
    fn emit(&self, event: ProgressEvent) {
        // Bridge each domain event to a typed SyncProgress via the pure
        // conversion (testable in isolation). The stage is carried directly
        // (no `format!("{:?}", stage)` stringification) so the front-end
        // keeps an exhaustive, pattern-matchable discriminant.
        let payload = event_to_sync_progress(event);
        // emit failures (no webview yet, serialization error) are non-fatal
        // for a progress event — drop rather than panic in the domain worker.
        let _ = payload.emit(&self.app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- format_bytes shape pins --

    #[test]
    fn format_bytes_renders_small_values_in_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1023), "1023 B");
    }

    #[test]
    fn format_bytes_promotes_to_kib_at_1024() {
        assert_eq!(format_bytes(1024), "1.0 KiB");
        assert_eq!(format_bytes(1536), "1.5 KiB");
    }

    #[test]
    fn format_bytes_handles_mib_gib_tib() {
        // 4 MiB exactly → "4.0 MiB"
        assert_eq!(format_bytes(4 * 1024 * 1024), "4.0 MiB");
        // 1.2 GiB ≈ 1288490188 bytes — the exact display rounding may be
        // 1.2 or 1.3, so assert the suffix and order-of-magnitude only.
        let gib = format_bytes(1_288_490_188);
        assert!(gib.ends_with(" GiB"), "expected GiB suffix, got {gib}");
        // Pathological u64::MAX clamps to TiB (the largest unit).
        let huge = format_bytes(u64::MAX);
        assert!(huge.ends_with(" TiB"), "expected TiB suffix, got {huge}");
    }

    // -- D-08 / D-09 conversion pins --

    /// D-09: `GitCloneProgress` folds into `SyncStage::Reconcile` with the
    /// sink-owned byte-formatted subtitle. Pinning Pitfall 4 / Assumption A4
    /// at the conversion site so a re-routing refactor (e.g. someone
    /// switching the target stage to Discover) is caught here. The
    /// real-pipeline event-order assertion lives in `tome::progress`
    /// (`recording_sink_pins_reconcile_start_before_git_clone_progress`).
    #[test]
    fn git_clone_progress_folds_into_reconcile_with_subtitle() {
        let payload = event_to_sync_progress(ProgressEvent::GitCloneProgress {
            directory: "my-repo".to_string(),
            received: 4 * 1024 * 1024, // 4 MiB exactly
        });

        assert_eq!(payload.stage, SyncStage::Reconcile);
        assert_eq!(payload.current, 4 * 1024 * 1024);
        assert_eq!(payload.total, 0);
        let item = payload.item.expect("git-clone fold-in must set Some(item)");
        assert!(
            item.starts_with("git: my-repo"),
            "subtitle must lead with `git: <dir>`, got: {item}",
        );
        assert!(
            item.contains("4.0 MiB"),
            "subtitle must embed the byte-formatted size, got: {item}",
        );
    }

    /// D-09: `BackupSnapshot` folds into `SyncStage::Save` with the message
    /// verbatim as the subtitle (no formatting).
    #[test]
    fn backup_snapshot_folds_into_save_with_message_verbatim() {
        let payload = event_to_sync_progress(ProgressEvent::BackupSnapshot {
            message: "writing snapshot".to_string(),
        });

        assert_eq!(
            payload,
            SyncProgress {
                stage: SyncStage::Save,
                current: 0,
                total: 0,
                item: Some("writing snapshot".to_string()),
            },
        );
    }

    /// D-08 pass-through: `SyncStageProgress` carries its `item` field
    /// through to `SyncProgress` unchanged. Pins the contract that the sink
    /// does NOT inject its own subtitle when the domain has already supplied
    /// one (avoids double-rendering / per-stage drift).
    #[test]
    fn sync_stage_progress_passes_item_through_unchanged() {
        let payload = event_to_sync_progress(ProgressEvent::SyncStageProgress {
            stage: SyncStage::Discover,
            current: 5,
            total: 10,
            item: Some("foo".to_string()),
        });

        assert_eq!(
            payload,
            SyncProgress {
                stage: SyncStage::Discover,
                current: 5,
                total: 10,
                item: Some("foo".to_string()),
            },
        );
    }

    /// `SyncStageStarted` and `SyncStageFinished` have no per-unit subtitle —
    /// the GUI surfaces the stage's static label on its own. Pins
    /// `item: None` for both so a future refactor can't accidentally inject
    /// a transient subtitle that leaks across event boundaries.
    #[test]
    fn started_and_finished_events_carry_item_none() {
        let started = event_to_sync_progress(ProgressEvent::SyncStageStarted {
            stage: SyncStage::Consolidate,
        });
        assert_eq!(
            started,
            SyncProgress {
                stage: SyncStage::Consolidate,
                current: 0,
                total: 0,
                item: None,
            },
        );

        let finished = event_to_sync_progress(ProgressEvent::SyncStageFinished {
            stage: SyncStage::Cleanup,
        });
        assert_eq!(
            finished,
            SyncProgress {
                stage: SyncStage::Cleanup,
                current: 0,
                total: 0,
                item: None,
            },
        );
    }
}
