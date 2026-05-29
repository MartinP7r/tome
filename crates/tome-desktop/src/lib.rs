//! tome desktop GUI shell (Tauri 2).
//!
//! The Rust IPC backend for the v1.0 desktop GUI. This crate path-depends on
//! `crates/tome` (the canonical domain library) with the `bindings` feature on
//! so cross-boundary types derive `specta::Type`.
//!
//! [`make_builder`] is the single source of truth for the command + event
//! registry — shared by `main.rs` (which mounts it on the real Tauri app) and
//! the `gen-bindings` bin (which exports `bindings.ts` from it). Keeping one
//! Builder constructor means `bindings.ts` can never drift from what the app
//! actually exposes.

pub mod commands;
pub mod error;
pub mod menu;
pub mod sink;
pub mod watcher;

use tauri_specta::{Builder, collect_commands, collect_events};

/// Construct the shared `tauri_specta::Builder` registering every command and
/// event that crosses the IPC boundary.
///
/// This is the one place the command/event registry is declared. `main.rs`
/// calls it to mount the handlers + events on the live app; `gen-bindings`
/// calls it to export `ui/src/bindings.ts`. Adding a command/event here makes
/// it appear in `bindings.ts` on the next `gen-bindings` run.
pub fn make_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::get_status,
            commands::list_skills,
            // Phase 26 plan 26-03 (VIEW-03 / D-05/D-06/D-07).
            commands::get_skill_detail,
            commands::set_skill_disabled,
            commands::open_source_folder,
            commands::copy_path,
            // Phase 26 plan 26-05 (VIEW-05 / D-09/D-10/D-11/D-12).
            commands::get_doctor_report,
            commands::doctor_repair_one,
        ])
        .events(collect_events![
            sink::SyncProgress,
            watcher::ManifestChanged,
            watcher::LockfileChanged,
            watcher::LibraryChanged,
            watcher::MachinePrefsChanged,
            // Phase 26 plan 26-07 (NF-03). The native macOS menu bar
            // is gated `#[cfg(target_os = "macos")]` inside `menu.rs`,
            // but the `MenuAction` enum stays compiled everywhere so
            // `bindings.ts` is stable across platforms — the React
            // `useMenuActions` hook can always subscribe; the event
            // simply never fires off-mac.
            menu::MenuAction,
        ])
        // `StatusReport`'s count fields are `Option<usize>` (skill/health
        // tallies). specta forbids exporting `usize`/`u64` to TS by default to
        // guard against BigInt precision loss. These counts are small, bounded
        // integers, so casting them to TS `number` is lossless in practice and
        // keeps the GUI working with plain numbers (not `bigint`). Setting this
        // on the shared Builder applies it uniformly to main.rs + gen-bindings,
        // and avoids changing the library-canonical `CountOrError` type shape.
        .dangerously_cast_bigints_to_number()
}
