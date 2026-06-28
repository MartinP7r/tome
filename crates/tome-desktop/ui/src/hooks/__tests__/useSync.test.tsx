// useSync tests — Pitfall 6 discipline + cancel handler (plan 27-01b).
//
// Pitfall 6: the hook MUST NOT subscribe to manifestChanged /
// lockfileChanged / libraryChanged / machinePrefsChanged. Those events
// are emitted by the file watcher as a side effect of the sync writing
// the manifest + lockfile; subscribing here would create a feedback
// loop. The plan tests this by mocking the bindings, calling each
// watcher event's listen registration, asserting the hook never
// registered a callback for them. (Equivalently: confirm `useSync` only
// touched `events.syncProgress`.)

import { act, render, screen } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock the generated bindings. We track which events have `listen` called
// against them so the Pitfall-6 assertion can check the subscription set.
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

// Now import the hook (must come AFTER the vi.mock above).
import { SyncProvider, useSync } from "../useSync";

/** Tiny probe component that surfaces the hook's state into the DOM so
 *  testing-library queries can drive assertions. */
function Probe({ onMount }: { onMount?: (api: ReturnType<typeof useSync>) => void }) {
  const sync = useSync();
  if (onMount) {
    onMount(sync);
  }
  return (
    <div>
      <span data-testid="is-running">{String(sync.isRunning)}</span>
      <span data-testid="outcome-kind">{sync.outcome?.kind ?? "null"}</span>
      <button type="button" onClick={() => void sync.cancel()}>
        Cancel sync
      </button>
      <button type="button" onClick={() => void sync.start()}>
        Run sync
      </button>
    </div>
  );
}

describe("useSync — Pitfall 6 watcher-feedback discipline", () => {
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
    // Plan 27-05: startSync now returns a SyncOutcomeWire on success
    // (clean: result=null, retry_from=null, partial_failures=[]).
    startSyncSpy.mockResolvedValue({
      status: "ok",
      data: { result: null, retry_from: null, partial_failures: [] },
    });
    cancelSyncSpy.mockResolvedValue({ status: "ok", data: null });
    getLockfileDiffSpy.mockResolvedValue({
      status: "ok",
      data: { added: [], changed: [], removed: [] },
    });
  });

  it("subscribes to events.syncProgress and (via useLockfileDiff) events.lockfileChanged, but to nothing else", () => {
    render(
      <SyncProvider>
        <Probe />
      </SyncProvider>,
    );

    expect(listenSpies.syncProgress).toHaveBeenCalledTimes(1);
    // Plan 27-02 — useSync now composes useLockfileDiff to drive the
    // triage panel, which subscribes to lockfileChanged. The Pitfall 6
    // discipline is upheld inside useLockfileDiff: the handler gates on
    // isRunningRef.current so a mid-sync rewrite does NOT trigger a
    // refetch loop (verified by useLockfileDiff's own test suite).
    expect(listenSpies.lockfileChanged).toHaveBeenCalledTimes(1);
    // The remaining watcher events MUST stay unobserved by useSync —
    // manifest / library / machine-prefs writes that don't reach the
    // lockfile have no effect on the diff payload.
    expect(listenSpies.manifestChanged).not.toHaveBeenCalled();
    expect(listenSpies.libraryChanged).not.toHaveBeenCalled();
    expect(listenSpies.machinePrefsChanged).not.toHaveBeenCalled();
    // menuAction is unrelated — useSync should not own it either.
    expect(listenSpies.menuAction).not.toHaveBeenCalled();
  });

  it("renders one syncProgress subscription even with the provider mounted once", () => {
    // Re-pin that the provider does NOT register multiple listeners
    // (a regression where someone calls useSyncInternal() in two places
    // inside the provider would surface here).
    render(
      <SyncProvider>
        <Probe />
        <Probe />
        <Probe />
      </SyncProvider>,
    );
    expect(listenSpies.syncProgress).toHaveBeenCalledTimes(1);
  });
});

describe("useSync — cancel handler", () => {
  beforeEach(() => {
    cancelSyncSpy.mockReset();
    getLockfileDiffSpy.mockReset();
    cancelSyncSpy.mockResolvedValue({ status: "ok", data: null });
    getLockfileDiffSpy.mockResolvedValue({
      status: "ok",
      data: { added: [], changed: [], removed: [] },
    });
  });

  it("clicking [Cancel sync] calls commands.cancelSync exactly once", async () => {
    render(
      <SyncProvider>
        <Probe />
      </SyncProvider>,
    );

    await act(async () => {
      screen.getByText("Cancel sync").click();
    });

    expect(cancelSyncSpy).toHaveBeenCalledTimes(1);
  });
});

describe("useSync — start handler", () => {
  beforeEach(() => {
    startSyncSpy.mockReset();
    getLockfileDiffSpy.mockReset();
    // Plan 27-05: startSync now returns a SyncOutcomeWire on success
    // (clean: result=null, retry_from=null, partial_failures=[]).
    startSyncSpy.mockResolvedValue({
      status: "ok",
      data: { result: null, retry_from: null, partial_failures: [] },
    });
    getLockfileDiffSpy.mockResolvedValue({
      status: "ok",
      data: { added: [], changed: [], removed: [] },
    });
  });

  it("clicking [Run sync] calls commands.startSync exactly once", async () => {
    render(
      <SyncProvider>
        <Probe />
      </SyncProvider>,
    );

    await act(async () => {
      screen.getByText("Run sync").click();
    });

    expect(startSyncSpy).toHaveBeenCalledTimes(1);
  });

  it("surfaces a Conflict error as the terminal outcome (T-27-01b-07)", async () => {
    startSyncSpy.mockResolvedValueOnce({
      status: "error",
      error: {
        code: "Conflict",
        message: "sync already in progress",
        context: [],
      },
    });

    render(
      <SyncProvider>
        <Probe />
      </SyncProvider>,
    );

    await act(async () => {
      screen.getByText("Run sync").click();
    });

    expect(screen.getByTestId("outcome-kind").textContent).toBe("err");
    expect(screen.getByTestId("is-running").textContent).toBe("false");
  });
});
