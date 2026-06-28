// TriageDetail — UI-SPEC §TriageDetail (Phase 27 plan 27-02 / SYNC-02).
//
// Right-column detail pane for the selected TriageRow. Mirrors Phase 26
// DetailHeader's 3-row composition (title + metadata grid + actions):
//
//   Row 1: skill name (text-title 22px / 600) + change-kind Badge.
//   Row 2: metadata grid — SOURCE, CONTENT HASH, SYNCED.
//   Row 3: canonical action picker
//     - Added / Changed: RadioGroup (Keep / Disable / [View source]).
//     - Removed: omit RadioGroup; render the verbatim copy
//       "This skill will be removed from the lockfile. No action required."
//   Row 4 (Changed only): collapsed disclosure with old vs new
//     registry_id / version / git_commit_sha.
//
// **`View source` pseudo-radio.** Per UI-SPEC, selecting `View source`
// fires the `openSourceFolder` action and the radio bounces back to the
// previously-selected legitimate decision. The pseudo-radio only appears
// for managed-and-git-sourced rows (git_commit_sha_new is non-null).

import { useRef } from "react";
import { Radio, RadioGroup } from "react-aria-components";
import type { TriageEntry } from "../bindings";
import { formatRelative } from "../lib/relativeTime";
import { Badge } from "./Badge";
import { type TriageDecision } from "./TriageRow";
import styles from "./TriageDetail.module.css";

/** Sentinel value for the "View source" pseudo-radio. Selecting it
 *  fires `onViewSource()` and immediately reverts to the previously
 *  selected legitimate decision. */
const VIEW_SOURCE_VALUE = "__view_source__";

export interface TriageDetailProps {
  /** Selected entry, or null when no row is selected (placeholder). */
  entry: TriageEntry | null;
  /** Current decision (Added/Changed only). Removed entries don't
   *  participate in the picker. */
  decision: TriageDecision;
  /** Apply a new decision (excludes the View-source pseudo-radio). */
  onDecisionChange: (decision: TriageDecision) => void;
  /** Reveal the resolved source path in Finder (D-14, plan 26-03
   *  open_source_folder command). Only invoked when the entry is
   *  git-sourced (git_commit_sha_new is non-null). */
  onViewSource: () => void;
}

/** Truncate a SHA-256 hex to the first 8 chars + ellipsis. */
function truncateHash(h: string): string {
  return `sha256:${h.slice(0, 8)}…`;
}

/** Compose the hash cell content per the change kind. */
function hashCell(entry: TriageEntry): string {
  switch (entry.change_kind) {
    case "added":
      return entry.content_hash_new !== null
        ? truncateHash(entry.content_hash_new)
        : "—";
    case "removed":
      return entry.content_hash_old !== null
        ? truncateHash(entry.content_hash_old)
        : "—";
    case "changed": {
      const oldH =
        entry.content_hash_old !== null
          ? truncateHash(entry.content_hash_old)
          : "—";
      const newH =
        entry.content_hash_new !== null
          ? truncateHash(entry.content_hash_new)
          : "—";
      return `${oldH} → ${newH}`;
    }
  }
}

function changeBadgeLabel(kind: TriageEntry["change_kind"]): string {
  switch (kind) {
    case "added":
      return "New";
    case "changed":
      return "Changed";
    case "removed":
      return "Removed";
  }
}

