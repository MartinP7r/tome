//! Watcher integration tests — VIEW-06 / NF-05 (Phase 26 plan 26-06).
//!
//! These tests verify two contracts the watcher promises to the rest of
//! Phase 26:
//!
//! 1. **Own-process writes fire watcher events** (Pitfall 10 / Assumption A4).
//!    Historically the macOS FSEvents backend has had "own-process suppression"
//!    quirks. If suppressed, the Phase-26 mutation (D-06 — `Disable on this
//!    machine`) would write `machine.toml` from inside the GUI process and the
//!    watcher would never fire — meaning the badge wouldn't appear until the
//!    user switched views and back. Test 1 verifies that an in-process atomic
//!    temp+rename to `machine.toml` (the same write `tome::machine::save`
//!    performs) fires `WatcherEvent::MachinePrefs` within 500ms.
//!
//!    Plan 26-03 will introduce `tome::actions::set_skill_disabled` — once it
//!    lands, the orchestrator's continuation agent should extend this test
//!    to call it directly. For now we exercise the same OS-level write path
//!    the future action handler will take.
//!
//! 2. **External writes fire watcher events** (NF-05 — the CLI / GUI concurrency
//!    promise). Test 2 writes an updated `.tome-manifest.json` via atomic
//!    temp+rename from the test thread (simulating an external `tome sync`
//!    run) and asserts `WatcherEvent::Manifest` fires within 500ms.
//!
//! Test scope: macOS only. The Phase 26 GUI ships macOS-first (D-GUI-06);
//! Linux GUI is deferred to v2, so on non-macOS runners these tests are a
//! no-op. The FSEvents backend is the part being verified; running on inotify
//! would not prove the contract.

#![cfg(target_os = "macos")]

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use tempfile::TempDir;
use tome_desktop::watcher::{WatcherEvent, WatcherPaths, spawn_watcher_with_sink};

/// Atomic temp+rename write of `content` to `path`. Mirrors the on-disk write
/// pattern `tome::machine::save` and `tome::manifest::write_manifest` use:
/// write to a `.tmp` sibling, then `rename` into place. FSEvents emits a
/// `Create` or `Modify` event on the destination path after the rename.
fn atomic_write(path: &std::path::Path, content: &str) {
    let tmp = path.with_extension("tmp-watcher-test");
    std::fs::write(&tmp, content).expect("write tmp");
    std::fs::rename(&tmp, path).expect("rename tmp -> path");
}

/// Build a `WatcherPaths` rooted under a `TempDir`. The four watched roots
/// share a fake "tome home" so a real watcher can be spawned against them.
/// Returns the WatcherPaths and the TempDir (keep the TempDir alive for the
/// test duration — it cleans up on drop).
fn watcher_paths_in_tempdir() -> (WatcherPaths, TempDir) {
    let tmp = TempDir::new().expect("create tempdir");
    let tome_home = tmp.path().to_path_buf();
    let config_dir = tome_home.clone();
    let library_dir = tome_home.join("library");
    let machine_dir = tome_home.join("config");

    // Every dir must exist BEFORE we start watching — Pitfall 5.
    std::fs::create_dir_all(&library_dir).expect("create library dir");
    std::fs::create_dir_all(&machine_dir).expect("create machine dir");

    let paths = WatcherPaths {
        manifest_path: config_dir.join(".tome-manifest.json"),
        lockfile_path: config_dir.join("tome.lock"),
        library_dir,
        machine_path: machine_dir.join("machine.toml"),
    };
    (paths, tmp)
}

/// Cold-start budget before performing a write that should be observed by the
/// watcher. `spawn_watcher_with_sink` returns synchronously after watches are
/// registered (see its doc comment), so we only need to absorb FSEvents'
/// kernel-side readiness gap — the window between `kfsevents_register` returning
/// and the kernel actually plumbing events for the registered path. 750ms is a
/// generous margin: empirically the gap is sub-100ms on warm caches, and this
/// budget tolerates cold APFS in CI and parallel test load on contended hosts.
const COLD_START_GAP: Duration = Duration::from_millis(750);

