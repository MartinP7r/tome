// 60fps-search bench — NF-01 (Phase 26 plan 26-08 Task 2).
//
// Verifies that with 2000 synthetic skills loaded into the React Aria
// `<Virtualizer>` (path A, locked in plan 26-02), typing a 3-character
// query into the SearchField sustains 60fps. Asserts p95 inter-frame
// delta < 18ms (~55fps p95) over a 2-second sampling window.
//
// **Why p95, not max?** A single dropped frame doesn't break the user's
// perception of smoothness; a sustained pattern of late frames does.
// The p95 metric captures "the slow tail" without false-failing on a
// browser hiccup or GC pause that any honest measurement will hit.
//
// **Why p95 < 18ms, not 16.7ms?** Strict 60fps over 1s is unrealistic
// in a Playwright-driven Chromium with vsync — typing emits compositor
// events that occasionally land just-past-vsync. 18ms leaves ~8% slack
// while still failing on a true regression (e.g., a non-virtualised
// render pass that walks all 2000 rows per keystroke).
//
// **OQ-1 fallback path.** If this bench fails, 26-PERF-REPORT.md
// documents the React-Aria-Virtualizer-vs-TanStack-Virtual decision.
// Don't silently lower the threshold — the right move is to either
// refactor the SkillsView to use TanStack (the path-B fallback flagged
// in 26-RESEARCH.md §Standard Stack — Virtualisation) or to identify
// and fix the specific render-pipeline regression.

// Same relative-import resolution trick as the a11y spec — see
// `tests/a11y/axe.spec.ts` for the rationale.
import { test, expect } from "../../ui/node_modules/playwright/test";
import * as fs from "node:fs";
import * as path from "node:path";

// FPS-sampler globals injected by `fps-sampler.js` via `addInitScript`.
// The .js file is plain JS (no TS imports) so the page can run it
// verbatim; we type the surface here in the spec for use under
// `page.evaluate(() => window.__startFpsSampler(...))` calls.
declare global {
  interface Window {
    __fpsFrames: number[];
    __startFpsSampler: (durationMs: number) => void;
  }
}

// Path to the perf-runs log at the workspace root.
//
// Why a separate `.tsv` file rather than appending directly to
// `26-PERF-REPORT.md`? Appending to the report's middle table is messy
// (`fs.appendFileSync` only writes to file end, which lands rows after
// later prose sections). The TSV captures the raw timeline; a human (or
// the CI artefact-upload step) moves clean baselines into the report
// table during review.
//
//   __dirname = crates/tome-desktop/tests/perf/
//   4 levels up = workspace root.
const PERF_RUNS_LOG = path.resolve(
  __dirname,
  "..",
  "..",
  "..",
  "..",
  ".planning",
  "phases",
  "26-read-only-views-alpha-cut",
  "26-PERF-RUNS.tsv",
);

const SAMPLING_WINDOW_MS = 2000;
const SAMPLING_HEADROOM_MS = 500; // wait this long beyond the window to be sure tick() finished
const P95_TARGET_MS = 18; // ~55fps p95 — see comment block above

