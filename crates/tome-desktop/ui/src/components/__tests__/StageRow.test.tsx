// StageRow tests — Phase 27 plan 27-04 / UI-SPEC §StageRow.
//
// Pin every variant's rendered shape (icon glyph + label semantics +
// trailing content) AND the VoiceOver label template per UI-SPEC
// §VoiceOver labels.

import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import { StageRow, type StageStatus } from "../StageRow";
import type { TomeError } from "../../bindings";

function err(code = "Generic", message = "boom"): TomeError {
  return { code: code as TomeError["code"], message, context: [] };
}

describe("StageRow — pending variant", () => {
  it("renders outline circle icon + '—' trailing", () => {
    render(
      <StageRow
        stage="Reconcile"
        label="Reconcile"
        status={{ kind: "pending" }}
      />,
    );
    const row = screen.getByRole("listitem", {
      name: "Reconcile stage, pending",
    });
    expect(row).toBeInTheDocument();
    expect(row.textContent).toContain("Reconcile");
    expect(row.textContent).toContain("—");
  });
});

describe("StageRow — active variant", () => {
  it("renders the currentItem subtitle + progress bar when total > 0", () => {
    const status: StageStatus = {
      kind: "active",
      currentItem: "axiom-build",
      current: 47,
      total: 120,
    };
    render(<StageRow stage="Consolidate" label="Consolidate" status={status} />);
    const row = screen.getByRole("listitem", {
      name: "Consolidate stage, running, axiom-build, 47 of 120",
    });
    expect(row).toBeInTheDocument();
    expect(row.textContent).toContain("axiom-build");
    expect(row.textContent).toContain("47/120");
    expect(row.textContent).toContain("running…");
  });

  it("omits the progress bar when total === 0 (D-09 git-clone case)", () => {
    const status: StageStatus = {
      kind: "active",
      currentItem: "git: my-repo (4.2 MiB)",
      current: 0,
      total: 0,
    };
    render(<StageRow stage="Reconcile" label="Reconcile" status={status} />);
    const row = screen.getByRole("listitem", {
      name: "Reconcile stage, running, git: my-repo (4.2 MiB)",
    });
    expect(row).toBeInTheDocument();
    expect(row.textContent).toContain("git: my-repo (4.2 MiB)");
    // No "47/120" style content because total === 0.
    expect(row.textContent).not.toMatch(/\d+\/\d+/);
  });

  it("collapses the subtitle line when currentItem === null", () => {
    const status: StageStatus = {
      kind: "active",
      currentItem: null,
      current: 0,
      total: 0,
    };
    render(<StageRow stage="Discover" label="Discover" status={status} />);
    const row = screen.getByRole("listitem", {
      name: "Discover stage, running, preparing",
    });
    expect(row).toBeInTheDocument();
  });
});

describe("StageRow — complete variant", () => {
  it("renders ✓ icon + duration text in trailing", () => {
    render(
      <StageRow
        stage="Discover"
        label="Discover"
        status={{ kind: "complete", durationMs: 8200, partialFailures: [] }}
      />,
    );
    const row = screen.getByRole("listitem", {
      name: "Discover stage, complete in 8.2s",
    });
    expect(row).toBeInTheDocument();
    expect(row.textContent).toContain("8.2s");
    expect(row.textContent).toContain("✓");
  });

  it("renders a [⚠ K issues] badge + FindingRow list when partialFailures > 0 (D-20)", () => {
    render(
      <StageRow
        stage="Distribute"
        label="Distribute"
        status={{
          kind: "complete",
          durationMs: 1100,
          partialFailures: [
            { itemName: "alpha", error: err("Generic", "write failed") },
            { itemName: "beta", error: err("Generic", "write failed") },
          ],
        }}
      />,
    );
    const row = screen.getByRole("listitem", {
      name: "Distribute stage, complete in 1.1s, 2 issues",
    });
    expect(row).toBeInTheDocument();
    expect(row.textContent).toContain("⚠");
    expect(row.textContent).toContain("alpha");
    expect(row.textContent).toContain("beta");
  });

  it("renders NO issues badge when partialFailures is empty", () => {
    render(
      <StageRow
        stage="Distribute"
        label="Distribute"
        status={{ kind: "complete", durationMs: 300, partialFailures: [] }}
      />,
    );
    const row = screen.getByRole("listitem", {
      name: "Distribute stage, complete in 0.3s",
    });
    expect(row).toBeInTheDocument();
    expect(row.textContent).not.toContain("⚠");
    expect(row.textContent).not.toContain("issues");
  });
});

describe("StageRow — failed variant", () => {
  it("renders ! icon + inline [ErrorCode] message", () => {
    render(
      <StageRow
        stage="Distribute"
        label="Distribute"
        status={{
          kind: "failed",
          durationMs: 1100,
          error: err("Conflict", "two skills named foo"),
        }}
      />,
    );
    const row = screen.getByRole("listitem", {
      name: "Distribute stage, failed in 1.1s, Conflict, two skills named foo",
    });
    expect(row).toBeInTheDocument();
    expect(row.textContent).toContain("[Conflict]");
    expect(row.textContent).toContain("two skills named foo");
  });

  it("renders a Show error chain disclosure when context is non-empty", () => {
    const errorWithContext: TomeError = {
      code: "Generic" as TomeError["code"],
      message: "outer",
      context: ["middle", "innermost"],
    };
    render(
      <StageRow
        stage="Distribute"
        label="Distribute"
        status={{
          kind: "failed",
          durationMs: 200,
          error: errorWithContext,
        }}
      />,
    );
    const summary = screen.getByText("Show error chain");
    expect(summary).toBeInTheDocument();
    // The <ul> renders both context entries:
    expect(screen.getByText("middle")).toBeInTheDocument();
    expect(screen.getByText("innermost")).toBeInTheDocument();
  });
});

describe("StageRow — cancelled variant", () => {
  it("renders ⊘ icon + 'cancelled' trailing", () => {
    render(
      <StageRow
        stage="Distribute"
        label="Distribute"
        status={{ kind: "cancelled" }}
      />,
    );
    const row = screen.getByRole("listitem", {
      name: "Distribute stage, cancelled",
    });
    expect(row).toBeInTheDocument();
    expect(row.textContent).toContain("⊘");
    expect(row.textContent).toContain("cancelled");
  });
});
