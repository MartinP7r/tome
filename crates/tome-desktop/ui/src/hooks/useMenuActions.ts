// useMenuActions — Phase 26 plan 26-07, NF-03.
//
// Subscribes to the typed `menuAction` Tauri event emitted by the
// native macOS menu bar (`crates/tome-desktop/src/menu.rs`). The four
// View-menu items (Status / Skills / Health / Focus Search) are
// in-process custom menu items; their click routes through the typed
// event so React can switch views or focus the SearchField without
// duplicating the keyboard map in two places.
//
// The Edit menu's ⌘C/⌘V/⌘X/⌘A/⌘Z/⌘⇧Z are Predefined macOS items —
// they route to the focused webview control automatically and never
// reach this hook (Pitfall 9 mitigation, T-26-07-01).
//
// Off-macOS this hook is a no-op subscriber: the Tauri side never
// emits `menuAction` on non-mac (the menu module's installation is
// `#[cfg(target_os = "macos")]`), so the listener attaches but never
// fires. We keep the subscription unconditional so the binding shape
// stays cross-platform stable.

import { useEffect } from "react";
import { events } from "../bindings";
import { setView } from "../stores/router";

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
 *  event for the lifetime of the app. Cleanup unlistens on unmount. */
export function useMenuActions(): void {
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
}
