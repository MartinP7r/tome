// useSync — Sync view data hook (Phase 27 plan 27-01b / SYNC-01).
//
// Owns the in-flight sync state machine:
//
//   idle  ──[start()]──▶  running  ──[result]──▶  terminal
//     ▲                       │                       │
//     │                       │                       │
//     └─────[dismiss()]───────┴───[dismiss()]─────────┘
//
// `start()` invokes `commands.startSync()` (which the Rust side runs via
// `tauri::async_runtime::spawn_blocking` — Pitfall 5); progress is read from
// the `events.syncProgress` stream that the `TauriEventSink` emits, and
// accumulates per-stage `StageStatus` rows.
//
// **Pitfall 6 watcher-feedback discipline.** This hook subscribes ONLY to
// `events.syncProgress`. It does NOT subscribe to `manifestChanged`,
// `lockfileChanged`, `libraryChanged`, or `machinePrefsChanged`, because the
// sync pipeline writes those files as a side effect — subscribing here would
// create a feedback loop where the user's sync triggers spurious refetches
// in the very view that is supposed to be showing the sync's progress.
//
// Idle hooks (`useStatus`, `useSkills`, `useDoctorReport`) keep their watcher
// subscriptions — those views WANT to refresh after a sync finishes so the
// user sees the post-sync state. This hook isolates itself.
//
// The terminal state currently exposes `outcome: 'ok' | TomeError | null`.
// Plan 27-05 will swap this for a structured `SyncOutcomeWire` payload
// (StageStepper rendering, partial-failure rows, etc.). Today we render a
// thin "Sync complete" placeholder.

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
  type RefObject,
} from "react";
import { commands, events } from "../bindings";
import type {
  DirectoryName,
  LockfileDiff,
  PartialFailureWire,
  SkillName,
  SyncOutcomeWire,
  SyncProgress,
  SyncStage,
  TomeError,
} from "../bindings";
import {
  type BulkScope,
} from "../components/TriagePanel";
import { type TriageDecision } from "../components/TriageRow";
import { useLockfileDiff } from "./useLockfileDiff";

/** All six pipeline stages, in run order. The pinned ordering matches
 *  `tome::progress::SyncStage::ALL`. */
export const SYNC_STAGES: readonly SyncStage[] = [
  "Reconcile",
  "Discover",
  "Consolidate",
  "Distribute",
  "Cleanup",
  "Save",
] as const;

/** Per-stage status. Plan 27-01b shipped pending/active/complete; plan
 *  27-04 adds cancelled + failed to support the StageStepper terminal
 *  rendering (D-18). Plan 27-05 will populate `partialFailures` on the
 *  complete variant from `SyncOutcomeWire.partialFailures` (D-20). */
export type StageStatus =
  | { kind: "pending" }
  | {
      kind: "active";
      current: number;
      total: number;
      currentItem: string | null;
    }
  | {
      kind: "complete";
      durationMs: number;
      /** D-20 partial-failure rows. Empty until 27-05 populates. */
      partialFailures: PartialFailure[];
    }
  | { kind: "failed"; durationMs: number; error: TomeError }
  | { kind: "cancelled" };

/** Per-operation failure inside an otherwise-successful stage (D-20).
 *  Phase 27 plan 27-05: shape mirrors `PartialFailureWire` from bindings
 *  (the IPC wire-side mirror of `tome::PartialFailure`). The StageRow
 *  renders one FindingRow per entry below the per-stage row.
 *
 *  `itemName` is derived from `pf.skill` (or "operation" fallback) for
 *  the StageRow's `<strong>` rendering — 27-04 shipped the renderer
 *  expecting an `itemName` field, so we keep the wider shape so the
 *  render code is untouched. */
export interface PartialFailure {
  itemName: string;
  error: TomeError;
}

/** Build a UI-facing `PartialFailure` from a `PartialFailureWire`
 *  payload. The wire shape exposes `skill: Option<String>` + `error:
 *  TomeError`; the StageRow expects `itemName: string`. We collapse
 *  None into the literal string "operation" so the renderer never
 *  shows an empty placeholder. */
export function partialFailureFromWire(pf: PartialFailureWire): PartialFailure {
  return {
    itemName: pf.skill ?? "operation",
    error: pf.error,
  };
}

