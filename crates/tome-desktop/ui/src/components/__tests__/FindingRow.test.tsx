// FindingRow tests — Phase 27 plan 27-03 / SYNC-03.
//
// Pitfall 3 invariant: when the PreviewPopover slot refactor lands, the
// existing Doctor's Fix caller MUST be updated in the same plan to keep
// the refactor atomic. These tests pin the post-refactor Doctor flow:
//
// 1. Auto-fixable finding renders a Fix button (the PreviewPopover trigger
//    defaults to "Fix" when no `triggerLabel` override is passed).
// 2. Opening the popover surfaces the `dry_run_description` text inside
//    the body slot (the Doctor passes it as a `<p>` child).
// 3. Non-fixable findings render the remediation hint text and NO button
//    (D-12 — never a dead control).

import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import { FindingRow } from "../FindingRow";
import type { DoctorFinding } from "../../bindings";

function autoFixableFinding(): DoctorFinding {
  return {
    id: { kind: "unparsable_frontmatter", path: "/skills/foo/SKILL.md" } as any,
    title: "Broken frontmatter in foo",
    description: "foo's SKILL.md frontmatter doesn't parse",
    repair_kind: { kind: "delete_orphan" } as any,
    dry_run_description:
      "will delete the real directory and replace it with a symlink into the library",
  } as unknown as DoctorFinding;
}

function nonFixableFinding(): DoctorFinding {
  return {
    id: { kind: "unparsable_frontmatter", path: "/skills/bar/SKILL.md" } as any,
    title: "Unparsable frontmatter in bar",
    description: "bar's SKILL.md frontmatter doesn't parse",
    repair_kind: null,
    dry_run_description: null,
  } as unknown as DoctorFinding;
}

describe("FindingRow — Doctor's Fix flow (post-Pitfall-3 refactor)", () => {
  it("renders a Fix button for auto-fixable findings", () => {
    render(
      <FindingRow
        finding={autoFixableFinding()}
        onApplyFix={() => Promise.resolve()}
      />,
    );
    expect(screen.getByRole("button", { name: "Fix" })).toBeInTheDocument();
  });

  it("opens the popover and renders the dry_run_description in the slot body", async () => {
    render(
      <FindingRow
        finding={autoFixableFinding()}
        onApplyFix={() => Promise.resolve()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Fix" }));
    await waitFor(() => {
      expect(
        screen.getByText(
          "will delete the real directory and replace it with a symlink into the library",
        ),
      ).toBeInTheDocument();
    });
  });

  it("renders a remediation hint instead of a button for non-fixable findings", () => {
    render(
      <FindingRow
        finding={nonFixableFinding()}
        onApplyFix={() => Promise.resolve()}
      />,
    );
    expect(screen.queryByRole("button", { name: "Fix" })).not.toBeInTheDocument();
    // The hint copy from getRemediationHint for unparsable_frontmatter:
    expect(screen.getByText(/Edit the file's YAML frontmatter/i)).toBeInTheDocument();
  });
});
