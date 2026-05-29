// useDoctorReport — Health view data hook (Phase 26 plan 26-05 + 26-06).
//
// Fetches `commands.getDoctorReport()` on mount and on every watcher event
// that can shift the DoctorView shape. Per the plan 26-06 event-subscription
// matrix:
//   - manifest-changed: a stale entry was added/removed.
//   - library-changed: a SKILL.md frontmatter became parseable (or did
//     the opposite), a broken legacy symlink was removed, an orphan dir
//     appeared, etc.
//   - lockfile-changed: doctor's GUI surface in alpha doesn't render
//     lockfile-derived state (UnparsableFrontmatter / DivergingTarget /
//     LibraryStaleManifest / etc. are manifest+library-derived). Subscribed
//     anyway because lockfile state is part of the broader "is this library
//     healthy" question — when 26-08 adds the lockfile-divergence finding
//     this hook is ready. Cheap to refetch.
//   - machine-prefs-changed: NOT subscribed. Disabling a skill on this
//     machine doesn't change the doctor findings shape (disabled skills are
//     still distribution-tracked and can still be stale).
//
// Returns `{ report, err, refetch }` — the same shape useStatus /
// useSkillDetail / useSkills established in plan 26-02 / 26-03 / 26-06.

import { useCallback, useEffect, useState } from "react";
import { commands, events } from "../bindings";
import type { DoctorView_Serialize, TomeError } from "../bindings";
import { useTauriEvent } from "./useTauriEvent";

export interface UseDoctorReportResult {
  report: DoctorView_Serialize | null;
  err: TomeError | null;
  /** Manually trigger a refetch (used after a successful PreviewPopover
   *  Apply so the row drops within ~100ms without waiting for the
   *  watcher round-trip). */
  refetch: () => Promise<void>;
}

export function useDoctorReport(): UseDoctorReportResult {
  const [report, setReport] = useState<DoctorView_Serialize | null>(null);
  const [err, setErr] = useState<TomeError | null>(null);

  const refetch = useCallback(async () => {
    // S-8 Result-narrowing — no try/catch around the typed discriminated
    // union.
    const res = await commands.getDoctorReport();
    if (res.status === "ok") {
      setReport(res.data);
      setErr(null);
    } else {
      setErr(res.error);
    }
  }, []);

  useEffect(() => {
    refetch();
  }, [refetch]);

  // Plan 26-06 event-subscription matrix — manifest + library + lockfile.
  // machine-prefs NOT subscribed (per-machine disabled state doesn't affect
  // doctor findings).
  useTauriEvent(events.manifestChanged, refetch);
  useTauriEvent(events.libraryChanged, refetch);
  useTauriEvent(events.lockfileChanged, refetch);

  return { report, err, refetch };
}
