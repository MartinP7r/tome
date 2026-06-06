// SectionHeader — Health view + Sync triage section divider
// (UI-SPEC §Components §SectionHeader; extended in Phase 27 plan 27-02).
//
// Phase 26 ships SectionHeader at one nesting level (Health view's
// AUTO-FIXABLE / NEEDS ATTENTION groupings) — a single `<h2>` with label +
// count chip. Phase 27 plan 27-02 extends the component to support TWO
// nesting levels (D-11):
//
//   - Outer level (NEW / CHANGED / REMOVED in TriagePanel)        → `<h2>`
//   - Inner level (source-group within each outer, e.g. PLUGINS)  → `<h3>`
//
// Both levels reuse the same typography (--text-small 12px / 600 uppercase,
// --label-secondary). The inner level is indented 20px to read as nested
// when the screen reader's headings rotor traverses h2 → h3 → row.
//
// A `trailing` slot lets the outer triage section attach bulk-action
// buttons ("Disable all new", "Disable all new from PLUGINS") to the right
// of the count chip without re-rendering the heading structure (D-13).
//
// **Back-compat (preserved by default values).** Omitting `level` yields
// `<h2>` (matches Phase 26 Health-view consumer); omitting `trailing` yields
// no extra DOM. Existing call sites in HealthView.tsx work unchanged.

import type { ReactNode } from "react";
import styles from "./SectionHeader.module.css";

/** Heading level for the rendered element. `2` (default) → `<h2>`;
 *  `3` → `<h3>`. UI-SPEC §TriagePanel calls for `<h2>` outer + `<h3>`
 *  inner so the VoiceOver headings rotor reads nested groupings
 *  correctly. */
export type SectionHeaderLevel = 2 | 3;

export interface SectionHeaderProps {
  label: string;
  count: number;
  /** Heading level — `2` (default, back-compat) or `3` (inner source-group
   *  inside TriagePanel). */
  level?: SectionHeaderLevel;
  /** Right-aligned slot for a bulk-action button (D-13). Renders to the
   *  right of the count chip when provided. `undefined` (the default)
   *  yields no extra DOM and matches the Phase 26 visual contract. */
  trailing?: ReactNode;
}

export function SectionHeader({
  label,
  count,
  level = 2,
  trailing,
}: SectionHeaderProps) {
  // Level-specific class lets the CSS set indentation + nesting visuals
  // without affecting font weight or size (both stay --text-small / 600).
  const levelClass =
    level === 3 ? styles["header--level-3"] : styles["header--level-2"];

  const content = (
    <>
      <span className={styles.label}>{label}</span>
      <span className={styles.count}>({count})</span>
      {trailing !== undefined && (
        <span className={styles.trailing}>{trailing}</span>
      )}
    </>
  );

  // The headings rotor cares about the tag name (h2 vs h3), so we branch
  // at the JSX root rather than `as` prop-ing into a generic component.
  if (level === 3) {
    return (
      <h3 className={`${styles.header} ${levelClass}`}>{content}</h3>
    );
  }
  return <h2 className={`${styles.header} ${levelClass}`}>{content}</h2>;
}
