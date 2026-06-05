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
#[derive(Debug, Clone, serde::Serialize, specta::Type, tauri_specta::Event)]
pub struct SyncProgress {
    /// Which pipeline stage this event refers to.
    pub stage: SyncStage,
    /// Units processed so far in this stage (0 when not applicable).
    pub current: u32,
    /// Total units in this stage (0 when unknown / not applicable).
    pub total: u32,
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
        // Bridge each domain event to a typed SyncProgress. The stage is
        // carried directly (no `format!("{:?}", stage)` stringification) so the
        // front-end keeps an exhaustive, pattern-matchable discriminant.
        let payload = match event {
            ProgressEvent::SyncStageStarted { stage }
            | ProgressEvent::SyncStageFinished { stage } => SyncProgress {
                stage,
                current: 0,
                total: 0,
            },
            ProgressEvent::SyncStageProgress {
                stage,
                current,
                total,
            } => SyncProgress {
                stage,
                current: saturate_usize(current),
                total: saturate_usize(total),
            },
            // Git-clone byte progress folds into the Reconcile stage (the
            // pipeline stage that resolves git sources). BackupSnapshot has no
            // numeric progress, so it folds into Save with zeroed counts.
            ProgressEvent::GitCloneProgress { received, .. } => SyncProgress {
                stage: SyncStage::Reconcile,
                current: saturate_u64(received),
                total: 0,
            },
            ProgressEvent::BackupSnapshot { .. } => SyncProgress {
                stage: SyncStage::Save,
                current: 0,
                total: 0,
            },
        };
        // emit failures (no webview yet, serialization error) are non-fatal for
        // a progress event — drop rather than panic in the domain worker.
        let _ = payload.emit(&self.app);
    }
}
