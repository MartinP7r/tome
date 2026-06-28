//! Managed app state for the long-running `sync` pipeline.
//!
//! Holds the `CancelToken` of the currently-running sync (if any) behind a
//! `Mutex`. Two commands share this state:
//!
//! - `start_sync` (in `commands.rs`) is `async`. On entry it:
//!     1. Takes the mutex.
//!     2. If `Some(_)`, returns `ErrorCode::Conflict` ("sync already in
//!        progress") — the double-fire guard, T-27-01b-07.
//!     3. Otherwise stores a fresh `CancelToken::new()`, drops the mutex,
//!        and runs `tome::sync` via `tauri::async_runtime::spawn_blocking`
//!        (Pitfall 5 — never block the IPC reactor with a synchronous
//!        long-runner).
//!     4. On return (success OR error), clears the slot back to `None` so
//!        a subsequent `start_sync` can run.
//!
//! - `cancel_sync` is synchronous + idempotent. It reads the mutex; if a
//!   token is present it calls `token.cancel()` (flipping the inner
//!   `Arc<AtomicBool>`). The second call observes the already-flipped
//!   bool and is a no-op by construction.
//!
//! The mutex is a `std::sync::Mutex` (not `tokio::Mutex`) because both
//! commands hold the guard only across cheap reads / writes — never
//! across an `.await`. Mutex poisoning is treated as a fatal invariant
//! violation (the only path that could poison is a panic inside the
//! guard scope, which would be a programmer error).

use std::sync::Mutex;
use tome::progress::CancelToken;

/// Tauri managed app state shared between `start_sync` and `cancel_sync`.
///
/// `cancel.lock().unwrap()` carries either:
///
/// - `None` — no sync in flight; `start_sync` may begin a new run.
/// - `Some(token)` — a sync is currently running. The token is cloneable;
///   `cancel_sync` calls `token.cancel()` to request a graceful early-out
///   at the next stage boundary (`tome::sync` polls `cancel.is_cancelled()`
///   between stages).
///
/// `Default` (and the `new()` ctor) start in the idle state.
pub struct SyncState {
    pub cancel: Mutex<Option<CancelToken>>,
}

impl SyncState {
    /// Construct an idle SyncState (no in-flight sync).
    pub fn new() -> Self {
        Self {
            cancel: Mutex::new(None),
        }
    }
}

impl Default for SyncState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_starts_idle() {
        let state = SyncState::new();
        assert!(state.cancel.lock().expect("poisoned").is_none());
    }

    #[test]
    fn default_matches_new() {
        let state = SyncState::default();
        assert!(state.cancel.lock().expect("poisoned").is_none());
    }

    #[test]
    fn stored_token_can_be_cancelled() {
        // Pin the SyncState ↔ CancelToken contract: storing a token inside
        // the mutex must let an outside holder of a clone flip the shared
        // AtomicBool (RESEARCH §"Code Examples — spawn_blocking" lines
        // 549-577). This is the second-line guarantee that backs
        // `cancel_sync` being idempotent + concurrent-safe.
        let state = SyncState::new();
        let token = CancelToken::new();
        let outside = token.clone();
        *state.cancel.lock().expect("poisoned") = Some(token);

        assert!(!outside.is_cancelled());
        // Cancel via the slotted reference, observe via the outside clone.
        if let Some(t) = state.cancel.lock().expect("poisoned").as_ref() {
            t.cancel();
        }
        assert!(outside.is_cancelled());
    }
}
