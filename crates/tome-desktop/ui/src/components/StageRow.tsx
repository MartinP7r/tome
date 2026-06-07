// StageRow — single pipeline stage line inside StageStepper (Phase 27
// plan 27-04 / UI-SPEC §StageRow / D-07 / D-09 / D-10 / D-18 / D-20).
//
// Five variants drive the rendered shape (icon + label + trailing) and
// the aria-label template, mapping 1:1 to StageStatus.kind:
//
//   pending    → outline circle, "—" trailing, label weight 400 / dim
//   active     → spinner, "running…" trailing + currentItem subtitle
//                + inline progress bar `current / total` (bar hidden
//                when total === 0 — D-09 git-clone case)
//   complete   → ✓ icon, duration trailing; partial-failure badge +
//                FindingRow list appear when partialFailures.length > 0
//                (D-20; rendered "render-ready" — 27-05 populates the
//                array)
//   failed     → ! icon, label weight 600 / danger, duration trailing;
//                inline [ErrorCode] message + ▶ Show error chain
//                disclosure mirrored verbatim from FindingRow (D-18 /
//                Phase 26 D-11)
//   cancelled  → ⊘ icon (amber), "cancelled" trailing, label dim
//
// **Variant rendering pattern carry-over from FindingRow.** The inline
// `[ErrorCode] message` + `<details>` disclosure for the failed case
// is structurally identical to FindingRow.tsx:70-85. Same shape, same
// rendering — copying it here keeps both surfaces visually
// consistent without an abstraction layer (only two callers).
//
// **A11y label template.** Per UI-SPEC §VoiceOver labels:
//   pending    → "${stageName} stage, pending"
//   active     → "${stageName} stage, running, ${item || 'preparing'}
//                 [, ${current} of ${total}]"
//   complete   → "${stageName} stage, complete in ${dur}[, ${K} issues]"
//   failed     → "${stageName} stage, failed in ${dur}, ${code}, ${msg}"
//   cancelled  → "${stageName} stage, cancelled"

import type { SyncStage, TomeError } from "../bindings";
import { formatDuration } from "../lib/formatDuration";
import styles from "./StageRow.module.css";

/** One element of the StageStepper's `stages` array; mirrors the
 *  contract published by `useSync().stages` Map + the per-stage
 *  metadata (label + tooltip + retry flag). */
export type StageStatus =
  | { kind: "pending" }
  | {
      kind: "active";
      currentItem: string | null;
      current: number;
      total: number;
    }
  | { kind: "complete"; durationMs: number; partialFailures: PartialFailure[] }
  | { kind: "failed"; durationMs: number; error: TomeError }
  | { kind: "cancelled" };

/** One per-operation failure inside an otherwise-successful stage
 *  (D-20 / SAFE-01 K-failures semantics). Plan 27-05 will populate
 *  this from `SyncOutcomeWire.partialFailures`. */
export interface PartialFailure {
  /** Skill or operation that failed. */
  itemName: string;
  /** Structured failure payload — same shape FindingRow renders. */
  error: TomeError;
}

export interface StageRowProps {
  stage: SyncStage;
  /** Display label per UI-SPEC §StageStepper props (e.g., "Reconcile"). */
  label: string;
  status: StageStatus;
}

