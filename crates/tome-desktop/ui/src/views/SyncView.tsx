// SyncView — Phase 27 plan 27-04 / SYNC-04 terminal-state wiring.
//
// Renders the four shapes of the sync section per useSync's state
// machine + the UI-SPEC §Per-view Design wireframes:
//
//   isRunning === false && terminalKind === null     →  idle hero +
//                                                       triage (if pending)
//   isRunning === true                               →  StageStepper
//                                                       (active variants) +
//                                                       TriagePanel +
//                                                       TriageDetail in
//                                                       split-pane
//   terminalKind === "success"                       →  SyncToast +
//                                                       auto-dismiss to idle
//   terminalKind === "cancelled"                     →  StageStepper with
//                                                       cancelled rows +
//                                                       summary block
//                                                       ("Sync cancelled" +
//                                                       sub-line + Run sync +
//                                                       Dismiss)
//   terminalKind === "failed" / "partial"            →  StageStepper placeholder
//                                                       for 27-05 to fully wire
//
// **D-18 supersedes D-06 cancellation phrasing.** Cancellation surfaces
// INLINE in the StageStepper's terminal branch (heading + sub-line + Run
// sync + Dismiss buttons), NOT as a SyncToast. Only success uses
// SyncToast.
//
// **Layout note.** The middle column hosts the stepper + (when diff is
// non-empty) the TriagePanel; the right column hosts TriageDetail. The
// SyncView's existing flex-split CSS keeps the visual contract from
// 27-02; 27-04 only swaps the in-progress placeholder for the real
// StageStepper and adds the terminal-state summary block.

import { useMemo } from "react";
import { useStatus } from "../hooks/useStatus";
import { SYNC_STAGES, useSync } from "../hooks/useSync";
import { formatRelative } from "../lib/relativeTime";
import type { StageState } from "../components/StageStepper";
import { StageStepper } from "../components/StageStepper";
import { SyncToast } from "../components/SyncToast";
import { TriagePanel } from "../components/TriagePanel";
import { TriageDetail } from "../components/TriageDetail";
import { Button } from "../components/Button";
import type { SyncStage, TriageEntry } from "../bindings";
import styles from "./SyncView.module.css";

/** Verbatim per-stage labels used by the stepper (UI-SPEC §StageStepper). */
const STAGE_LABELS: Record<SyncStage, string> = {
  Reconcile: "Reconcile",
  Discover: "Discover",
  Consolidate: "Consolidate",
  Distribute: "Distribute",
  Cleanup: "Cleanup",
  Save: "Save",
};

function findEntry(
  diff: NonNullable<ReturnType<typeof useSync>["diff"]>,
  skill: string,
): TriageEntry | null {
  for (const e of diff.added) {
    if (e.name === skill) return e;
  }
  for (const e of diff.changed) {
    if (e.name === skill) return e;
  }
  for (const e of diff.removed) {
    if (e.name === skill) return e;
  }
  return null;
}

