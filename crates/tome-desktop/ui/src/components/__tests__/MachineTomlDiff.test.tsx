// MachineTomlDiff tests — Phase 27 plan 27-03 / SYNC-03.
//
// Pins:
// 1. Header summary surfaces the additions/removals totals.
// 2. Each diff line renders as a row with the kind discriminator on the
//    container (`.row--removed` / `.row--added` / `.row--unchanged`).
// 3. Removed + added rows carry an aria-label naming the change kind +
//    line number; unchanged rows are aria-hidden (UI-SPEC §VoiceOver labels —
//    reduce screen-reader noise on equal lines).
// 4. The container is a `<table role="table">` with an aria-label including
//    the additions/removals totals.

import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import { MachineTomlDiff } from "../MachineTomlDiff";
import type { MachineTomlPreview } from "../../bindings";

function previewWithLines(): MachineTomlPreview {
  return {
    lines: [
      { kind: "unchanged", line_number: 12, content: "[machine_prefs]" },
      { kind: "removed", line_number: 13, content: 'enabled = []' },
      { kind: "added", line_number: 13, content: 'enabled = ["foo"]' },
    ],
    added_count: 1,
    removed_count: 1,
  };
}

describe("MachineTomlDiff — header summary", () => {
  it("surfaces the additions/removals counts in the header", () => {
    render(<MachineTomlDiff preview={previewWithLines()} />);
    // Header is a heading or a header element with the counts. Match
    // textContent so "1 addition, 1 removal" rendering is robust to
    // copywriting tweaks within the spec.
    const header = screen.getByText(/1 addition/i);
    expect(header).toBeInTheDocument();
    expect(header.textContent ?? "").toMatch(/1 removal/i);
  });
});

describe("MachineTomlDiff — table semantics", () => {
  it("renders a table with an aria-label that includes the totals", () => {
    render(<MachineTomlDiff preview={previewWithLines()} />);
    const table = screen.getByRole("table");
    // The aria-label should mention the additions/removals counts so
    // VoiceOver announces the table size on focus.
    const label = table.getAttribute("aria-label") ?? "";
    expect(label).toMatch(/1 addition/i);
    expect(label).toMatch(/1 removal/i);
    expect(label.toLowerCase()).toContain("machine.toml");
  });

  it("emits a row per DiffLine", () => {
    render(<MachineTomlDiff preview={previewWithLines()} />);
    const rows = screen.getAllByRole("row");
    expect(rows.length).toBe(3);
  });
});

describe("MachineTomlDiff — accessibility per line kind", () => {
  it("labels the removed row with 'removed line 13'", () => {
    render(<MachineTomlDiff preview={previewWithLines()} />);
    const row = screen.getByLabelText(/removed line 13/i);
    expect(row).toBeInTheDocument();
  });

  it("labels the added row with 'added line 13'", () => {
    render(<MachineTomlDiff preview={previewWithLines()} />);
    const row = screen.getByLabelText(/added line 13/i);
    expect(row).toBeInTheDocument();
  });

  it("hides the unchanged row from the accessibility tree (aria-hidden)", () => {
    // Unchanged lines carry aria-hidden so VoiceOver doesn't read every
    // equal line of a multi-page TOML; the visual content still renders.
    render(<MachineTomlDiff preview={previewWithLines()} />);
    // Find the unchanged row by its content + aria-hidden attribute.
    const cells = screen.getAllByText("[machine_prefs]");
    // At least one ancestor row should have aria-hidden="true".
    const hidden = cells.some((cell) => {
      let cur: HTMLElement | null = cell;
      while (cur) {
        if (cur.getAttribute("aria-hidden") === "true") return true;
        cur = cur.parentElement;
      }
      return false;
    });
    expect(hidden).toBe(true);
  });
});

describe("MachineTomlDiff — empty / no-op preview", () => {
  it("renders cleanly when there are zero changes", () => {
    const preview: MachineTomlPreview = {
      lines: [
        { kind: "unchanged", line_number: 1, content: "[machine_prefs]" },
      ],
      added_count: 0,
      removed_count: 0,
    };
    render(<MachineTomlDiff preview={preview} />);
    // Header summarises 0 additions and 0 removals.
    const header = screen.getByText(/0 addition/i);
    expect(header.textContent ?? "").toMatch(/0 removal/i);
  });
});
