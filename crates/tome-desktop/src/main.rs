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
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            // Wire the typed events so `SyncProgress::emit(&app)` reaches the
            // front-end listeners registered by the generated bindings.
            builder.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tome-desktop");
}
