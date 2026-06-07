// SyncToast tests — Phase 27 plan 27-04.
//
// Pin the Pitfall 2 contract: hand-rolled role=status aria-live=polite
// live region (NOT react-aria-components UNSTABLE_ToastRegion) with a
// 5s setTimeout auto-dismiss + explicit Dismiss button. Test uses
// vitest fake timers to drive the lifecycle deterministically.

import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, it, expect, vi } from "vitest";
import { SyncToast } from "../SyncToast";

describe("SyncToast — Pitfall 2 hand-rolled live region", () => {
  it("renders role=status aria-live=polite aria-atomic=true with the message", () => {
    render(<SyncToast message="Sync complete" />);
    const toast = screen.getByRole("status");
    expect(toast).toHaveAttribute("aria-live", "polite");
    expect(toast).toHaveAttribute("aria-atomic", "true");
    expect(toast.textContent).toContain("Sync complete");
  });

  it("renders an explicit [Dismiss] button (per UI-SPEC §SyncToast)", () => {
    const dismiss = vi.fn();
    render(<SyncToast message="Sync complete" onDismiss={dismiss} />);
    const button = screen.getByRole("button", {
      name: "Dismiss sync notification",
    });
    expect(button).toBeInTheDocument();
    button.click();
    expect(dismiss).toHaveBeenCalledTimes(1);
  });
});

describe("SyncToast — auto-dismiss lifecycle", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("calls onDismiss after the default 5000ms", () => {
    const dismiss = vi.fn();
    render(<SyncToast message="Sync complete" onDismiss={dismiss} />);
    expect(dismiss).not.toHaveBeenCalled();

    act(() => {
      vi.advanceTimersByTime(4999);
    });
    expect(dismiss).not.toHaveBeenCalled();

    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(dismiss).toHaveBeenCalledTimes(1);
  });

  it("honors a custom durationMs prop", () => {
    const dismiss = vi.fn();
    render(
      <SyncToast message="Sync complete" durationMs={1500} onDismiss={dismiss} />,
    );

    act(() => {
      vi.advanceTimersByTime(1499);
    });
    expect(dismiss).not.toHaveBeenCalled();

    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(dismiss).toHaveBeenCalledTimes(1);
  });

  it("clears the timer on unmount (no late onDismiss)", () => {
    const dismiss = vi.fn();
    const { unmount } = render(
      <SyncToast message="Sync complete" onDismiss={dismiss} />,
    );

    unmount();
    act(() => {
      vi.advanceTimersByTime(10_000);
    });
    expect(dismiss).not.toHaveBeenCalled();
  });
});
