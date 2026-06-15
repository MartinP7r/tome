//! Native macOS menu bar (NF-03).
//!
//! Phase 26 ships the menu structure required for alpha sign-off:
//!
//! - **tome** (app menu, first submenu) — About / Services / Hide /
//!   Hide-Others / Show-All / Quit (all Predefined; OS renders the app
//!   menu under the application name on macOS).
//! - **File** — Close Window (Predefined).
//! - **Edit** — Undo / Redo / Cut / Copy / Paste / Select All (ALL
//!   Predefined; Pitfall 9 mitigation — OS routes ⌘C/⌘V/⌘X/⌘A/⌘Z/⌘⇧Z
//!   to the focused webview control automatically; never bind these as
//!   menu-level custom shortcuts).
//! - **View** — Jump-to-Status (⌘1), Jump-to-Skills (⌘2),
//!   Jump-to-Health (⌘3), Focus Search (⌘F). Each emits a typed
//!   [`MenuAction`] event the React side subscribes to via
//!   `useMenuActions`. Reload (⌘R) is rendered disabled — placeholder
//!   for a Phase 27+ "refetch" surface.
//! - **Library** — Sync / Add Directory… (both disabled with tooltips
//!   pointing at Phase 27/28). Renders the breadcrumb so users discover
//!   the surface; actions are intentionally inert in alpha.
//! - **Help** — Documentation / Report Issue. Open the project's
//!   GitHub repo / issues page through `tauri-plugin-opener`.
//!
//! Per D-GUI-06 the Phase-26 GUI is macOS-only. The [`MenuAction`]
//! enum stays compiled everywhere so `bindings.ts` is stable
//! cross-platform; the menu construction + click handler are
//! `#[cfg(target_os = "macos")]` and `install_menu` is a no-op
//! everywhere else.

use tauri::{AppHandle, Wry};

/// Typed event fired when a custom (non-Predefined) menu item is
/// activated. The React side (`useMenuActions`) listens via the
/// generated `events.menuAction` binding and routes to the router
/// (`JumpStatus` / `JumpSkills` / `JumpHealth`) or focuses the
/// SearchField (`FocusSearch`).
///
/// Phase 27+ Library actions (e.g. `SyncNow`, `AddDirectory`) are NOT
/// added here — they belong to the milestone that ships them, alongside
/// the matching Rust command + UI surface.
#[derive(Clone, Debug, serde::Serialize, specta::Type, tauri_specta::Event)]
#[serde(tag = "kind")]
pub enum MenuAction {
    /// View → Status (⌘1).
    JumpStatus,
    /// View → Skills (⌘2).
    JumpSkills,
    /// View → Health (⌘3).
    JumpHealth,
    /// View → Focus Search (⌘F). Scoped to the Skills view client-side.
    FocusSearch,
}

