// useStatus — Status view data hook (RESEARCH §"Pattern 2 — React side").
//
// Fetches via commands.getStatus(), narrows the discriminated-union result,
// and tracks an `updatedAt` timestamp so the StatusView can render the
// transient "Updated" pill (D-03). Plan 26-06 added event-driven auto-
// refetch: every watcher event (manifest / lockfile / library / machine-
// prefs) refetches and resets `updatedAt = Date.now()` so the Pill flashes
// for ~2s on every silent refresh.
//
// Status subscribes to ALL FOUR events because every StatusReport field
// can shift when any of the four roots change (library count, lockfile
// state, machine prefs summary, last sync). Plan 26-06 §interfaces matrix.

import { useCallback, useEffect, useState } from "react";
import { commands, events } from "../bindings";
import type { StatusReport_Serialize, TomeError } from "../bindings";
import { useTauriEvent } from "./useTauriEvent";

export interface UseStatusResult {
  status: StatusReport_Serialize | null;
  err: TomeError | null;
  /** Last time `status` was refreshed (ms since epoch) — feeds the "Updated"
   *  pill timing. Null until the first successful fetch. */
  updatedAt: number | null;
  /** Manually trigger a refetch. */
  refetch: () => Promise<void>;
}

export function useStatus(): UseStatusResult {
  const [status, setStatus] = useState<StatusReport_Serialize | null>(null);
  const [err, setErr] = useState<TomeError | null>(null);
  const [updatedAt, setUpdatedAt] = useState<number | null>(null);

  const refetch = useCallback(async () => {
    // Result-narrowing pattern from Phase 25 App.tsx — no try/catch around
    // the typed discriminated-union result.
    const res = await commands.getStatus();
    if (res.status === "ok") {
      setStatus(res.data);
      setErr(null);
      setUpdatedAt(Date.now());
    } else {
      setErr(res.error);
    }
  }, []);

  useEffect(() => {
    refetch();
  }, [refetch]);

  // Plan 26-06 event-subscription matrix — Status row depends on all 4
  // watched roots. Each subscription is a separate hook call so cleanup is
  // owned per-event (matches React useEffect mental model).
  useTauriEvent(events.manifestChanged, refetch);
  useTauriEvent(events.lockfileChanged, refetch);
  useTauriEvent(events.libraryChanged, refetch);
  useTauriEvent(events.machinePrefsChanged, refetch);

  return { status, err, updatedAt, refetch };
}
