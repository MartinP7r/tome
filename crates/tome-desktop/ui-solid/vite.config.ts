import { defineConfig } from "vite";
import solid from "vite-plugin-solid";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// Spike frontends share the ONE canonical bindings.ts via an alias — never a
// per-spike copy (BLOCKER-3 / SC#3). `@bindings` resolves to the committed
// crates/tome-desktop/ui/src/bindings.ts.
export default defineConfig({
  plugins: [solid()],
  clearScreen: false,
  server: { port: 1421, strictPort: true },
  resolve: {
    alias: {
      "@bindings": path.resolve(__dirname, "../ui/src/bindings.ts"),
    },
  },
});
