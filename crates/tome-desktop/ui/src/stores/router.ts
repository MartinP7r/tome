// router.ts — tiny `view` store backed by `useSyncExternalStore`.
//
// We deliberately avoid Redux/Zustand for Phase 26 (RESEARCH §Anti-Patterns —
// the Phase 26 surface area is too small to justify a full state library;
// adding one now would saddle every later phase with that dependency for no
// real gain). A 25-line subscribable + `useSyncExternalStore` gives every
// consumer a tear-free read of the current view and a stable `setView`
// dispatcher.
//
// State shape stays flat — adding a new view (e.g. Sync, Config in Phase 27)
// is a literal-union extension.
//
// Phase 27 plan 27-01b extends the union with `"sync"` between `"skills"` and
// `"health"` to match the sidebar render order (Status → Skills → Sync →
// Health) and the menu-bar accelerators (⌘1..⌘4 in the same order).

import { useSyncExternalStore } from "react";

export type View = "status" | "skills" | "sync" | "health";

type Listener = () => void;

let currentView: View = "status"; // D-02 — lands on Status.
const listeners = new Set<Listener>();

function subscribe(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function getSnapshot(): View {
  return currentView;
}

export function setView(view: View): void {
  if (currentView === view) return;
  currentView = view;
  for (const l of listeners) l();
}

export function useRouter(): { view: View; setView: (v: View) => void } {
  const view = useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
  return { view, setView };
}
