import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { StatusReport_Serialize } from "../../bindings";
import { StatusView } from "../StatusView";

const status: StatusReport_Serialize = {
  configured: true,
  tome_home: "/portable/tome",
  library_dir: "/external/skills",
  library_count: { count: 2, error: null },
  last_sync: null,
  directories: [],
  unowned: [],
  lockfile: { kind: "missing" },
  machine_prefs_summary: {
    disabled_count: 0,
    disabled_directory_count: 0,
  },
  health: { count: 0, error: null },
};

vi.mock("../../hooks/useStatus", () => ({
  useStatus: () => ({
    status,
    err: null,
    updatedAt: null,
    refetch: vi.fn(),
  }),
}));

describe("StatusView", () => {
  it("renders canonical Tome data and library folders independently", () => {
    render(<StatusView />);

    expect(screen.getByText("TOME DATA FOLDER")).toBeInTheDocument();
    expect(screen.getByText("/portable/tome")).toBeInTheDocument();
    expect(screen.getByText("LIBRARY")).toBeInTheDocument();
    expect(screen.getByText("/external/skills")).toBeInTheDocument();
    expect(
      screen.getByText(/machine settings live in ~\/\.config\/tome/i),
    ).toBeInTheDocument();
    expect(screen.queryByText("TOME HOME")).not.toBeInTheDocument();
  });
});
