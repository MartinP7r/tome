//! Standalone bindings exporter.
//!
//! Constructs the shared [`tome_desktop::make_builder`] registry and exports
//! `crates/tome-desktop/ui/src/bindings.ts`. The path is relative to the
//! workspace root, which is where CI runs `cargo run -p tome-desktop --bin
//! gen-bindings` followed by `git diff --exit-code` to gate freshness (D-07,
//! corrected per RESEARCH Pitfall 1 — build.rs cannot see the commands).
//!
//! Gated `#[cfg(target_os = "macos")]` (D-GUI-06) — the CI bindings job runs
//! on macos-latest so the macOS-only Tauri runtime is always available there.

#[cfg(target_os = "macos")]
fn main() {
    use specta_typescript::Typescript;

    let builder = tome_desktop::make_builder();
    builder
        .export(
            Typescript::default(),
            "crates/tome-desktop/ui/src/bindings.ts",
        )
        .expect("failed to export bindings.ts");
    eprintln!("wrote crates/tome-desktop/ui/src/bindings.ts");
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("gen-bindings: macOS only — no-op on other platforms");
    std::process::exit(1);
}
