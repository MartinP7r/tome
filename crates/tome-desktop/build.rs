//! Tauri build script.
//!
//! Calls `tauri_build::build()` ONLY — it generates the Tauri context and
//! processes `capabilities/`. It must NOT export `bindings.ts`: a build script
//! runs in a separate compilation unit and cannot see the `#[tauri::command]`
//! functions defined in `src/`, so the specta export lives in `gen-bindings`
//! (and `main.rs` under `#[cfg(debug_assertions)]`) instead — RESEARCH Pitfall 1.
fn main() {
    tauri_build::build();
}
