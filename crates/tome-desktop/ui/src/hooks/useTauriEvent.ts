// useTauriEvent — generic Tauri event listener with cleanup-on-unmount.
//
// Phase 26 plan 26-06. Subscribes to a tauri-specta event and invokes
// `handler` on every fire. Guards the late-listen race: if the component
// unmounts before `listen()` resolves, the eventual `unlisten` is called
// straight away and the handler is suppressed for any in-flight fire.
//
// Anti-pattern guard (RESEARCH §Anti-Patterns "subscribe-only-what-you-
// depend-on"): each consumer hook subscribes EXPLICITLY to the events it
// depends on. There is no "subscribe to everything" sugar — over-subscribing
// triggers unnecessary refetches and wastes IPC bandwidth.

import { useEffect } from "react";
import type { Event, EventCallback } from "@tauri-apps/api/event";

/**
 * The shape of a tauri-specta event object's `listen` surface. We only need
 * `listen` here; `once` / `emit` are not the consumer's concern. The wider
 * `events.X` object satisfies this contract structurally.
 */
export interface EventListener<T> {
  listen: (cb: EventCallback<T>) => Promise<() => void>;
}

/**
 * Subscribe to `event` for the lifetime of the component. The handler is
 * invoked with no arguments — Phase-26 watcher events are unit structs (no
 * payload); consumers refetch on the fact of an event firing, not its
 * contents.
 *
 * `event` and `handler` are included in the effect deps because TypeScript
 * narrowing on a stable identity is the consumer's responsibility (in
 * practice both are stable references from `bindings.ts` / `useCallback`).
 */
export function useTauriEvent<T = unknown>(
  event: EventListener<T>,
  handler: () => void,
): void {
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;
    event
      .listen(((_e: Event<T>) => {
        if (!cancelled) handler();
      }) as EventCallback<T>)
      .then((un) => {
        if (cancelled) un();
        else unlisten = un;
      });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [event, handler]);
}
