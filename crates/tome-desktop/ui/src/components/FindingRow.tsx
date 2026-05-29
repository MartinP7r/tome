// FindingRow — single Doctor finding (UI-SPEC §Components §FindingRow).
//
// Two rendered shapes selected by `finding.repair_kind`:
//   - Some(kind) → auto-fixable. Trailing slot is the PreviewPopover Fix
//     button. Apply runs `onApplyFix(finding.id)`. Failures surface inline
//     as `[Code] message` + collapsible context (D-11 / SAFE-01); the row
//     keeps the Fix button available for retry.
//   - None → non-fixable (UnparsableFrontmatter, DivergingTarget). Trailing
//     slot is verbatim UI-SPEC §Copywriting remediation hint text. NO button
//     (D-12: never a dead control).
//
// Successful fixes don't animate the row away — the watcher (plan 26-06)
// fires `LibraryChanged` / `ManifestChanged` and `useDoctorReport.refetch`
// removes the row on the next render. The popover Apply handler triggers
// an explicit `refetch` for instant feedback before the watcher round-trip
// (mirrors the plan-26-03 pattern in `useSkillActions`).
//
// A11y: `role="group"` with the verbatim UI-SPEC §VoiceOver labels template
// `${severity} finding: ${title}. ${description}. ${fixable ? 'Fix
// available' : 'Manual remediation required'}.`

import { useState } from "react";
import type { DoctorFinding, FindingId, TomeError } from "../bindings";
import { PreviewPopover } from "./PreviewPopover";
import { SeverityIcon } from "./SeverityIcon";
import styles from "./FindingRow.module.css";

export interface FindingRowProps {
  finding: DoctorFinding;
  /** Dispatched by the PreviewPopover Apply button (auto-fixable only).
   *  Returns a promise that rejects on repair failure so the inline
   *  TomeError disclosure can pick the failure up. */
  onApplyFix: (id: FindingId) => Promise<void>;
}

/** Verbatim UI-SPEC §Copywriting manual-remediation hints. Used for
 *  non-fixable findings (D-12 — no button). */
function getRemediationHint(finding: DoctorFinding): string {
  if (!finding.id) return "";
  switch (finding.id.kind) {
    case "unparsable_frontmatter":
      return "Edit the file's YAML frontmatter so it parses (delimiters ---, valid keys). Then re-open Health.";
    case "diverging_target":
      return "Re-run tome sync to consolidate, or restore the affected target from backup. Then re-open Health.";
    default:
      // Auto-fixable variants reach this only if `repair_kind` is null
      // (shouldn't happen — the Rust side guarantees the pairing) — fall
      // back to the verbatim description.
      return finding.description;
  }
}

export function FindingRow({ finding, onApplyFix }: FindingRowProps) {
  const [localError, setLocalError] = useState<TomeError | null>(null);
  const fixable = finding.repair_kind != null;
  const severity = fixable ? "warning" : "blocked";
  const severityWord = fixable ? "Warning" : "Blocked";
  const ariaLabel = `${severityWord} finding: ${finding.title}. ${finding.description}. ${
    fixable ? "Fix available" : "Manual remediation required"
  }.`;

  return (
    <div className={styles.row} role="group" aria-label={ariaLabel}>
      <div className={styles.icon}>
        <SeverityIcon severity={severity} />
      </div>
      <div className={styles.text}>
        <div className={styles.title}>{finding.title}</div>
        <div className={styles.description}>{finding.description}</div>
        {localError != null && (
          <div className={styles.failed}>
            <span className={styles.errCode}>[{localError.code}]</span>{" "}
            {localError.message}
            {localError.context.length > 0 && (
              <details className={styles.disclosure}>
                <summary>Show context</summary>
                <ul>
                  {localError.context.map((c, i) => (
                    <li key={i}>{c}</li>
                  ))}
                </ul>
              </details>
            )}
          </div>
        )}
      </div>
      <div className={styles.trailing}>
        {fixable && finding.dry_run_description != null ? (
          <PreviewPopover
            dryRunDescription={finding.dry_run_description}
            onApply={async () => {
              // Clear any prior failure so the retry attempt starts clean.
              setLocalError(null);
              await onApplyFix(finding.id);
            }}
            onError={setLocalError}
          />
        ) : (
          // D-12: non-fixable findings show a verbatim remediation hint and
          // NO button.
          <span className={styles.hint}>{getRemediationHint(finding)}</span>
        )}
      </div>
    </div>
  );
}
