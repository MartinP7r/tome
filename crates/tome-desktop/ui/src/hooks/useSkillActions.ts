// useSkillActions — shared action dispatcher for the Skills detail header,
// context menu, and ⌘C/⌘O/⌘D shortcuts.
//
// Owns three responsibilities:
//   1. invoke the 3 Tauri commands (`open_source_folder`, `copy_path`,
//      `set_skill_disabled`) with structured-error narrowing,
//   2. drive the transient "copied" state for the Copy button (D-07),
//   3. emit aria-live announcements after a successful Disable / Enable so
//      VoiceOver users hear the state change (UI-SPEC §VoiceOver labels).
//
// `refetch` is passed in from `useSkillDetail` so the disable click can pull
// the fresh disabled flag immediately — in production the file watcher will
// also fire `machine-prefs-changed` and the hook will refetch a second time
// (cheap; both reads are idempotent).

import { useCallback, useEffect, useRef, useState } from "react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { commands } from "../bindings";
import type { TomeError } from "../bindings";
import {
  disableSuccessAnnouncement,
  enableSuccessAnnouncement,
} from "../lib/ariaLabels";
import type { CopyState } from "../components/DetailHeader";

export interface UseSkillActionsArgs {
  /** The skill name to operate on. */
  name: string | null;
  /** Current disabled flag (drives the Disable / Enable label). */
  disabled: boolean;
  /** Caller refetch — invoked after a Disable click for instant feedback. */
  refetch: () => Promise<void> | void;
}

export interface UseSkillActionsResult {
  copyState: CopyState;
  err: TomeError | null;
  /** Last aria-live message; clears after ~2s. */
  announcement: string;
  onOpenSource: () => Promise<void>;
  onCopyPath: () => Promise<void>;
  onDisableToggle: () => Promise<void>;
  clearError: () => void;
}

const COPY_FLASH_MS = 2000;
const ANNOUNCEMENT_CLEAR_MS = 2000;

export function useSkillActions({
  name,
  disabled,
  refetch,
}: UseSkillActionsArgs): UseSkillActionsResult {
  const [copyState, setCopyState] = useState<CopyState>("idle");
  const [err, setErr] = useState<TomeError | null>(null);
  const [announcement, setAnnouncement] = useState<string>("");
  // Track running timers so we can cancel on unmount / re-trigger.
  const copyTimer = useRef<number | null>(null);
  const announceTimer = useRef<number | null>(null);

  useEffect(() => {
    return () => {
      if (copyTimer.current !== null) window.clearTimeout(copyTimer.current);
      if (announceTimer.current !== null)
        window.clearTimeout(announceTimer.current);
    };
  }, []);

  const clearError = useCallback(() => setErr(null), []);

  const onOpenSource = useCallback(async () => {
    if (name === null) return;
    const res = await commands.openSourceFolder(name);
    if (res.status === "error") setErr(res.error);
  }, [name]);

  const onCopyPath = useCallback(async () => {
    if (name === null) return;
    const res = await commands.copyPath(name);
    if (res.status === "error") {
      setErr(res.error);
      return;
    }
    try {
      await writeText(res.data);
    } catch (e) {
      // Clipboard plugin rejection — surface as a generic error banner. The
      // path was resolved successfully; only the OS-side write failed.
      setErr({
        code: "Internal",
        message: `Clipboard write failed: ${e instanceof Error ? e.message : String(e)}`,
        context: [],
      });
      return;
    }
    setCopyState("copied");
    if (copyTimer.current !== null) window.clearTimeout(copyTimer.current);
    copyTimer.current = window.setTimeout(() => {
      setCopyState("idle");
      copyTimer.current = null;
    }, COPY_FLASH_MS);
  }, [name]);

  const onDisableToggle = useCallback(async () => {
    if (name === null) return;
    const next = !disabled;
    const res = await commands.setSkillDisabled(name, next);
    if (res.status === "error") {
      setErr(res.error);
      return;
    }
    // Aria-live announcement. The file watcher will fire
    // `machine-prefs-changed` and `useSkillDetail` will refetch on its own;
    // the explicit `refetch()` keeps the in-flight UI snappy for the user
    // who just clicked.
    const message = next
      ? disableSuccessAnnouncement(name)
      : enableSuccessAnnouncement(name);
    setAnnouncement(message);
    if (announceTimer.current !== null)
      window.clearTimeout(announceTimer.current);
    announceTimer.current = window.setTimeout(() => {
      setAnnouncement("");
      announceTimer.current = null;
    }, ANNOUNCEMENT_CLEAR_MS);
    await refetch();
  }, [name, disabled, refetch]);

  return {
    copyState,
    err,
    announcement,
    onOpenSource,
    onCopyPath,
    onDisableToggle,
    clearError,
  };
}
