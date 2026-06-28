// useLockfileDiff — Phase 27 plan 27-02 / SYNC-02 triage data hook.
//
// Fetches `commands.getLockfileDiff()` on mount and on `lockfileChanged`
// events from the file watcher. The hook owns the read-only diff state
// that drives the SyncView's triage panel; triage *decisions* live
// separately in `useSync` (which is the cross-component state machine
// from 27-01b).
//
// **Pitfall 6 watcher-feedback discipline.** The hook MUST NOT refetch
// while a sync is running — the sync's Save stage rewrites
// `~/.tome/tome.lock` which would trigger `lockfileChanged` and recurse
// into a refetch loop mid-sync. The caller passes an `isRunningRef:
// RefObject<boolean>` (extracted from `useSync().isRunning`) and the
// `lockfileChanged` handler checks it before invoking `refetch`. The
// mount-time fetch is unconditional (the panel needs an initial
// snapshot; if a sync happens to be running at mount, the diff will
// reflect the pre-sync state which is still useful context).
//
// Only `lockfileChanged` is subscribed — NOT `manifestChanged`,
// `libraryChanged`, or `machinePrefsChanged`. The lockfile is what
// drives the diff projection; manifest changes that don't reach the
// lockfile (e.g. mid-sync stamping) don't change the diff payload.

import { useCallback, useEffect, useState, type RefObject } from "react";
import { commands, events } from "../bindings";
import type { LockfileDiff, TomeError } from "../bindings";

export interface UseLockfileDiffResult {
  /** Latest diff snapshot — `null` before the first fetch resolves. */
  diff: LockfileDiff | null;
  /** Most recent fetch error (cleared on a successful refetch). */
  err: TomeError | null;
  /** Manually trigger a refetch. The triage panel uses this after an
   *  Apply (27-03) lands. */
  refetch: () => Promise<void>;
}

/**
 * Subscribe to the lockfile diff for the SYNC-02 triage panel.
 *
 * @param isRunningRef Ref to the `useSync().isRunning` flag. When `true`,
 *                     the `lockfileChanged` handler suppresses the refetch
 *                     to honor the Pitfall 6 watcher-feedback discipline.
 */
export function useLockfileDiff(
  isRunningRef: RefObject<boolean>,
): UseLockfileDiffResult {
  const [diff, setDiff] = useState<LockfileDiff | null>(null);
  const [err, setErr] = useState<TomeError | null>(null);

  const refetch = useCallback(async () => {
    // S-8 Result-narrowing — no try/catch around the typed
    // discriminated-union return.
    const res = await commands.getLockfileDiff();
    if (res.status === "ok") {
      setDiff(res.data);
      setErr(null);
    } else {
      setErr(res.error);
    }
  }, []);

  // Mount-time fetch — unconditional. The panel needs an initial
  // snapshot regardless of whether a sync is in flight.
  useEffect(() => {
    void refetch();
  }, [refetch]);

  // Watcher-driven refetches — gated by `isRunningRef` per Pitfall 6.
  // Subscribed directly (not via `useTauriEvent`) so we can check the
  // ref before forwarding to `refetch`. The same shape as `useSync`'s
  // direct subscription pattern (27-01b decisions §note).
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;
    events.lockfileChanged
      .listen(() => {
        if (cancelled) return;
        // Pitfall 6 gate: skip refetches that originate from our own
        // in-flight sync's Save-stage lockfile rewrite.
        if (isRunningRef.current) return;
        void refetch();
      })
      .then((un) => {
        if (cancelled) un();
        else unlisten = un;
      });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [refetch, isRunningRef]);

  return { diff, err, refetch };
}
