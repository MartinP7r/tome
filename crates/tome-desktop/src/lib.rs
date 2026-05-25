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
pub mod sink;

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
        .commands(collect_commands![commands::get_status])
        .events(collect_events![sink::SyncProgress])
        // `StatusReport`'s count fields are `Option<usize>` (skill/health
        // tallies). specta forbids exporting `usize`/`u64` to TS by default to
        // guard against BigInt precision loss. These counts are small, bounded
        // integers, so casting them to TS `number` is lossless in practice and
        // keeps the GUI working with plain numbers (not `bigint`). Setting this
        // on the shared Builder applies it uniformly to main.rs + gen-bindings,
        // and avoids changing the library-canonical `CountOrError` type shape.
        .dangerously_cast_bigints_to_number()
}
