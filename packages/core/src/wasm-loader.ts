import init, { WasmScanner } from "../generated/index.js";

let initialization: Promise<unknown> | undefined;

export async function createWasmScanner(policy: unknown): Promise<{
  scan(payload: string): unknown;
  detect(payload: string): unknown;
  schemes(): unknown;
}> {
  initialization ??= init();
  await initialization;
  return new WasmScanner(policy);
}
