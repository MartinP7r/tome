// StageStepper tests — Phase 27 plan 27-04 / UI-SPEC §StageStepper.
//
// Pin the rendered shape across four state shapes:
//
//   - all-pending → 6 rows, no [Cancel sync] / [Dismiss] / [Retry]
//                   (idle skeleton)
//   - mid-run     → 6 rows, [Cancel sync] button rendered
//   - terminal cancelled → 6 rows, [Dismiss] (no [Run sync] inside the
//                          stepper — that lives in the parent's summary)
//   - terminal failed-with-retry → [Retry from <stage>] + [Dismiss]
//
// And pin the A11y contract: outer wrapper has role=status aria-live=polite,
// inner list has role=list aria-label="Sync pipeline progress", every row
// is role=listitem.

import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { StageStepper, type StageState } from "../StageStepper";
import type { TomeError } from "../../bindings";

const STAGES = ["Reconcile", "Discover", "Consolidate", "Distribute", "Cleanup", "Save"] as const;

function pendingStages(): StageState[] {
  return STAGES.map((stage) => ({
    stage,
    label: stage,
    status: { kind: "pending" },
  }));
}

function err(code = "Generic", message = "boom"): TomeError {
  return { code: code as TomeError["code"], message, context: [] };
}

describe("StageStepper — A11y contract", () => {
  it("wraps the rows in role=list aria-label='Sync pipeline progress'", () => {
    render(<StageStepper stages={pendingStages()} />);
    const list = screen.getByRole("list", {
      name: "Sync pipeline progress",
    });
    expect(list).toBeInTheDocument();
  });

  it("renders exactly 6 rows in pipeline order", () => {
    render(<StageStepper stages={pendingStages()} />);
    const rows = screen.getAllByRole("listitem");
    expect(rows).toHaveLength(6);
    expect(rows[0].textContent).toContain("Reconcile");
    expect(rows[1].textContent).toContain("Discover");
    expect(rows[2].textContent).toContain("Consolidate");
    expect(rows[3].textContent).toContain("Distribute");
    expect(rows[4].textContent).toContain("Cleanup");
    expect(rows[5].textContent).toContain("Save");
  });

  it("the outer wrapper is role=status aria-live=polite", () => {
    render(<StageStepper stages={pendingStages()} />);
    const status = screen.getByRole("status");
    expect(status).toHaveAttribute("aria-live", "polite");
    expect(status).toHaveAttribute("aria-busy", "false");
  });
});

describe("StageStepper — idle / all-pending (no handlers)", () => {
  it("renders no action buttons when no handlers are provided", () => {
    // Idle path: SyncView doesn't mount the stepper at all when idle,
    // so 'all pending without any handlers' is the contract we pin
    // here — passing no onCancel / no onDismiss yields no buttons.
    render(<StageStepper stages={pendingStages()} />);
    expect(
      screen.queryByRole("button", { name: /Cancel sync/i }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /Dismiss/i }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /Retry/i }),
    ).not.toBeInTheDocument();
  });

  it("renders [Cancel sync] when onCancel is provided (the SyncView in-progress path before the first event)", () => {
    // SyncView passes onCancel while isRunning is true. The very first
    // moment after [Run sync] but BEFORE the first SyncStageStarted{Reconcile}
    // event arrives has stages all-pending — but the cancel affordance
    // must still appear (D-17 "always visible during the pipeline run").
    const onCancel = vi.fn();
    render(<StageStepper stages={pendingStages()} onCancel={onCancel} />);
    const cancel = screen.getByRole("button", {
      name: "Cancel sync at next stage boundary",
    });
    cancel.click();
    expect(onCancel).toHaveBeenCalledTimes(1);
  });
});

