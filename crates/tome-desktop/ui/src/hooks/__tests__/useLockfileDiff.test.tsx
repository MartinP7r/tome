// useLockfileDiff tests — Phase 27 plan 27-02.
//
// Pins:
// 1. Mount fires a single fetch via commands.getLockfileDiff.
// 2. lockfileChanged event triggers a refetch — but ONLY when
//    isRunningRef.current is false (Pitfall 6 watcher-feedback
//    discipline).
// 3. Hook does NOT subscribe to manifestChanged, libraryChanged, or
//    machinePrefsChanged (only lockfileChanged drives the diff).

import { act, render } from "@testing-library/react";
import { useRef } from "react";
import { describe, it, expect, vi, beforeEach } from "vitest";

const listenSpies = {
  syncProgress: vi.fn(),
  manifestChanged: vi.fn(),
  lockfileChanged: vi.fn(),
  libraryChanged: vi.fn(),
  machinePrefsChanged: vi.fn(),
  menuAction: vi.fn(),
};

// Captures the listen callbacks so tests can fire events manually.
const listenCallbacks: {
  lockfileChanged: Array<() => void>;
} = { lockfileChanged: [] };

const getLockfileDiffSpy = vi.fn();

vi.mock("../../bindings", () => ({
  events: {
    syncProgress: {
      listen: (cb: () => void) => {
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
        listenCallbacks.lockfileChanged.push(cb);
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
    getLockfileDiff: () => getLockfileDiffSpy(),
  },
}));

// Import after vi.mock.
import { useLockfileDiff } from "../useLockfileDiff";

function Probe({ isRunning }: { isRunning: boolean }) {
  const ref = useRef(isRunning);
  // Reflect new isRunning into the ref every render so tests can flip it.
  ref.current = isRunning;
  const { diff, err } = useLockfileDiff(ref);
  return (
    <div>
      <span data-testid="diff-kind">
        {diff === null ? "null" : "loaded"}
      </span>
      <span data-testid="err-kind">{err === null ? "null" : "err"}</span>
    </div>
  );
}

describe("useLockfileDiff — mount + Pitfall 6 discipline", () => {
  beforeEach(() => {
    listenSpies.syncProgress.mockReset();
    listenSpies.manifestChanged.mockReset();
    listenSpies.lockfileChanged.mockReset();
    listenSpies.libraryChanged.mockReset();
    listenSpies.machinePrefsChanged.mockReset();
    listenSpies.menuAction.mockReset();
    listenCallbacks.lockfileChanged = [];
    getLockfileDiffSpy.mockReset();
    getLockfileDiffSpy.mockResolvedValue({
      status: "ok",
      data: { added: [], changed: [], removed: [] },
    });
  });

  it("fetches once on mount via commands.getLockfileDiff", async () => {
    await act(async () => {
      render(<Probe isRunning={false} />);
    });
    expect(getLockfileDiffSpy).toHaveBeenCalledTimes(1);
  });

  it("subscribes ONLY to events.lockfileChanged (not manifest / library / machine-prefs)", () => {
    render(<Probe isRunning={false} />);
    expect(listenSpies.lockfileChanged).toHaveBeenCalledTimes(1);
    // Pitfall 6 / event-subscription discipline: the diff hook DOES NOT
    // observe these. Manifest/library/machine-prefs writes that don't
    // cross into the lockfile have no effect on the triage panel.
    expect(listenSpies.manifestChanged).not.toHaveBeenCalled();
    expect(listenSpies.libraryChanged).not.toHaveBeenCalled();
    expect(listenSpies.machinePrefsChanged).not.toHaveBeenCalled();
    expect(listenSpies.syncProgress).not.toHaveBeenCalled();
  });

  it("refetches when lockfileChanged fires AND isRunningRef.current is false", async () => {
    await act(async () => {
      render(<Probe isRunning={false} />);
    });
    expect(getLockfileDiffSpy).toHaveBeenCalledTimes(1);

    // Simulate the watcher firing a lockfileChanged event.
    await act(async () => {
      for (const cb of listenCallbacks.lockfileChanged) {
        cb();
      }
    });
    expect(getLockfileDiffSpy).toHaveBeenCalledTimes(2);
  });

  it("does NOT refetch when lockfileChanged fires AND isRunningRef.current is true (Pitfall 6)", async () => {
    await act(async () => {
      render(<Probe isRunning={true} />);
    });
    expect(getLockfileDiffSpy).toHaveBeenCalledTimes(1);

    // Simulate the watcher firing while the sync is running. The hook
    // MUST suppress this refetch — otherwise the Save-stage lockfile
    // rewrite would cascade into a feedback loop mid-sync.
    await act(async () => {
      for (const cb of listenCallbacks.lockfileChanged) {
        cb();
      }
    });
    expect(getLockfileDiffSpy).toHaveBeenCalledTimes(1);
  });
});
