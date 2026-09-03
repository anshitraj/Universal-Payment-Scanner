import { createWasmScanner } from "@universal-payment-qr/wasm";
import type { Detection, PaymentEngine, PaymentIntent, ScannerOptions, SchemeMetadata } from "./types.js";

export * from "./types.js";

class LazyWasmEngine implements PaymentEngine {
  readonly #policy: Omit<ScannerOptions, "engine">;
  #engine?: Promise<PaymentEngine>;

  constructor(options: ScannerOptions) {
    const { engine: _engine, ...policy } = options;
    this.#policy = policy;
  }

  #load(): Promise<PaymentEngine> {
    this.#engine ??= createWasmScanner(this.#policy) as Promise<PaymentEngine>;
    return this.#engine;
  }

  async scan(payload: string): Promise<PaymentIntent> {
    return (await this.#load()).scan(payload);
  }

  async detect(payload: string): Promise<Detection> {
    return (await this.#load()).detect(payload);
  }

  async schemes(): Promise<SchemeMetadata[]> {
    return (await this.#load()).schemes();
  }
}

export interface Scanner {
  scan(payload: string): Promise<PaymentIntent>;
  detect(payload: string): Promise<Detection>;
  validate(payload: string): Promise<PaymentIntent["validation"]>;
  getSupportedSchemes(): Promise<SchemeMetadata[]>;
  getCapabilities(): Promise<SchemeMetadata[]>;
}

export function createScanner(options: ScannerOptions = {}): Scanner {
  const engine = options.engine ?? new LazyWasmEngine(options);
  return {
    scan: async (payload) => engine.scan(payload),
    detect: async (payload) => engine.detect(payload),
    validate: async (payload) => (await engine.scan(payload)).validation,
    getSupportedSchemes: async () => (await engine.schemes()).filter((scheme) => scheme.maturity !== "experimental"),
    getCapabilities: async () => engine.schemes(),
  };
}

const defaultScanner = createScanner();

export const parsePaymentQR = (payload: string) => defaultScanner.scan(payload);
export const normalizePaymentQR = parsePaymentQR;
export const detectPaymentQR = (payload: string) => defaultScanner.detect(payload);
export const validatePaymentQR = (payload: string) => defaultScanner.validate(payload);
export const getSupportedSchemes = () => defaultScanner.getSupportedSchemes();
export const getCapabilities = () => defaultScanner.getCapabilities();

export function preset(name: NonNullable<ScannerOptions["preset"]>, overrides: Omit<ScannerOptions, "preset"> = {}): ScannerOptions {
  return { preset: name, ...overrides };
}

