// Tauri event mock (axe-core/playwright a11y gate, plan 26-07 Task 3).
//
// Replaces `@tauri-apps/api/event` when `A11Y_TEST=1` (Vite alias in
// `vite.config.ts`). Returns no-op listeners — the a11y gate doesn't
// exercise event-driven refresh flows; it scans the initial render
// tree only. (`useFsEvents` / `useDoctorReport` etc. still call
// `listen`; we hand them a `unlisten` function and never fire.)

/* eslint-disable @typescript-eslint/no-explicit-any */

export type EventCallback<T> = (event: { payload: T }) => void;

export async function listen<T>(
  _eventName: string,
  _cb: EventCallback<T>,
): Promise<() => void> {
  return () => undefined;
}

export async function once<T>(
  _eventName: string,
  _cb: EventCallback<T>,
): Promise<() => void> {
  return () => undefined;
}

export async function emit(_eventName: string, _payload?: any): Promise<void> {
  return undefined;
}

export const TauriEvent = {} as const;
