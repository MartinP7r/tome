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

/// Watcher event kind — the testable enum the FSEvents → Tauri-emit bridge
/// switches on. The production code path translates each variant to the
/// corresponding [`tauri_specta::Event::emit`] call; the integration test in
/// `tests/watcher_smoke.rs` collects these directly via
/// [`spawn_watcher_with_sink`] without depending on a live Tauri app handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatcherEvent {
    /// `.tome-manifest.json` was rewritten.
    Manifest,
    /// `tome.lock` was rewritten.
    Lockfile,
    /// Library directory contents changed.
    Library,
    /// `machine.toml` was rewritten.
    MachinePrefs,
}

/// Configuration bundle for the watcher — the four watched roots plus the
/// machine.toml path (which the production code derives from
/// [`tome::default_machine_path`] but tests supply directly so they can point
/// at a [`tempfile::TempDir`] copy).
#[derive(Debug, Clone)]
pub struct WatcherPaths {
    pub manifest_path: PathBuf,
    pub lockfile_path: PathBuf,
    pub library_dir: PathBuf,
    pub machine_path: PathBuf,
}

impl WatcherPaths {
    /// Production constructor — mirrors `spawn_watcher`'s path resolution.
    pub fn from_tome_paths(paths: &tome::TomePaths) -> Result<Self> {
        Ok(Self {
            manifest_path: paths.manifest_path(),
            lockfile_path: paths.lockfile_path(),
            library_dir: paths.library_dir().to_path_buf(),
            machine_path: tome::default_machine_path()
                .context("failed to resolve default machine.toml path")?,
        })
    }
}

/// Spawn the file watcher on a dedicated thread.
///
/// The watcher captures paths derived from [`tome::TomePaths`] +
/// [`tome::default_machine_path`] at startup. Live `tome relocate`
/// (Phase 29 / OPS-03) is out of scope — users must restart the GUI after
/// relocating the library.
///
/// Errors bubble up to the Tauri `setup` closure (Tauri reports them as
/// failed app startup — clear feedback if the FSEvents backend cannot init).
/// Background-thread death is logged to stderr but does not crash the app
/// (a degraded "GUI stops auto-refreshing" failure mode is preferable to a
/// hard crash on a watchdog edge case).
pub fn spawn_watcher(app: tauri::AppHandle, paths: tome::TomePaths) -> Result<()> {
    let watcher_paths = WatcherPaths::from_tome_paths(&paths)?;
    // Production sink: each high-level WatcherEvent becomes the matching
    // typed tauri-specta emit call.
    spawn_watcher_with_sink(watcher_paths, move |event| match event {
        WatcherEvent::Manifest => {
            let _ = ManifestChanged.emit(&app);
        }
        WatcherEvent::Lockfile => {
            let _ = LockfileChanged.emit(&app);
        }
        WatcherEvent::Library => {
            let _ = LibraryChanged.emit(&app);
        }
        WatcherEvent::MachinePrefs => {
            let _ = MachinePrefsChanged.emit(&app);
        }
    })
}

/// Spawn the file watcher with an arbitrary event sink — the testable variant
/// used by `tests/watcher_smoke.rs` to collect emitted events without
/// depending on a live Tauri app handle.
///
/// This is the cleanest path around Pitfall 10: the FSEvents interaction is
/// exercised by real OS-level writes inside a [`tempfile::TempDir`], and the
/// test sink records the high-level [`WatcherEvent`]s that would be emitted
/// to the webview in production.
pub fn spawn_watcher_with_sink<F>(paths: WatcherPaths, sink: F) -> Result<()>
where
    F: Fn(WatcherEvent) + Send + 'static,
{
    std::thread::spawn(move || {
        if let Err(e) = run_watcher_with_sink(paths, sink) {
            eprintln!("warning: watcher thread exited: {e:#}");
        }
    });
    Ok(())
}

fn run_watcher_with_sink<F>(paths: WatcherPaths, sink: F) -> Result<()>
where
    F: Fn(WatcherEvent),
{
    let WatcherPaths {
        manifest_path,
        lockfile_path,
        library_dir,
        machine_path,
    } = paths;

    // FSEvents on macOS reports events with canonicalized paths (e.g.
    // `/var/folders/...` becomes `/private/var/folders/...`). If we compare
    // raw user-supplied paths against the FSEvents-reported paths, the
    // comparison can fail on symlinked path prefixes. Canonicalize each
    // PARENT dir up front and rebuild the full file path via `parent.join(
    // file_name)` so the equality checks below survive symlink resolution.
    // Falling back to the original parent on canonicalize errors is safe —
    // the comparisons against not-yet-existing paths simply won't match
    // until the file appears (and then the next canonicalize during a re-
    // watch would succeed). The library root is canonicalized directly
    // because it's a dir-prefix match (`starts_with`), not a file-equality.
    let canon_parent = |p: &PathBuf| -> PathBuf {
        p.parent()
            .and_then(|d| std::fs::canonicalize(d).ok())
            .unwrap_or_else(|| p.parent().map(PathBuf::from).unwrap_or_default())
    };
    let rebuild_file = |p: &PathBuf, canon_dir: &PathBuf| -> PathBuf {
        canon_dir.join(p.file_name().map(PathBuf::from).unwrap_or_default())
    };
    let manifest_parent_canon = canon_parent(&manifest_path);
    let lockfile_parent_canon = canon_parent(&lockfile_path);
    let machine_parent_canon = canon_parent(&machine_path);
    let library_canon = std::fs::canonicalize(&library_dir).unwrap_or_else(|_| library_dir.clone());
    let manifest_canon = rebuild_file(&manifest_path, &manifest_parent_canon);
    let lockfile_canon = rebuild_file(&lockfile_path, &lockfile_parent_canon);
    let machine_canon = rebuild_file(&machine_path, &machine_parent_canon);

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
        (manifest_parent_canon.clone(), RecursiveMode::NonRecursive),
        (lockfile_parent_canon.clone(), RecursiveMode::NonRecursive),
        (library_canon.clone(), RecursiveMode::Recursive),
        (machine_parent_canon.clone(), RecursiveMode::NonRecursive),
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
                if path == &manifest_canon {
                    saw_manifest = true;
                }
                if path == &lockfile_canon {
                    saw_lockfile = true;
                }
                if path == &machine_canon {
                    saw_machine = true;
                }
                if path.starts_with(&library_canon) {
                    saw_library = true;
                }
            }
        }
        if saw_manifest {
            sink(WatcherEvent::Manifest);
        }
        if saw_lockfile {
            sink(WatcherEvent::Lockfile);
        }
        if saw_library {
            sink(WatcherEvent::Library);
        }
        if saw_machine {
            sink(WatcherEvent::MachinePrefs);
        }
    }

    Ok(())
}
