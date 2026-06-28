// router tests — Phase 27 plan 27-01b.
//
// Pin the literal-union extension to include "sync" and the round-trip
// through setView + useRouter. The shape is small enough that one test
// per invariant keeps the diff readable.

import { renderHook, act } from "@testing-library/react";
import { describe, it, expect } from "vitest";

import { setView, useRouter, type View } from "../router";

describe("router — Phase 27 plan 27-01b", () => {
  it("the View union includes 'sync' between 'skills' and 'health'", () => {
    // Compile-time pin: the literal must accept "sync". This is a value
    // assignment, not a comparison, so a tsc regression (e.g. someone
    // removing 'sync' from the union) trips the typecheck.
    const view: View = "sync";
    expect(view).toBe("sync");
  });

  it("setView('sync') round-trips through the store", () => {
    // Reset to the default before asserting.
    act(() => setView("status"));
    const { result } = renderHook(() => useRouter());
    expect(result.current.view).toBe("status");

    act(() => setView("sync"));
    expect(result.current.view).toBe("sync");

    // Cleanup so subsequent tests don't inherit the sticky state.
    act(() => setView("status"));
  });
});
