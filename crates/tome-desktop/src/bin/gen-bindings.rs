//! Standalone bindings exporter.
//!
//! Constructs the shared [`tome_desktop::make_builder`] registry and exports
//! `crates/tome-desktop/ui/src/bindings.ts`. The path is relative to the
//! workspace root, which is where CI runs `cargo run -p tome-desktop --bin
//! gen-bindings` followed by `git diff --exit-code` to gate freshness (D-07,
//! corrected per RESEARCH Pitfall 1 — build.rs cannot see the commands).

use specta_typescript::Typescript;

fn main() {
    let builder = tome_desktop::make_builder();
    builder
        .export(
            Typescript::default(),
            "crates/tome-desktop/ui/src/bindings.ts",
        )
        .expect("failed to export bindings.ts");
    eprintln!("wrote crates/tome-desktop/ui/src/bindings.ts");
}
