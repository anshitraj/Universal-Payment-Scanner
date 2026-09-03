import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const { WasmScanner } = require("../generated-node/index.js") as {
  WasmScanner: new (policy: unknown) => {
    scan(payload: string): unknown;
    detect(payload: string): unknown;
    schemes(): unknown;
  };
};

export async function createWasmScanner(policy: unknown): Promise<{
  scan(payload: string): unknown;
  detect(payload: string): unknown;
  schemes(): unknown;
}> {
  return new WasmScanner(policy);
}