export function SyncView() {
  const {
    isRunning,
    outcome,
    terminalKind,
    stages,
    failureCount,
    start,
    cancel,
    dismiss,
    retryFromStage,
    retryFailedItems,
    diff,
    decisions,
    selectedTriageSkill,
    pendingDiffCount,
    onDecisionChange,
    onBulkAction,
    selectTriageSkill,
    applyComplete,
  } = useSync();
  const { status } = useStatus();

  // Build the StageState[] from useSync's stages Map in pipeline order.
  // Memoised so the StageStepper's prop identity is stable across
  // renders that don't touch the stages Map.
  const stageStates: StageState[] = useMemo(
    () =>
      SYNC_STAGES.map((stage) => ({
        stage,
        label: STAGE_LABELS[stage],
        status: stages.get(stage) ?? { kind: "pending" },
      })),
    [stages],
  );

  // The triage panel is rendered whenever the diff is non-empty.
  const showTriage = diff !== null && pendingDiffCount > 0;
  const selectedEntry =
    showTriage && selectedTriageSkill !== null
      ? findEntry(diff, selectedTriageSkill)
      : null;
  const selectedDecision = selectedTriageSkill !== null
    ? decisions.get(selectedTriageSkill) ?? "keep"
    : "keep";

  // -------- Terminal: success → SyncToast over idle hero --------
  // Render the success toast on top of the idle hero. The toast's
  // onDismiss fires after 5s OR when the user clicks [Dismiss] inside
  // it; both paths call useSync.dismiss() which resets the state machine
  // back to idle.
  const showSuccessToast = terminalKind === "success";

  // -------- Terminal: cancelled summary block --------
  // Rendered ABOVE the stepper via the StageStepper.summary slot. The
  // copywriting is verbatim from UI-SPEC §Terminal cancelled.
  const cancelledSummary =
    terminalKind === "cancelled" ? (
      <div className={styles.cancelledSummary}>
        <h1 className={styles.cancelledHeading}>Sync cancelled</h1>
        <p className={styles.cancelledSubline}>
          The library is in a consistent state. You can run sync again at
          any time.
        </p>
        <div className={styles.cancelledActions}>
          <Button
            variant="primary"
            onPress={() => {
              void start();
            }}
            ariaLabel="Run sync"
          >
            Run sync
          </Button>
          <Button
            variant="secondary"
            onPress={dismiss}
            ariaLabel="Dismiss sync summary"
          >
            Dismiss
          </Button>
        </div>
      </div>
    ) : null;

  // -------- Terminal: failed (D-19 — Plan 27-05 / SYNC-05) --------
  // "Sync failed" heading + sub-line + [Retry from <stage>] (when
  // retry_from is non-null) + [Dismiss]. The retry button is gated on
  // the structured `retry_from` value carried by the outcome: Save
  // errors don't carry a retry hint (the user must clear the underlying
  // issue), so the action triplet collapses to just [Dismiss].
  const failedRetryFrom =
    outcome?.kind === "err" ? outcome.retry_from : null;
  const failedSummary =
    terminalKind === "failed" ? (
      <div className={styles.cancelledSummary}>
        <h1 className={styles.cancelledHeading}>Sync failed</h1>
        {outcome?.kind === "err" && (
          <p className={styles.cancelledSubline}>
            <strong>[{outcome.error.code}]</strong> {outcome.error.message}
          </p>
        )}
        <div className={styles.cancelledActions}>
          {failedRetryFrom !== null && (
            <Button
              variant="primary"
              onPress={() => {
                void retryFromStage(failedRetryFrom);
              }}
              ariaLabel={`Retry from ${STAGE_LABELS[failedRetryFrom]}`}
            >
              Retry from {STAGE_LABELS[failedRetryFrom]}
            </Button>
          )}
          <Button
            variant="secondary"
            onPress={dismiss}
            ariaLabel="Dismiss sync summary"
          >
            Dismiss
          </Button>
        </div>
      </div>
    ) : null;

  // -------- Terminal: partial-failure (D-20 — Plan 27-05 / SYNC-05) --------
  // "Sync complete with K issues" heading + sub-line + [Retry failed
  // items] + [Dismiss]. The stage rows themselves carry the amber
  // [⚠ K issues] badge + auto-expanded FindingRow list (StageRow's
  // complete-with-partialFailures variant from 27-04).
  const partialSummary =
    terminalKind === "partial" ? (
      <div className={styles.cancelledSummary}>
        <h1 className={styles.cancelledHeading}>
          Sync complete with {failureCount}{" "}
          {failureCount === 1 ? "issue" : "issues"}
        </h1>
        <p className={styles.cancelledSubline}>
          Library and lockfile are saved. {failureCount}{" "}
          {failureCount === 1
            ? "individual operation failed."
            : "individual operations failed."}
        </p>
        <div className={styles.cancelledActions}>
          <Button
            variant="primary"
            onPress={() => {
              void retryFailedItems();
            }}
            ariaLabel="Retry failed items"
          >
            Retry failed items
          </Button>
          <Button
            variant="secondary"
            onPress={dismiss}
            ariaLabel="Dismiss sync summary"
          >
            Dismiss
          </Button>
        </div>
      </div>
    ) : null;

  // ===== Render branches =====

  if (!isRunning && terminalKind === null) {
    // -------- Idle hero (+ inline triage when changes pending) --------
    const lastSync = status?.last_sync ?? null;
    const headline =
      lastSync === null
        ? "You haven't synced yet."
        : `Last synced ${formatRelative(lastSync)}`;

    return (
      <div className={styles.idle}>
        <section role="status" aria-label="Sync status" className={styles.hero}>
          <RefreshGlyph />
          <h1>{headline}</h1>
          {showTriage ? (
            <p>
              {diff.added.length} new · {diff.changed.length} changed ·{" "}
              {diff.removed.length} removed since last sync
            </p>
          ) : lastSync !== null ? (
            <p>0 new · 0 changed · 0 removed since last sync</p>
          ) : null}
          <button
            type="button"
            onClick={() => {
              void start();
            }}
            aria-label="Run sync"
          >
            Run sync
          </button>
          <details>
            <summary>Recent changes</summary>
            <p>No changes recorded in the previous sync.</p>
          </details>
        </section>
        {showTriage && (
          <div className={styles.splitBody}>
            <div className={styles.triageColumn}>
              <TriagePanel
                diff={diff}
                decisions={decisions}
                onDecisionChange={onDecisionChange}
                selectedSkill={selectedTriageSkill}
                onSelect={selectTriageSkill}
                onBulkAction={onBulkAction}
                onApplied={applyComplete}
              />
            </div>
            <div className={styles.detailColumn}>
              <TriageDetail
                entry={selectedEntry}
                decision={selectedDecision}
                onDecisionChange={(d) => {
                  if (selectedTriageSkill !== null) {
                    onDecisionChange(selectedTriageSkill, d);
                  }
                }}
                onViewSource={() => {
                  /* Plan 27-03 will wire open_source_folder via the
                   * Phase 26 command — for 27-02, no-op stub. */
                }}
              />
            </div>
          </div>
        )}
      </div>
    );
  }

  // -------- Success — SyncToast over idle hero --------
  if (showSuccessToast) {
    const lastSync = status?.last_sync ?? null;
    const headline =
      lastSync === null
        ? "You haven't synced yet."
        : `Last synced ${formatRelative(lastSync)}`;
    return (
      <div className={styles.idle}>
        <section role="status" aria-label="Sync status" className={styles.hero}>
          <RefreshGlyph />
          <h1>{headline}</h1>
        </section>
        <SyncToast message="Sync complete" onDismiss={dismiss} />
      </div>
    );
  }

  // -------- In-progress / terminal-with-stepper branches --------
  // The stepper renders for: running, cancelled, failed, partial. It
  // accepts a summary slot that the cancelled / failed branches use to
  // surface their verbatim copy (Run sync / Dismiss) above the rows.
  const summary =
    cancelledSummary ?? failedSummary ?? partialSummary ?? null;

  return (
    <section
      role="region"
      aria-busy={isRunning}
      aria-label="Sync pipeline"
      className={styles.inProgress}
    >
      <StageStepper
        stages={stageStates}
        onCancel={isRunning ? cancel : undefined}
        onDismiss={!isRunning ? dismiss : undefined}
        // Plan 27-05: thread the retry handlers so the stepper's trailing
        // action row also surfaces [Retry from <stage>] / [Retry failed
        // items]. The summary block in SyncView is the user's primary
        // affordance; the stepper buttons are a redundant convenience
        // (UI-SPEC §StageStepper §action row).
        onRetryFromStage={
          terminalKind === "failed" && failedRetryFrom !== null
            ? retryFromStage
            : undefined
        }
        onRetryFailedItems={
          terminalKind === "partial" ? retryFailedItems : undefined
        }
        summary={summary ?? undefined}
      />
      {showTriage && (
        <div className={styles.splitBody}>
          <div className={styles.triageColumn}>
            <TriagePanel
              diff={diff}
              decisions={decisions}
              onDecisionChange={onDecisionChange}
              selectedSkill={selectedTriageSkill}
              onSelect={selectTriageSkill}
              onBulkAction={onBulkAction}
              onApplied={applyComplete}
            />
          </div>
          <div className={styles.detailColumn}>
            <TriageDetail
              entry={selectedEntry}
              decision={selectedDecision}
              onDecisionChange={(d) => {
                if (selectedTriageSkill !== null) {
                  onDecisionChange(selectedTriageSkill, d);
                }
              }}
              onViewSource={() => {
                /* 27-03 wires this. */
              }}
            />
          </div>
        </div>
      )}
    </section>
  );
}

/** Lightweight refresh glyph — `lucide-react` would be the conventional
 *  choice but the project doesn't ship it as a dep today. Inline SVG keeps
 *  the skeleton self-contained; plan 27-04 (StageStepper) is the right
 *  moment to introduce a shared icon library if needed. */
function RefreshGlyph() {
  return (
    <svg
      width="64"
      height="64"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M3 12a9 9 0 0 1 15-6.7L21 8" />
      <path d="M21 3v5h-5" />
      <path d="M21 12a9 9 0 0 1-15 6.7L3 16" />
      <path d="M3 21v-5h5" />
    </svg>
  );
}
