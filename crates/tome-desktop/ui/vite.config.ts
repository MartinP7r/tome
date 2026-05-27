import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// The frontend and the canonical generated bindings.ts are co-located in this
// dir (src/bindings.ts). App source imports it directly via a relative
// `./bindings` import, so no Vite resolve alias is needed anymore — bindings.ts
// remains the single source of truth at crates/tome-desktop/ui/src/bindings.ts
// (regenerated + freshness-gated by the gen-bindings bin, D-07).
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
});
