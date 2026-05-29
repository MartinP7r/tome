//! File watcher — manifest / lockfile / library / machine.toml (Phase 26 — VIEW-06 + NF-05).
//!
//! Rust-side [`notify 8.2`](https://crates.io/crates/notify) +
//! [`notify-debouncer-full 0.7`](https://crates.io/crates/notify-debouncer-full)
//! watching the four roots that share state with the CLI. Each filesystem
//! change emits one of four typed [`tauri_specta::Event`]s the React side
//! subscribes to via `useTauriEvent`. Debounce window 200ms — matches D-03 /
//! SC#1's 200ms refresh target and rides the atomic-rename window safely
//! (Pitfall 1).
//!
//! OQ-3 resolution: Rust-side rather than JS-side
//! `@tauri-apps/plugin-fs::watch` — typed events route React refetches
//! correctly, and the IPC surface stays auditable (no `fs:default` permission
//! widening).
//!
//! Pitfall 5 — `notify::watch` errors on non-existent paths. The watcher
//! watches PARENT dirs (which always exist or are creatable) and filters by
//! the exact file path inside the debouncer callback. Recursive mode is used
//! for the library root only.

use anyhow::{Context, Result};
use notify::RecursiveMode;
use notify_debouncer_full::{DebouncedEvent, new_debouncer};
use std::path::PathBuf;
use std::time::Duration;
use tauri_specta::Event;

/// The on-disk manifest (`.tome-manifest.json`) was rewritten.
///
/// React hooks that derive from manifest state (skill list, status, doctor)
/// refetch on this event.
#[derive(Clone, Debug, serde::Serialize, specta::Type, tauri_specta::Event)]
pub struct ManifestChanged;

/// The lockfile (`tome.lock`) was rewritten.
///
/// React hooks that surface lockfile state (status, doctor) refetch on this
/// event. Skills/detail views do NOT subscribe because lockfile changes don't
/// affect the discovered skill list shape (NF-05 contract).
#[derive(Clone, Debug, serde::Serialize, specta::Type, tauri_specta::Event)]
pub struct LockfileChanged;

/// Library directory contents changed (skill add/remove/edit).
///
/// React hooks that depend on library content (status, skills list, detail,
/// doctor) refetch on this event. Recursive mode for `<tome_home>/skills/`.
#[derive(Clone, Debug, serde::Serialize, specta::Type, tauri_specta::Event)]
pub struct LibraryChanged;

/// Per-machine preferences (`machine.toml`) were rewritten.
///
/// Fires for own-process writes too (Phase-26 mutation D-06 — verified by the
/// integration test in `tests/watcher_smoke.rs`). React hooks whose render
/// branches on disabled/enabled state (status, skills, detail) refetch.
#[derive(Clone, Debug, serde::Serialize, specta::Type, tauri_specta::Event)]
pub struct MachinePrefsChanged;

/// Spawn the file watcher on a dedicated thread.
///
/// The watcher captures paths derived from [`tome::TomePaths`] +
/// [`tome::machine::default_machine_path`] at startup. Live `tome relocate`
/// (Phase 29 / OPS-03) is out of scope — users must restart the GUI after
/// relocating the library.
///
/// Errors bubble up to the Tauri `setup` closure (Tauri reports them as
/// failed app startup — clear feedback if the FSEvents backend cannot init).
/// Background-thread death is logged to stderr but does not crash the app
/// (a degraded "GUI stops auto-refreshing" failure mode is preferable to a
/// hard crash on a watchdog edge case).
pub fn spawn_watcher(app: tauri::AppHandle, paths: tome::TomePaths) -> Result<()> {
    let manifest_path = paths.manifest_path();
    let lockfile_path = paths.lockfile_path();
    let library_dir = paths.library_dir().to_path_buf();
    let machine_path = tome::default_machine_path()
        .context("failed to resolve default machine.toml path")?;

    let app2 = app.clone();
    std::thread::spawn(move || {
        if let Err(e) =
            run_watcher(app2, manifest_path, lockfile_path, library_dir, machine_path)
        {
            eprintln!("warning: watcher thread exited: {e:#}");
        }
    });
    Ok(())
}

fn run_watcher(
    app: tauri::AppHandle,
    manifest_path: PathBuf,
    lockfile_path: PathBuf,
    library_dir: PathBuf,
    machine_path: PathBuf,
) -> Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer = new_debouncer(
        Duration::from_millis(200),
        None,
        move |res: Result<Vec<DebouncedEvent>, Vec<notify::Error>>| {
            if let Ok(events) = res {
                // Send failures here are benign — they only happen if the
                // receiver thread has been dropped (which means we're shutting
                // down).
                let _ = tx.send(events);
            }
        },
    )
    .context("failed to init notify debouncer")?;

    // Watch PARENT dirs (always exist or are creatable), not file paths
    // themselves — Pitfall 5. Recursive only for the library root, where skill
    // edits live nested inside `<library>/<skill>/SKILL.md`. We filter to the
    // exact file paths inside the debouncer callback below.
    let watch_targets: Vec<(PathBuf, RecursiveMode)> = vec![
        (
            manifest_path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from(".")),
            RecursiveMode::NonRecursive,
        ),
        (
            lockfile_path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from(".")),
            RecursiveMode::NonRecursive,
        ),
        (library_dir.clone(), RecursiveMode::Recursive),
        (
            machine_path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from(".")),
            RecursiveMode::NonRecursive,
        ),
    ];
    for (path, mode) in &watch_targets {
        if path.exists()
            && let Err(e) = debouncer.watch(path, *mode)
        {
            eprintln!("warning: failed to watch {}: {e:#}", path.display());
        }
    }

    while let Ok(events) = rx.recv() {
        let mut saw_manifest = false;
        let mut saw_lockfile = false;
        let mut saw_library = false;
        let mut saw_machine = false;
        for ev in events {
            for path in &ev.paths {
                if path == &manifest_path {
                    saw_manifest = true;
                }
                if path == &lockfile_path {
                    saw_lockfile = true;
                }
                if path == &machine_path {
                    saw_machine = true;
                }
                if path.starts_with(&library_dir) {
                    saw_library = true;
                }
            }
        }
        if saw_manifest {
            let _ = ManifestChanged.emit(&app);
        }
        if saw_lockfile {
            let _ = LockfileChanged.emit(&app);
        }
        if saw_library {
            let _ = LibraryChanged.emit(&app);
        }
        if saw_machine {
            let _ = MachinePrefsChanged.emit(&app);
        }
    }

    Ok(())
}
