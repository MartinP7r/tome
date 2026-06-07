// formatDuration tests — Phase 27 plan 27-04 / UI-SPEC §StageRow §Duration
// format rule.

import { describe, it, expect } from "vitest";
import { formatDuration } from "../formatDuration";

describe("formatDuration — UI-SPEC §StageRow §Duration format rule", () => {
  it("formats sub-second durations as 'X.Xs' (1 decimal)", () => {
    expect(formatDuration(300)).toBe("0.3s");
    expect(formatDuration(0)).toBe("0.0s");
    expect(formatDuration(999)).toBe("1.0s"); // toFixed rounds — at 999 ms it rounds up
  });

  it("formats 1s..60s durations as 'X.Xs' (1 decimal)", () => {
    expect(formatDuration(1000)).toBe("1.0s");
    expect(formatDuration(8200)).toBe("8.2s");
    expect(formatDuration(59_900)).toBe("59.9s");
  });

  it("formats 1m+ durations as 'Mm Ss' (whole seconds)", () => {
    expect(formatDuration(60_000)).toBe("1m 0s");
    expect(formatDuration(74_000)).toBe("1m 14s");
    expect(formatDuration(125_500)).toBe("2m 5s");
  });

  it("clamps negative durations to 0", () => {
    expect(formatDuration(-100)).toBe("0.0s");
  });

  it("returns empty string for non-finite inputs", () => {
    expect(formatDuration(Number.NaN)).toBe("");
    expect(formatDuration(Number.POSITIVE_INFINITY)).toBe("");
  });
});
