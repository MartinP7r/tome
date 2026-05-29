// useStatus — Status view data hook (RESEARCH §"Pattern 2 — React side").
//
// Fetches via commands.getStatus(), narrows the discriminated-union result,
// and tracks an `updatedAt` timestamp so the StatusView can render the
// transient "Updated" pill (D-03). Plan 26-06 will add event-driven
// auto-refetch (manifest-changed / lockfile-changed / machine-prefs-changed
// / library-changed) — that's intentionally NOT in this plan; the hook
// surface accepts the extension without breaking changes.

import { useCallback, useEffect, useState } from "react";
import { commands } from "../bindings";
import type { StatusReport_Serialize, TomeError } from "../bindings";

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

  return { status, err, updatedAt, refetch };
}
