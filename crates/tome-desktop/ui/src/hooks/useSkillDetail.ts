// useSkillDetail — Skills detail-pane data hook.
//
// Fetches `commands.getSkillDetail(name)` on every `name` change. Subscribes
// to the watcher events that affect the detail-pane payload (D-03 silent
// refresh contract from plan 26-06):
//   - manifest-changed: source path / hash / sync timestamp.
//   - library-changed: SKILL.md body content.
//   - machine-prefs-changed: disabled flag.
//
// Lockfile changes are NOT subscribed (NF-05 contract — the detail pane
// doesn't render lockfile state). When `name === null` the hook returns
// `{ detail: null, err: null }` immediately.

import { useCallback, useEffect, useState } from "react";
import { commands, events } from "../bindings";
import type { SkillDetail, TomeError } from "../bindings";
import { useTauriEvent } from "./useTauriEvent";

export interface UseSkillDetailResult {
  detail: SkillDetail | null;
  err: TomeError | null;
  /** Manually trigger a refetch (used after own-process mutations). */
  refetch: () => Promise<void>;
}

export function useSkillDetail(name: string | null): UseSkillDetailResult {
  const [detail, setDetail] = useState<SkillDetail | null>(null);
  const [err, setErr] = useState<TomeError | null>(null);

  const refetch = useCallback(async () => {
    if (name === null) {
      setDetail(null);
      setErr(null);
      return;
    }
    // S-8 Result-narrowing — no try/catch around the typed discriminated-union.
    const res = await commands.getSkillDetail(name);
    if (res.status === "ok") {
      setDetail(res.data);
      setErr(null);
    } else {
      setErr(res.error);
      setDetail(null);
    }
  }, [name]);

  useEffect(() => {
    refetch();
  }, [refetch]);

  // Plan 26-06 event-subscription matrix — DetailHeader depends on manifest
  // (source path / hash / sync), library (SKILL.md body), and machine prefs
  // (disabled flag). Lockfile changes don't affect the detail shape (NF-05).
  useTauriEvent(events.manifestChanged, refetch);
  useTauriEvent(events.libraryChanged, refetch);
  useTauriEvent(events.machinePrefsChanged, refetch);

  return { detail, err, refetch };
}
