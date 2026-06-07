// useSync triage-state tests — Phase 27 plan 27-02.
//
// Pins (additive to useSync.test.tsx — kept in a sibling file so the
// 27-01b discipline tests stay narrow):
// 1. diff + pendingDiffCount populate from the mocked
//    commands.getLockfileDiff response.
// 2. decisions seed to "keep" for every Added + Changed entry on first
//    diff load.
// 3. onDecisionChange mutates the decisions map.
// 4. onBulkAction applies to all NEW entries (section scope) or only the
//    matching source-group (source-group scope).
// 5. pendingDecisionCount tracks non-default decisions across Added +
//    Changed (Removed is implicit per D-13).

import { act, render } from "@testing-library/react";
import { useEffect } from "react";
import { describe, it, expect, vi, beforeEach } from "vitest";

const listenSpies = {
  syncProgress: vi.fn(),
  manifestChanged: vi.fn(),
  lockfileChanged: vi.fn(),
  libraryChanged: vi.fn(),
  machinePrefsChanged: vi.fn(),
  menuAction: vi.fn(),
};

const getLockfileDiffSpy = vi.fn();
const startSyncSpy = vi.fn();
const cancelSyncSpy = vi.fn();

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

import { SyncProvider, useSync } from "../useSync";

function makeEntry(name: string, source: string | null, kind: "added" | "changed" | "removed") {
  return {
    name,
    change_kind: kind,
    source_name: source,
    previous_source: null,
    origin: { kind: "local" as const },
    content_hash_old: kind === "added" ? null : "a".repeat(64),
    content_hash_new: kind === "removed" ? null : "b".repeat(64),
    registry_id: null,
    version_old: null,
    version_new: null,
    git_commit_sha_old: null,
    git_commit_sha_new: null,
    synced_at: null,
  };
}

/** Captures the most recent useSync() snapshot so tests can assert on
 *  state derived from the React tree. */
function StateCapture({
  onState,
}: {
  onState: (api: ReturnType<typeof useSync>) => void;
}) {
  const sync = useSync();
  useEffect(() => {
    onState(sync);
  });
  return null;
}

describe("useSync — triage state (Plan 27-02)", () => {
  beforeEach(() => {
    listenSpies.syncProgress.mockReset();
    listenSpies.manifestChanged.mockReset();
    listenSpies.lockfileChanged.mockReset();
    listenSpies.libraryChanged.mockReset();
    listenSpies.machinePrefsChanged.mockReset();
    listenSpies.menuAction.mockReset();
    getLockfileDiffSpy.mockReset();
    startSyncSpy.mockReset();
    cancelSyncSpy.mockReset();
    // Plan 27-05: startSync now returns a SyncOutcomeWire on success.
    startSyncSpy.mockResolvedValue({
      status: "ok",
      data: { result: null, retry_from: null, partial_failures: [] },
    });
    cancelSyncSpy.mockResolvedValue({ status: "ok", data: null });
  });

  it("populates diff + pendingDiffCount from getLockfileDiff response", async () => {
    const diffPayload = {
      added: [makeEntry("alpha", "plugins", "added")],
      changed: [makeEntry("beta", "plugins", "changed")],
      removed: [makeEntry("gamma", "plugins", "removed")],
    };
    getLockfileDiffSpy.mockResolvedValue({ status: "ok", data: diffPayload });

    let latest: ReturnType<typeof useSync> | null = null;
    await act(async () => {
      render(
        <SyncProvider>
          <StateCapture onState={(s) => (latest = s)} />
        </SyncProvider>,
      );
    });
    // After the mount-time fetch resolves, diff is non-null.
    expect(latest).not.toBeNull();
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    expect(latest!.diff).not.toBeNull();
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    expect(latest!.pendingDiffCount).toBe(3);
  });

  it("seeds decisions to 'keep' for every Added + Changed entry (not Removed)", async () => {
    const diffPayload = {
      added: [makeEntry("alpha", "plugins", "added")],
      changed: [makeEntry("beta", "plugins", "changed")],
      removed: [makeEntry("gamma", "plugins", "removed")],
    };
    getLockfileDiffSpy.mockResolvedValue({ status: "ok", data: diffPayload });

    let latest: ReturnType<typeof useSync> | null = null;
    await act(async () => {
      render(
        <SyncProvider>
          <StateCapture onState={(s) => (latest = s)} />
        </SyncProvider>,
      );
    });
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const dec = latest!.decisions;
    expect(dec.get("alpha")).toBe("keep");
    expect(dec.get("beta")).toBe("keep");
    // Removed entries are not seeded (implicit per D-13).
    expect(dec.has("gamma")).toBe(false);
  });

  it("pendingDecisionCount counts Added + Changed with decision !== 'keep'", async () => {
    const diffPayload = {
      added: [
        makeEntry("alpha", "plugins", "added"),
        makeEntry("beta", "plugins", "added"),
      ],
      changed: [makeEntry("gamma", "plugins", "changed")],
      removed: [],
    };
    getLockfileDiffSpy.mockResolvedValue({ status: "ok", data: diffPayload });

    let latest: ReturnType<typeof useSync> | null = null;
    await act(async () => {
      render(
        <SyncProvider>
          <StateCapture onState={(s) => (latest = s)} />
        </SyncProvider>,
      );
    });
    // All seeded to 'keep' — pendingDecisionCount = 0.
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    expect(latest!.pendingDecisionCount).toBe(0);

    // Flip alpha → disable.
    await act(async () => {
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      latest!.onDecisionChange("alpha", "disable");
    });
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    expect(latest!.pendingDecisionCount).toBe(1);
  });

  it("onBulkAction with section scope applies decision to all NEW entries", async () => {
    const diffPayload = {
      added: [
        makeEntry("alpha", "plugins", "added"),
        makeEntry("beta", "my-repo", "added"),
      ],
      changed: [makeEntry("gamma", "plugins", "changed")],
      removed: [],
    };
    getLockfileDiffSpy.mockResolvedValue({ status: "ok", data: diffPayload });

    let latest: ReturnType<typeof useSync> | null = null;
    await act(async () => {
      render(
        <SyncProvider>
          <StateCapture onState={(s) => (latest = s)} />
        </SyncProvider>,
      );
    });
    await act(async () => {
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      latest!.onBulkAction({ kind: "section", section: "new" }, "disable");
    });
    // Both NEW entries are now disabled; Changed entry untouched.
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    expect(latest!.decisions.get("alpha")).toBe("disable");
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    expect(latest!.decisions.get("beta")).toBe("disable");
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    expect(latest!.decisions.get("gamma")).toBe("keep");
  });

  it("onBulkAction with source-group scope applies only to matching source", async () => {
    const diffPayload = {
      added: [
        makeEntry("alpha", "plugins", "added"),
        makeEntry("beta", "my-repo", "added"),
      ],
      changed: [],
      removed: [],
    };
    getLockfileDiffSpy.mockResolvedValue({ status: "ok", data: diffPayload });

    let latest: ReturnType<typeof useSync> | null = null;
    await act(async () => {
      render(
        <SyncProvider>
          <StateCapture onState={(s) => (latest = s)} />
        </SyncProvider>,
      );
    });
    await act(async () => {
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      latest!.onBulkAction(
        { kind: "source-group", section: "new", source: "plugins" },
        "disable",
      );
    });
    // alpha (plugins) flipped; beta (my-repo) unchanged.
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    expect(latest!.decisions.get("alpha")).toBe("disable");
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    expect(latest!.decisions.get("beta")).toBe("keep");
  });
});
