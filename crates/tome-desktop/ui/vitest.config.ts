// Vitest config — bootstrapped in Phase 26 plan 04 for the MarkdownBody
// snapshot test (also reused by plans 26-05 / 26-07).
//
// - `jsdom` environment: react-markdown + @testing-library/react need a DOM.
// - `globals: true`: lets tests use `describe`/`it`/`expect`/`vi` without
//   per-file imports (matches @testing-library/jest-dom's expectations).
// - `setupFiles`: pulls in @testing-library/jest-dom's matchers.

import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test-setup.ts"],
    css: true,
  },
});
