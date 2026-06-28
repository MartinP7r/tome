// useSync — Phase 27 plan 27-05 SyncOutcomeWire classification tests.
//
// Pins the structured-outcome contract:
//
// 1. A SyncOutcomeWire with result=null + partial_failures=[] classifies
//    as terminalKind === "success"; outcome.kind === "ok".
// 2. A SyncOutcomeWire with result=null + non-empty partial_failures
//    classifies as terminalKind === "partial"; the failures populate
//    the relevant stages' partialFailures arrays so the StageRow
//    renders its [⚠ K issues] badge.
// 3. A SyncOutcomeWire with result=non-null + retry_from=Some(stage)
//    classifies as terminalKind === "failed"; outcome.retry_from
//    carries the stage hint for the SyncView's [Retry from <stage>]
//    button.
// 4. A SyncOutcomeWire with result=non-null + retry_from=null
//    classifies as terminalKind === "failed" with retry_from=null
//    (Save-failure shape; no retry affordance).
// 5. retryFromStage(stage) calls commands.retrySyncFrom(stage).
// 6. retryFailedItems() calls commands.retryFailedItems(failures).
// 7. unresolvedFailureCount surfaces failureCount when >0; otherwise
//    falls back to 1 on err outcomes; 0 on clean success.

import { act, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const listenSpies = {
  syncProgress: vi.fn(),
  manifestChanged: vi.fn(),
  lockfileChanged: vi.fn(),
  libraryChanged: vi.fn(),
  machinePrefsChanged: vi.fn(),
  menuAction: vi.fn(),
};

const startSyncSpy = vi.fn();
const cancelSyncSpy = vi.fn();
const retrySyncFromSpy = vi.fn();
const retryFailedItemsSpy = vi.fn();
const getLockfileDiffSpy = vi.fn();

vi.mock("../../bindings", () => ({
  events: {
    syncProgress: {
      listen: (cb: (evt: { payload: unknown }) => void) => {
        listenSpies.syncProgress(cb);
        return Promise.resolve(() => undefined);
      },
    },
    manifestChanged: {
      listen: (cb: () => void) => {
        listenSpies.manifestChanged(cb);
        return Promise.resolve(() => undefined);
      },
    },
    lockfileChanged: {
      listen: (cb: () => void) => {
        listenSpies.lockfileChanged(cb);
        return Promise.resolve(() => undefined);
      },
    },
    libraryChanged: {
      listen: (cb: () => void) => {
        listenSpies.libraryChanged(cb);
        return Promise.resolve(() => undefined);
      },
    },
    machinePrefsChanged: {
      listen: (cb: () => void) => {
        listenSpies.machinePrefsChanged(cb);
        return Promise.resolve(() => undefined);
      },
    },
    menuAction: {
      listen: (cb: () => void) => {
        listenSpies.menuAction(cb);
        return Promise.resolve(() => undefined);
      },
    },
  },
  commands: {
    startSync: () => startSyncSpy(),
    cancelSync: () => cancelSyncSpy(),
    retrySyncFrom: (stage: string) => retrySyncFromSpy(stage),
    retryFailedItems: (failures: unknown) => retryFailedItemsSpy(failures),
    getLockfileDiff: () => getLockfileDiffSpy(),
  },
}));

import { SyncProvider, useSync, type UseSyncResult } from "../useSync";

function Probe({ capture }: { capture: (api: UseSyncResult) => void }) {
  const api = useSync();
  capture(api);
  return (
    <div>
      <span data-testid="terminal-kind">{String(api.terminalKind)}</span>
      <span data-testid="outcome-kind">{api.outcome?.kind ?? "null"}</span>
      <span data-testid="failure-count">{api.failureCount}</span>
      <span data-testid="unresolved-count">{api.unresolvedFailureCount}</span>
      <button
        type="button"
        onClick={() => void api.start()}
        data-testid="start"
      >
        start
      </button>
      <button
        type="button"
        onClick={() => void api.retryFromStage("Discover")}
        data-testid="retry-from"
      >
        retry-from
      </button>
      <button
        type="button"
        onClick={() => void api.retryFailedItems()}
        data-testid="retry-failed"
      >
        retry-failed
      </button>
      <button type="button" onClick={() => api.dismiss()} data-testid="dismiss">
        dismiss
      </button>
    </div>
  );
}

function resetSpies() {
  listenSpies.syncProgress.mockReset();
  listenSpies.manifestChanged.mockReset();
  listenSpies.lockfileChanged.mockReset();
  listenSpies.libraryChanged.mockReset();
  listenSpies.machinePrefsChanged.mockReset();
  listenSpies.menuAction.mockReset();
  startSyncSpy.mockReset();
  cancelSyncSpy.mockReset();
  retrySyncFromSpy.mockReset();
  retryFailedItemsSpy.mockReset();
  getLockfileDiffSpy.mockReset();
  cancelSyncSpy.mockResolvedValue({ status: "ok", data: null });
  getLockfileDiffSpy.mockResolvedValue({
    status: "ok",
    data: { added: [], changed: [], removed: [] },
  });
}

describe("useSync — Plan 27-05 SyncOutcomeWire classification", () => {
  beforeEach(resetSpies);

  it("clean Ok outcome (result=null, no partials) is terminalKind='success'", async () => {
    startSyncSpy.mockResolvedValueOnce({
      status: "ok",
      data: { result: null, retry_from: null, partial_failures: [] },
    });

    render(
      <SyncProvider>
        <Probe capture={() => undefined} />
      </SyncProvider>,
    );

    await act(async () => {
      screen.getByTestId("start").click();
    });

    expect(screen.getByTestId("terminal-kind").textContent).toBe("success");
    expect(screen.getByTestId("outcome-kind").textContent).toBe("ok");
    expect(screen.getByTestId("failure-count").textContent).toBe("0");
    expect(screen.getByTestId("unresolved-count").textContent).toBe("0");
  });

  it("Ok-with-partials outcome is terminalKind='partial'; populates stages", async () => {
    // Drive a Distribute SyncStageStarted + SyncStageFinished event so
    // the stages Map has a 'complete' row to attach the partial-failure
    // to. (finalizeOutcome populates the existing complete row's
    // partialFailures array.)
    startSyncSpy.mockReturnValueOnce(
      new Promise(() => {
        /* never resolves until we trigger it below */
      }),
    );

    let resolveStart: (v: unknown) => void = () => undefined;
    startSyncSpy.mockReset();
    startSyncSpy.mockReturnValueOnce(
      new Promise((resolve) => {
        resolveStart = resolve;
      }),
    );

    render(
      <SyncProvider>
        <Probe capture={() => undefined} />
      </SyncProvider>,
    );

    await act(async () => {
      screen.getByTestId("start").click();
    });

    // Drive Distribute start + finish via the syncProgress handler.
    const handler = listenSpies.syncProgress.mock.calls[0]?.[0] as
      | ((evt: { payload: unknown }) => void)
      | undefined;
    expect(handler).toBeTypeOf("function");
    await act(async () => {
      handler?.({
        payload: { stage: "Distribute", current: 0, total: 0, item: null },
      });
      handler?.({
        payload: { stage: "Distribute", current: 0, total: 0, item: null },
      });
    });

    // Now resolve startSync with an Ok outcome carrying one partial
    // failure against Distribute.
    await act(async () => {
      resolveStart({
        status: "ok",
        data: {
          result: null,
          retry_from: null,
          partial_failures: [
            {
              stage: "Distribute",
              operation: "Distribution",
              skill: "axiom-build",
              error: {
                code: "Internal",
                message: "permission denied",
                context: ["permission denied"],
              },
            },
          ],
        },
      });
    });

    expect(screen.getByTestId("terminal-kind").textContent).toBe("partial");
    expect(screen.getByTestId("outcome-kind").textContent).toBe("ok");
    expect(screen.getByTestId("failure-count").textContent).toBe("1");
    expect(screen.getByTestId("unresolved-count").textContent).toBe("1");
  });

  it("Err outcome with retry_from=Discover is terminalKind='failed' carrying the hint", async () => {
    startSyncSpy.mockResolvedValueOnce({
      status: "ok",
      data: {
        result: {
          code: "Permission",
          message: "consolidate failed",
          context: ["permission denied at /tmp/foo"],
        },
        retry_from: "Discover",
        partial_failures: [],
      },
    });

    let captured: UseSyncResult | null = null;
    render(
      <SyncProvider>
        <Probe
          capture={(api) => {
            captured = api;
          }}
        />
      </SyncProvider>,
    );

    await act(async () => {
      screen.getByTestId("start").click();
    });

    expect(screen.getByTestId("terminal-kind").textContent).toBe("failed");
    expect(screen.getByTestId("outcome-kind").textContent).toBe("err");
    expect(captured!.outcome).toMatchObject({
      kind: "err",
      retry_from: "Discover",
    });
    // Single-error outcomes count as 1 unresolved failure for the
    // Sidebar badge.
    expect(screen.getByTestId("unresolved-count").textContent).toBe("1");
  });

  it("Err outcome with retry_from=null is terminalKind='failed' with no retry hint (Save shape)", async () => {
    startSyncSpy.mockResolvedValueOnce({
      status: "ok",
      data: {
        result: {
          code: "Io",
          message: "disk full",
          context: ["disk full"],
        },
        retry_from: null,
        partial_failures: [],
      },
    });

    let captured: UseSyncResult | null = null;
    render(
      <SyncProvider>
        <Probe
          capture={(api) => {
            captured = api;
          }}
        />
      </SyncProvider>,
    );

    await act(async () => {
      screen.getByTestId("start").click();
    });

    expect(screen.getByTestId("terminal-kind").textContent).toBe("failed");
    expect(captured!.outcome).toMatchObject({
      kind: "err",
      retry_from: null,
    });
  });

  it("retryFromStage('Discover') invokes commands.retrySyncFrom('Discover')", async () => {
    retrySyncFromSpy.mockResolvedValueOnce({
      status: "ok",
      data: { result: null, retry_from: null, partial_failures: [] },
    });

    render(
      <SyncProvider>
        <Probe capture={() => undefined} />
      </SyncProvider>,
    );

    await act(async () => {
      screen.getByTestId("retry-from").click();
    });

    expect(retrySyncFromSpy).toHaveBeenCalledTimes(1);
    expect(retrySyncFromSpy).toHaveBeenCalledWith("Discover");
  });

  it("retryFailedItems() invokes commands.retryFailedItems with the collected wire failures", async () => {
    retryFailedItemsSpy.mockResolvedValueOnce({
      status: "ok",
      data: { result: null, retry_from: null, partial_failures: [] },
    });

    render(
      <SyncProvider>
        <Probe capture={() => undefined} />
      </SyncProvider>,
    );

    await act(async () => {
      screen.getByTestId("retry-failed").click();
    });

    expect(retryFailedItemsSpy).toHaveBeenCalledTimes(1);
    // Empty stages Map → empty failures array; the call still happens
    // so the Rust side gets a clean retry-zero-items invocation.
    expect(retryFailedItemsSpy).toHaveBeenCalledWith([]);
  });
});
