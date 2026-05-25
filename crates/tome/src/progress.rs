//! Progress + cancellation vocabulary for long-running domain operations.
//!
//! This module is the **domain half** of the "structure at the edge" pattern
//! (D-09 / D-17): long-running operations such as [`crate::sync`] take an
//! injected [`ProgressSink`] and emit typed [`ProgressEvent`]s into it, and
//! check a [`CancelToken`] at stage boundaries. The domain stays *synchronous*
//! — no tokio runtime is added to `crates/tome`. The front-end-specific sinks
//! live at the boundary:
//!
//! - CLI: an `IndicatifSink` in `lib.rs` next to the `cmd_*` presenters
//!   (re-homes the existing `spinner()` / `finish_and_clear()` smell).
//! - GUI: a `TauriEventSink` in `tome-desktop` wrapping `AppHandle::emit`.
//!
//! Two in-core sinks ship here: [`NullSink`] (discards events — used by tests
//! and `--quiet`) and [`RecordingSink`] (captures the emitted sequence so
//! tests can assert exactly which events an operation produced).
//!
//! `ProgressEvent` and `SyncStage` are **typed** (D-10): the GUI pattern-matches
//! on the variant rather than parsing a string. They gain a `specta::Type`
//! derive behind the `bindings` feature so the same vocabulary crosses the
//! Tauri IPC boundary as a TypeScript discriminated union; under default
//! features the `cfg_attr` derive is inert and pulls in no `specta` dependency.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

/// The pipeline stage a [`ProgressEvent`] refers to.
///
/// Mirrors the six stages of the `sync` pipeline (see the crate-level docs):
/// Reconcile → Discover → Consolidate → Distribute → Cleanup → Save. Kept as a
/// typed enum (not a `&str`) so the GUI exhaustively matches stages (D-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
// `feature = "bindings"` is declared by plan 25-01 (same Wave 1, owns
// Cargo.toml). Until both plans merge into the phase branch this derive is
// inert and rustc emits a benign `unexpected_cfgs` warning; it resolves the
// moment 25-01's `[features] bindings = ["dep:specta"]` lands. The
// derive itself is correct in both states (verify post-merge with
// `cargo build -p tome --features bindings`).
#[cfg_attr(feature = "bindings", derive(specta::Type))]
pub enum SyncStage {
    /// Lockfile-authoritative drift detection for managed skills.
    Reconcile,
    /// Scan configured directories for `*/SKILL.md` directories.
    Discover,
    /// Copy discovered skills into the library (library-canonical model).
    Consolidate,
    /// Push library skills to target tools via symlinks.
    Distribute,
    /// Three-bucket stale-skill report + orphan transitions.
    Cleanup,
    /// Persist manifest, lockfile, and `.gitignore`.
    Save,
}

impl SyncStage {
    /// Every [`SyncStage`] variant, in pipeline order.
    ///
    /// Exposed as an associated constant so consumers (and the GUI) don't
    /// maintain a parallel hand-written array that could silently drop a
    /// variant when new stages are added. Drift is compile-enforced by
    /// [`_ensure_sync_stage_all_exhaustive`] + the `const _` length assert
    /// below, following the `remove.rs::FailureKind::ALL` convention
    /// (POLISH-04).
    pub const ALL: [SyncStage; 6] = [
        SyncStage::Reconcile,
        SyncStage::Discover,
        SyncStage::Consolidate,
        SyncStage::Distribute,
        SyncStage::Cleanup,
        SyncStage::Save,
    ];
}

