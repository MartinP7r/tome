// useSkills — Skills view data hook.
//
// Fetches `commands.listSkills()` on mount, then refetches on file-watcher
// events that affect the discovered skill list shape. Plan 26-06 added the
// event subscriptions; lockfile changes are intentionally NOT subscribed —
// they don't shift the list shape (NF-05 contract; matrix in plan 26-06
// §interfaces).

import { useCallback, useEffect, useState } from "react";
import { commands, events } from "../bindings";
import type { DiscoveredSkill, TomeError } from "../bindings";
import { useTauriEvent } from "./useTauriEvent";

export interface UseSkillsResult {
  skills: DiscoveredSkill[] | null;
  warnings: string[];
  err: TomeError | null;
  /** Manually trigger a refetch. */
  refetch: () => Promise<void>;
}

export function useSkills(): UseSkillsResult {
  const [skills, setSkills] = useState<DiscoveredSkill[] | null>(null);
  const [warnings, setWarnings] = useState<string[]>([]);
  const [err, setErr] = useState<TomeError | null>(null);

  const refetch = useCallback(async () => {
    // S-8 Result-narrowing — no try/catch around the typed discriminated-union.
    const res = await commands.listSkills();
    if (res.status === "ok") {
      setSkills(res.data.skills);
      setWarnings(res.data.warnings);
      setErr(null);
    } else {
      setErr(res.error);
    }
  }, []);

  useEffect(() => {
    refetch();
  }, [refetch]);

  // Plan 26-06 event-subscription matrix — Skills depends on manifest +
  // library + machine-prefs. Lockfile changes don't affect the list shape.
  useTauriEvent(events.manifestChanged, refetch);
  useTauriEvent(events.libraryChanged, refetch);
  useTauriEvent(events.machinePrefsChanged, refetch);

  return { skills, warnings, err, refetch };
}
