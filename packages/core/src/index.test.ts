import { describe, expect, it } from "vitest";
import { createScanner, preset, type PaymentEngine, type PaymentIntent } from "./index.js";

const result: PaymentIntent = {
  schemaVersion: "1.0.0",
  recognized: true,
  supported: false,
  category: "crypto",
  scheme: "solana_pay",
  metadata: {},
  validation: { valid: true, errors: [], warnings: [] },
  support: { enabled: false, reason: "SCHEME_DISABLED" },
};
const engine: PaymentEngine = {
  scan: () => result,
  detect: () => ({ recognized: true, scheme: "solana_pay", confidence: 100 }),
  schemes: () => [],
};

describe("createScanner", () => {
  it("supports an injected engine for native and test environments", async () => {
    expect((await createScanner({ engine }).scan("solana:anything")).supported).toBe(false);
  });

  it("composes presets with explicit overrides", () => {
    expect(preset("crypto-only", { schemes: { solana_pay: false } })).toEqual({ preset: "crypto-only", schemes: { solana_pay: false } });
  });
});
