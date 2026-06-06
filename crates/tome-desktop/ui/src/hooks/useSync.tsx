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
  SkillName,
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

/** Per-stage status. The shape is intentionally narrow for plan 27-01b —
 *  StageStepper (27-04) and SyncOutcome (27-05) will expand with terminal
 *  flags + partial-failure rows. */
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
    };

export type SyncTerminal =
  /** Sync finished successfully — render the post-sync "Sync complete"
   *  placeholder. Plan 27-05 swaps for a structured SyncOutcomeWire. */
  | { kind: "ok" }
  /** Sync returned an error from `commands.startSync()` (including the
   *  T-27-01b-07 Conflict from a double-fire). */
  | { kind: "err"; error: TomeError };

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
  /** Kick off a sync. Idempotent against double-fire — the Rust side
   *  returns ErrorCode::Conflict if a sync is already in flight; the
   *  hook surfaces that as the outcome (T-27-01b-07). */
  start: () => Promise<void>;
  /** Cancel an in-flight sync. Sync command on the Rust side; idempotent. */
  cancel: () => Promise<void>;
  /** Reset to idle from the terminal state. */
  dismiss: () => void;
  /** Plan 27-02 — set the decision for a single skill. */
  onDecisionChange: (skill: SkillName, decision: TriageDecision) => void;
  /** Plan 27-02 — apply a bulk-action scope to a decision. */
  onBulkAction: (scope: BulkScope, decision: TriageDecision) => void;
  /** Plan 27-02 — set or clear the selected TriageRow. */
  selectTriageSkill: (skill: SkillName | null) => void;
  /** Plan 27-02 — manually refetch the lockfile diff. The triage panel's
   *  Apply flow (27-03) will trigger this once it lands. */
  refetchDiff: () => Promise<void>;
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

  const start = useCallback(async (): Promise<void> => {
    // Reset per-run state.
    setStages(initialStages());
    stageStartAt.current.clear();
    setOutcome(null);
    setIsRunning(true);
    isRunningRef.current = true;

    const res = await commands.startSync();

    // The terminal flip + outcome write happen together so the UI doesn't
    // briefly render the in-progress placeholder past the actual stop.
    isRunningRef.current = false;
    setIsRunning(false);
    if (res.status === "ok") {
      setOutcome({ kind: "ok" });
    } else {
      setOutcome({ kind: "err", error: res.error });
    }
  }, []);

  const cancel = useCallback(async (): Promise<void> => {
    // Fire-and-forget on the Rust side; the actual stop signal flows via
    // CancelToken which `tome::sync` polls at stage boundaries. The Rust
    // command is idempotent — calling it twice (or while no sync is
    // running) is safe.
    const res = await commands.cancelSync();
    if (res.status === "error") {
      // Defensive — cancel_sync never returns an error today, but if it
      // ever did we'd surface it through the outcome state so the user
      // sees something happened. Keep silent otherwise.
      setOutcome({ kind: "err", error: res.error });
    }
  }, []);

  const dismiss = useCallback((): void => {
    setStages(initialStages());
    stageStartAt.current.clear();
    setOutcome(null);
    // Plan 27-02: reset the triage state too so the post-Apply / post-
    // cancel idle view returns to a clean slate. The diff itself will be
    // refetched by the watcher (lockfileChanged fires from sync's Save
    // stage) — the seed effect will re-populate decisions on next load.
    setDecisions(new Map());
    setSelectedTriageSkill(null);
  }, []);

  return {
    stages,
    isRunning,
    isRunningRef,
    outcome,
    diff,
    diffError,
    decisions,
    selectedTriageSkill,
    pendingDecisionCount,
    pendingDiffCount,
    failureCount: 0, // Stub — 27-05 populates.
    start,
    cancel,
    dismiss,
    onDecisionChange,
    onBulkAction,
    selectTriageSkill,
    refetchDiff,
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
