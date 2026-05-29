//! Tauri command surface (webview → Rust trust boundary).
//!
//! Phase-26 alpha commands. Read-only commands resolve a real
//! [`tome::status::StatusReport`] / [`tome::list::ListReport`] /
//! [`tome::skill::SkillDetail`]; the lone Phase-26 mutation
//! ([`set_skill_disabled`]) goes through the shared
//! [`tome::actions::set_skill_disabled`] helper so the TUI and GUI hit the
//! same atomic temp+rename code path. The IPC surface stays minimal —
//! `opener:default` + `clipboard-manager:allow-write-text` plus
//! `core:default`/`core:event:default`, no `fs:default` or shell widening
//! (T-25-04-EoP mitigation).

use tome::SkillName;
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
///
/// `pub` since plan 26-06 — `main.rs::setup` calls it to derive the
/// `TomePaths` it hands to the file watcher (`watcher::spawn_watcher`).
pub fn load_context() -> anyhow::Result<(Config, TomePaths)> {
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

/// Return the discovered skill list backing the GUI's VIEW-02 (Skills view).
///
/// Thin wrapper over [`tome::list::collect`] — the CORE-01 collect-shape
/// function. The GUI fetches once on mount, then runs fuzzy filter / sort /
/// group-by JS-side (RESEARCH §"Standard Stack — Fuzzy search"); per-keystroke
/// IPC would blow the 60fps budget.
#[tauri::command]
#[specta::specta]
pub fn list_skills(_app: tauri::AppHandle) -> Result<tome::list::ListReport, TomeError> {
    let (config, _paths) = load_context().map_err(TomeError::from)?;
    tome::list::collect(&config).map_err(TomeError::from)
}

/// Aggregate a single skill's right-pane payload for the GUI's
/// `DetailHeader` + `MarkdownBody` (Phase 26 plan 26-03 / VIEW-03 / D-05).
///
/// Wraps [`tome::skill::collect_detail`] — manifest entry + parsed
/// frontmatter projection + machine-prefs disabled flag + capped markdown
/// body. Body length is capped at 1 MiB at the domain layer so the webview
/// render path is bounded.
#[tauri::command]
#[specta::specta]
pub fn get_skill_detail(
    _app: tauri::AppHandle,
    name: SkillName,
) -> Result<tome::skill::SkillDetail, TomeError> {
    let (config, paths) = load_context().map_err(TomeError::from)?;
    tome::skill::collect_detail(&name, &config, &paths).map_err(TomeError::from)
}

/// Toggle a skill's membership in the global `disabled` set in `machine.toml`
/// (Phase 26 plan 26-03 / D-06 — the lone Phase 26 mutation).
///
/// Routes through the shared [`tome::actions::set_skill_disabled`] helper, so
/// the GUI and the browse TUI hit the same atomic temp+rename. The Phase-26
/// file watcher (plan 26-06) fires `MachinePrefsChanged` for the resulting
/// write — own-process writes are observed verbatim, no manual refresh
/// signal needed.
#[tauri::command]
#[specta::specta]
pub fn set_skill_disabled(
    _app: tauri::AppHandle,
    name: SkillName,
    disabled: bool,
) -> Result<(), TomeError> {
    let machine_path = tome::default_machine_path().map_err(TomeError::from)?;
    tome::actions::set_skill_disabled(&name, disabled, &machine_path).map_err(TomeError::from)
}

/// Reveal the resolved source folder of a skill in Finder (Phase 26 plan
/// 26-03 / D-07).
///
/// Resolves the source path through [`tome::actions::resolve_source_path`]
/// (Owned manifest source / Unowned library-canonical fallback), then asks
/// `tauri-plugin-opener` to do the OS-call. The plugin maps to `open -R` on
/// macOS, `xdg-open` parents on Linux, `explorer.exe /select,` on Windows.
#[tauri::command]
#[specta::specta]
pub fn open_source_folder(app: tauri::AppHandle, name: SkillName) -> Result<(), TomeError> {
    use tauri_plugin_opener::OpenerExt;
    let (config, paths) = load_context().map_err(TomeError::from)?;
    let src =
        tome::actions::resolve_source_path(&name, &config, &paths).map_err(TomeError::from)?;
    app.opener()
        .reveal_item_in_dir(&src)
        .map_err(|e| TomeError::from(anyhow::anyhow!("opener: {e}")))
}

/// Return the resolved source path of a skill as a UTF-8 string (Phase 26
/// plan 26-03 / D-07).
///
/// The Rust side resolves the path; the React side calls
/// `@tauri-apps/plugin-clipboard-manager::writeText` with the returned
/// string. Splitting the work this way keeps the IPC contract narrow (a
/// single `String` return type; no clipboard-write plumbing crossing the
/// boundary).
#[tauri::command]
#[specta::specta]
pub fn copy_path(_app: tauri::AppHandle, name: SkillName) -> Result<String, TomeError> {
    let (config, paths) = load_context().map_err(TomeError::from)?;
    let src =
        tome::actions::resolve_source_path(&name, &config, &paths).map_err(TomeError::from)?;
    Ok(src.display().to_string())
}

/// Return the full doctor report for the GUI Health view (Phase 26 plan
/// 26-05 / VIEW-05).
///
/// Wraps [`tome::doctor::collect_doctor_view`] — the GUI-facing projection of
/// `DoctorReport` that exposes only the 6 surfaced finding categories (4
/// auto-fixable + 2 informational) plus pre-computed `auto_fixable_count` /
/// `manual_count` so the React section headers render without re-walking the
/// list. Non-GUI issues (orphan dirs, missing SKILL.md, config issues,
/// foreign symlinks) intentionally do NOT cross the IPC boundary in Phase 26.
#[tauri::command]
#[specta::specta]
pub fn get_doctor_report(_app: tauri::AppHandle) -> Result<tome::doctor::DoctorView, TomeError> {
    let (config, paths) = load_context().map_err(TomeError::from)?;
    tome::doctor::collect_doctor_view(&config, &paths).map_err(TomeError::from)
}

/// Dispatch a per-item doctor repair for the GUI's `PreviewPopover` Apply
/// button (Phase 26 plan 26-05 / VIEW-05 / D-09).
///
/// Wraps [`tome::doctor::repair_one`] — re-runs `check()` to locate the live
/// issue, then matches the [`tome::doctor::RepairKind`] exhaustively against
/// per-item helpers. NF-04 preview-then-confirm: this command is only reached
/// after the user clicks Apply inside the `PreviewPopover` (no keyboard
/// shortcut bypasses it; T-26-05-01 mitigation).
///
/// Returns a structured `TomeError` for the two GUI-visible failure modes:
/// stale FindingId ("no longer present" — T-26-05-02), or non-auto-fixable
/// kind ("not auto-fixable" — defensive, the GUI never sends one of these).
/// The watcher (plan 26-06) fires `LibraryChanged` / `ManifestChanged` /
/// `MachinePrefsChanged` for the resulting writes; the React Health view
/// refetches on those.
#[tauri::command]
#[specta::specta]
pub fn doctor_repair_one(
    _app: tauri::AppHandle,
    finding_id: tome::doctor::FindingId,
) -> Result<(), TomeError> {
    let (config, paths) = load_context().map_err(TomeError::from)?;
    tome::doctor::repair_one(&finding_id, &config, &paths).map_err(TomeError::from)
}
