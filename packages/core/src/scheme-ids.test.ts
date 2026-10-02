import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { ALL_SCHEME_IDS } from "./types.js";

// registry/schemes.json is generated from the Rust registry itself (`npm run registry:generate`),
// so it is the authority on which scheme ids exist. ALL_SCHEME_IDS is hand-maintained; when it
// falls behind, an app's enabled-list silently stops controlling the schemes it forgot (twelve
// had drifted out before this test existed).
const registry = JSON.parse(readFileSync(new URL("../../../registry/schemes.json", import.meta.url), "utf8")) as {
  schemes: { scheme: string }[];
};

describe("ALL_SCHEME_IDS", () => {
  it("lists exactly the schemes in the generated registry", () => {
    expect([...ALL_SCHEME_IDS].sort()).toEqual(registry.schemes.map((s) => s.scheme).sort());
  });

  it("has no duplicates", () => {
    expect(new Set(ALL_SCHEME_IDS).size).toBe(ALL_SCHEME_IDS.length);
  });
});
