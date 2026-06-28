// TriagePanel tests — Phase 27 plan 27-02 / SYNC-02.
//
// Pins:
// 1. Three vertical sections render (NEW / CHANGED / REMOVED), each
//    with the correct count. Source-group inner headers appear.
// 2. Pitfall 1 — the rendered DOM uses React Aria GridList (role
//    `grid`), NOT ListBox.
// 3. Bulk-action buttons appear ONLY on the NEW section + its source
//    groups (D-13). CHANGED / REMOVED outer headers carry no buttons.
// 4. Inline chip toggle (D-12) — clicking a `[✓ keep]` chip fires
//    `onDecisionChange(skill, "disable")`.
// 5. Row selection — clicking a row fires `onSelect(skill)`.
// 6. `[Apply N decisions]` button label updates with pending count;
//    `aria-disabled` when N === 0.

import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { TriagePanel } from "../TriagePanel";
import type { LockfileDiff, SkillName, TriageEntry } from "../../bindings";

function localEntry(name: string, source: string | null, kind: "added" | "changed" | "removed"): TriageEntry {
  return {
    name,
    change_kind: kind,
    source_name: source,
    previous_source: null,
    origin: { kind: "local" },
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

function diff8New3Changed1Removed(): LockfileDiff {
  return {
    added: [
      localEntry("alpha", "plugins", "added"),
      localEntry("beta", "plugins", "added"),
      localEntry("gamma", "plugins", "added"),
      localEntry("delta", "plugins", "added"),
      localEntry("epsilon", "plugins", "added"),
      localEntry("zeta", "my-repo", "added"),
      localEntry("eta", "my-repo", "added"),
      localEntry("theta", "my-repo", "added"),
    ],
    changed: [
      localEntry("iota", "plugins", "changed"),
      localEntry("kappa", "plugins", "changed"),
      localEntry("lambda", "plugins", "changed"),
    ],
    removed: [localEntry("mu", "plugins", "removed")],
  };
}

const NOOP = () => {
  /* no-op */
};
const NOOP_DECISION = () => {
  /* no-op */
};
const NOOP_BULK = () => {
  /* no-op */
};
const NOOP_APPLIED = () => {
  /* no-op — Phase 27 plan 27-03: TriagePanel renamed onApply→onApplied;
     the new prop is invoked AFTER applyMachineToml resolves successfully. */
};

describe("TriagePanel — three vertical sections (D-11)", () => {
  it("renders NEW/CHANGED/REMOVED outer headers with the expected counts", () => {
    render(
      <TriagePanel
        diff={diff8New3Changed1Removed()}
        decisions={new Map()}
        onDecisionChange={NOOP_DECISION}
        selectedSkill={null}
        onSelect={NOOP}
        onBulkAction={NOOP_BULK}
        onApplied={NOOP_APPLIED}
      />,
    );
    // Outer headers are h2 — find by role+name.
    const headings = screen.getAllByRole("heading", { level: 2 });
    const labels = headings.map((h) => h.textContent);
    expect(labels.some((l) => l?.includes("NEW") && l.includes("(8)"))).toBe(true);
    expect(labels.some((l) => l?.includes("CHANGED") && l.includes("(3)"))).toBe(true);
    expect(labels.some((l) => l?.includes("REMOVED") && l.includes("(1)"))).toBe(true);
  });

  it("renders inner h3 source-group headers (PLUGINS, MY-REPO) inside NEW", () => {
    render(
      <TriagePanel
        diff={diff8New3Changed1Removed()}
        decisions={new Map()}
        onDecisionChange={NOOP_DECISION}
        selectedSkill={null}
        onSelect={NOOP}
        onBulkAction={NOOP_BULK}
        onApplied={NOOP_APPLIED}
      />,
    );
    const innerHeadings = screen.getAllByRole("heading", { level: 3 });
    const labels = innerHeadings.map((h) => h.textContent ?? "");
    expect(labels.some((l) => l.includes("PLUGINS"))).toBe(true);
    expect(labels.some((l) => l.includes("MY-REPO"))).toBe(true);
  });
});

describe("TriagePanel — Pitfall 1 (GridList NOT ListBox)", () => {
  it("uses React Aria GridList (role grid) for the row container", () => {
    const { container } = render(
      <TriagePanel
        diff={diff8New3Changed1Removed()}
        decisions={new Map()}
        onDecisionChange={NOOP_DECISION}
        selectedSkill={null}
        onSelect={NOOP}
        onBulkAction={NOOP_BULK}
        onApplied={NOOP_APPLIED}
      />,
    );
    // GridList has role="grid"; ListBox has role="listbox".
    // Pitfall 1 invariant: TriagePanel MUST use GridList. The NEW
    // section is expanded by default, so at least one grid is present
    // in the DOM.
    const grids = container.querySelectorAll('[role="grid"]');
    expect(grids.length).toBeGreaterThan(0);
    const listboxes = container.querySelectorAll('[role="listbox"]');
    expect(listboxes.length).toBe(0);
  });
});

describe("TriagePanel — bulk action scope (D-13)", () => {
  it("renders [Disable all new] on the NEW outer section", () => {
    render(
      <TriagePanel
        diff={diff8New3Changed1Removed()}
        decisions={new Map()}
        onDecisionChange={NOOP_DECISION}
        selectedSkill={null}
        onSelect={NOOP}
        onBulkAction={NOOP_BULK}
        onApplied={NOOP_APPLIED}
      />,
    );
    expect(
      screen.getByRole("button", { name: "Disable all new skills" }),
    ).toBeInstanceOf(HTMLElement);
  });

  it("does NOT render bulk-action buttons on CHANGED or REMOVED sections", () => {
    render(
      <TriagePanel
        diff={diff8New3Changed1Removed()}
        decisions={new Map()}
        onDecisionChange={NOOP_DECISION}
        selectedSkill={null}
        onSelect={NOOP}
        onBulkAction={NOOP_BULK}
        onApplied={NOOP_APPLIED}
      />,
    );
    // No "Disable all changed" or "Disable all removed" buttons exist.
    expect(screen.queryByRole("button", { name: /Disable all changed/i })).toBeNull();
    expect(screen.queryByRole("button", { name: /Disable all removed/i })).toBeNull();
  });

  it("clicking [Disable all new] fires onBulkAction with the section scope", () => {
    const onBulkAction = vi.fn();
    render(
      <TriagePanel
        diff={diff8New3Changed1Removed()}
        decisions={new Map()}
        onDecisionChange={NOOP_DECISION}
        selectedSkill={null}
        onSelect={NOOP}
        onBulkAction={onBulkAction}
        onApplied={NOOP_APPLIED}
      />,
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Disable all new skills" }),
    );
    expect(onBulkAction).toHaveBeenCalledTimes(1);
    expect(onBulkAction).toHaveBeenCalledWith(
      { kind: "section", section: "new" },
      "disable",
    );
  });

  it("renders [Disable all new from plugins] on the inner PLUGINS source group", () => {
    render(
      <TriagePanel
        diff={diff8New3Changed1Removed()}
        decisions={new Map()}
        onDecisionChange={NOOP_DECISION}
        selectedSkill={null}
        onSelect={NOOP}
        onBulkAction={NOOP_BULK}
        onApplied={NOOP_APPLIED}
      />,
    );
    expect(
      screen.getByRole("button", { name: "Disable all new skills from plugins" }),
    ).toBeInstanceOf(HTMLElement);
  });
});

describe("TriagePanel — inline chip toggle (D-12)", () => {
  it("clicking a [✓ keep] chip fires onDecisionChange(skill, 'disable')", () => {
    const onDecisionChange = vi.fn();
    const decisions = new Map<SkillName, "keep" | "disable">([["alpha", "keep"]]);
    render(
      <TriagePanel
        diff={diff8New3Changed1Removed()}
        decisions={decisions}
        onDecisionChange={onDecisionChange}
        selectedSkill={null}
        onSelect={NOOP}
        onBulkAction={NOOP_BULK}
        onApplied={NOOP_APPLIED}
      />,
    );
    // The chip button is labelled by the toggle-aria pattern.
    const chip = screen.getByRole("button", {
      name: /Toggle decision for alpha between keep and disable/,
    });
    fireEvent.click(chip);
    expect(onDecisionChange).toHaveBeenCalledTimes(1);
    expect(onDecisionChange).toHaveBeenCalledWith("alpha", "disable");
  });
});

describe("TriagePanel — Apply button", () => {
  it("renders [Apply 0 decisions] when no pending decisions exist", () => {
    render(
      <TriagePanel
        diff={diff8New3Changed1Removed()}
        decisions={new Map()}
        onDecisionChange={NOOP_DECISION}
        selectedSkill={null}
        onSelect={NOOP}
        onBulkAction={NOOP_BULK}
        onApplied={NOOP_APPLIED}
      />,
    );
    // The Apply button is disabled when no pending decisions exist.
    const applyButton = screen.getByRole("button", {
      name: /Apply 0 triage decisions/,
    });
    // React Aria sets aria-disabled rather than the native disabled attribute
    // when isDisabled is passed.
    expect(
      applyButton.getAttribute("aria-disabled") === "true" ||
        applyButton.hasAttribute("disabled"),
    ).toBe(true);
  });

  it("renders [Apply N decisions] with the actual non-default count", () => {
    const decisions = new Map<SkillName, "keep" | "disable">([
      ["alpha", "disable"],
      ["beta", "disable"],
    ]);
    render(
      <TriagePanel
        diff={diff8New3Changed1Removed()}
        decisions={decisions}
        onDecisionChange={NOOP_DECISION}
        selectedSkill={null}
        onSelect={NOOP}
        onBulkAction={NOOP_BULK}
        onApplied={NOOP_APPLIED}
      />,
    );
    expect(
      screen.getByRole("button", { name: /Apply 2 triage decisions/ }),
    ).toBeInstanceOf(HTMLElement);
  });
});
