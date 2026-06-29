//! Tauri build script.
//!
//! Calls `tauri_build::build()` ONLY — it generates the Tauri context and
//! processes `capabilities/`. It must NOT export `bindings.ts`: a build script
//! runs in a separate compilation unit and cannot see the `#[tauri::command]`
//! functions defined in `src/`, so the specta export lives in `gen-bindings`
//! (and `main.rs` under `#[cfg(debug_assertions)]`) instead — RESEARCH Pitfall 1.
//!
//! `tauri-build` is a macOS-only build-dependency (gdk-3.0 / webkit2gtk guard).
//! On non-macOS the build script is a no-op so the Linux release runner can
//! compile the whole workspace without GTK3 system libraries.
fn main() {
    #[cfg(target_os = "macos")]
    tauri_build::build();
}
