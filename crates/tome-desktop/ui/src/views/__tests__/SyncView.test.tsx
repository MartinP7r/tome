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
const retrySyncFromSpy = vi.fn();
const retryFailedItemsSpy = vi.fn();
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
    retrySyncFrom: (stage: string) => retrySyncFromSpy(stage),
    retryFailedItems: (failures: unknown) => retryFailedItemsSpy(failures),
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
  retrySyncFromSpy.mockReset();
  retryFailedItemsSpy.mockReset();
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
    // Plan 27-05: startSync now returns SyncOutcomeWire on success.
    startSyncSpy.mockResolvedValueOnce({
      status: "ok",
      data: { result: null, retry_from: null, partial_failures: [] },
    });

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

describe("SyncView — terminal failed branch (Plan 27-05)", () => {
  it("renders the 'Sync failed' summary + [Retry from <stage>] + [Dismiss] when retry_from is set", async () => {
    startSyncSpy.mockResolvedValueOnce({
      status: "ok",
      data: {
        result: {
          code: "Permission",
          message: "consolidate failed",
          context: ["permission denied"],
        },
        retry_from: "Discover",
        partial_failures: [],
      },
    });

    render(
      <SyncProvider>
        <SyncView />
      </SyncProvider>,
    );
    const runButton = await screen.findByRole("button", { name: "Run sync" });
    await act(async () => {
      runButton.click();
    });

    // Heading shape per UI-SPEC §Terminal failed.
    expect(
      screen.getByRole("heading", { level: 1, name: "Sync failed" }),
    ).toBeInTheDocument();
    // The structured error code is surfaced inline.
    expect(screen.getByText(/\[Permission\]/)).toBeInTheDocument();
    // [Retry from Discover] surfaces both in the summary block AND in
    // the stepper's trailing action row per UI-SPEC §StageStepper.
    expect(
      screen.getAllByRole("button", { name: "Retry from Discover" }).length,
    ).toBeGreaterThanOrEqual(1);
    // Dismiss is always available as a fallback.
    expect(
      screen.getAllByRole("button", { name: "Dismiss sync summary" }).length,
    ).toBeGreaterThanOrEqual(1);
  });

  it("renders ONLY [Dismiss] when retry_from is null (Save-failure shape)", async () => {
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

    render(
      <SyncProvider>
        <SyncView />
      </SyncProvider>,
    );
    const runButton = await screen.findByRole("button", { name: "Run sync" });
    await act(async () => {
      runButton.click();
    });

    expect(
      screen.getByRole("heading", { level: 1, name: "Sync failed" }),
    ).toBeInTheDocument();
    // No retry-from button should render.
    expect(
      screen.queryByRole("button", { name: /Retry from/ }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Dismiss sync summary" }),
    ).toBeInTheDocument();
  });

  it("clicking [Retry from Discover] invokes commands.retrySyncFrom('Discover')", async () => {
    startSyncSpy.mockResolvedValueOnce({
      status: "ok",
      data: {
        result: {
          code: "Permission",
          message: "consolidate failed",
          context: [],
        },
        retry_from: "Discover",
        partial_failures: [],
      },
    });
    retrySyncFromSpy.mockResolvedValueOnce({
      status: "ok",
      data: { result: null, retry_from: null, partial_failures: [] },
    });

    render(
      <SyncProvider>
        <SyncView />
      </SyncProvider>,
    );
    const runButton = await screen.findByRole("button", { name: "Run sync" });
    await act(async () => {
      runButton.click();
    });
    await act(async () => {
      // Two buttons surface (summary block + stepper trailing slot);
      // clicking the first one is enough to exercise the wire.
      screen
        .getAllByRole("button", { name: "Retry from Discover" })[0]!
        .click();
    });

    expect(retrySyncFromSpy).toHaveBeenCalledTimes(1);
    expect(retrySyncFromSpy).toHaveBeenCalledWith("Discover");
  });
});

describe("SyncView — terminal partial branch (Plan 27-05)", () => {
  it("renders 'Sync complete with K issues' summary + [Retry failed items] + [Dismiss]", async () => {
    // To classify as 'partial' the stages Map needs a complete row
    // carrying the partial failure. Drive a Distribute Start + Finish
    // event before resolving the outcome so finalizeOutcome populates
    // the existing complete row.
    let resolveStart: (v: unknown) => void = () => undefined;
    startSyncSpy.mockReturnValueOnce(
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
              skill: "foo",
              error: {
                code: "Internal",
                message: "permission denied",
                context: ["permission denied"],
              },
            },
            {
              stage: "Distribute",
              operation: "Distribution",
              skill: "bar",
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

    // Heading + sub-line match UI-SPEC §Terminal partial.
    expect(
      screen.getByRole("heading", {
        level: 1,
        name: "Sync complete with 2 issues",
      }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/2 individual operations failed/),
    ).toBeInTheDocument();
    // [Retry failed items] surfaces both in the summary block AND in the
    // stepper's trailing action row (the stepper renders it whenever
    // onRetryFailedItems is wired). Both are intentional per UI-SPEC
    // (primary affordance + redundant convenience).
    expect(
      screen.getAllByRole("button", { name: "Retry failed items" }).length,
    ).toBeGreaterThanOrEqual(1);
    expect(
      screen.getAllByRole("button", { name: "Dismiss sync summary" }).length,
    ).toBeGreaterThanOrEqual(1);
  });
});
