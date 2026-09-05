# react-native-universal-payment-scanner

## The real constraint, stated plainly

Every other JS-capable SDK in this project (`packages/core`) works by loading a WASM build of
`crates/core`. **That doesn't work in React Native.** React Native's default JS engine, Hermes,
does not implement WebAssembly - there is no `WebAssembly.instantiate` to call. This isn't a
packaging problem this binding can route around; it's a missing engine capability. (An app that
disables Hermes and ships JSC instead could load WASM - but that's a per-app engine choice this
package can't make for you, and it gives up Hermes' startup-time and memory advantages to get it.)

So a React Native package has two honest options, not one:

1. **Native modules** wrapping the same on-device bindings this repository already built and
   verified for [Android/Kotlin](../android) and [iOS/Swift](../swift) - the parsing stays fully
   local, no network call, exactly like every other platform in this project.
2. **The hosted API** (`services/api-go`) - works from pure JS with zero native code, but means
   every scan leaves the device. That trade-off should be the integrating app's explicit choice,
   never this package's silent default - see the repository root README's privacy section.

This package takes path 1. Path 2 is one HTTP call away if an app wants it (see "Hosted API
fallback" below) but is never automatic.

## Status: interface defined, native module wiring not built

What exists: the TypeScript surface below (`src/index.ts`), matching every other SDK in this
project (`parse`, `detect`, `createScanner`, `getCapabilities`). What doesn't exist yet: the
actual `NativeModules.UniversalPaymentQR` implementation - a Kotlin class in `android/` calling
[`org.universalpaymentqr.UniversalPaymentQR`](../android) (that binding is built and tested;
wiring it into RN's bridge or a TurboModule is what's missing), and an equivalent Swift/Obj-C
class in `ios/` calling [`UniversalPaymentQR`](../swift) (which itself hasn't been compiled at
all - see that package's README).

Building and verifying those needs a real React Native app project (for autolinking/codegen) and,
for the iOS half, a Mac - neither was available in the session that built this. Calling anything
in this package right now throws `NativeModuleNotLinkedError` with a message pointing back here.

```ts
import { parsePaymentQR } from "react-native-universal-payment-scanner";

// Throws NativeModuleNotLinkedError until android/ and ios/ are wired up - see above.
const result = await parsePaymentQR(payload);
```

## Hosted API fallback

For a pure-JS path with no native module work, call the hosted API directly instead of this
package - it runs the exact same Rust core (`services/api-go` invokes the same CLI the local SDKs
embed, so results are byte-for-byte identical), at the cost of sending the payload over the
network:

```ts
const response = await fetch("https://your-deployment/v1/parse", {
  method: "POST",
  headers: { "content-type": "application/json" },
  body: JSON.stringify({ payload }),
});
const result = await response.json();
```
