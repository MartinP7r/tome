// useMenuActions — Phase 26 plan 26-07 (NF-03) + Phase 27 plan 27-01b.
//
// Subscribes to the typed `menuAction` Tauri event emitted by the
// native macOS menu bar (`crates/tome-desktop/src/menu.rs`). The five
// View / Library menu items dispatch through one typed event so the
// React side can switch views, focus the SearchField, or kick off a
// sync without duplicating the keyboard map across files.
//
// **Phase 27 plan 27-01b additions.**
//
//   - `JumpSync` routes `setView("sync")`. Same intent shape as
//     JumpStatus / JumpSkills / JumpHealth.
//   - Global ⌘R keybinding. When idle → kick off a sync via
//     `useSync().start()`. When running → `useSync().cancel()`.
//     When terminal → no-op for now (retry handlers land in 27-04).
//     The Library → Sync menu item ALSO sends a JumpSync event with
//     accelerator ⌘R, so on macOS the menu fires the navigation +
//     the global keybinding handles the run-now intent in parallel.
//   - Global ⌘. (period). Cancels an in-flight sync; no-op otherwise.
//     This mirrors the system convention (e.g., Finder cancel).
//   - Both global keys ABSTAIN when the user has a text input focused
//     so they don't collide with text-editing — Pitfall 9 idiom from
//     Phase 26 SkillsView (now shared via `lib/textInputFocus.ts`).
//
// The Edit menu's ⌘C/⌘V/⌘X/⌘A/⌘Z/⌘⇧Z are Predefined macOS items —
// they route to the focused webview control automatically and never
// reach this hook (Pitfall 9 mitigation, T-26-07-01).
//
// Off-macOS the menu module's installation is `#[cfg(target_os = "macos")]`
// so the typed `menuAction` event never fires. The global ⌘R / ⌘. window
// listeners still attach unconditionally because Linux + Windows builds
// would still benefit if they ever land (D-GUI-06 currently keeps the
// GUI macOS-only).

import { useEffect } from "react";
import { events } from "../bindings";
import { isTextInputFocused } from "../lib/textInputFocus";
import { setView } from "../stores/router";
import { useSync } from "./useSync";

/** Selector for the SearchField input — sourced from
 *  `SearchField.tsx`'s `aria-label="Search skills"` (UI-SPEC
 *  §VoiceOver labels). React Aria nests the actual `<input>` inside
 *  the labelled `<div role="searchbox">`, so we walk both
 *  candidates. */
const SEARCH_FIELD_INPUT_SELECTORS = [
  'input[aria-label="Search skills"]',
  '[aria-label="Search skills"] input',
];

function focusSearchField(): void {
  for (const sel of SEARCH_FIELD_INPUT_SELECTORS) {
    const el = document.querySelector<HTMLInputElement>(sel);
    if (el) {
      el.focus();
      return;
    }
  }
  // Not on the Skills view (search field isn't mounted) — switch to
  // Skills first so the next render brings the input into the DOM,
  // then focus on the next microtask.
  setView("skills");
  requestAnimationFrame(() => {
    for (const sel of SEARCH_FIELD_INPUT_SELECTORS) {
      const el = document.querySelector<HTMLInputElement>(sel);
      if (el) {
        el.focus();
        return;
      }
    }
  });
}

/** Mount once at the App root. Subscribes to the typed `menuAction`
 *  event for the lifetime of the app + binds the global ⌘R / ⌘.
 *  keyboard handlers (Phase 27 plan 27-01b). Cleanup unlistens on
 *  unmount. */
export function useMenuActions(): void {
  // Sync intent split (plan 27-01b): the menu's Library → Sync item
  // dispatches JumpSync (navigation); the global ⌘R keybinding kicks
  // off the actual run. On macOS the menu accelerator fires the menu
  // event, so both intents happen — we navigate AND start. On Linux/
  // Windows (when the menu installation no-ops) the keybinding still
  // works, so the surface degrades gracefully.
  const sync = useSync();

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;

    events.menuAction
      .listen((evt) => {
        if (cancelled) return;
        switch (evt.payload.kind) {
          case "JumpStatus":
            setView("status");
            break;
          case "JumpSkills":
            setView("skills");
            break;
          case "JumpSync":
            setView("sync");
            break;
          case "JumpHealth":
            setView("health");
            break;
          case "FocusSearch":
            focusSearchField();
            break;
        }
      })
      .then((un) => {
        if (cancelled) un();
        else unlisten = un;
      });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  // Global keyboard handlers (plan 27-01b). Captured at the window level
  // so they fire regardless of which view is mounted. Both abstain when
  // text input has focus so they don't collide with editing.
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent): void {
      // ⌘R — start (idle) / cancel (running). The Library → Sync menu
      // item ALSO has CmdOrCtrl+R registered as a Tauri accelerator,
      // which on macOS routes through the menu rather than the
      // webview's keydown stream. We keep the global handler so the
      // intent works on non-mac builds + as a defensive fallback if
      // the menu accelerator ever misses. Run-now does NOT abstain on
      // input-focus — Sync is not an Edit action; user input doesn't
      // own ⌘R semantics.
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "r") {
        // Browser default reload — we own this key.
        e.preventDefault();
        if (sync.isRunning) {
          void sync.cancel();
        } else if (sync.outcome === null) {
          // Idle.
          void sync.start();
        }
        // Terminal → no-op for plan 27-01b. Retry / re-run handlers
        // land in 27-04 (cancel-aware re-arm) + 27-05 (failure-aware
        // retry of failed items).
        return;
      }

      // ⌘. (period) — universal cancel convention. Only meaningful
      // while a sync is running; no-op otherwise. Abstains on input
      // focus so the user's text-editing cancel-shortcut (e.g. ESC
      // alternatives in some editors) doesn't collide.
      if ((e.metaKey || e.ctrlKey) && e.key === ".") {
        if (isTextInputFocused()) return;
        if (sync.isRunning) {
          e.preventDefault();
          void sync.cancel();
        }
        return;
      }
    }

    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [sync]);
}