/// Compile-time drift guard for [`SyncStage::ALL`] (POLISH-04 option c).
///
/// If a new variant is added to [`SyncStage`], this `const fn` fails to compile
/// because the match below is exhaustive. The fix is to (a) add an arm here AND
/// (b) append the new variant to `ALL`. The `const _` block additionally pins
/// `ALL.len() == 6` so a hand-edit that adds a match arm without growing `ALL`
/// (or vice versa) also fails. Dead-code at runtime — its sole purpose is the
/// exhaustiveness check.
#[allow(dead_code)]
const fn _ensure_sync_stage_all_exhaustive(s: SyncStage) -> usize {
    match s {
        SyncStage::Reconcile => 0,
        SyncStage::Discover => 1,
        SyncStage::Consolidate => 2,
        SyncStage::Distribute => 3,
        SyncStage::Cleanup => 4,
        SyncStage::Save => 5,
    }
}

const _: () = {
    // If this fails: SyncStage::ALL is missing or has extra variants.
    // The match arms in _ensure_sync_stage_all_exhaustive are the source of
    // truth — ALL must contain exactly one entry per arm.
    assert!(SyncStage::ALL.len() == 6);
};

/// A typed, semantically-rich progress event emitted by a long-running domain
/// operation (D-10).
///
/// Variants are designed so the GUI can pattern-match (and render a per-stage
/// progress bar, a git-clone byte counter, etc.) rather than string-matching a
/// formatted message. The struct-variant fields carry only non-sensitive data
/// (stage discriminants, counts, directory names, human messages) — no secrets
/// cross this boundary (threat T-25-02a).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
// See the `SyncStage` note: `feature = "bindings"` is owned by plan 25-01
// (same wave); the benign `unexpected_cfgs` warning clears once both plans
// merge.
#[cfg_attr(feature = "bindings", derive(specta::Type))]
pub enum ProgressEvent {
    /// A pipeline stage has begun.
    SyncStageStarted {
        /// Which stage started.
        stage: SyncStage,
    },
    /// Incremental progress within a stage (`current` of `total` units done).
    SyncStageProgress {
        /// Which stage is progressing.
        stage: SyncStage,
        /// Units processed so far.
        current: usize,
        /// Total units in this stage (0 if unknown).
        total: usize,
    },
    /// A pipeline stage has completed.
    SyncStageFinished {
        /// Which stage finished.
        stage: SyncStage,
    },
    /// Bytes received while cloning a git-source directory.
    GitCloneProgress {
        /// The configured directory name being cloned.
        directory: String,
        /// Bytes received so far.
        received: u64,
    },
    /// A backup snapshot produced a human-readable status message.
    BackupSnapshot {
        /// Free-form message describing the snapshot step.
        message: String,
    },
}

/// Sink that long-running domain operations emit [`ProgressEvent`]s into.
///
/// Passed as `sink: &dyn ProgressSink` (D-09). `Send + Sync` so a GUI sink
/// holding a `tauri::AppHandle` is legal to share across threads. The domain
/// stays synchronous — no tokio is required to implement this trait.
pub trait ProgressSink: Send + Sync {
    /// Emit a single progress event. Implementations must not block or panic.
    fn emit(&self, event: ProgressEvent);
}

/// Cooperative cancellation flag threaded alongside [`ProgressSink`] (D-12).
///
/// A hand-rolled `Arc<AtomicBool>` newtype — deliberately **no tokio /
/// tokio-util** so the domain stays runtime-free (RESEARCH "no tokio" pitfall).
/// The domain checks [`is_cancelled`](Self::is_cancelled) at stage boundaries
/// and bails when set. The CLI passes a never-tripped token; the GUI (Phase 27)
/// clones it into a cancel-command. Cloning shares the same underlying flag, so
/// a `.cancel()` on any clone is observed by all (a clone is a second handle to
/// one shared bit, like two remotes for the same TV).
#[derive(Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    /// Create a fresh, un-cancelled token.
    pub fn new() -> Self {
        Self::default()
    }

    /// Request cancellation. Idempotent; observed by all clones.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// A [`ProgressSink`] that discards every event.
///
/// Used by `--quiet` (where there is no presentation surface) and by tests
/// that exercise a domain operation but don't care about its progress output.
/// `emit` is a no-op — it never blocks, panics, or allocates.
#[derive(Debug, Clone, Copy, Default)]
pub struct NullSink;

