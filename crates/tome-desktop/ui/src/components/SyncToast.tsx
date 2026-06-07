// SyncToast — transient "Sync complete" notification (Phase 27 plan 27-04
// / D-06 success branch).
//
// **Pitfall 2 / RESEARCH §"Don't Hand-Roll".** This is a deliberate
// hand-roll: `role="status" aria-live="polite" aria-atomic="true"` on a
// `<div>` with a `useEffect` + `setTimeout` lifecycle. We are NOT using
// `react-aria-components::UNSTABLE_ToastRegion` / `UNSTABLE_ToastQueue`.
// The UNSTABLE prefix means the API can change in a non-major version of
// `react-aria-components` — a real production-risk for v1.0. The toast
// surface in v1.0 is exactly two places (this component, for "Sync
// complete"), so the UNSTABLE-prefixed React Aria API would be over-
// engineering. (If a future phase needs toasts in more places, promote
// to the UNSTABLE component then.)
//
// **Cancellation does NOT use SyncToast.** D-18 supersedes D-06's
// original "Sync cancelled toast" phrasing: cancellation surfaces INLINE
// in the StageStepper's terminal branch with a heading + Run sync /
// Dismiss buttons. Toasts ship for SUCCESS ONLY.
//
// **Pattern carry-over from Pill.tsx.** The role + aria triplet matches
// `components/Pill.tsx:18-20` verbatim — Phase 26's transient
// "Updated" acknowledgement next to "Last sync" in the Status view.
// Same a11y contract, different mount strategy (Pill stays mounted
// while in DOM; SyncToast self-unmounts via the parent's `onDismiss`
// after `durationMs`).

import { useEffect } from "react";
import styles from "./SyncToast.module.css";

export interface SyncToastProps {
  /** The user-facing message. v1.0 only ever sets "Sync complete". */
  message: string;
  /** Invoked after `durationMs` elapses so the parent can unmount the
   *  toast. The parent owns the rendered-or-not decision; this
   *  component owns only the visible content + the timer. */
  onDismiss?: () => void;
  /** Auto-dismiss timeout in milliseconds. Default 5000 per UI-SPEC
   *  §States §Terminal state (standard macOS NSAlert transient cadence:
   *  ~3s visible + ~200ms fades; we use 5s outer for the unmount and
   *  let CSS handle the fade transitions inside the lifetime). */
  durationMs?: number;
}

export function SyncToast({
  message,
  onDismiss,
  durationMs = 5000,
}: SyncToastProps) {
  useEffect(() => {
    const timer = setTimeout(() => {
      onDismiss?.();
    }, durationMs);
    return () => clearTimeout(timer);
  }, [durationMs, onDismiss]);

  return (
    <div
      role="status"
      aria-live="polite"
      aria-atomic="true"
      className={styles.toast}
    >
      <span className={styles.message}>{message}</span>
      <button
        type="button"
        onClick={onDismiss}
        aria-label="Dismiss sync notification"
        className={styles.dismiss}
      >
        Dismiss
      </button>
    </div>
  );
}