export type SyncTerminal =
  /** Sync finished. Phase 27 plan 27-05: the outcome wire payload is
   *  carried verbatim so the partial-failure terminal branch
   *  ("Sync complete with K issues") reads `outcome.kind === "ok" &&
   *  outcome.wire.partial_failures.length > 0`. */
  | { kind: "ok"; wire: SyncOutcomeWire }
  /** User clicked Cancel; the pipeline bailed at the next stage
   *  boundary. The library is in a consistent state (SC#4). */
  | { kind: "cancelled" }
  /** Sync returned an error. Plan 27-05: when the structured
   *  `SyncOutcomeWire.result` is non-null we wrap it here AND carry
   *  the `retry_from` hint so SyncView's terminal-failed branch can
   *  surface the `[Retry from <stage>]` action. The outer Tauri-level
   *  Result Err (setup / JoinError / T-27-01b-07 Conflict) maps to a
   *  `retry_from: null` value. */
  | { kind: "err"; error: TomeError; retry_from: SyncStage | null };

/** Derived classification of the in-flight run's outcome. Drives
 *  SyncView's terminal-state branch selection. `null` while idle or
 *  running. */
export type SyncTerminalKind =
  | "success"
  | "cancelled"
  | "failed"
  | "partial"
  | null;

export interface UseSyncResult {
  /** Per-stage state, in pipeline order. */
  stages: ReadonlyMap<SyncStage, StageStatus>;
  /** True while `commands.startSync()` is in flight (before its Result
   *  resolves). The Sidebar's spinner slot keys off this. */
  isRunning: boolean;
  /** Stable ref mirror of `isRunning`. Consumers (`useLockfileDiff`)
   *  that need to gate watcher-driven refetches without triggering
   *  re-subscriptions read this directly (Pitfall 6 carryover from
   *  27-01b — narrow surface). */
  isRunningRef: RefObject<boolean>;
  /** Terminal outcome — `null` while idle or running. */
  outcome: SyncTerminal | null;
  /** Plan 27-04 — derived classification of the outcome for SyncView's
   *  terminal-state branch selection (success / cancelled / failed /
   *  partial). `null` while idle or running. */
  terminalKind: SyncTerminalKind;
  /** Plan 27-02 — current lockfile diff snapshot driving the triage panel.
   *  `null` before first fetch resolves; `is_empty()` true when no
   *  changes are pending. */
  diff: LockfileDiff | null;
  /** Plan 27-02 — most recent diff fetch error (cleared on next ok). */
  diffError: TomeError | null;
  /** Plan 27-02 — per-skill triage decisions (controlled state). Seeded
   *  to `"keep"` for every Added/Changed entry on diff load. */
  decisions: ReadonlyMap<SkillName, TriageDecision>;
  /** Currently-selected TriageRow (drives TriageDetail in the right
   *  column). `null` when nothing is selected. */
  selectedTriageSkill: SkillName | null;
  /** Plan 27-02 — count of non-default decisions (Added+Changed where
   *  decision !== "keep"). Drives the [Apply N] button label. */
  pendingDecisionCount: number;
  /** Plan 27-02 — count of skills across all three buckets
   *  (added + changed + removed). Drives the Sidebar Sync badge per D-05. */
  pendingDiffCount: number;
  /** Plan-27-01b stub — `0` until plan 27-05 populates from
   *  SyncOutcomeWire.partialFailures.length. */
  failureCount: number;
  /** Phase 27 plan 27-05: Sidebar Sync-NavItem post-sync failure-badge
   *  count. Persists across `dismiss()` so a user who closes the
   *  partial-failure summary still sees the badge — only a successful
   *  retry that clears all issues resets it. Computed from the stages
   *  Map (partialFailures + failed kinds); supersedes `failureCount`'s
   *  ephemeral-only semantics. */
  unresolvedFailureCount: number;
  /** Kick off a sync. Idempotent against double-fire — the Rust side
   *  returns ErrorCode::Conflict if a sync is already in flight; the
   *  hook surfaces that as the outcome (T-27-01b-07). */
  start: () => Promise<void>;
  /** Cancel an in-flight sync. Sync command on the Rust side; idempotent. */
  cancel: () => Promise<void>;
  /** Reset to idle from the terminal state. */
  dismiss: () => void;
  /** Phase 27 plan 27-05: resume the pipeline from a named stage.
   *  Wired to the StageStepper's `[Retry from <stage>]` button in the
   *  terminal-failed branch. The stage argument is the `retry_from`
   *  value carried by the prior outcome. */
  retryFromStage: (stage: SyncStage) => Promise<void>;
  /** Phase 27 plan 27-05: retry the per-skill partial failures from a
   *  prior partial-success run. Wired to `[Retry failed items]`. */
  retryFailedItems: () => Promise<void>;
  /** Plan 27-02 — set the decision for a single skill. */
  onDecisionChange: (skill: SkillName, decision: TriageDecision) => void;
  /** Plan 27-02 — apply a bulk-action scope to a decision. */
  onBulkAction: (scope: BulkScope, decision: TriageDecision) => void;
  /** Plan 27-02 — set or clear the selected TriageRow. */
  selectTriageSkill: (skill: SkillName | null) => void;
  /** Plan 27-02 — manually refetch the lockfile diff. The triage panel's
   *  Apply flow (27-03) will trigger this once it lands. */
  refetchDiff: () => Promise<void>;
  /** Phase 27 plan 27-03 — invoked by the TriagePanel's Apply flow after
   *  `applyMachineToml` resolves successfully. Clears all triage state
   *  back to the all-keep idle so the user sees a fresh slate and the
   *  Sidebar `pendingDecisionCount` badge returns to zero. The watcher
   *  fires MachinePrefsChanged for free; idle hooks (useSkills,
   *  useDoctorReport) refetch on their own. The lockfile diff itself is
   *  NOT re-fetched here — the diff only changes when the lockfile does,
   *  and a machine.toml write doesn't touch the lockfile. */
  applyComplete: () => void;
}

