import { defineConfig } from "vitest/config";
import { fileURLToPath, URL } from "node:url";

export default defineConfig({
  resolve: {
    alias: {
      "@universal-payment-qr/wasm": fileURLToPath(new URL("./packages/wasm/src/index.ts", import.meta.url)),
    },
  },
});

