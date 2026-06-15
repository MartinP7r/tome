// Pill — atom (UI-SPEC §Atoms — Pill — Updated).
//
// The transient "Updated" acknowledgement next to "Last sync" in the Status
// view. role=status + aria-live=polite so VoiceOver reads it once. CSS owns
// the fade.

import type { ReactNode } from "react";
import styles from "./Pill.module.css";

export interface PillProps {
  variant: "updated";
  children: ReactNode;
}

export function Pill({ variant, children }: PillProps) {
  return (
    <span
      role="status"
      aria-live="polite"
      aria-atomic="true"
      className={[styles.pill, styles[variant]].join(" ")}
    >
      {children}
    </span>
  );
}
