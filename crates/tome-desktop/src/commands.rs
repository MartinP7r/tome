//! Tauri command surface (webview → Rust trust boundary).
//!
//! This phase exposes a single command: [`get_status`], which returns a real
//! [`tome::status::StatusReport`] gathered against the user's actual
//! `tome_home`. The IPC surface is deliberately minimal (T-25-04-EoP): no
//! `tauri-plugin-shell`/`-fs`, only `get_status` on the main window.

use tome::TomePaths;
use tome::config::Config;

use crate::error::TomeError;

/// Resolve the user's real `tome_home` + `Config` the same way the CLI does
/// with no flags: default config path, then default `tome_home`.
///
/// Mirrors `crates/tome/src/lib.rs::run`'s flag-free resolution branch so the
/// GUI observes exactly the same state the CLI would (`Config::load_or_default`
/// is missing-file tolerant — an unconfigured machine yields a default config
/// and `StatusReport { configured: false, .. }`).
fn load_context() -> anyhow::Result<(Config, TomePaths)> {
    let config_path = tome::config::default_config_path()?;
    let config = Config::load_or_default(Some(&config_path))?;
    let tome_home = tome::config::default_tome_home()?;
    let paths = TomePaths::new(tome_home, config.library_dir().to_path_buf())?;
    Ok((config, paths))
}

/// Return a read-only status snapshot of the tome system.
///
/// The single boundary command for this phase. The `app` handle is accepted so
/// later phases can inject a [`crate::sink::TauriEventSink`] for long-running
/// variants; for the read-only status path it is currently unused.
#[tauri::command]
#[specta::specta]
pub fn get_status(_app: tauri::AppHandle) -> Result<tome::status::StatusReport, TomeError> {
    // CORE-05 / D-13: classify the domain's `anyhow::Error` into a structured
    // `TomeError` at the IPC boundary. The front-end pattern-matches on
    // `TomeError.code`; the full anyhow chain is preserved in `context`.
    let (config, paths) = load_context().map_err(TomeError::from)?;
    tome::status::gather(&config, &paths).map_err(TomeError::from)
}
