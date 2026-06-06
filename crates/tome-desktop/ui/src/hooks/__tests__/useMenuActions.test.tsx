// useMenuActions tests — Phase 27 plan 27-01b.
//
// Pin that the menu event handler routes JumpSync → setView("sync") in the
// same shape JumpStatus / JumpSkills / JumpHealth do. The Rust side is
// covered by the menu module's exhaustiveness sentinel + the click
// dispatcher; the React side is the receiver.

import { render } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";

// Capture the menuAction listener that useMenuActions registers; expose
// a helper that synthesizes an emission.
let menuActionListener: ((evt: { payload: { kind: string } }) => void) | null = null;

vi.mock("../../bindings", () => ({
  events: {
    menuAction: {
      listen: (cb: (evt: { payload: { kind: string } }) => void) => {
        menuActionListener = cb;
        return Promise.resolve(() => undefined);
      },
    },
    // Sync provider also subscribes to syncProgress + (via 27-02
    // useLockfileDiff) lockfileChanged; stub both so the provider
    // mounts cleanly.
    syncProgress: {
      listen: () => Promise.resolve(() => undefined),
    },
    lockfileChanged: {
      listen: () => Promise.resolve(() => undefined),
    },
  },
  commands: {
    startSync: () => Promise.resolve({ status: "ok", data: null }),
    cancelSync: () => Promise.resolve({ status: "ok", data: null }),
    // Plan 27-02 — useSync composes useLockfileDiff which calls
    // commands.getLockfileDiff on mount.
    getLockfileDiff: () =>
      Promise.resolve({
        status: "ok",
        data: { added: [], changed: [], removed: [] },
      }),
  },
}));

// Capture setView calls.
const setViewSpy = vi.fn();
vi.mock("../../stores/router", () => ({
  setView: (view: string) => setViewSpy(view),
}));

import { SyncProvider } from "../useSync";
import { useMenuActions } from "../useMenuActions";

function Host() {
  useMenuActions();
  return null;
}

describe("useMenuActions — Phase 27 plan 27-01b", () => {
  beforeEach(() => {
    setViewSpy.mockReset();
    menuActionListener = null;
  });

  it("registers a menuAction listener on mount", async () => {
    render(
      <SyncProvider>
        <Host />
      </SyncProvider>,
    );
    // The listener registration is a Promise; wait a microtask so the
    // capture closure has the listener stored.
    await Promise.resolve();
    await Promise.resolve();
    expect(menuActionListener).not.toBeNull();
  });

  it("dispatches JumpSync → setView('sync')", async () => {
    render(
      <SyncProvider>
        <Host />
      </SyncProvider>,
    );
    await Promise.resolve();
    await Promise.resolve();

    expect(menuActionListener).not.toBeNull();
    menuActionListener!({ payload: { kind: "JumpSync" } });
    expect(setViewSpy).toHaveBeenCalledWith("sync");
  });

  it("dispatches JumpHealth → setView('health') (Pitfall 7 re-anchor preserved)", async () => {
    render(
      <SyncProvider>
        <Host />
      </SyncProvider>,
    );
    await Promise.resolve();
    await Promise.resolve();

    menuActionListener!({ payload: { kind: "JumpHealth" } });
    expect(setViewSpy).toHaveBeenCalledWith("health");
  });
});
