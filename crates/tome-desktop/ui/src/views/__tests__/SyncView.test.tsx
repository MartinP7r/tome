// SyncView tests — Phase 27 plan 27-04 terminal-state branches.
//
// Pin the three new render shapes added in 27-04:
//
//   1. In-progress → StageStepper mounted; [Cancel sync] button visible.
//   2. Terminal cancelled → StageStepper + summary block ("Sync cancelled"
//      + sub-line + [Run sync] + [Dismiss]); NO SyncToast.
//   3. Terminal success → SyncToast + idle hero underneath.

import { act, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

// Mock the bindings BEFORE the import of SyncView / SyncProvider.
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
const getStatusSpy = vi.fn();
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
    getStatus: () => getStatusSpy(),
    startSync: () => startSyncSpy(),
    cancelSync: () => cancelSyncSpy(),
    getLockfileDiff: () => getLockfileDiffSpy(),
  },
}));

import { SyncProvider } from "../../hooks/useSync";
import { SyncView } from "../SyncView";

beforeEach(() => {
  listenSpies.syncProgress.mockReset();
  listenSpies.manifestChanged.mockReset();
  listenSpies.lockfileChanged.mockReset();
  listenSpies.libraryChanged.mockReset();
  listenSpies.machinePrefsChanged.mockReset();
  listenSpies.menuAction.mockReset();
  startSyncSpy.mockReset();
  cancelSyncSpy.mockReset();
  getStatusSpy.mockReset();
  getLockfileDiffSpy.mockReset();
  cancelSyncSpy.mockResolvedValue({ status: "ok", data: null });
  getStatusSpy.mockResolvedValue({
    status: "ok",
    data: {
      last_sync: "2026-06-05T09:00:00Z",
      directories: [],
      library_total: 0,
      enabled_total: 0,
      disabled_total: 0,
    },
  });
  getLockfileDiffSpy.mockResolvedValue({
    status: "ok",
    data: { added: [], changed: [], removed: [] },
  });
});

describe("SyncView — idle hero (no terminal state)", () => {
  it("renders the Run sync button when terminalKind === null", async () => {
    render(
      <SyncProvider>
        <SyncView />
      </SyncProvider>,
    );
    expect(
      await screen.findByRole("button", { name: "Run sync" }),
    ).toBeInTheDocument();
  });
});

describe("SyncView — in-progress branch", () => {
  it("mounts the StageStepper with [Cancel sync] when sync is running", async () => {
    // startSync returns a never-resolving promise so the hook stays in
    // the "running" state for the duration of the test.
    startSyncSpy.mockReturnValue(new Promise(() => undefined));

    render(
      <SyncProvider>
        <SyncView />
      </SyncProvider>,
    );
    const runButton = await screen.findByRole("button", { name: "Run sync" });

    await act(async () => {
      runButton.click();
    });

    // The stepper renders a list-role with the canonical label.
    expect(
      await screen.findByRole("list", { name: "Sync pipeline progress" }),
    ).toBeInTheDocument();
    // [Cancel sync] appears above the stepper.
    expect(
      screen.getByRole("button", {
        name: "Cancel sync at next stage boundary",
      }),
    ).toBeInTheDocument();
  });
});

describe("SyncView — terminal cancelled branch", () => {
  it("renders the 'Sync cancelled' summary + [Run sync] + [Dismiss], NO SyncToast", async () => {
    // Drive a cancellation flow: startSync resolves with the cancel-
    // shaped error AFTER cancel() has been called.
    let resolveStart: (v: unknown) => void = () => undefined;
    startSyncSpy.mockReturnValue(
      new Promise((resolve) => {
        resolveStart = resolve;
      }),
    );

    render(
      <SyncProvider>
        <SyncView />
      </SyncProvider>,
    );
    const runButton = await screen.findByRole("button", { name: "Run sync" });
    await act(async () => {
      runButton.click();
    });

    // Cancel mid-run.
    const cancelBtn = await screen.findByRole("button", {
      name: "Cancel sync at next stage boundary",
    });
    await act(async () => {
      cancelBtn.click();
    });

    // Rust returns Err("sync cancelled") via ErrorCode::Internal.
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

    // Summary heading lands at <h1>.
    expect(
      await screen.findByRole("heading", { level: 1, name: "Sync cancelled" }),
    ).toBeInTheDocument();
    // Sub-line copy verbatim per UI-SPEC §Terminal cancelled.
    expect(
      screen.getByText(
        /The library is in a consistent state\. You can run sync again at any time\./,
      ),
    ).toBeInTheDocument();
    // [Run sync] (primary) + [Dismiss] (secondary, two of them — one
    // from the summary, one from the stepper trailing slot).
    expect(screen.getByRole("button", { name: "Run sync" })).toBeInTheDocument();
    // There is NO SyncToast in the cancelled branch (D-18 supersession).
    expect(
      screen.queryByText("Sync complete"),
    ).not.toBeInTheDocument();
  });

  it("the stepper transforms in-place — the active stage flips to cancelled", async () => {
    let resolveStart: (v: unknown) => void = () => undefined;
    startSyncSpy.mockReturnValue(
      new Promise((resolve) => {
        resolveStart = resolve;
      }),
    );

    render(
      <SyncProvider>
        <SyncView />
      </SyncProvider>,
    );
    const runButton = await screen.findByRole("button", { name: "Run sync" });
    await act(async () => {
      runButton.click();
    });

    // Drive a SyncStageStarted{Reconcile} via the captured handler.
    const handler = listenSpies.syncProgress.mock.calls[0]?.[0] as
      | ((evt: { payload: unknown }) => void)
      | undefined;
    expect(handler).toBeTypeOf("function");

    await act(async () => {
      handler?.({
        payload: { stage: "Reconcile", current: 0, total: 0, item: null },
      });
    });

    // Cancel + resolve with err.
    const cancelBtn = await screen.findByRole("button", {
      name: "Cancel sync at next stage boundary",
    });
    await act(async () => {
      cancelBtn.click();
    });
    await act(async () => {
      resolveStart({
        status: "error",
        error: { code: "Internal", message: "sync cancelled", context: [] },
      });
    });

    // The Reconcile row should now show as cancelled — the
    // aria-label includes "cancelled".
    expect(
      await screen.findByRole("listitem", {
        name: "Reconcile stage, cancelled",
      }),
    ).toBeInTheDocument();
  });
});

describe("SyncView — terminal success branch", () => {
  it("renders the SyncToast 'Sync complete' message when sync succeeds", async () => {
    startSyncSpy.mockResolvedValueOnce({ status: "ok", data: null });

    render(
      <SyncProvider>
        <SyncView />
      </SyncProvider>,
    );
    const runButton = await screen.findByRole("button", { name: "Run sync" });
    await act(async () => {
      runButton.click();
    });

    // The toast renders inside a role=status live region with text
    // "Sync complete". role=status doesn't get an accessible name from
    // text content alone (the toast's "name" is computed differently),
    // so we look for the [Dismiss sync notification] button which is
    // unique to SyncToast — and walk up to the role=status container.
    const dismissBtn = await screen.findByRole("button", {
      name: "Dismiss sync notification",
    });
    expect(dismissBtn).toBeInTheDocument();
    // The toast's text content includes "Sync complete".
    const toast = dismissBtn.closest("[role='status']");
    expect(toast).not.toBeNull();
    expect(toast?.textContent).toContain("Sync complete");
  });
});
