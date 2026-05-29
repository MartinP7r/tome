// SectionHeader — Health view section divider (UI-SPEC §Components §SectionHeader).
//
// Two callsites in the Health view: AUTO-FIXABLE / NEEDS ATTENTION. Both
// pair the uppercase caption label with a `(N)` count chip on the right.
//
// Rendered as `<h2>` so VoiceOver's headings rotor lists each section. The
// count is included in the heading text so screen readers announce
// "AUTO-FIXABLE, 3" in one breath. Plan 26-05.

import styles from "./SectionHeader.module.css";

export interface SectionHeaderProps {
  label: string;
  count: number;
}

export function SectionHeader({ label, count }: SectionHeaderProps) {
  return (
    <h2 className={styles.header}>
      <span className={styles.label}>{label}</span>
      <span className={styles.count}>({count})</span>
    </h2>
  );
}