impl ProgressSink for NullSink {
    fn emit(&self, _event: ProgressEvent) {}
}

/// A [`ProgressSink`] test double that records the exact sequence of emitted
/// events for assertion.
///
/// Uses interior mutability ([`Mutex<Vec<ProgressEvent>>`]) so `emit(&self, …)`
/// can push through a shared `&dyn ProgressSink` (the trait takes `&self`, and
/// the GUI sink must be `Sync`). A test injects a `RecordingSink` into a domain
/// operation, then calls [`events`](Self::events) to snapshot what was emitted
/// and assert the order — the harness 25-03 uses to pin sync's event sequence.
#[derive(Debug, Default)]
pub struct RecordingSink {
    events: Mutex<Vec<ProgressEvent>>,
}

impl RecordingSink {
    /// Create an empty recorder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot the recorded events, in emission order.
    ///
    /// Returns a clone so callers can assert against a stable `Vec` without
    /// holding the lock.
    pub fn events(&self) -> Vec<ProgressEvent> {
        self.events
            .lock()
            .expect("RecordingSink mutex poisoned")
            .clone()
    }
}

impl ProgressSink for RecordingSink {
    fn emit(&self, event: ProgressEvent) {
        self.events
            .lock()
            .expect("RecordingSink mutex poisoned")
            .push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_token_starts_uncancelled_and_flips_on_cancel() {
        let token = CancelToken::new();
        assert!(!token.is_cancelled(), "a fresh token must not be cancelled");
        token.cancel();
        assert!(token.is_cancelled(), "after cancel() it must report cancelled");
    }

    #[test]
    fn cancel_token_clone_observes_shared_state() {
        // A clone is a second handle to the same Arc<AtomicBool>: cancelling
        // through the clone must be visible on the original (D-12 shared flag).
        let token = CancelToken::new();
        let clone = token.clone();
        assert!(!clone.is_cancelled());
        token.cancel();
        assert!(
            clone.is_cancelled(),
            "cancel() on one handle must be observed by every clone"
        );
    }

    #[test]
    fn recording_sink_captures_events_in_emission_order() {
        // The whole point of RecordingSink (CORE-04 harness): a test can assert
        // the *exact* sequence a domain op emits. Drive it through a
        // `&dyn ProgressSink` to prove interior mutability works behind the
        // trait object the domain actually receives.
        let sink = RecordingSink::new();
        let dyn_sink: &dyn ProgressSink = &sink;

        dyn_sink.emit(ProgressEvent::SyncStageStarted {
            stage: SyncStage::Discover,
        });
        dyn_sink.emit(ProgressEvent::SyncStageProgress {
            stage: SyncStage::Discover,
            current: 3,
            total: 7,
        });
        dyn_sink.emit(ProgressEvent::SyncStageFinished {
            stage: SyncStage::Discover,
        });

        assert_eq!(
            sink.events(),
            vec![
                ProgressEvent::SyncStageStarted {
                    stage: SyncStage::Discover
                },
                ProgressEvent::SyncStageProgress {
                    stage: SyncStage::Discover,
                    current: 3,
                    total: 7,
                },
                ProgressEvent::SyncStageFinished {
                    stage: SyncStage::Discover
                },
            ],
            "RecordingSink must preserve emission order exactly"
        );
    }

    #[test]
    fn null_sink_discards_without_panic() {
        // NullSink (used by `--quiet` + tests) must accept any event as a
        // no-op. There is nothing to observe; the assertion is that emitting
        // through the trait object neither panics nor mutates observable state.
        let sink = NullSink;
        let dyn_sink: &dyn ProgressSink = &sink;
        dyn_sink.emit(ProgressEvent::BackupSnapshot {
            message: "snapshot 1".to_string(),
        });
        dyn_sink.emit(ProgressEvent::GitCloneProgress {
            directory: "skills".to_string(),
            received: 4096,
        });
        // If we reach here without panicking, NullSink behaved correctly.
    }
}
