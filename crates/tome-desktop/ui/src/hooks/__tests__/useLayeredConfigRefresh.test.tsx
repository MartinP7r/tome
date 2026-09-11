import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const { listeners, getDoctorReport, listSkills } = vi.hoisted(() => ({
  listeners: {
    manifestChanged: vi.fn(),
    lockfileChanged: vi.fn(),
    libraryChanged: vi.fn(),
    machinePrefsChanged: vi.fn(),
    poolPolicyChanged: vi.fn(),
    profilesChanged: vi.fn(),
    localSettingsChanged: vi.fn(),
  },
  getDoctorReport: vi.fn(),
  listSkills: vi.fn(),
}));

vi.mock("../../bindings", () => ({
  commands: { getDoctorReport, listSkills },
  events: Object.fromEntries(
    Object.entries(listeners).map(([name, listen]) => [name, {
      listen: (callback: () => void) => {
        listen(callback);
        return Promise.resolve(() => undefined);
      },
    }]),
  ),
}));

import { useDoctorReport } from "../useDoctorReport";
import { useSkills } from "../useSkills";

const layeredEvents = [
  "poolPolicyChanged",
  "profilesChanged",
  "localSettingsChanged",
] as const;

describe("layered configuration refresh consumers", () => {
  beforeEach(() => {
    Object.values(listeners).forEach((listener) => listener.mockReset());
    getDoctorReport.mockReset().mockResolvedValue({ status: "ok", data: { findings: [] } });
    listSkills.mockReset().mockResolvedValue({ status: "ok", data: { skills: [], warnings: [] } });
  });

  it.each(layeredEvents)("refetches skills when %s fires", async (eventName) => {
    renderHook(() => useSkills());

    await waitFor(() => expect(listSkills).toHaveBeenCalledTimes(1));
    const callback = listeners[eventName].mock.calls[0]?.[0];
    expect(callback).toBeTypeOf("function");
    await act(async () => callback());
    await waitFor(() => expect(listSkills).toHaveBeenCalledTimes(2));
  });

  it.each(layeredEvents)("refetches doctor data when %s fires", async (eventName) => {
    renderHook(() => useDoctorReport());

    await waitFor(() => expect(getDoctorReport).toHaveBeenCalledTimes(1));
    const callback = listeners[eventName].mock.calls[0]?.[0];
    expect(callback).toBeTypeOf("function");
    await act(async () => callback());
    await waitFor(() => expect(getDoctorReport).toHaveBeenCalledTimes(2));
  });
});