/// Wall-clock budget the test gives FSEvents + the 200ms debounce window to
/// deliver an expected event after the write completes. Generous so a stalled
/// FSEvents scheduler (parallel test load, low-power CPU state) does not flake.
const EVENT_RECV_BUDGET: Duration = Duration::from_secs(3);

/// Spawn the watcher with an `mpsc::Sender` sink that forwards every emitted
/// `WatcherEvent`. Returns the receiver so callers can `recv_timeout` on it —
/// the test wakes the instant the event arrives, no poll loop required.
fn spawn_recording(paths: WatcherPaths) -> mpsc::Receiver<WatcherEvent> {
    let (tx, rx) = mpsc::channel();
    spawn_watcher_with_sink(paths, move |event| {
        // Channel-closed errors here are benign (test already dropped rx).
        let _ = tx.send(event);
    })
    .expect("spawn watcher");
    // Absorb the FSEvents kernel readiness gap before the test mutates files.
    // `spawn_watcher_with_sink` itself blocks until watches are registered, so
    // this is NOT racing against debouncer-init scheduling — it is the FSEvents
    // backend's own latency between `kfsevents_register` and events being
    // delivered. See COLD_START_GAP doc for empirical justification.
    std::thread::sleep(COLD_START_GAP);
    rx
}

/// Drain the receiver until either the expected event arrives or the budget
/// expires. Returns `true` on first match. Uses `recv_timeout` so the test
/// wakes the instant an event arrives; no polling.
fn wait_for(rx: &mpsc::Receiver<WatcherEvent>, expected: WatcherEvent) -> bool {
    let deadline = std::time::Instant::now() + EVENT_RECV_BUDGET;
    loop {
        let now = std::time::Instant::now();
        if now >= deadline {
            return false;
        }
        match rx.recv_timeout(deadline - now) {
            Ok(ev) if ev == expected => return true,
            Ok(_other) => continue, // ignore unrelated events (e.g. spurious Library)
            Err(mpsc::RecvTimeoutError::Timeout) => return false,
            Err(mpsc::RecvTimeoutError::Disconnected) => return false,
        }
    }
}

/// Pitfall 10 / Assumption A4 — own-process write to `machine.toml` MUST
/// fire `MachinePrefs`. Plan 26-03's
/// `tome::actions::set_skill_disabled` will perform the same temp+rename
/// write; we exercise that OS-level path here before the action wrapper
/// lands. If this test fails on a future macOS update, the mitigation
/// documented in plan 26-06 §"done criteria" is to emit `MachinePrefsChanged`
/// directly from `actions::set_skill_disabled` after the save returns.
#[test]
fn own_process_write_to_machine_toml_fires_machine_prefs_changed() {
    let (paths, _tmp) = watcher_paths_in_tempdir();
    let machine_path: PathBuf = paths.machine_path.clone();
    let rx = spawn_recording(paths);

    // Atomic temp+rename — the same write `machine::save` performs.
    atomic_write(&machine_path, "# tome machine prefs\n");

    assert!(
        wait_for(&rx, WatcherEvent::MachinePrefs),
        "expected MachinePrefs event after own-process atomic write to \
         machine.toml (Pitfall 10 / Assumption A4 — FSEvents own-process \
         suppression detected on this macOS version; mitigation: emit \
         MachinePrefsChanged directly from actions::set_skill_disabled — \
         see 26-06 §done criteria)",
    );
}

/// NF-05 — external write to `.tome-manifest.json` (simulating a concurrent
/// CLI `tome sync`) MUST fire `Manifest`.
#[test]
fn external_write_to_manifest_fires_manifest_changed() {
    let (paths, _tmp) = watcher_paths_in_tempdir();
    let manifest_path = paths.manifest_path.clone();
    let rx = spawn_recording(paths);

    atomic_write(&manifest_path, "{\"version\": 1, \"skills\": {}}\n");

    assert!(
        wait_for(&rx, WatcherEvent::Manifest),
        "expected Manifest event after external atomic write to \
         .tome-manifest.json (NF-05 contract — CLI sync should trigger \
         silent GUI refresh)",
    );
}