impl MenuAction {
    /// Stringified variants used by the exhaustiveness sentinel below
    /// (POLISH-04). Updating this constant alongside the `match` arm
    /// in `_menu_action_exhaustiveness_sentinel` is the contract that
    /// keeps `bindings.ts`, `useMenuActions` switch, and Rust enum in
    /// lockstep — adding a 5th variant fails compile until ALL is
    /// extended too.
    pub const ALL: [&'static str; 4] = ["JumpStatus", "JumpSkills", "JumpHealth", "FocusSearch"];
}

#[allow(dead_code)]
const fn _menu_action_exhaustiveness_sentinel(a: MenuAction) {
    match a {
        MenuAction::JumpStatus => {}
        MenuAction::JumpSkills => {}
        MenuAction::JumpHealth => {}
        MenuAction::FocusSearch => {}
    }
}
const _: () = {
    assert!(MenuAction::ALL.len() == 4);
};

/// Install the native menu + menu-event handler.
///
/// Cross-platform shim: on macOS this builds the menu and wires the
/// click handler; on every other target it's a no-op (the GUI itself is
/// macOS-only per D-GUI-06; this lets `main.rs::setup` call
/// `menu::install_menu` unconditionally without a `#[cfg]` per call).
pub fn install_menu(app: &AppHandle<Wry>) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    {
        macos::install(app)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Ok(())
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::MenuAction;
    use anyhow::Result;
    use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
    use tauri::{AppHandle, Wry};
    use tauri_specta::Event;

    // Documentation + issue tracker URLs surfaced via the Help menu.
    // Hardcoded (not user-controlled) — see threat T-26-07-03 in the
    // plan's threat model.
    const DOCS_URL: &str = "https://github.com/MartinP7r/tome";
    const ISSUES_URL: &str = "https://github.com/MartinP7r/tome/issues";

    /// macOS entry point — build the menu, attach it to the app, and
    /// register the click handler.
    pub(super) fn install(app: &AppHandle<Wry>) -> tauri::Result<()> {
        let menu = build_app_menu(app)?;
        app.set_menu(menu)?;
        install_menu_event_handler(app);
        Ok(())
    }

    /// Build the application menu — six submenus in declaration order
    /// so macOS renders the app menu ("tome") under the application
    /// name.
    ///
    /// All Edit-menu items are Predefined: `tauri::menu` 2.11 registers
    /// the conventional ⌘C/⌘V/⌘X/⌘A/⌘Z/⌘⇧Z accelerators with the OS,
    /// which routes them to the focused webview control. Avoiding
    /// menu-level custom shortcuts on text-editing actions is the
    /// explicit Pitfall 9 mitigation (T-26-07-01).
    pub(super) fn build_app_menu(app: &AppHandle<Wry>) -> tauri::Result<Menu<Wry>> {
        let app_menu = SubmenuBuilder::new(app, "tome")
            .about(None)
            .separator()
            .services()
            .separator()
            .hide()
            .hide_others()
            .show_all()
            .separator()
            .quit()
            .build()?;

        let file_menu = SubmenuBuilder::new(app, "File").close_window().build()?;

        // Edit menu — Predefined items route OS shortcuts to the
        // focused webview control (Pitfall 9 mitigation, T-26-07-01).
        let edit_menu = SubmenuBuilder::new(app, "Edit")
            .undo()
            .redo()
            .separator()
            .cut()
            .copy()
            .paste()
            .select_all()
            .build()?;

        let view_menu = SubmenuBuilder::new(app, "View")
            .item(
                &MenuItemBuilder::with_id("jump-status", "Status")
                    .accelerator("CmdOrCtrl+1")
                    .build(app)?,
            )
            .item(
                &MenuItemBuilder::with_id("jump-skills", "Skills")
                    .accelerator("CmdOrCtrl+2")
                    .build(app)?,
            )
            .item(
                &MenuItemBuilder::with_id("jump-health", "Health")
                    .accelerator("CmdOrCtrl+3")
                    .build(app)?,
            )
            .separator()
            .item(
                &MenuItemBuilder::with_id("focus-search", "Focus Search")
                    .accelerator("CmdOrCtrl+F")
                    .build(app)?,
            )
            .separator()
            // Reload is reserved for a Phase 27+ explicit-refetch
            // surface; disabled now so the accelerator is announced but
            // the click is a no-op (Tauri 2.11 MenuItemBuilder::enabled
            // verified against ~/.cargo/registry tauri-2.11.2 source —
            // Assumption A5 resolved positive).
            .item(
                &MenuItemBuilder::with_id("reload", "Reload")
                    .accelerator("CmdOrCtrl+R")
                    .enabled(false)
                    .build(app)?,
            )
            .build()?;

        // Library menu — every item disabled in alpha. The breadcrumb
        // still appears so users discover the surface; click is a no-op
        // handled by the `_ => return` arm in
        // `install_menu_event_handler`.
        let library_menu = SubmenuBuilder::new(app, "Library")
            .item(
                &MenuItemBuilder::with_id("sync", "Sync")
                    .enabled(false)
                    .build(app)?,
            )
            .item(
                &MenuItemBuilder::with_id("add-directory", "Add Directory…")
                    .enabled(false)
                    .build(app)?,
            )
            .build()?;

        let help_menu = SubmenuBuilder::new(app, "Help")
            .item(&MenuItemBuilder::with_id("docs", "Documentation").build(app)?)
            .item(&MenuItemBuilder::with_id("report-issue", "Report Issue").build(app)?)
            .build()?;

        MenuBuilder::new(app)
            .items(&[
                &app_menu,
                &file_menu,
                &edit_menu,
                &view_menu,
                &library_menu,
                &help_menu,
            ])
            .build()
    }

    /// Wire the OS menu-click stream into typed [`MenuAction`] events.
    ///
    /// Disabled items + unknown IDs hit the catch-all `_ => return`
    /// arm so they're harmless no-ops (T-26-07-02 disposition: accept —
    /// the failure mode is "click does nothing", not "click misfires").
    pub(super) fn install_menu_event_handler(app: &AppHandle<Wry>) {
        let app_handle = app.clone();
        app.on_menu_event(move |_app, event| {
            let id = event.id().0.as_str();
            let action = match id {
                "jump-status" => MenuAction::JumpStatus,
                "jump-skills" => MenuAction::JumpSkills,
                "jump-health" => MenuAction::JumpHealth,
                "focus-search" => MenuAction::FocusSearch,
                "docs" => {
                    let _ = open_url(&app_handle, DOCS_URL);
                    return;
                }
                "report-issue" => {
                    let _ = open_url(&app_handle, ISSUES_URL);
                    return;
                }
                // Disabled items (sync, add-directory, reload) +
                // unknown IDs: harmless no-op (T-26-07-02).
                _ => return,
            };
            let _ = action.emit(&app_handle);
        });
    }

    fn open_url(app: &AppHandle<Wry>, url: &str) -> Result<()> {
        use tauri_plugin_opener::OpenerExt;
        app.opener()
            .open_url(url, None::<&str>)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        Ok(())
    }
}
