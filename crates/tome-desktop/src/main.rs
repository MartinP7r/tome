//! tome desktop GUI binary (Tauri 2 entry point).
//!
//! Mounts the shared [`tome_desktop::make_builder`] registry on a real Tauri
//! app. Under `#[cfg(debug_assertions)]` it also exports `ui/src/bindings.ts`
//! on startup (path relative to the crate dir) so a `cargo tauri dev` loop
//! keeps bindings fresh; CI uses the dedicated `gen-bindings` bin instead
//! (workspace-root-relative path) — RESEARCH Pitfall 1 / Q-A.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let builder = tome_desktop::make_builder();

    #[cfg(debug_assertions)]
    {
        use specta_typescript::Typescript;
        builder
            .export(Typescript::default(), "ui/src/bindings.ts")
            .expect("failed to export bindings.ts");
    }

    tauri::Builder::default()
        // Phase 26 plan 26-03 / D-07: opener plugin (Finder reveal for the
        // "Open source folder" action) and clipboard-manager plugin (the JS
        // side calls `writeText` after the `copy_path` command returns).
        // Both grants are narrowed in `capabilities/main.json` to the
        // specific permissions they need (no fs widening).
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        // Phase 27 plan 27-01b — register the managed `SyncState` so both
        // `start_sync` and `cancel_sync` (Tauri commands in
        // `commands.rs`) share the in-flight `CancelToken` slot
        // (T-27-01b-07 double-fire mitigation).
        .manage(tome_desktop::sync_state::SyncState::default())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            // Wire the typed events so `SyncProgress::emit(&app)` reaches the
            // front-end listeners registered by the generated bindings.
            builder.mount_events(app);

            // Phase 26 plan 26-06 — VIEW-06 / NF-05. Spawn the file watcher
            // on a background thread so manifest / lockfile / library /
            // machine.toml changes (from a concurrent CLI run or the GUI's
            // own mutations) emit typed events that drive silent React
            // refetches. Errors here propagate as setup errors — Tauri
            // reports them as failed app startup, which is the right signal
            // if the FSEvents backend cannot init.
            let handle = app.handle().clone();
            let (_config, paths) = tome_desktop::commands::load_context()?;
            tome_desktop::watcher::spawn_watcher(handle.clone(), paths)?;

            // Phase 26 plan 26-07 — NF-03. Install the native macOS
            // menu bar (tome / File / Edit / View / Library / Help)
            // alongside the watcher. `install_menu` is a cross-platform
            // shim: a no-op on non-macOS (D-GUI-06), so we don't need a
            // `#[cfg]` here. Failure surfaces as a setup error — the
            // same posture as the watcher above.
            tome_desktop::menu::install_menu(&handle)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tome-desktop");
}
