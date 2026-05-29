// useSkills — Skills view data hook.
//
// Fetches `commands.listSkills()` once on mount. Stores skills, warnings,
// and error state independently. No event subscriptions in this plan; the
// file-watcher refetch lands in plan 26-06 — the hook surface accepts the
// extension without breaking changes (matching the useStatus shape).

import { useCallback, useEffect, useState } from "react";
import { commands } from "../bindings";
import type { DiscoveredSkill, TomeError } from "../bindings";

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

  return { skills, warnings, err, refetch };
}
