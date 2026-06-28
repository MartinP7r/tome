// SectionHeader tests — Phase 27 plan 27-02 extension.
//
// The pre-Phase-27 SectionHeader rendered as <h2> only; Phase 27 extends
// it to also render <h3> at level=3 and accept a `trailing` slot for
// bulk-action buttons. Tests pin:
//
// 1. Back-compat — omitting `level` yields <h2> (matches Phase 26
//    HealthView callsite).
// 2. level=2 → <h2> (explicit).
// 3. level=3 → <h3> (TriagePanel inner source-group headers).
// 4. `trailing` slot renders the supplied node after the count chip.
// 5. `trailing` button is keyboard-focusable (it's just an HTML <button>).

import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import { SectionHeader } from "../SectionHeader";

describe("SectionHeader — heading level", () => {
  it("renders <h2> by default (Phase 26 back-compat)", () => {
    render(<SectionHeader label="AUTO-FIXABLE" count={3} />);
    const heading = screen.getByRole("heading", { level: 2 });
    expect(heading.tagName).toBe("H2");
    expect(heading.textContent).toContain("AUTO-FIXABLE");
    expect(heading.textContent).toContain("(3)");
  });

  it("level=2 explicitly renders <h2>", () => {
    render(<SectionHeader label="NEW" count={8} level={2} />);
    const heading = screen.getByRole("heading", { level: 2 });
    expect(heading.tagName).toBe("H2");
    expect(heading.textContent).toContain("NEW");
  });

  it("level=3 renders <h3> (TriagePanel inner source-group header)", () => {
    render(<SectionHeader label="PLUGINS" count={5} level={3} />);
    const heading = screen.getByRole("heading", { level: 3 });
    expect(heading.tagName).toBe("H3");
    expect(heading.textContent).toContain("PLUGINS");
    expect(heading.textContent).toContain("(5)");
  });
});

describe("SectionHeader — trailing slot", () => {
  it("renders the trailing slot after the count chip", () => {
    render(
      <SectionHeader
        label="NEW"
        count={8}
        level={2}
        trailing={<button type="button">Disable all new</button>}
      />,
    );
    // Heading should contain BOTH the label/count AND the button text —
    // the trailing slot is rendered inside the heading element.
    const heading = screen.getByRole("heading", { level: 2 });
    expect(heading.textContent).toContain("NEW");
    expect(heading.textContent).toContain("(8)");
    expect(heading.textContent).toContain("Disable all new");
    // The button itself is keyboard-focusable.
    const button = screen.getByRole("button", { name: "Disable all new" });
    expect(button).toBeInstanceOf(HTMLButtonElement);
  });

  it("omits the trailing wrapper element when no trailing prop is provided", () => {
    const { container } = render(
      <SectionHeader label="AUTO-FIXABLE" count={3} />,
    );
    // No buttons inside the heading.
    expect(container.querySelectorAll("button")).toHaveLength(0);
  });
});
