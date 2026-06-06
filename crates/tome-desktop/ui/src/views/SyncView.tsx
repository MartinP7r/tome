// SyncView — Phase 27 plan 27-01b skeleton (SYNC-01).
//
// Three render shapes corresponding to the useSync state machine:
//
//   isRunning === false && outcome === null   →  idle hero
//                                                ↺ glyph + headline +
//                                                "Run sync" CTA
//   isRunning === true                        →  in-progress placeholder
//                                                "Sync running…" + Cancel
//   isRunning === false && outcome !== null   →  terminal summary
//                                                "Sync complete" or
//                                                inline error
//
// **Skeleton scope.** Plan 27-01b only ships the substrate. The full surface
// lands across:
//
//   27-02 — Recent changes disclosure + triage panel population.
//   27-03 — Apply flow + PreviewPopover.
//   27-04 — Real StageStepper (the in-progress placeholder graduates).
//   27-05 — SyncOutcomeWire + partial-failure rendering.
//
// UI-SPEC §Per-view Design — Sync covers the idle / running / terminal
// composition; this skeleton lays the visual grammar without the polish.

import { useStatus } from "../hooks/useStatus";
import { useSync } from "../hooks/useSync";
import { formatRelative } from "../lib/relativeTime";

export function SyncView() {
  const { isRunning, outcome, start, cancel, dismiss } = useSync();
  const { status } = useStatus();

  if (!isRunning && outcome === null) {
    // -------- Idle hero --------
    const lastSync = status?.last_sync ?? null;
    const headline =
      lastSync === null
        ? "You haven't synced yet."
        : `Last synced ${formatRelative(lastSync)}`;

    return (
      <section role="status" aria-label="Sync status">
        <RefreshGlyph />
        <h1>{headline}</h1>
        {/* Plan 27-02 will replace this with `${new} new · ${changed}
         * changed · ${removed} removed since last sync`, populated from
         * the lockfile diff feed. */}
        {lastSync !== null && (
          <p>Run a sync to refresh the library.</p>
        )}
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
          {/* Plan 27-02 owns the population of this disclosure. */}
          <p>No changes recorded in the previous sync.</p>
        </details>
      </section>
    );
  }

  if (isRunning) {
    // -------- In-progress placeholder --------
    // Plan 27-04 swaps this for the real StageStepper component. The
    // aria-busy + aria-live="polite" pairing announces stage transitions
    // to VoiceOver in the meantime.
    return (
      <section
        role="region"
        aria-busy="true"
        aria-live="polite"
        aria-label="Sync pipeline"
      >
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
      </section>
    );
  }

  // -------- Terminal summary --------
  // outcome is non-null and isRunning is false.
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
