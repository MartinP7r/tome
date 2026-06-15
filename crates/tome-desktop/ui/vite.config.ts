import { defineConfig, type Alias } from "vite";
import react from "@vitejs/plugin-react";
import * as path from "node:path";

// The frontend and the canonical generated bindings.ts are co-located in this
// dir (src/bindings.ts). App source imports it directly via a relative
// `./bindings` import, so no Vite resolve alias is needed anymore — bindings.ts
// remains the single source of truth at crates/tome-desktop/ui/src/bindings.ts
// (regenerated + freshness-gated by the gen-bindings bin, D-07).
//
// Plan 26-07 Task 3 (a11y gate) / Plan 26-08 Task 2 (perf gate): when
// either `A11Y_TEST=1` or `PERF_TEST=1` is set in the Node environment,
// alias every `@tauri-apps/*` runtime import to a small mock module under
// `src/__mocks__/`. The mocks return deterministic fixture data so the
// React render tree is well-formed for axe-core / Playwright to scan,
// without spinning up a real Tauri runtime in CI.
//
// Both modes share the same mock modules; the `commands.listSkills` mock
// branches on `import.meta.env.VITE_PERF_TEST === "1"` (exposed below) to
// return the 2000-skill perf fixture instead of the 3-skill a11y fixture
// when the perf bench is the consumer.
const a11yTest = process.env.A11Y_TEST === "1";
const perfTest = process.env.PERF_TEST === "1";
const mockTauri = a11yTest || perfTest;

const mockAliases: Alias[] = mockTauri
  ? [
      {
        find: "@tauri-apps/api/core",
        replacement: path.resolve(
          __dirname,
          "src/__mocks__/tauri-api-core.ts",
        ),
      },
      {
        find: "@tauri-apps/api/event",
        replacement: path.resolve(
          __dirname,
          "src/__mocks__/tauri-api-event.ts",
        ),
      },
      {
        find: "@tauri-apps/plugin-clipboard-manager",
        replacement: path.resolve(
          __dirname,
          "src/__mocks__/tauri-plugin-clipboard.ts",
        ),
      },
      {
        find: "@tauri-apps/plugin-opener",
        replacement: path.resolve(
          __dirname,
          "src/__mocks__/tauri-plugin-opener.ts",
        ),
      },
    ]
  : [];

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  resolve: { alias: mockAliases },
  // Vite's `define` pre-substitutes `import.meta.env.VITE_PERF_TEST`
  // references at bundle time. The `VITE_` prefix is the
  // documented-and-required convention for client-exposed env vars
  // (Vite's default contract). The mock module reads this to switch
  // between the a11y fixture and the perf fixture.
  define: {
    "import.meta.env.VITE_PERF_TEST": JSON.stringify(perfTest ? "1" : "0"),
  },
});
