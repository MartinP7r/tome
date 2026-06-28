// Sidebar tests — Phase 27 plan 27-01b.
//
// Pin the 4-NavItem render order (Status → Skills → Sync → Health) and the
// Sync row's spinner + dual-meaning badge slots. The aria-label flavors for
// the Sync row are spec-fixed (UI-SPEC §VoiceOver labels); we pin each
// permutation here so a future copy edit goes through the spec, not a
// silent component change.

import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";

// The Sidebar reads `useStatus` for the footer; stub the bindings so the
// test doesn't try to invoke a real Tauri command. The status payload only
// needs `library_count.count` for the footer text.
vi.mock("../../bindings", () => ({
  commands: {
    getStatus: () =>
      Promise.resolve({
        status: "ok",
        data: {
          library_count: { count: 3, error: null },
        },
      }),
  },
  events: {
    manifestChanged: { listen: () => Promise.resolve(() => undefined) },
    lockfileChanged: { listen: () => Promise.resolve(() => undefined) },
    libraryChanged: { listen: () => Promise.resolve(() => undefined) },
    machinePrefsChanged: { listen: () => Promise.resolve(() => undefined) },
  },
}));

import { Sidebar } from "../Sidebar";

describe("Sidebar — Phase 27 plan 27-01b", () => {
  it("renders the four NavItems in order Status, Skills, Sync, Health", () => {
    render(<Sidebar selected="status" onChange={() => undefined} />);
    const items = screen.getAllByRole("option");
    expect(items).toHaveLength(4);
    // React Aria mangles the `id` prop (it scopes to the ListBox), but
    // preserves the literal `textValue` we pass on each `<ListBoxItem>`.
    // The pin we care about is render order; matching textContent is the
    // user-visible contract.
    expect(items[0].textContent).toContain("Status");
    expect(items[1].textContent).toContain("Skills");
    expect(items[2].textContent).toContain("Sync");
    expect(items[3].textContent).toContain("Health");
  });

  it("Sync row aria-label reflects idle state when not running and no badge", () => {
    render(<Sidebar selected="status" onChange={() => undefined} />);
    const sync = screen.getByRole("option", { name: "Sync, Sync section" });
    expect(sync).toBeInTheDocument();
  });

  it("Sync row aria-label announces in-progress when syncInProgress is true", () => {
    render(
      <Sidebar
        selected="status"
        onChange={() => undefined}
        syncInProgress={true}
      />,
    );
    expect(
      screen.getByRole("option", {
        name: "Sync, Sync section, sync in progress",
      }),
    ).toBeInTheDocument();
  });

  it("Sync row aria-label announces pending decisions when badge is pending", () => {
    render(
      <Sidebar
        selected="status"
        onChange={() => undefined}
        syncBadge={{ kind: "pending", count: 7 }}
      />,
    );
    expect(
      screen.getByRole("option", {
        name: "Sync, Sync section, 7 pending decisions",
      }),
    ).toBeInTheDocument();
  });

  it("Sync row aria-label announces failures when badge is failures", () => {
    render(
      <Sidebar
        selected="status"
        onChange={() => undefined}
        syncBadge={{ kind: "failures", count: 3 }}
      />,
    );
    expect(
      screen.getByRole("option", {
        name: "Sync, Sync section, 3 failures",
      }),
    ).toBeInTheDocument();
  });

  it("Health row keeps its danger badge wired through badgeCount", () => {
    render(
      <Sidebar
        selected="status"
        onChange={() => undefined}
        badgeCount={2}
      />,
    );
    expect(
      screen.getByRole("option", {
        name: "Health, Health section, 2 health issues",
      }),
    ).toBeInTheDocument();
  });
});
