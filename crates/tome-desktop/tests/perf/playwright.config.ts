// Playwright config for the NF-01 perf bench (plan 26-08 Task 2).
//
// Mirrors `tests/a11y/playwright.config.ts` (plan 26-07) — same
// system-Chrome fallback, same relative-import resolution trick — but
// drives `npm run dev:perf` instead of `npm run dev:a11y`, so the Vite
// bundle has `PERF_TEST=1` set and the tauri-api-core mock returns
// the 2000-skill perf fixture.
//
// The web-server is the same Vite dev server the user runs locally
// (`http://localhost:1420`); the spec just navigates to the Skills
// view by clicking the Sidebar NavItem and types into the SearchField.
// Headless Chromium drives a single test (`60fps-search.spec.ts`);
// workers=1 + fullyParallel=false eliminate parallel-execution noise
// in the FPS sampling.

// Like `tests/a11y/playwright.config.ts`, we import via the relative
// `ui/node_modules/` path so this config + the spec under it resolve
// regardless of whether `playwright test` runs from `ui/` (the
// npm-script origin) or directly from `tests/perf/`.
import { defineConfig, devices } from "../../ui/node_modules/playwright/test";
import * as path from "node:path";

const UI_DIR = path.resolve(__dirname, "..", "..", "ui");

// `PW_USE_SYSTEM_CHROME=1` — fall back to the system-installed Chrome
// (via `channel: "chrome"`) when Playwright's bundled Chromium can't
// be downloaded. Identical pattern to the a11y config, added in 26-07
// to keep restricted-egress local runs viable.
const useSystemChrome = process.env.PW_USE_SYSTEM_CHROME === "1";

export default defineConfig({
  testDir: ".",
  testMatch: ["**/*.spec.ts"],
  // Perf bench: no parallelism. Two concurrent specs would contend for
  // the same browser/CPU, which would inflate inter-frame deltas
  // unrelated to the surface under test.
  fullyParallel: false,
  workers: 1,
  // No retries — a flake means real signal worth investigating, not
  // something to paper over. If the bench becomes flaky in CI, raise
  // the p95 threshold from 18ms → 20ms (50fps) in a documented
  // follow-up; don't add retries first.
  retries: 0,
  // Generous test timeout: the spec waits ~2.5s for the FPS window to
  // close + has up to 10s for the initial 2000-row render. 30s leaves
  // headroom on slow CI runners.
  timeout: 30_000,
  reporter: [["list"]],
  use: {
    headless: true,
    viewport: { width: 1100, height: 740 },
    baseURL: "http://localhost:1420",
    ...(useSystemChrome ? { channel: "chrome" } : {}),
  },
  projects: [
    {
      name: "chromium",
      use: useSystemChrome
        ? { ...devices["Desktop Chrome"], channel: "chrome" }
        : devices["Desktop Chrome"],
    },
  ],
  webServer: {
    // `dev:perf` starts Vite with `PERF_TEST=1`. The fixture
    // generator (Task 1) MUST have run first so
    // `crates/tome-desktop/ui/src/__mocks__/perf-skills.json` carries
    // the real 2000-skill snapshot, not the in-tree 3-skill stub.
    command: "npm run dev:perf",
    cwd: UI_DIR,
    url: "http://localhost:1420",
    reuseExistingServer: !process.env.CI,
    // Vite + 2000-skill bundle takes ~3-4s to first-paint in dev mode;
    // 60s timeout gives generous headroom on cold-cache runners.
    timeout: 60_000,
    env: { PERF_TEST: "1" },
  },
});