export function StageRow({ stage, label, status }: StageRowProps) {
  // Compose the aria-label per UI-SPEC §VoiceOver labels.
  const ariaLabel = buildAriaLabel(label, status);

  return (
    <div
      role="listitem"
      aria-label={ariaLabel}
      className={[styles.row, styles[`row--${status.kind}`]].join(" ")}
      data-stage={stage}
    >
      <div className={styles.icon} aria-hidden="true">
        {renderIcon(status)}
      </div>
      <div className={styles.body}>
        <div className={styles.label}>{label}</div>
        {status.kind === "active" && status.currentItem !== null && (
          <div className={styles.subtitle}>
            <span
              className={
                isPathLike(status.currentItem)
                  ? styles.subtitleMono
                  : styles.subtitlePlain
              }
            >
              {status.currentItem}
            </span>
            {status.total > 0 && (
              <span className={styles.progressBar} aria-hidden="true">
                <span
                  className={styles.progressFill}
                  style={{ width: `${progressPct(status.current, status.total)}%` }}
                />
                <span className={styles.progressLabel}>
                  {status.current}/{status.total}
                </span>
              </span>
            )}
          </div>
        )}
        {status.kind === "failed" && (
          <div className={styles.failed}>
            <span className={styles.errCode}>[{status.error.code}]</span>{" "}
            {status.error.message}
            {status.error.context.length > 0 && (
              <details className={styles.disclosure}>
                <summary>Show error chain</summary>
                <ul>
                  {status.error.context.map((c, i) => (
                    <li key={i}>{c}</li>
                  ))}
                </ul>
              </details>
            )}
          </div>
        )}
        {status.kind === "complete" && status.partialFailures.length > 0 && (
          <ul className={styles.partialFailures}>
            {status.partialFailures.map((pf, i) => (
              <li key={`${pf.itemName}-${i}`} className={styles.partialFailure}>
                <strong>{pf.itemName}</strong>{" "}
                <span className={styles.errCode}>[{pf.error.code}]</span>{" "}
                {pf.error.message}
                {pf.error.context.length > 0 && (
                  <details className={styles.disclosure}>
                    <summary>Show context</summary>
                    <ul>
                      {pf.error.context.map((c, i) => (
                        <li key={i}>{c}</li>
                      ))}
                    </ul>
                  </details>
                )}
              </li>
            ))}
          </ul>
        )}
      </div>
      <div className={styles.trailing}>{renderTrailing(status)}</div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Rendering helpers
// ---------------------------------------------------------------------------

function renderIcon(status: StageStatus) {
  switch (status.kind) {
    case "pending":
      return <span className={styles.iconPending}>○</span>;
    case "active":
      return <span className={styles.iconActive}>●</span>;
    case "complete":
      return <span className={styles.iconComplete}>✓</span>;
    case "failed":
      return <span className={styles.iconFailed}>!</span>;
    case "cancelled":
      return <span className={styles.iconCancelled}>⊘</span>;
  }
}

function renderTrailing(status: StageStatus) {
  switch (status.kind) {
    case "pending":
      return <span className={styles.trailingDim}>—</span>;
    case "active":
      return <span className={styles.trailingDim}>running…</span>;
    case "complete":
      return (
        <>
          <span className={styles.duration}>
            {formatDuration(status.durationMs)}
          </span>
          {status.partialFailures.length > 0 && (
            <span className={styles.issuesBadge}>
              ⚠ {status.partialFailures.length} issues
            </span>
          )}
        </>
      );
    case "failed":
      return (
        <span className={styles.duration}>
          {formatDuration(status.durationMs)}
        </span>
      );
    case "cancelled":
      return <span className={styles.trailingCancelled}>cancelled</span>;
  }
}

function buildAriaLabel(label: string, status: StageStatus): string {
  switch (status.kind) {
    case "pending":
      return `${label} stage, pending`;
    case "active": {
      const sub = status.currentItem ?? "preparing";
      const tail = status.total > 0
        ? `, ${status.current} of ${status.total}`
        : "";
      return `${label} stage, running, ${sub}${tail}`;
    }
    case "complete": {
      const dur = formatDuration(status.durationMs);
      const tail = status.partialFailures.length > 0
        ? `, ${status.partialFailures.length} issues`
        : "";
      return `${label} stage, complete in ${dur}${tail}`;
    }
    case "failed": {
      const dur = formatDuration(status.durationMs);
      return `${label} stage, failed in ${dur}, ${status.error.code}, ${status.error.message}`;
    }
    case "cancelled":
      return `${label} stage, cancelled`;
  }
}

/** Detect whether a `currentItem` string is path-shaped (contains a path
 *  separator OR starts with `git:`) so the renderer can apply monospace
 *  styling. Mirrors UI-SPEC §StageRow §active variant note. */
function isPathLike(s: string): boolean {
  return s.startsWith("git:") || s.includes("/") || s.includes("\\");
}

function progressPct(current: number, total: number): number {
  if (total <= 0) return 0;
  const pct = (current / total) * 100;
  if (!Number.isFinite(pct)) return 0;
  return Math.max(0, Math.min(100, pct));
}