export function TriageDetail({
  entry,
  decision,
  onDecisionChange,
  onViewSource,
}: TriageDetailProps) {
  // Track the last legitimate decision so the View-source pseudo-radio
  // can bounce back after firing the side-effect.
  const lastDecisionRef = useRef<TriageDecision>(decision);
  if (decision === "keep" || decision === "disable") {
    lastDecisionRef.current = decision;
  }

  if (entry === null) {
    return (
      <section
        className={styles.placeholder}
        aria-label="No change selected"
      >
        <p>Select a change to view details</p>
      </section>
    );
  }

  const isGitSourced =
    entry.origin.kind === "managed" &&
    entry.git_commit_sha_new !== null &&
    entry.git_commit_sha_new !== undefined;

  const sourceLabel = entry.source_name ?? "unowned";
  const syncedLabel =
    entry.synced_at === null || entry.synced_at === undefined
      ? "—"
      : formatRelative(entry.synced_at);

  return (
    <section
      className={styles.detail}
      aria-label={`${entry.name} change details`}
    >
      {/* Row 1: title + change badge */}
      <div className={styles.titleRow}>
        <h2 className={styles.title}>{entry.name}</h2>
        <Badge subtype="type-git">{changeBadgeLabel(entry.change_kind)}</Badge>
      </div>

      {/* Row 2: metadata grid */}
      <dl className={styles.metaGrid}>
        <div className={styles.metaCell}>
          <dt className={styles.metaLabel}>SOURCE</dt>
          <dd className={styles.metaValueMono}>{sourceLabel}</dd>
        </div>
        <div className={styles.metaCell}>
          <dt className={styles.metaLabel}>CONTENT HASH</dt>
          <dd className={styles.metaValueMono}>{hashCell(entry)}</dd>
        </div>
        <div className={styles.metaCell}>
          <dt className={styles.metaLabel}>SYNCED</dt>
          <dd className={styles.metaValue}>{syncedLabel}</dd>
        </div>
      </dl>

      {/* Row 3: canonical action picker */}
      {entry.change_kind === "removed" ? (
        <p className={styles.removedHelper}>
          This skill will be removed from the lockfile. No action required.
        </p>
      ) : (
        <RadioGroup
          aria-label={`Decision for ${entry.name}`}
          value={decision}
          onChange={(value) => {
            if (value === VIEW_SOURCE_VALUE) {
              // Pseudo-radio: fire side-effect, then revert to the last
              // legitimate decision so the radio doesn't visually stick
              // on a non-decision value.
              onViewSource();
              onDecisionChange(lastDecisionRef.current);
              return;
            }
            if (value === "keep" || value === "disable") {
              onDecisionChange(value);
            }
          }}
          className={styles.radioGroup}
        >
          <Radio value="keep">Keep this skill</Radio>
          <Radio value="disable">Disable on this machine</Radio>
          {isGitSourced && (
            <Radio value={VIEW_SOURCE_VALUE}>
              View source (open in Finder)
            </Radio>
          )}
        </RadioGroup>
      )}

      {/* Row 4 (Changed only): collapsed diff metadata disclosure. */}
      {entry.change_kind === "changed" && (
        <details className={styles.diffMetadata}>
          <summary>Show diff metadata</summary>
          <dl className={styles.metaGrid}>
            {entry.registry_id !== null &&
              entry.registry_id !== undefined && (
                <div className={styles.metaCell}>
                  <dt className={styles.metaLabel}>REGISTRY ID</dt>
                  <dd className={styles.metaValueMono}>{entry.registry_id}</dd>
                </div>
              )}
            {(entry.version_old !== null || entry.version_new !== null) && (
              <div className={styles.metaCell}>
                <dt className={styles.metaLabel}>VERSION</dt>
                <dd className={styles.metaValueMono}>
                  {(entry.version_old ?? "—") + " → " + (entry.version_new ?? "—")}
                </dd>
              </div>
            )}
            {(entry.git_commit_sha_old !== null ||
              entry.git_commit_sha_new !== null) && (
              <div className={styles.metaCell}>
                <dt className={styles.metaLabel}>GIT COMMIT SHA</dt>
                <dd className={styles.metaValueMono}>
                  {(entry.git_commit_sha_old ?? "—") +
                    " → " +
                    (entry.git_commit_sha_new ?? "—")}
                </dd>
              </div>
            )}
          </dl>
        </details>
      )}
    </section>
  );
}
