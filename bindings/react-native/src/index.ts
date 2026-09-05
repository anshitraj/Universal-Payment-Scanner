/**
 * React Native bindings for unipayscan. Every function here has the exact shape its
 * WASM/Python/Kotlin/Dart counterparts do, so app code reads identically across platforms - but
 * every one of them currently throws {@link NativeModuleNotLinkedError}. See README.md for why
 * (short version: Hermes doesn't support WebAssembly, so this can't just reuse
 * `unipayscan`'s WASM build the way a plain web or Node app does) and what's
 * needed to make it real: an Android Kotlin native module wrapping the already-built,
 * already-tested `bindings/android` package, and an iOS Swift native module wrapping
 * `bindings/swift` (which itself needs a Mac to even compile - see that package's README).
 */

export class NativeModuleNotLinkedError extends Error {
  constructor(fn: string) {
    super(
      `react-native-universal-payment-scanner: ${fn}() has no native implementation yet. ` +
        "This package's TypeScript interface is defined, but android/ and ios/ native module " +
        "wiring is not - see this package's README.md for exactly what's missing and why, " +
        "including a pure-JS hosted-API fallback that needs no native code.",
    );
  }
}

function notLinked(fn: string): never {
  throw new NativeModuleNotLinkedError(fn);
}

/** Matches `CapabilityPolicy`'s JSON shape - see `crates/core/src/policy.rs`. */
export interface CapabilityPolicy {
  preset?: "india" | "asia-pacific" | "banking-only" | "crypto-only" | "all-stable" | "all";
  schemes?: Record<string, boolean>;
  categories?: Record<string, boolean>;
  countries?: Record<string, Record<string, boolean>>;
}

/** A normalized PaymentIntent - see docs/schema.md in the repository root. */
export type PaymentIntent = Record<string, unknown>;

export async function parsePaymentQR(
  payload: string,
  policy?: CapabilityPolicy,
): Promise<PaymentIntent> {
  void payload;
  void policy;
  notLinked("parsePaymentQR");
}

export async function detectPaymentQR(
  payload: string,
  policy?: CapabilityPolicy,
): Promise<PaymentIntent> {
  void payload;
  void policy;
  notLinked("detectPaymentQR");
}

export async function getCapabilities(policy?: CapabilityPolicy): Promise<PaymentIntent[]> {
  void policy;
  notLinked("getCapabilities");
}

export async function getSupportedSchemes(policy?: CapabilityPolicy): Promise<PaymentIntent[]> {
  void policy;
  notLinked("getSupportedSchemes");
}

export interface Scanner {
  scan(payload: string): Promise<PaymentIntent>;
  detect(payload: string): Promise<PaymentIntent>;
  schemes(): Promise<PaymentIntent[]>;
}

export function createScanner(policy?: CapabilityPolicy): Scanner {
  void policy;
  return {
    scan: (payload) => parsePaymentQR(payload, policy),
    detect: (payload) => detectPaymentQR(payload, policy),
    schemes: () => getCapabilities(policy),
  };
}
