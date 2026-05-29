---
phase: 26-read-only-views-alpha-cut
plan: 07
subsystem: desktop-gui
tags: [tauri, menu, macos, a11y, hig, axe-core, playwright, NF-02, NF-03]
status: in-progress
requires:
  - 26-01-SUMMARY  # StatusReport surface
  - 26-02-SUMMARY  # router store + Sidebar shell
  - 26-03-SUMMARY  # DetailHeader + ⌘C/⌘O/⌘D bindings audited here
  - 26-04-SUMMARY  # MarkdownBody + Vitest setup
  - 26-05-SUMMARY  # HealthView + PreviewPopover
  - 26-06-SUMMARY  # main.rs::setup watcher coexistence
provides:
  - "tauri::menu::Menu (macOS) — tome / File / Edit / View / Library / Help"
  - "tauri-specta::Event `MenuAction` (4 variants)"
  - "React hook `useMenuActions` (App.tsx-mounted)"
  - "axe-core/playwright CI gate (planned — Task 3)"
  - "VoiceOver smoke-test checklist (26-A11Y-AUDIT.md — planned Task 2)"
affects:
  - crates/tome-desktop/src/main.rs       # setup closure now installs menu
  - crates/tome-desktop/src/lib.rs        # collect_events! includes MenuAction
  - crates/tome-desktop/ui/src/App.tsx    # mounts useMenuActions; drops the doc-level ⌘1/⌘2/⌘3 listener
  - crates/tome-desktop/ui/src/views/SkillsView.tsx  # drops the ⌘F listener (menu owns it)
  - crates/tome-desktop/ui/src/bindings.ts  # regenerated to expose events.menuAction + MenuAction type
tech-stack:
  added:
    - "tauri::menu::{MenuBuilder, SubmenuBuilder, MenuItemBuilder} (already in tauri 2.11)"
    - "@axe-core/playwright@^4.11.3 (dev) — planned Task 3"
    - "playwright@^1.60.0 (dev) — planned Task 3"
  patterns:
    - "Cross-platform shim — install_menu() compiles everywhere; macOS submodule does the real work"
    - "POLISH-04 sentinel — MenuAction::ALL + exhaustiveness match block fails compile when variants drift"
    - "Predefined Edit items (Pitfall 9 mitigation, T-26-07-01)"
key-files:
  created:
    - crates/tome-desktop/src/menu.rs
    - crates/tome-desktop/ui/src/hooks/useMenuActions.ts
  modified:
    - crates/tome-desktop/src/lib.rs
    - crates/tome-desktop/src/main.rs
    - crates/tome-desktop/ui/src/App.tsx
    - crates/tome-desktop/ui/src/views/SkillsView.tsx
    - crates/tome-desktop/ui/src/bindings.ts
decisions:
  - "Compile MenuAction unconditionally cross-platform so bindings.ts is stable (Phase 26 is macOS-only but the binding shape must not flicker per-target)"
  - "Cross-platform install_menu() shim — main.rs::setup calls it without #[cfg], the macOS submodule does the real work"
  - "Approved package versions (Task 0): @axe-core/playwright@^4.11.3 (MPL-2.0, Deque), playwright@^1.60.0 (Apache-2.0, Microsoft)"
metrics:
  duration: in-progress
  completed: in-progress
  started: "2026-05-29T08:00:00Z"
---

# Phase 26 Plan 07: Cross-cutting a11y + macOS native menu bar — Summary (IN PROGRESS)

> Connection-resilience checkpoint — `crates/tome-desktop/src/menu.rs` + companion React hook landed in Task 1; Task 2 (keyboard audit + 26-A11Y-AUDIT.md) and Task 3 (axe-core CI gate) still ahead. SUMMARY will be rewritten on completion.

## What ships (so far)

A native macOS menu bar with the six submenus the alpha contract requires
(`tome` / `File` / `Edit` / `View` / `Library` / `Help`), wired through a
typed `MenuAction` Tauri event to the React router. The View menu's
Status / Skills / Health / Focus-Search items now drive the same router
state the Sidebar drives, with the OS rendering the accelerators
(⌘1 / ⌘2 / ⌘3 / ⌘F) directly under each item label.

The Edit menu is built entirely from `PredefinedMenuItem` calls — the OS
routes ⌘C / ⌘V / ⌘X / ⌘A / ⌘Z / ⌘⇧Z to the focused webview control by
itself, which is the explicit Pitfall 9 mitigation (no menu-level custom
shortcuts collide with text-input copy/paste).

## Tasks (live status)

- [x] Task 0 — Package legitimacy gate (`@axe-core/playwright` + `playwright`) — APPROVED by user with version + repo verification.
- [x] Task 1 — Native macOS menu bar + `MenuAction` event + `useMenuActions` hook (commit `c9ca2bb`).
- [ ] Task 2 — Keyboard shortcut conflict audit + UI-SPEC §Keyboard Map amendment + `26-A11Y-AUDIT.md`.
- [ ] Task 3 — axe-core/playwright CI gate (4 tests: Status / Skills / Health / PreviewPopover).

## Commits so far

| Task | Hash      | Subject |
|------|-----------|---------|
| 1    | `c9ca2bb` | feat(26-07): native macOS menu bar + MenuAction event + useMenuActions |

## Deviations from plan

None yet — the menu module's structure follows RESEARCH §"Pattern 7" line-for-line.

One implementation note worth recording explicitly: `MenuAction` is compiled cross-platform (the plan's interface block was ambiguous on this) so `bindings.ts` exports the same shape on every target. The `#[cfg(target_os = "macos")]` gate sits one level deeper, inside `menu.rs`'s `macos` submodule that owns `build_app_menu` + `install_menu_event_handler`. The public `install_menu()` function is a cross-platform shim that's a no-op off-mac — this means `main.rs::setup` doesn't need a per-target `#[cfg]` and the binding contract is target-stable.

## Self-Check

Pending — re-run after Tasks 2 + 3 land.
