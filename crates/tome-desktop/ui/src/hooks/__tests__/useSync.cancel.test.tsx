// useSync — Phase 27 plan 27-04 cancellation classification tests.
//
// Pin the cancel-detection contract added in plan 27-04:
//
// 1. Calling cancel() and then awaiting startSync's promise produces
//    outcome.kind === "cancelled" (NOT "err"), even though the Rust
//    side returns Err. The cancelRequestedRef branch in start() owns
//    the disambiguation.
// 2. The terminalKind derived flag classifies cancelled outcomes
//    correctly.
// 3. dismiss() clears the cancel-requested ref so a subsequent run
//    starts fresh (won't false-positive classify as cancelled).
// 4. Without a cancel() call, an Err outcome falls through to
//    terminalKind === "failed".

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
    getLockfileDiff: () => getLockfileDiffSpy(),
  },
}));

import { SyncProvider, useSync, type UseSyncResult } from "../useSync";

function Probe({
  capture,
}: {
  capture: (api: UseSyncResult) => void;
}) {
  const api = useSync();
  capture(api);
  return (
    <div>
      <span data-testid="terminal-kind">{String(api.terminalKind)}</span>
      <span data-testid="outcome-kind">{api.outcome?.kind ?? "null"}</span>
      <button
        type="button"
        onClick={() => void api.start()}
        data-testid="start"
      >
        start
      </button>
      <button
        type="button"
        onClick={() => void api.cancel()}
        data-testid="cancel"
      >
        cancel
      </button>
      <button type="button" onClick={() => api.dismiss()} data-testid="dismiss">
        dismiss
      </button>
    </div>
  );
}

describe("useSync — Plan 27-04 cancel classification", () => {
  beforeEach(() => {
    listenSpies.syncProgress.mockReset();
    listenSpies.manifestChanged.mockReset();
    listenSpies.lockfileChanged.mockReset();
    listenSpies.libraryChanged.mockReset();
    listenSpies.machinePrefsChanged.mockReset();
    listenSpies.menuAction.mockReset();
    startSyncSpy.mockReset();
    cancelSyncSpy.mockReset();
    getLockfileDiffSpy.mockReset();
    cancelSyncSpy.mockResolvedValue({ status: "ok", data: null });
    getLockfileDiffSpy.mockResolvedValue({
      status: "ok",
      data: { added: [], changed: [], removed: [] },
    });
  });

  it("classifies a post-cancel Err outcome as terminalKind='cancelled'", async () => {
    // Simulate the Rust side: startSync awaits while user clicks cancel,
    // then resolves with Err("sync cancelled") → ErrorCode::Internal.
    let resolveStart: (v: unknown) => void = () => undefined;
    startSyncSpy.mockReturnValue(
      new Promise((resolve) => {
        resolveStart = resolve;
      }),
    );

    render(
      <SyncProvider>
        <Probe capture={() => undefined} />
      </SyncProvider>,
    );

    // Kick off the run.
    await act(async () => {
      screen.getByTestId("start").click();
    });
    // The hook is now mid-run; cancel() flips the local flag.
    await act(async () => {
      screen.getByTestId("cancel").click();
    });
    expect(cancelSyncSpy).toHaveBeenCalledTimes(1);

    // Now resolve startSync with the cancel-shaped error the Rust side
    // would emit (ErrorCode::Internal + "sync cancelled" message).
    await act(async () => {
      resolveStart({
        status: "error",
        error: {
          code: "Internal",
          message: "sync cancelled",
          context: [],
        },
      });
    });

    expect(screen.getByTestId("terminal-kind").textContent).toBe("cancelled");
    expect(screen.getByTestId("outcome-kind").textContent).toBe("cancelled");
  });

  it("classifies a no-cancel Err outcome as terminalKind='failed'", async () => {
    // Run finishes with a real failure — user never clicked cancel.
    startSyncSpy.mockResolvedValueOnce({
      status: "error",
      error: {
        code: "Conflict",
        message: "two skills named foo",
        context: [],
      },
    });

    render(
      <SyncProvider>
        <Probe capture={() => undefined} />
      </SyncProvider>,
    );

    await act(async () => {
      screen.getByTestId("start").click();
    });

    expect(screen.getByTestId("terminal-kind").textContent).toBe("failed");
    expect(screen.getByTestId("outcome-kind").textContent).toBe("err");
  });

  it("classifies a clean Ok outcome as terminalKind='success'", async () => {
    startSyncSpy.mockResolvedValueOnce({ status: "ok", data: null });

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
  });

  it("dismiss() clears the cancel-requested flag for the next run", async () => {
    let resolveStart: (v: unknown) => void = () => undefined;
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
    await act(async () => {
      screen.getByTestId("cancel").click();
    });
    await act(async () => {
      resolveStart({
        status: "error",
        error: { code: "Internal", message: "sync cancelled", context: [] },
      });
    });
    expect(screen.getByTestId("terminal-kind").textContent).toBe("cancelled");

    // Reset to idle.
    await act(async () => {
      screen.getByTestId("dismiss").click();
    });
    expect(screen.getByTestId("terminal-kind").textContent).toBe("null");

    // Next run: Err outcome should be 'failed' (NOT cancelled).
    startSyncSpy.mockResolvedValueOnce({
      status: "error",
      error: {
        code: "Conflict",
        message: "double-fire",
        context: [],
      },
    });
    await act(async () => {
      screen.getByTestId("start").click();
    });
    expect(screen.getByTestId("terminal-kind").textContent).toBe("failed");
  });
});

