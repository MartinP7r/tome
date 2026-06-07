// StageStepper — vertical 6-stage progress visualisation (Phase 27 plan
// 27-04 / UI-SPEC §StageStepper / D-07 / D-09 / D-10 / D-18 / D-19 / D-20).
//
// Renders live progress AND terminal state in the same component — there
// is no separate "outcome panel" (D-18). The component receives the
// per-stage status array (`StageState[]`) from `useSync()` and decides
// trailing-slot buttons based on the aggregate state:
//
//   - Any stage active   → [Cancel sync] button above the stepper.
//   - All stages terminal AND retry handler provided →
//                          [Retry from <stage>] + [Dismiss] above.
//   - All stages terminal, no retry              → [Dismiss] above.
//
// **The cancellation summary block ("Sync cancelled" heading + sub-line +
// [Run sync] + [Dismiss]) is rendered BY the parent view (SyncView).**
// This component only renders the stepper rows + the action buttons that
// belong to the stepper itself. See SyncView for the D-18 terminal-state
// summary composition.
//
// **No analog in Phase 26.** The closest behavioural reference is
// HealthView's SectionHeader-driven grouping, but the visual shape
// (connector-line between rows + in-place live→terminal transformation)
// is net-new. Fall-back recipe documented in PATTERNS.md §"Stepper outer
// container (No analog)".
//
// **Pattern carry-overs:**
// - Outer `<section role="list" aria-label="Sync pipeline progress">`
//   per UI-SPEC.
// - Each StageRow is a `role="listitem"` per UI-SPEC.
// - State-change announcements rely on the outer `role="status"`
//   `aria-live="polite"` wrapper around the stepper — the same triplet
//   Pill.tsx ships (Pitfall 2 carry-over).

import type { ReactNode } from "react";
import type { SyncStage } from "../bindings";
import { Button } from "./Button";
import { StageRow, type StageStatus } from "./StageRow";
import styles from "./StageStepper.module.css";

/** Per-row config: which `SyncStage` this row represents + the verbatim
 *  label + the live status (driven by useSync's per-stage Map). */
export interface StageState {
  stage: SyncStage;
  label: string;
  status: StageStatus;
}

export interface StageStepperProps {
  /** Exactly 6 entries in pipeline order (Reconcile → Save). */
  stages: StageState[];
  /** Cancel handler — wired only while any stage is `active`. */
  onCancel?: () => void;
  /** Reset to idle handler — wired in terminal state. */
  onDismiss?: () => void;
  /** Retry from a specific stage — wired in terminal-failed state
   *  (27-05). When not provided, the [Retry from <stage>] button is
   *  not rendered. */
  onRetryFromStage?: (stage: SyncStage) => void;
  /** Retry partial failures — wired in terminal-partial state (27-05).
   *  When not provided, the [Retry failed items] button is not rendered. */
  onRetryFailedItems?: () => void;
  /** Optional summary block rendered ABOVE the stepper trailing-button
   *  row. Used by the cancelled / failed / partial terminal branches
   *  in SyncView to show a heading + sub-line per UI-SPEC §Terminal
   *  state. The stepper itself does not own the copy; the parent view
   *  does. */
  summary?: ReactNode;
}

export function StageStepper({
  stages,
  onCancel,
  onDismiss,
  onRetryFromStage,
  onRetryFailedItems,
  summary,
}: StageStepperProps) {
  const anyActive = stages.some((s) => s.status.kind === "active");
  const anyTerminalOutcome = stages.some(
    (s) =>
      s.status.kind === "complete" ||
      s.status.kind === "failed" ||
      s.status.kind === "cancelled",
  );
  // Terminal = no stage is still running AND at least one stage has a
  // non-pending outcome. A fresh stepper (all pending) is "idle" — not
  // terminal — so the Dismiss / Retry buttons stay hidden.
  const terminal = !anyActive && anyTerminalOutcome;
  const firstFailedOrCancelled = stages.find(
    (s) => s.status.kind === "failed" || s.status.kind === "cancelled",
  );

  return (
    <div
      role="status"
      aria-live="polite"
      aria-busy={anyActive}
      className={styles.outer}
    >
      {summary !== undefined && (
        <div className={styles.summary}>{summary}</div>
      )}
      <div className={styles.actionRow}>
        {/* Cancel button is rendered whenever the parent passes onCancel
         * — typically while `isRunning` per useSync. Per UI-SPEC §StageStepper
         * the button is "rendered above the stepper outer container when at
         * least one stage is `active`", but the parent owns the
         * "any active OR isRunning" predicate (the brief gap between
         * [Run sync] and the first SyncStageStarted event should still
         * surface the cancel affordance — D-17 promises cancel is "always
         * visible during the pipeline run"). */}
        {!terminal && onCancel !== undefined && (
          <Button
            variant="secondary"
            onPress={onCancel}
            ariaLabel="Cancel sync at next stage boundary"
          >
            Cancel sync
          </Button>
        )}
        {terminal && (
          <>
            {onRetryFailedItems !== undefined && (
              <Button
                variant="primary"
                onPress={onRetryFailedItems}
                ariaLabel="Retry failed items"
              >
                Retry failed items
              </Button>
            )}
            {onRetryFromStage !== undefined &&
              firstFailedOrCancelled !== undefined && (
                <Button
                  variant="primary"
                  onPress={() =>
                    onRetryFromStage(firstFailedOrCancelled.stage)
                  }
                  ariaLabel={`Retry from ${firstFailedOrCancelled.label}`}
                >
                  Retry from {firstFailedOrCancelled.label}
                </Button>
              )}
            {onDismiss !== undefined && (
              <Button
                variant="secondary"
                onPress={onDismiss}
                ariaLabel="Dismiss sync summary"
              >
                Dismiss
              </Button>
            )}
          </>
        )}
      </div>
      <section
        role="list"
        aria-label="Sync pipeline progress"
        className={styles.stepper}
      >
        {stages.map((s) => (
          <StageRow
            key={s.stage}
            stage={s.stage}
            label={s.label}
            status={s.status}
          />
        ))}
      </section>
    </div>
  );
}

