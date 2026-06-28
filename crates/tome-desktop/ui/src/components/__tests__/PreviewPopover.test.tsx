// PreviewPopover tests — Phase 27 plan 27-03 / SYNC-03 slot refactor.
//
// Pins (Pitfall 3 — atomic refactor invariant):
// 1. The body slot renders whatever ReactNode the caller passes as
//    `children`. Doctor's existing single-sentence body shape still works
//    by wrapping the description in a `<p>`.
// 2. `triggerLabel` (string) overrides the default trigger button label
//    ("Fix" — Doctor) for the Apply flow ("Apply 3 decisions").
// 3. `triggerAriaLabel` overrides the trigger button's aria-label.
// 4. A custom `trigger` (ReactNode) slot replaces the trigger entirely so
//    TriagePanel can pass its own Apply button instance.
// 5. `helperText` overrides the default "This change is reversible by
//    running tome sync." text for the Apply flow.
// 6. `width` controls the popover's data-width attribute so CSS can hook
//    a wider variant for the diff view (480px instead of 320px default).
// 7. Apply click still fires onApply and closes the popover on resolve;
//    rejection forwards to onError.

import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { PreviewPopover } from "../PreviewPopover";
import type { TomeError } from "../../bindings";

const NOOP_ERR = (_e: TomeError) => {
  /* no-op */
};

describe("PreviewPopover — slot refactor (Pitfall 3)", () => {
  it("renders a ReactNode body via the children slot", async () => {
    render(
      <PreviewPopover onApply={() => Promise.resolve()} onError={NOOP_ERR}>
        <p>Custom body sentence</p>
      </PreviewPopover>,
    );
    // Open the popover so the body is in the DOM.
    fireEvent.click(screen.getByRole("button", { name: "Fix" }));
    await waitFor(() => {
      expect(screen.getByText("Custom body sentence")).toBeInTheDocument();
    });
  });

  it("supports the Doctor caller shape (single-sentence body)", async () => {
    const description =
      "will delete the real directory and replace it with a symlink into the library";
    render(
      <PreviewPopover onApply={() => Promise.resolve()} onError={NOOP_ERR}>
        <p>{description}</p>
      </PreviewPopover>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Fix" }));
    await waitFor(() => {
      expect(screen.getByText(description)).toBeInTheDocument();
    });
  });
});

describe("PreviewPopover — trigger customization", () => {
  it("uses 'Fix' as the default trigger label", () => {
    render(
      <PreviewPopover onApply={() => Promise.resolve()} onError={NOOP_ERR}>
        <p>x</p>
      </PreviewPopover>,
    );
    expect(screen.getByRole("button", { name: "Fix" })).toBeInTheDocument();
  });

  it("triggerLabel overrides the default 'Fix' text", () => {
    render(
      <PreviewPopover
        triggerLabel="Apply 3 decisions"
        onApply={() => Promise.resolve()}
        onError={NOOP_ERR}
      >
        <p>x</p>
      </PreviewPopover>,
    );
    expect(
      screen.getByRole("button", { name: "Apply 3 decisions" }),
    ).toBeInTheDocument();
  });

  it("triggerAriaLabel overrides the trigger button's aria-label", () => {
    render(
      <PreviewPopover
        triggerLabel="Apply 3 decisions"
        triggerAriaLabel="Apply 3 triage decisions, preview machine.toml diff"
        onApply={() => Promise.resolve()}
        onError={NOOP_ERR}
      >
        <p>x</p>
      </PreviewPopover>,
    );
    expect(
      screen.getByRole("button", {
        name: "Apply 3 triage decisions, preview machine.toml diff",
      }),
    ).toBeInTheDocument();
  });

  it("a custom `trigger` slot replaces the default Fix button entirely", () => {
    render(
      <PreviewPopover
        trigger={<button type="button">My custom trigger</button>}
        onApply={() => Promise.resolve()}
        onError={NOOP_ERR}
      >
        <p>x</p>
      </PreviewPopover>,
    );
    expect(
      screen.getByRole("button", { name: "My custom trigger" }),
    ).toBeInTheDocument();
  });
});

describe("PreviewPopover — helperText and width", () => {
  it("renders the default helper text when not overridden", async () => {
    render(
      <PreviewPopover onApply={() => Promise.resolve()} onError={NOOP_ERR}>
        <p>x</p>
      </PreviewPopover>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Fix" }));
    await waitFor(() => {
      expect(
        screen.getByText(/This change is reversible by running tome sync\./i),
      ).toBeInTheDocument();
    });
  });

  it("renders a custom helperText override for the Apply flow", async () => {
    const apply =
      "Applying writes ~/.config/tome/machine.toml. The CLI sees this change immediately.";
    render(
      <PreviewPopover
        helperText={apply}
        onApply={() => Promise.resolve()}
        onError={NOOP_ERR}
      >
        <p>x</p>
      </PreviewPopover>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Fix" }));
    await waitFor(() => {
      expect(screen.getByText(apply)).toBeInTheDocument();
    });
  });

  it("width=480 exposes the wider variant via a data-attribute", async () => {
    render(
      <PreviewPopover
        width={480}
        onApply={() => Promise.resolve()}
        onError={NOOP_ERR}
      >
        <p>x</p>
      </PreviewPopover>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Fix" }));
    await waitFor(() => {
      const dialog = screen.getByRole("dialog");
      // The data-width attribute lives on the popover container or the
      // dialog itself — either works as long as the value is "480" so the
      // CSS rule can hook it.
      const popover = dialog.closest('[data-width="480"]') ?? dialog;
      expect(popover.getAttribute("data-width")).toBe("480");
    });
  });
});

describe("PreviewPopover — Apply / Cancel actions", () => {
  it("Apply click invokes onApply", async () => {
    const onApply = vi.fn().mockResolvedValue(undefined);
    render(
      <PreviewPopover onApply={onApply} onError={NOOP_ERR}>
        <p>x</p>
      </PreviewPopover>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Fix" }));
    await waitFor(() => screen.getByRole("button", { name: "Apply" }));
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    await waitFor(() => {
      expect(onApply).toHaveBeenCalledTimes(1);
    });
  });

  it("Apply rejection forwards to onError", async () => {
    const tomeErr: TomeError = {
      code: "Permission",
      message: "writing /etc/foo: permission denied",
      context: ["wrap1", "root"],
    };
    const onError = vi.fn();
    render(
      <PreviewPopover
        onApply={() => Promise.reject(tomeErr)}
        onError={onError}
      >
        <p>x</p>
      </PreviewPopover>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Fix" }));
    await waitFor(() => screen.getByRole("button", { name: "Apply" }));
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    await waitFor(() => {
      expect(onError).toHaveBeenCalledWith(tomeErr);
    });
  });
});