/** Build a fresh stages Map with every stage as `{ kind: "pending" }`. */
function initialStages(): Map<SyncStage, StageStatus> {
  const m = new Map<SyncStage, StageStatus>();
  for (const stage of SYNC_STAGES) m.set(stage, { kind: "pending" });
  return m;
}

/** Internal — the actual state machine. Lifted into a Context provider
 *  (`SyncProvider`) so the Sidebar (App.tsx), useMenuActions (global ⌘R
 *  handler), and SyncView all observe the SAME in-flight state. Three
 *  independent `useSync()` calls in three components would otherwise
 *  spawn three independent state machines + three independent
 *  syncProgress listeners — the Sidebar's spinner wouldn't track the
 *  SyncView's run, and ⌘R wouldn't kick off a sync the user sees. */
function useSyncInternal(): UseSyncResult {
  const [stages, setStages] = useState<Map<SyncStage, StageStatus>>(
    () => initialStages(),
  );
  const [isRunning, setIsRunning] = useState(false);
  const [outcome, setOutcome] = useState<SyncTerminal | null>(null);

  // D-10 per-stage durations: track each stage's start timestamp so the
  // Finished handler can compute `durationMs`. Lives in a ref because it's
  // ephemeral per-run state; not part of the render-visible result.
  const stageStartAt = useRef<Map<SyncStage, number>>(new Map());

  // Plan 27-04: cancel-detection ref. Set when the user clicks
  // [Cancel sync]; read when `commands.startSync()` resolves to decide
  // whether to classify the outcome as { kind: "cancelled" } (per D-17 /
  // D-18) versus the generic { kind: "err" } branch (Conflict, etc.).
  //
  // Why a ref and not state: this is per-run ephemeral data — the value
  // only matters during the window between `cancel()` and the
  // `startSync` Result resolving — and changing it must not trigger a
  // re-render mid-run. Cleared on `start()` and `dismiss()` so a fresh
  // run starts from a clean slate.
  const cancelRequestedRef = useRef(false);

  // Pitfall 6 gate. The progress handler reads this to decide whether to
  // accumulate or drop events. Set to true on `start()` BEFORE the
  // `await commands.startSync()` so an early `SyncStageStarted` event
  // can't race the state update; flipped to false in the same place
  // we flip `isRunning` (and we also flip on the result branch so the
  // tail-end events after the last SyncStageFinished don't surprise us).
  const isRunningRef = useRef(false);

  // Plan 27-02 — triage state. The lockfile diff and the per-skill
  // decisions live here so the Sidebar (via pendingDiffCount) and the
  // SyncView (via diff + decisions) share the same source of truth.
  const { diff, err: diffError, refetch: refetchDiff } =
    useLockfileDiff(isRunningRef);
  const [decisions, setDecisions] = useState<Map<SkillName, TriageDecision>>(
    () => new Map(),
  );
  const [selectedTriageSkill, setSelectedTriageSkill] =
    useState<SkillName | null>(null);

  // Seed decisions from the diff once on first non-null load. Re-seed
  // when the diff identity changes AND the decisions map is empty (so
  // we don't clobber in-progress edits when the watcher refetches).
  useEffect(() => {
    if (diff === null) return;
    if (decisions.size > 0) return;
    const seeded = new Map<SkillName, TriageDecision>();
    for (const entry of diff.added) seeded.set(entry.name, "keep");
    for (const entry of diff.changed) seeded.set(entry.name, "keep");
    // Removed entries are implicit per D-13; not seeded.
    setDecisions(seeded);
  }, [diff, decisions.size]);

  const onDecisionChange = useCallback(
    (skill: SkillName, decision: TriageDecision) => {
      setDecisions((prev) => {
        const next = new Map(prev);
        next.set(skill, decision);
        return next;
      });
    },
    [],
  );

  const onBulkAction = useCallback(
    (scope: BulkScope, decision: TriageDecision) => {
      setDecisions((prev) => {
        if (diff === null) return prev;
        const next = new Map(prev);
        // D-13 invariant: bulk actions apply only to the NEW section.
        const newEntries = diff.added;
        const matches = (sourceName: DirectoryName | null): boolean => {
          if (scope.kind === "section") return true;
          // source-group scope — match the source_name (or "unowned").
          const key = sourceName ?? "unowned";
          return key === scope.source;
        };
        for (const entry of newEntries) {
          if (matches(entry.source_name)) {
            next.set(entry.name, decision);
          }
        }
        return next;
      });
    },
    [diff],
  );

  const selectTriageSkill = useCallback((skill: SkillName | null) => {
    setSelectedTriageSkill(skill);
  }, []);

  // Counts derived from diff + decisions. `useMemo` keeps the badge +
  // Apply button label stable across renders that don't touch either.
  const pendingDecisionCount = useMemo(() => {
    if (diff === null) return 0;
    let n = 0;
    for (const entry of diff.added) {
      if ((decisions.get(entry.name) ?? "keep") !== "keep") n += 1;
    }
    for (const entry of diff.changed) {
      if ((decisions.get(entry.name) ?? "keep") !== "keep") n += 1;
    }
    return n;
  }, [diff, decisions]);

  const pendingDiffCount = useMemo(() => {
    if (diff === null) return 0;
    return diff.added.length + diff.changed.length + diff.removed.length;
  }, [diff]);

  const handleProgress = useCallback((payload: SyncProgress) => {
    // Tail-end / replay defense — events arriving after we've already
    // returned from `commands.startSync()` (e.g. a late SyncStageFinished
    // that the Rust event channel was holding) are dropped. The `start()`
    // method sets the ref before awaiting so the in-flight path keeps
    // events.
    if (!isRunningRef.current) return;

    setStages((prev) => {
      const next = new Map(prev);
      const existing = next.get(payload.stage);

      if (payload.current === 0 && payload.total === 0 && payload.item === null) {
        // SyncStageStarted (mapped at the sink into current=0, total=0,
        // item=None) or SyncStageFinished (same shape). Distinguish by
        // existing state: if the stage isn't active yet, this is Started;
        // if it IS active, this is Finished.
        if (existing?.kind !== "active") {
          stageStartAt.current.set(payload.stage, Date.now());
          next.set(payload.stage, {
            kind: "active",
            current: 0,
            total: 0,
            currentItem: null,
          });
        } else {
          const startedAt = stageStartAt.current.get(payload.stage) ?? Date.now();
          next.set(payload.stage, {
            kind: "complete",
            durationMs: Date.now() - startedAt,
            partialFailures: [], // Plan 27-05 populates from SyncOutcomeWire.
          });
        }
        return next;
      }

      // SyncStageProgress (or a folded-in GitCloneProgress / BackupSnapshot
      // payload from the sink — they arrive as SyncStageProgress in
      // Reconcile / Save respectively per D-09). Update the active stage.
      // Defensive: if Started was dropped (e.g. very first event for the
      // stage), synthesize the active state here.
      if (existing?.kind !== "active") {
        stageStartAt.current.set(payload.stage, Date.now());
      }
      next.set(payload.stage, {
        kind: "active",
        current: payload.current,
        total: payload.total,
        currentItem: payload.item,
      });
      return next;
    });
  }, []);

  // Pitfall 6 watcher-feedback discipline — ONLY `syncProgress`. NO
  // subscription to manifestChanged / lockfileChanged / libraryChanged /
  // machinePrefsChanged. The sync pipeline rewrites those files as part
  // of the Save stage; subscribing to them here would fire a refetch
  // mid-sync inside the very hook that owns the sync state. The idle-
  // state hooks (`useStatus`, `useSkills`, `useDoctorReport`) keep their
  // watcher subscriptions so the post-sync UI refreshes on its own.
  //
  // Direct listener registration (not `useTauriEvent`) because the typed
  // payload (`SyncProgress`) needs to flow into `handleProgress`; the
  // shared `useTauriEvent` helper discards the payload by design (Phase
  // 26 hooks only ever needed the fact of an event firing). We could
  // overload `useTauriEvent` to forward the payload, but Pitfall 6
  // restraint is the lesson — keep the surface small.
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;
    events.syncProgress
      .listen((evt) => {
        if (cancelled) return;
        handleProgress(evt.payload);
      })
      .then((un) => {
        if (cancelled) un();
        else unlisten = un;
      });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [handleProgress]);

  /** Internal helper — finalize the outcome of a sync / retry call. The
   *  three callers (start, retryFromStage, retryFailedItems) share the
   *  same terminal-state classification: cancelled-vs-failed
   *  disambiguation via cancelRequestedRef, partial-failure population
   *  into the stages Map, error-vs-success branch. Keeping this in one
   *  place ensures the three retry entry points cannot drift. */
  const finalizeOutcome = useCallback(
    (
      res:
        | { status: "ok"; data: SyncOutcomeWire }
        | { status: "error"; error: TomeError },
    ) => {
      isRunningRef.current = false;
      setIsRunning(false);

      if (res.status === "error") {
        // The outer command Result is Err — setup / JoinError / Conflict.
        if (cancelRequestedRef.current) {
          // Plan 27-04: user clicked Cancel mid-run. The Rust side
          // returned Err("sync cancelled"). Classify as cancelled rather
          // than as a generic error and transform the stages Map so any
          // still-active or still-pending stage renders as cancelled in
          // the stepper (D-18: stepper transforms in place).
          setStages((prev) => {
            const next = new Map<SyncStage, StageStatus>();
            for (const [stage, status] of prev) {
              if (status.kind === "active" || status.kind === "pending") {
                next.set(stage, { kind: "cancelled" });
              } else {
                next.set(stage, status);
              }
            }
            return next;
          });
          setOutcome({ kind: "cancelled" });
        } else {
          // Outer Err: no retry_from hint available.
          setOutcome({ kind: "err", error: res.error, retry_from: null });
        }
        return;
      }

      // Inner success: SyncOutcomeWire shape.
      const wire = res.data;
      if (wire.result !== null) {
        // Plan 27-05: structured stage-level failure. Surface retry_from
        // so SyncView's terminal-failed branch can render the
        // [Retry from <stage>] action.
        setOutcome({
          kind: "err",
          error: wire.result,
          retry_from: wire.retry_from,
        });
        return;
      }

      // Plan 27-05: partial-failure population. Bucket each wire
      // partial-failure into its stage's complete row so StageRow's
      // amber [⚠ K issues] badge + FindingRow list render automatically
      // (D-20). Stages not present in the partial-failure list keep
      // their existing complete status from the progress events.
      if (wire.partial_failures.length > 0) {
        setStages((prev) => {
          const next = new Map<SyncStage, StageStatus>();
          // Group partial failures by stage.
          const byStage = new Map<SyncStage, PartialFailure[]>();
          for (const pf of wire.partial_failures) {
            const list = byStage.get(pf.stage) ?? [];
            list.push(partialFailureFromWire(pf));
            byStage.set(pf.stage, list);
          }
          for (const [stage, status] of prev) {
            const failures = byStage.get(stage) ?? [];
            if (status.kind === "complete" && failures.length > 0) {
              next.set(stage, {
                ...status,
                partialFailures: [...status.partialFailures, ...failures],
              });
            } else if (failures.length > 0 && status.kind !== "complete") {
              // Defensive: failure recorded for a stage that didn't
              // emit a complete event. Synthesize a zero-duration
              // complete row so the badge + FindingRows still surface.
              next.set(stage, {
                kind: "complete",
                durationMs: 0,
                partialFailures: failures,
              });
            } else {
              next.set(stage, status);
            }
          }
          return next;
        });
      }

      setOutcome({ kind: "ok", wire });
    },
    [],
  );

  const start = useCallback(async (): Promise<void> => {
    // Reset per-run state.
    setStages(initialStages());
    stageStartAt.current.clear();
    setOutcome(null);
    setIsRunning(true);
    isRunningRef.current = true;
    cancelRequestedRef.current = false;

    const res = await commands.startSync();
    finalizeOutcome(res);
  }, [finalizeOutcome]);

  const retryFromStage = useCallback(
    async (stage: SyncStage): Promise<void> => {
      setStages(initialStages());
      stageStartAt.current.clear();
      setOutcome(null);
      setIsRunning(true);
      isRunningRef.current = true;
      cancelRequestedRef.current = false;

      const res = await commands.retrySyncFrom(stage);
      finalizeOutcome(res);
    },
    [finalizeOutcome],
  );

  const retryFailedItems = useCallback(async (): Promise<void> => {
    // Collect the wire shapes from the current stages Map so the
    // boundary sees a fresh-from-React payload (the prior outcome state
    // may have been cleared by a dismiss). Each partial failure carries
    // enough info for the Rust side to re-run that operation.
    const failures: PartialFailureWire[] = [];
    for (const [stage, status] of stages) {
      if (status.kind === "complete") {
        for (const pf of status.partialFailures) {
          failures.push({
            stage,
            // Domain operation defaults to Distribution; future plans
            // that surface per-operation provenance on the partial
            // failure will fold the real op through here. Today the
            // domain dispatch reads only stage + skill — the operation
            // tag is informational on the retry path.
            operation: "Distribution",
            skill: pf.itemName,
            error: pf.error,
          });
        }
      }
    }

    setStages(initialStages());
    stageStartAt.current.clear();
    setOutcome(null);
    setIsRunning(true);
    isRunningRef.current = true;
    cancelRequestedRef.current = false;

    const res = await commands.retryFailedItems(failures);
    finalizeOutcome(res);
  }, [finalizeOutcome, stages]);

  const cancel = useCallback(async (): Promise<void> => {
    // Plan 27-04: flip the local cancel-requested flag so the start()
    // promise's branch can classify the outcome as { kind: "cancelled" }
    // instead of generic { kind: "err" }. The Rust side returns
    // Err("sync cancelled") with ErrorCode::Internal (the catch-all)
    // because the cancellation pathway doesn't carry its own ErrorCode
    // sentinel; the React-side ref disambiguates.
    cancelRequestedRef.current = true;
    // Fire-and-forget on the Rust side; the actual stop signal flows via
    // CancelToken which `tome::sync` polls at stage boundaries. The Rust
    // command is idempotent — calling it twice (or while no sync is
    // running) is safe.
    const res = await commands.cancelSync();
    if (res.status === "error") {
      // Defensive — cancel_sync never returns an error today, but if it
      // ever did we'd surface it through the outcome state so the user
      // sees something happened. Keep silent otherwise. No retry_from
      // hint is available for cancel-command errors (they don't carry a
      // failed_stage tag).
      setOutcome({ kind: "err", error: res.error, retry_from: null });
    }
  }, []);

  const dismiss = useCallback((): void => {
    setStages(initialStages());
    stageStartAt.current.clear();
    setOutcome(null);
    cancelRequestedRef.current = false;
    // Plan 27-02: reset the triage state too so the post-Apply / post-
    // cancel idle view returns to a clean slate. The diff itself will be
    // refetched by the watcher (lockfileChanged fires from sync's Save
    // stage) — the seed effect will re-populate decisions on next load.
    setDecisions(new Map());
    setSelectedTriageSkill(null);
  }, []);

  // Plan 27-03 — Apply-success handler. Clears the decisions Map so the
  // [Apply N] button label drops back to 0 and the Sidebar badge clears.
  // We do NOT call refetchDiff() here because writing machine.toml doesn't
  // change the lockfile — the diff stays the same; only the user's
  // decisions are reset. The seed effect re-populates decisions to
  // all-keep on the next render (since `decisions.size === 0`).
  const applyComplete = useCallback((): void => {
    setDecisions(new Map());
    setSelectedTriageSkill(null);
  }, []);

  // Plan 27-04 + 27-05: derived classification of the run's outcome.
  // Drives SyncView's terminal-branch selection (success → SyncToast;
  // cancelled → inline summary in the stepper; failed → "Sync failed"
  // summary + Retry from <stage> + Dismiss; partial → "Sync complete
  // with K issues" + Retry failed items + Dismiss). `null` while idle
  // or running.
  const terminalKind: SyncTerminalKind = useMemo(() => {
    if (outcome === null || isRunning) return null;
    if (outcome.kind === "cancelled") return "cancelled";
    if (outcome.kind === "err") return "failed";
    // outcome.kind === "ok" — partial failures live on each stage's
    // complete row (populated by finalizeOutcome from
    // SyncOutcomeWire.partial_failures). Plan 27-05 D-20.
    let totalPartialFailures = 0;
    for (const status of stages.values()) {
      if (status.kind === "complete") {
        totalPartialFailures += status.partialFailures.length;
      }
    }
    return totalPartialFailures > 0 ? "partial" : "success";
  }, [outcome, isRunning, stages]);

  // Plan 27-04 + 27-05: failure count derived from stages'
  // partialFailures + failed-stage rows.
  const failureCount = useMemo(() => {
    let n = 0;
    for (const status of stages.values()) {
      if (status.kind === "complete") n += status.partialFailures.length;
      if (status.kind === "failed") n += 1;
    }
    return n;
  }, [stages]);

  // Plan 27-05: post-sync failure-badge count. The Sidebar Sync NavItem
  // reads this to render its danger-fill failure badge. Semantics:
  // count = partial failures in the most recent terminal state, or 1
  // for a failed run, or 0 for clean success. Persists across
  // `dismiss()` so a user who closes the summary still sees the badge;
  // resets only when a fresh sync run completes cleanly. The
  // `failureCount` derivation above already gives us the cumulative
  // count; we add the err-without-partial branch on top so a fatal
  // failure also surfaces.
  const unresolvedFailureCount = useMemo(() => {
    if (failureCount > 0) return failureCount;
    if (outcome?.kind === "err") return 1;
    return 0;
  }, [failureCount, outcome]);

  return {
    stages,
    isRunning,
    isRunningRef,
    outcome,
    terminalKind,
    diff,
    diffError,
    decisions,
    selectedTriageSkill,
    pendingDecisionCount,
    pendingDiffCount,
    failureCount,
    unresolvedFailureCount,
    start,
    cancel,
    dismiss,
    retryFromStage,
    retryFailedItems,
    onDecisionChange,
    onBulkAction,
    selectTriageSkill,
    refetchDiff,
    applyComplete,
  };
}

// -------------------------------------------------------------------
// Context wiring — the single state machine the whole app shares.
// -------------------------------------------------------------------

const SyncContext = createContext<UseSyncResult | null>(null);

/** Wrap the App root. All `useSync()` consumers below share one machine. */
export function SyncProvider({ children }: { children: ReactNode }) {
  const value = useSyncInternal();
  return (
    <SyncContext.Provider value={value}>{children}</SyncContext.Provider>
  );
}

/** Public hook — reads the shared SyncContext. Throws helpfully when used
 *  outside the provider (catches a regression where someone forgets to
 *  wrap App in `<SyncProvider>`). */
export function useSync(): UseSyncResult {
  const ctx = useContext(SyncContext);
  if (ctx === null) {
    throw new Error(
      "useSync() must be used inside <SyncProvider>. Wrap App.tsx or " +
        "the root render entry with SyncProvider so all consumers share " +
        "one in-flight state machine.",
    );
  }
  return ctx;
}
