// Playwright config for the axe-core a11y gate (plan 26-07 Task 3).
//
// Drives a static-server fixture: `npm run dev:a11y` starts Vite with
// `A11Y_TEST=1`, which swaps the `@tauri-apps/*` imports for the mock
// modules under `src/__mocks__/`. The four tests in `axe.spec.ts` then
// navigate to each Phase-26 view, wait for its key landmark to render,
// and run axe with `wcag2a` + `wcag2aa` tags. Any violation fails the
// build.
//
// Path A from the plan — the real Tauri IPC behaviour is verified
// manually + by the watcher integration test in plan 26-06; this gate
// only validates that the React render tree is WCAG-AA-clean.

// `playwright test` runs from the ui/ dir (via `npm run test:a11y`), so
// node-resolution walks up from `ui/node_modules/`. We import via the
// relative path inside `ui/node_modules/` so the config resolves
// regardless of where the user invokes the script from.
import { defineConfig, devices } from "../../ui/node_modules/playwright/test";
import * as path from "node:path";

const UI_DIR = path.resolve(__dirname, "..", "..", "ui");

// `playwright install chromium` downloads ~150MB of binaries. CI has
// the bandwidth budget for that on first run; on tight sandboxes (e.g.
// the project's `--dangerously-skip-permissions`-style local runs)
// the download can be blocked. When `PW_USE_SYSTEM_CHROME=1` is set we
// fall back to the system-installed Google Chrome via the `channel`
// option — same Playwright driver, different browser binary.
const useSystemChrome = process.env.PW_USE_SYSTEM_CHROME === "1";

export default defineConfig({
  testDir: ".",
  testMatch: ["**/*.spec.ts"],
  fullyParallel: false,
  retries: 0,
  workers: 1,
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
    command: "npm run dev:a11y",
    cwd: UI_DIR,
    url: "http://localhost:1420",
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
    env: { A11Y_TEST: "1" },
  },
});