describe("StageStepper — mid-run / active", () => {
  it("renders [Cancel sync] when at least one stage is active", () => {
    const stages = pendingStages();
    stages[2] = {
      stage: "Consolidate",
      label: "Consolidate",
      status: {
        kind: "active",
        currentItem: "axiom-build",
        current: 47,
        total: 120,
      },
    };
    const onCancel = vi.fn();
    render(<StageStepper stages={stages} onCancel={onCancel} />);
    const cancel = screen.getByRole("button", {
      name: "Cancel sync at next stage boundary",
    });
    cancel.click();
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("aria-busy is true while any stage is active", () => {
    const stages = pendingStages();
    stages[1] = {
      stage: "Discover",
      label: "Discover",
      status: { kind: "active", currentItem: null, current: 0, total: 0 },
    };
    render(<StageStepper stages={stages} onCancel={vi.fn()} />);
    expect(screen.getByRole("status")).toHaveAttribute("aria-busy", "true");
  });
});

describe("StageStepper — terminal cancelled", () => {
  it("renders [Dismiss] but NOT [Cancel sync] when cancellation is terminal", () => {
    const stages = pendingStages();
    stages[0] = {
      stage: "Reconcile",
      label: "Reconcile",
      status: { kind: "complete", durationMs: 300, partialFailures: [] },
    };
    // Discover ran for a bit then got cancelled
    stages[1] = {
      stage: "Discover",
      label: "Discover",
      status: { kind: "cancelled" },
    };
    stages[2] = { stage: "Consolidate", label: "Consolidate", status: { kind: "cancelled" } };
    stages[3] = { stage: "Distribute", label: "Distribute", status: { kind: "cancelled" } };
    stages[4] = { stage: "Cleanup", label: "Cleanup", status: { kind: "cancelled" } };
    stages[5] = { stage: "Save", label: "Save", status: { kind: "cancelled" } };

    const onDismiss = vi.fn();
    render(<StageStepper stages={stages} onDismiss={onDismiss} />);

    expect(
      screen.queryByRole("button", { name: /Cancel sync/i }),
    ).not.toBeInTheDocument();
    const dismiss = screen.getByRole("button", {
      name: "Dismiss sync summary",
    });
    dismiss.click();
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });
});

describe("StageStepper — terminal failed with retry", () => {
  it("renders [Retry from <stage>] + [Dismiss]", () => {
    const stages = pendingStages();
    stages[0] = {
      stage: "Reconcile",
      label: "Reconcile",
      status: { kind: "complete", durationMs: 300, partialFailures: [] },
    };
    stages[1] = {
      stage: "Discover",
      label: "Discover",
      status: { kind: "complete", durationMs: 1100, partialFailures: [] },
    };
    stages[2] = {
      stage: "Consolidate",
      label: "Consolidate",
      status: {
        kind: "failed",
        durationMs: 200,
        error: err("Conflict", "two skills named foo"),
      },
    };
    stages[3] = { stage: "Distribute", label: "Distribute", status: { kind: "cancelled" } };
    stages[4] = { stage: "Cleanup", label: "Cleanup", status: { kind: "cancelled" } };
    stages[5] = { stage: "Save", label: "Save", status: { kind: "cancelled" } };

    const onRetry = vi.fn();
    const onDismiss = vi.fn();
    render(
      <StageStepper
        stages={stages}
        onRetryFromStage={onRetry}
        onDismiss={onDismiss}
      />,
    );

    const retry = screen.getByRole("button", {
      name: "Retry from Consolidate",
    });
    retry.click();
    expect(onRetry).toHaveBeenCalledWith("Consolidate");

    const dismiss = screen.getByRole("button", { name: "Dismiss sync summary" });
    dismiss.click();
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });
});

describe("StageStepper — summary slot", () => {
  it("renders the summary node above the stepper when provided", () => {
    render(
      <StageStepper
        stages={pendingStages()}
        summary={<h1>Sync cancelled</h1>}
      />,
    );
    expect(
      screen.getByRole("heading", { level: 1, name: "Sync cancelled" }),
    ).toBeInTheDocument();
  });
});
