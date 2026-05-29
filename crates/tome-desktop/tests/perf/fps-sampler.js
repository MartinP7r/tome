// FPS sampler — injected into the page via Playwright's `addInitScript`
// (plan 26-08 Task 2, NF-01 perf bench).
//
// Plain JS (not TS) because `addInitScript({ path: ... })` reads the
// file and runs it verbatim in the page — no TypeScript transpilation.
//
// Exposes two globals on `window`:
//
//   __startFpsSampler(durationMs)   — reset the buffer and start
//                                     a fresh sampling window.
//   __fpsFrames: number[]            — inter-frame deltas (ms).
//
// Why a manual start rather than auto-sampling from page load? Auto-
// sampling would include first-paint + initial-render cost, which
// dominates the median and distorts the p95. The bench wants to
// measure **search-as-you-type** specifically — the steady-state cost
// of typing into a populated list — so it calls __startFpsSampler
// right before keyboard.type() to scope the window cleanly.
//
// requestAnimationFrame is the canonical browser API for per-frame
// timing (Assumption A10 in 26-RESEARCH.md). On Chromium with vsync
// the browser drives tick(t) at the display refresh rate, so frame
// deltas of ~16.7ms indicate sustained 60fps and deltas of >18ms
// indicate a missed frame.

// Initialise empty on every page navigation. The bench MUST call
// __startFpsSampler before typing — without that call the array
// stays empty and the bench's "samples > 30" assertion fails fast.
window.__fpsFrames = [];

window.__startFpsSampler = function startFpsSampler(durationMs) {
  window.__fpsFrames = [];
  var start = performance.now();
  var last = start;
  function tick(t) {
    window.__fpsFrames.push(t - last);
    last = t;
    if (t - start < durationMs) {
      requestAnimationFrame(tick);
    }
  }
  requestAnimationFrame(tick);
};
