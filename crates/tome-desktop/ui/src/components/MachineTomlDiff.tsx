// MachineTomlDiff — line-by-line diff renderer for the SYNC-03 Apply flow
// (Phase 27 plan 27-03 / UI-SPEC §MachineTomlDiff).
//
// Consumes a [`MachineTomlPreview`] from the Rust side (produced by
// `preview_machine_toml`) and renders it as a 3-column table:
//   - line-number gutter (right-aligned monospace; label-secondary).
//   - change-glyph gutter ('+' for added, '−' for removed, ' ' for
//     unchanged; weighted; danger/success/label-secondary color).
//   - content (left-aligned monospace; label-primary).
//
// A11y (UI-SPEC §VoiceOver labels):
//   - The table carries an aria-label with the additions / removals totals
//     so VoiceOver announces the diff size on focus.
//   - Removed + added rows each carry an aria-label naming the change kind
//     and the line number — VoiceOver reads "removed line 13" / "added
//     line 13".
//   - Unchanged rows are aria-hidden so VoiceOver doesn't read every equal
//     line of a multi-page TOML (the visual content still renders).
//
// Long lines wrap inside the popover's bounded width (480px max in
// PreviewPopover.module.css) — no horizontal scroll. The popover body's
// max-height + overflow-y keeps very large diffs scrollable.

import type { DiffLine, MachineTomlPreview } from "../bindings";
import styles from "./MachineTomlDiff.module.css";

export interface MachineTomlDiffProps {
  preview: MachineTomlPreview;
}

const ROW_CLASS_BY_KIND = {
  unchanged: styles["row--unchanged"],
  removed: styles["row--removed"],
  added: styles["row--added"],
} as const;

const GLYPH_BY_KIND = {
  unchanged: " ", // non-breaking space — preserves the column width
  removed: "−", // U+2212 MINUS SIGN
  added: "+",
} as const;

const GLYPH_ARIA_BY_KIND = {
  unchanged: "",
  removed: "removed",
  added: "added",
} as const;

function pluralize(n: number, singular: string): string {
  return `${n} ${singular}${n === 1 ? "" : "s"}`;
}

function tableLabel(preview: MachineTomlPreview): string {
  // VoiceOver reads "machine.toml diff, N additions, M removals" — pinned
  // by UI-SPEC §VoiceOver labels.
  return `machine.toml diff, ${pluralize(preview.added_count, "addition")}, ${pluralize(
    preview.removed_count,
    "removal",
  )}`;
}

function rowAriaLabel(line: DiffLine): string | undefined {
  // Unchanged rows are aria-hidden; the React side returns undefined so
  // the row has no label. Removed + added rows surface the change kind +
  // line number for VoiceOver.
  if (line.kind === "unchanged") return undefined;
  return `${GLYPH_ARIA_BY_KIND[line.kind]} line ${line.line_number}`;
}

export function MachineTomlDiff({ preview }: MachineTomlDiffProps) {
  const label = tableLabel(preview);
  return (
    <div className={styles.container}>
      <header className={styles.summary}>
        {pluralize(preview.added_count, "addition")},{" "}
        {pluralize(preview.removed_count, "removal")}
      </header>
      <table className={styles.table} role="table" aria-label={label}>
        <tbody>
          {preview.lines.map((line, idx) => {
            const isUnchanged = line.kind === "unchanged";
            return (
              <tr
                // Multiple lines can share a line_number (e.g. a removed
                // line and the added line that replaces it) — index keeps
                // the key unique. The diff is deterministic + non-reordered,
                // so index-as-key is stable.
                key={idx}
                role="row"
                className={ROW_CLASS_BY_KIND[line.kind]}
                aria-label={rowAriaLabel(line)}
                aria-hidden={isUnchanged ? "true" : undefined}
              >
                <td className={styles.lineNumber}>{line.line_number}</td>
                <td className={styles.glyph}>{GLYPH_BY_KIND[line.kind]}</td>
                <td className={styles.content}>{line.content}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