describe("useSync — stages map transforms on cancel", () => {
  beforeEach(() => {
    listenSpies.syncProgress.mockReset();
    listenSpies.manifestChanged.mockReset();
    listenSpies.lockfileChanged.mockReset();
    listenSpies.libraryChanged.mockReset();
    listenSpies.machinePrefsChanged.mockReset();
    listenSpies.menuAction.mockReset();
    startSyncSpy.mockReset();
    cancelSyncSpy.mockReset();
    getLockfileDiffSpy.mockReset();
    cancelSyncSpy.mockResolvedValue({ status: "ok", data: null });
    getLockfileDiffSpy.mockResolvedValue({
      status: "ok",
      data: { added: [], changed: [], removed: [] },
    });
  });

  it("flips active+pending stages to cancelled when sync was cancelled", async () => {
    let captured: UseSyncResult | null = null;

    let resolveStart: (v: unknown) => void = () => undefined;
    startSyncSpy.mockReturnValue(
      new Promise((resolve) => {
        resolveStart = resolve;
      }),
    );

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

    // Drive a SyncStageStarted for Reconcile + a complete for it +
    // a Started for Discover (mid-Discover state) via the captured
    // syncProgress handler.
    const handler = listenSpies.syncProgress.mock.calls[0]?.[0] as
      | ((evt: { payload: unknown }) => void)
      | undefined;
    expect(handler).toBeTypeOf("function");

    // Helper to drive the handler synchronously inside act().
    function fire(payload: {
      stage: string;
      current: number;
      total: number;
      item: string | null;
    }) {
      handler?.({ payload });
    }

    await act(async () => {
      fire({ stage: "Reconcile", current: 0, total: 0, item: null });
      fire({ stage: "Reconcile", current: 0, total: 0, item: null }); // finish
      fire({ stage: "Discover", current: 0, total: 0, item: null }); // start
    });

    // Verify pre-cancel: Reconcile complete, Discover active.
    expect(captured!.stages.get("Reconcile")?.kind).toBe("complete");
    expect(captured!.stages.get("Discover")?.kind).toBe("active");

    // Cancel + resolve sync with err.
    await act(async () => {
      screen.getByTestId("cancel").click();
    });
    await act(async () => {
      resolveStart({
        status: "error",
        error: { code: "Internal", message: "sync cancelled", context: [] },
      });
    });

    // Reconcile (already complete) stays complete.
    expect(captured!.stages.get("Reconcile")?.kind).toBe("complete");
    // Discover (was active) flips to cancelled.
    expect(captured!.stages.get("Discover")?.kind).toBe("cancelled");
    // Pending stages flip to cancelled.
    expect(captured!.stages.get("Consolidate")?.kind).toBe("cancelled");
    expect(captured!.stages.get("Distribute")?.kind).toBe("cancelled");
    expect(captured!.stages.get("Cleanup")?.kind).toBe("cancelled");
    expect(captured!.stages.get("Save")?.kind).toBe("cancelled");
  });
});
