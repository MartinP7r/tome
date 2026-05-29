// KeyValueRow — Status view atom (UI-SPEC §Molecules — KeyValueRow).
//
// Renders a label / value / optional trailing slot. Used by the Status view
// for every field row (TOME HOME / LIBRARY / LAST SYNC / LOCKFILE / MACHINE).

import type { ReactNode } from "react";
import styles from "./KeyValueRow.module.css";

export interface KeyValueRowProps {
  label: string;
  value: ReactNode;
  /** Render the value in monospace (paths, hashes). */
  mono?: boolean;
  /** Optional trailing slot — Pill, StatusDot, Badge, count text. */
  trailing?: ReactNode;
}

export function KeyValueRow({
  label,
  value,
  mono = false,
  trailing,
}: KeyValueRowProps) {
  return (
    <div className={styles.row}>
      <div className={styles.label}>{label}</div>
      <div className={mono ? styles.valueMono : styles.value}>{value}</div>
      <div className={styles.trailing}>{trailing}</div>
    </div>
  );
}
