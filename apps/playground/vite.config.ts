import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath, URL } from "node:url";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@universal-payment-qr/wasm": fileURLToPath(new URL("../../packages/wasm/src/index.ts", import.meta.url)),
      "@universal-payment-qr/core": fileURLToPath(new URL("../../packages/core/src/index.ts", import.meta.url)),
      "@universal-payment-qr/scanner": fileURLToPath(new URL("../../packages/scanner/src/index.ts", import.meta.url)),
      "@universal-payment-qr/react": fileURLToPath(new URL("../../packages/react/src/index.ts", import.meta.url))
    }
  },
  server: { port: 4173 },
});

