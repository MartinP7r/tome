// SyncView — Phase 27 plan 27-01b skeleton + 27-02 triage panel.
//
// Three render shapes corresponding to the useSync state machine:
//
//   isRunning === false && outcome === null   →  idle hero
//                                                ↺ glyph + headline +
//                                                "Run sync" CTA
//                                              + post-Reconcile triage panel
//                                                (only if diff is non-empty —
//                                                rendered in a split layout)
//   isRunning === true                        →  in-progress placeholder
//                                                stepper placeholder +
//                                                TriagePanel (when diff
//                                                non-empty) in the middle
//                                                column; TriageDetail in
//                                                the right column.
//   isRunning === false && outcome !== null   →  terminal summary
//                                                "Sync complete" or
//                                                inline error
//
// **Plan 27-02 split-layout note.** The UI-SPEC §"In-progress" wireframe
// shows the stepper + triage panel in the middle column with TriageDetail
// in the right column. ContentPane is currently single-column; for plan
// 27-02 we render the triage flow inside a flex split inside SyncView
// itself (the real ContentPane split-mode lands when the broader spatial
// shell wave catches up — 27-04 will graduate the stepper). The flex
// split here keeps the split visually correct without a shell refactor.
//
// **Plan 27-03 Apply flow (now wired).** TriagePanel owns its own
// PreviewPopover + MachineTomlDiff + applyError state internally; the
// only seam SyncView passes down is `onApplied={applyComplete}` from
// useSync, which clears decisions + selected triage skill on success.
// The watcher's MachinePrefsChanged event fires for free on the atomic
// machine.toml write; idle hooks (useStatus, useSkills, useDoctorReport)
// refetch on their own.

import { useStatus } from "../hooks/useStatus";
import { useSync } from "../hooks/useSync";
import { formatRelative } from "../lib/relativeTime";
import { TriagePanel } from "../components/TriagePanel";
import { TriageDetail } from "../components/TriageDetail";
import type { TriageEntry } from "../bindings";
import styles from "./SyncView.module.css";

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
    start,
    cancel,
    dismiss,
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

  // The triage panel is rendered whenever the diff is non-empty
  // (UI-SPEC §"In-progress state": panel hidden until non-empty). For
  // plan 27-02 we render it across both idle AND in-progress views so
  // the user can review pending changes pre-sync. 27-04 will refine
  // this gating once the stepper / cancellation states settle.
  const showTriage = diff !== null && pendingDiffCount > 0;
  const selectedEntry =
    showTriage && selectedTriageSkill !== null
      ? findEntry(diff, selectedTriageSkill)
      : null;
  const selectedDecision = selectedTriageSkill !== null
    ? decisions.get(selectedTriageSkill) ?? "keep"
    : "keep";

  if (!isRunning && outcome === null) {
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

  if (isRunning) {
    // -------- In-progress placeholder + triage panel --------
    // Plan 27-04 swaps the placeholder for the real StageStepper. The
    // triage panel renders inside the split-pane middle column once the
    // diff is non-empty (after Reconcile completes).
    return (
      <section
        role="region"
        aria-busy="true"
        aria-live="polite"
        aria-label="Sync pipeline"
        className={styles.inProgress}
      >
        <div className={styles.stepperPlaceholder}>
          <p>Sync running…</p>
          <button
            type="button"
            onClick={() => {
              void cancel();
            }}
            aria-label="Cancel sync"
          >
            Cancel sync
          </button>
        </div>
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

  // -------- Terminal summary --------
  if (outcome?.kind === "err") {
    return (
      <section role="status" aria-label="Sync result">
        <p>
          <strong>[{outcome.error.code}]</strong> {outcome.error.message}
        </p>
        {outcome.error.context.length > 0 && (
          <ul>
            {outcome.error.context.map((c, i) => (
              <li key={i}>{c}</li>
            ))}
          </ul>
        )}
        <button
          type="button"
          onClick={dismiss}
          aria-label="Dismiss sync result"
        >
          Dismiss
        </button>
      </section>
    );
  }

  return (
    <section role="status" aria-label="Sync result">
      <p>Sync complete</p>
      <button
        type="button"
        onClick={dismiss}
        aria-label="Dismiss sync result"
      >
        Dismiss
      </button>
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
