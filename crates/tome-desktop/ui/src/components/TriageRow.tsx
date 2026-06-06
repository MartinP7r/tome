// TriageRow — UI-SPEC §TriageRow (Phase 27 plan 27-02).
//
// One row inside a TriagePanel section. 52px-tall layout matches the
// Phase 26 SkillListRow rhythm (so the two views read at the same
// vertical cadence). Composes:
//
//   - Primary line — skill name (text-body 13px / 400; 600 when selected).
//   - Secondary line — `${source} · ${managed|local} · synced ${rel}`.
//   - Trailing chip — inline `[✓ keep]` or `[⊘ disabled here]` toggle
//     button for Added/Changed rows; static `[implicit remove]` span for
//     Removed rows (D-13 invariant — REMOVED has no user decision).
//
// **Pitfall 1 — GridListItem, NOT ListBoxItem.** React Aria forbids
// interactive children inside ListBoxItem because they break keyboard
// nav and screen-reader semantics. The inline `[✓ keep]` toggle IS a
// button (it's the one-click shorthand for the canonical RadioGroup
// picker in TriageDetail per D-12), so the parent must be a GridList.
// TriagePanel wraps this in `<GridListItem>` so the row composes safely.
//
// **Parent contract.** This component renders the row body only — the
// parent GridListItem owns selection state, keyboard nav, and the
// `data-selected` attribute the CSS targets for the accent fill. The
// trailing chip uses an HTML `<button>` (NOT React Aria's Button) so
// clicks inside it don't trigger the row's `onAction` selection.

import type { DirectoryName, SkillName, SkillOrigin } from "../bindings";
import { formatRelative } from "../lib/relativeTime";
import styles from "./TriageRow.module.css";

/** The triage decision a row can carry. `"keep"` (default) leaves the
 *  skill enabled on this machine; `"disable"` adds it to the machine's
 *  `disabled` set on Apply. */
export type TriageDecision = "keep" | "disable";

export interface TriageRowProps {
  name: SkillName;
  /** Which bucket this row belongs to. Removed rows render an
   *  `[implicit remove]` non-interactive chip per D-13. */
  changeKind: "added" | "changed" | "removed";
  /** Owning directory (or null for Unowned-at-time-of-removal rows). */
  sourceName: DirectoryName | null;
  /** Managed-vs-local classification (drives the secondary-line label). */
  origin: SkillOrigin;
  /** Manifest synced_at (or null when no manifest entry — e.g. Added). */
  syncedAt: string | null;
  /** Current decision for this row (Added/Changed only). Removed rows
   *  ignore this prop. */
  decision: TriageDecision;
  /** Toggle keep ⇄ disable (D-12 inline chip handler). Removed rows
   *  never invoke this (the chip is non-interactive). */
  onDecisionToggle: () => void;
  /** Render selection state — drives accent-fill background via CSS. */
  isSelected: boolean;
}

/** Format the secondary line per UI-SPEC. Added rows have no
 *  synced_at — render `synced —`. */
function buildSecondary(
  sourceName: DirectoryName | null,
  origin: SkillOrigin,
  syncedAt: string | null,
): string {
  const sourceLabel = sourceName ?? "unowned";
  const originLabel = origin.kind === "managed" ? "managed" : "local";
  const syncedLabel = syncedAt === null ? "synced —" : `synced ${formatRelative(syncedAt)}`;
  return `${sourceLabel} · ${originLabel} · ${syncedLabel}`;
}

export function TriageRow({
  name,
  changeKind,
  sourceName,
  origin,
  syncedAt,
  decision,
  onDecisionToggle,
  isSelected,
}: TriageRowProps) {
  const secondary = buildSecondary(sourceName, origin, syncedAt);

  // Determine chip rendering. Removed rows show a static label per D-13;
  // Added/Changed rows show the toggleable keep/disable chip.
  const chip =
    changeKind === "removed" ? (
      <span
        className={`${styles.chip} ${styles.chipRemoved}`}
        aria-hidden="true"
      >
        implicit remove
      </span>
    ) : (
      // Inline HTML button — NOT a React Aria Button — so click events
      // here do NOT bubble up to trigger the parent GridListItem's
      // `onAction` (selection). The aria-label fully describes the toggle
      // action per UI-SPEC §VoiceOver labels (D-12).
      <button
        type="button"
        className={`${styles.chip} ${
          decision === "keep" ? styles.chipKeep : styles.chipDisable
        }`}
        onClick={(e) => {
          // Stop bubbling so the row click handler (selection) and the
          // chip click handler (decision toggle) are independent
          // actions — the chip is the canonical D-12 one-click affordance.
          e.stopPropagation();
          onDecisionToggle();
        }}
        aria-label={`Toggle decision for ${name} between keep and disable on this machine`}
      >
        {decision === "keep" ? "✓ keep" : "⊘ disabled here"}
      </button>
    );

  return (
    <div
      className={styles.row}
      data-selected={isSelected ? "true" : undefined}
    >
      <div className={styles.text}>
        <span className={styles.primary}>{name}</span>
        <span className={styles.secondary}>{secondary}</span>
      </div>
      <span className={styles.trailing}>{chip}</span>
    </div>
  );
}
