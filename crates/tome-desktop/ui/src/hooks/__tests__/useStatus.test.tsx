import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { listeners, getStatus } = vi.hoisted(() => ({
  listeners: {
    manifestChanged: vi.fn(),
    lockfileChanged: vi.fn(),
    libraryChanged: vi.fn(),
    machinePrefsChanged: vi.fn(),
    poolPolicyChanged: vi.fn(),
    profilesChanged: vi.fn(),
    localSettingsChanged: vi.fn(),
  },
  getStatus: vi.fn(),
}));

vi.mock("../../bindings", () => ({
  commands: { getStatus },
  events: Object.fromEntries(
    Object.entries(listeners).map(([name, listen]) => [name, {
      listen: (callback: () => void) => {
        listen(callback);
        return Promise.resolve(() => undefined);
      },
    }]),
  ),
}));

import { useStatus } from "../useStatus";

const status = {
  configured: true,
  tome_home: "/pool",
  library_dir: "/pool/skills",
  library_count: { count: 0, error: null },
  last_sync: null,
  directories: [],
  unowned: [],
  lockfile: { kind: "missing" as const },
  machine_prefs_summary: { disabled_count: 0, disabled_directory_count: 0 },
  health: { count: 0, error: null },
  profile: { kind: "available" as const, name: "laptop" },
  git: { kind: "unavailable" as const, reason: "not a Git repository" },
};

describe("useStatus layered configuration refresh", () => {
  beforeEach(() => {
    Object.values(listeners).forEach((listener) => listener.mockReset());
    getStatus.mockReset();
    getStatus.mockResolvedValue({ status: "ok", data: status });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it.each([
    ["pool policy", "poolPolicyChanged"],
    ["profiles", "profilesChanged"],
    ["local settings", "localSettingsChanged"],
  ] as const)("refetches status when %s change", async (_label, eventName) => {
    renderHook(() => useStatus());

    await waitFor(() => expect(getStatus).toHaveBeenCalledTimes(1));
    const callback = listeners[eventName].mock.calls[0]?.[0];
    expect(callback).toBeTypeOf("function");

    await act(async () => callback());
    await waitFor(() => expect(getStatus).toHaveBeenCalledTimes(2));
  });
});
