// TriageRow tests — Phase 27 plan 27-02 / SYNC-02.
//
// Pins:
// 1. Primary skill name + secondary `source · managed|local · synced …`
//    line render per UI-SPEC §TriageRow.
// 2. Added entries with no synced_at render `synced —`.
// 3. Decision chip flips between `✓ keep` and `⊘ disabled here`.
// 4. Removed rows render a non-interactive `implicit remove` chip
//    (D-13 invariant).
// 5. The chip aria-label matches UI-SPEC §VoiceOver labels
//    "Toggle decision for ${name} between keep and disable on this
//    machine".

import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { TriageRow } from "../TriageRow";

const LOCAL = { kind: "local" as const };
const MANAGED = {
  kind: "managed" as const,
  provenance: { registry_id: "axiom@npm", version: "1.0.0", git_commit_sha: null },
};

describe("TriageRow — primary + secondary content", () => {
  it("renders skill name + source · managed|local · synced — for an Added local row", () => {
    const { container } = render(
      <TriageRow
        name="axiom-build"
        changeKind="added"
        sourceName="plugins"
        origin={LOCAL}
        syncedAt={null}
        decision="keep"
        onDecisionToggle={() => undefined}
        isSelected={false}
      />,
    );
    expect(container.textContent).toContain("axiom-build");
    expect(container.textContent).toContain("plugins");
    expect(container.textContent).toContain("local");
    expect(container.textContent).toContain("synced —");
  });

  it("renders 'managed' label when origin.kind === 'managed'", () => {
    const { container } = render(
      <TriageRow
        name="axiom-build"
        changeKind="added"
        sourceName="plugins"
        origin={MANAGED}
        syncedAt={null}
        decision="keep"
        onDecisionToggle={() => undefined}
        isSelected={false}
      />,
    );
    expect(container.textContent).toContain("managed");
    expect(container.textContent).not.toContain("local");
  });
});

describe("TriageRow — chip toggle (D-12)", () => {
  it("renders ✓ keep when decision === 'keep'", () => {
    render(
      <TriageRow
        name="axiom-build"
        changeKind="added"
        sourceName="plugins"
        origin={LOCAL}
        syncedAt={null}
        decision="keep"
        onDecisionToggle={() => undefined}
        isSelected={false}
      />,
    );
    expect(screen.getByText(/✓ keep/)).toBeInstanceOf(HTMLElement);
  });

  it("renders ⊘ disabled here when decision === 'disable'", () => {
    render(
      <TriageRow
        name="axiom-build"
        changeKind="added"
        sourceName="plugins"
        origin={LOCAL}
        syncedAt={null}
        decision="disable"
        onDecisionToggle={() => undefined}
        isSelected={false}
      />,
    );
    expect(screen.getByText(/⊘ disabled here/)).toBeInstanceOf(HTMLElement);
  });

  it("clicking the chip fires onDecisionToggle exactly once", () => {
    const toggle = vi.fn();
    render(
      <TriageRow
        name="axiom-build"
        changeKind="added"
        sourceName="plugins"
        origin={LOCAL}
        syncedAt={null}
        decision="keep"
        onDecisionToggle={toggle}
        isSelected={false}
      />,
    );
    fireEvent.click(
      screen.getByRole("button", {
        name: /Toggle decision for axiom-build between keep and disable/,
      }),
    );
    expect(toggle).toHaveBeenCalledTimes(1);
  });

  it("chip aria-label matches UI-SPEC VoiceOver template", () => {
    render(
      <TriageRow
        name="axiom-build"
        changeKind="added"
        sourceName="plugins"
        origin={LOCAL}
        syncedAt={null}
        decision="keep"
        onDecisionToggle={() => undefined}
        isSelected={false}
      />,
    );
    const button = screen.getByRole("button");
    expect(button.getAttribute("aria-label")).toBe(
      "Toggle decision for axiom-build between keep and disable on this machine",
    );
  });
});

describe("TriageRow — removed-row non-interactive chip (D-13)", () => {
  it("removed rows render 'implicit remove' as a non-button span", () => {
    render(
      <TriageRow
        name="axiom-build"
        changeKind="removed"
        sourceName="plugins"
        origin={LOCAL}
        syncedAt="2026-06-01T00:00:00Z"
        decision="keep"
        onDecisionToggle={() => undefined}
        isSelected={false}
      />,
    );
    expect(screen.getByText("implicit remove")).toBeInstanceOf(HTMLElement);
    // No button — the row has no interactive chip on Removed entries.
    expect(screen.queryByRole("button")).toBeNull();
  });
});