test("2000-skill search-as-you-type sustains 60fps p95", async ({ page }) => {
  // 1. Inject the FPS sampler before any page script runs. The sampler
  //    only defines globals — it doesn't start sampling until the bench
  //    calls `__startFpsSampler` below.
  //
  //    `addInitScript({ path })` reads the file verbatim into the page
  //    as JavaScript — no TypeScript transpilation in flight — so the
  //    sampler ships as plain `.js` (TS types live in this spec file
  //    via the `declare global` block below).
  await page.addInitScript({
    path: path.join(__dirname, "fps-sampler.js"),
  });

  // 2. Navigate to the app shell + click into the Skills view. The
  //    Sidebar NavItems are React Aria ListBoxItems → role="option"
  //    (same pattern as the a11y spec).
  await page.goto("/");
  await page
    .getByRole("option", { name: /^Skills, Skills section/ })
    .click();

  // 3. Wait for the Skills view to populate from the (mocked) 2000-row
  //    fixture. Need both the SearchField (so we can focus it) and the
  //    ListBox (so we know virtualisation has materialised at least one
  //    row).
  await page
    .getByRole("searchbox", { name: "Search skills" })
    .waitFor({ state: "visible", timeout: 10_000 });
  await page
    .getByRole("listbox", { name: "Skills" })
    .waitFor({ state: "visible", timeout: 10_000 });

  // 4. Focus the SearchField. React Aria's `<AriaSearchField>` puts the
  //    `aria-label="Search skills"` on the OUTER wrapper while the inner
  //    `<input type="search">` carries the implicit `role="searchbox"`.
  //    `getByRole` walks the accessible-name tree so the inner input
  //    inherits the wrapper's label — `name: "Search skills"` resolves
  //    to the input correctly. (A bare CSS selector
  //    `[role="searchbox"][aria-label="Search skills"]` does NOT match
  //    because those two attributes live on different DOM nodes.)
  await page
    .getByRole("searchbox", { name: "Search skills" })
    .focus();

  // 5. Start the FPS sampler. The bench MUST do this before typing —
  //    sampling from page load would include first-paint cost and
  //    distort the p95.
  await page.evaluate(
    (window_ms) => window.__startFpsSampler(window_ms),
    SAMPLING_WINDOW_MS,
  );

  // 6. Type the query character-by-character at 15ms inter-keystroke
  //    delay. 3 chars × 15ms = 45ms of typing inside the 2000ms window,
  //    so the bulk of the sampled frames cover the post-keystroke
  //    re-render + virtualisation work, which is what we care about.
  await page.keyboard.type("tdd", { delay: 15 });

  // 7. Wait for the sampling window to elapse + a small headroom so the
  //    final tick() lands.
  await page.waitForTimeout(SAMPLING_WINDOW_MS + SAMPLING_HEADROOM_MS);

  // 8. Read back the frame buffer and compute the percentiles.
  const frames: number[] = await page.evaluate(() => window.__fpsFrames);
  expect(
    frames.length,
    "FPS sampler returned no frames — did __startFpsSampler fire?",
  ).toBeGreaterThan(30);

  const sorted = [...frames].sort((a, b) => a - b);
  const pct = (p: number): number =>
    sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * p))];
  const p50 = pct(0.5);
  const p95 = pct(0.95);
  const max = sorted[sorted.length - 1];
  const dropped = frames.filter((d) => d > P95_TARGET_MS).length;

  // Console-log unconditionally so the CI artefact captures the numbers
  // even when the assertion passes.
  // eslint-disable-next-line no-console
  console.log(
    `[perf] samples=${frames.length} p50=${p50.toFixed(2)}ms ` +
      `p95=${p95.toFixed(2)}ms max=${max.toFixed(2)}ms ` +
      `dropped(>${P95_TARGET_MS}ms)=${dropped} ` +
      `(target: p95 < ${P95_TARGET_MS}ms)`,
  );

  // 9. Append the run to the rolling perf-runs TSV. Captures every run
  //    as raw data; the human (or CI artefact-upload step) moves clean
  //    baselines into the 26-PERF-REPORT.md table during review.
  //    TSV header is seeded on first write so the file is self-describing.
  const timestamp = new Date().toISOString();
  const verdict = p95 < P95_TARGET_MS ? "PASS" : "FAIL";
  const headerLine =
    "timestamp\tverdict\tsamples\tp50_ms\tp95_ms\tmax_ms\tdropped\n";
  const dataLine =
    `${timestamp}\t${verdict}\t${frames.length}\t${p50.toFixed(2)}\t` +
    `${p95.toFixed(2)}\t${max.toFixed(2)}\t${dropped}\n`;
  try {
    if (!fs.existsSync(PERF_RUNS_LOG)) {
      // Ensure the parent dir exists (CI's checkout always has it; this
      // is belt + braces for any reorganisation).
      fs.mkdirSync(path.dirname(PERF_RUNS_LOG), { recursive: true });
      fs.writeFileSync(PERF_RUNS_LOG, headerLine);
    }
    fs.appendFileSync(PERF_RUNS_LOG, dataLine);
  } catch (e) {
    // eslint-disable-next-line no-console
    console.warn(`[perf] could not write ${PERF_RUNS_LOG}: ${e}`);
  }

  // 10. The actual NF-01 assertion.
  expect(
    p95,
    `p95 inter-frame delta ${p95.toFixed(2)}ms exceeds the ${P95_TARGET_MS}ms NF-01 target. ` +
      `If this is a genuine regression, refactor SkillsView to use TanStack Virtual ` +
      `(path-B fallback documented in 26-RESEARCH.md §Standard Stack — Virtualisation) ` +
      `rather than silently raising the threshold.`,
  ).toBeLessThan(P95_TARGET_MS);
});
