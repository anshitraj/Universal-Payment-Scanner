# Universal Payment QR

> **One scanner for every payment QR.**  
> **Detect any supported payment standard. Enable only the ones your app accepts.**

Universal Payment QR is open-source payment QR intelligence. It recognizes, parses, validates,
normalizes, and classifies a decoded QR payload. It does **not** process payments, hold funds,
open links automatically, sign transactions, or verify recipient identity.

```ts
import { createScanner } from "@universal-payment-qr/core";

const scanner = createScanner({
  schemes: { upi: true, pix: true, bitcoin: true, ethereum: true, solana_pay: false },
});

const result = await scanner.scan(qrData);
```

A disabled scheme remains recognized:

```json
{
  "recognized": true,
  "scheme": "solana_pay",
  "supported": false,
  "support": {
    "enabled": false,
    "reason": "SCHEME_DISABLED",
    "messageKey": "scanner.scheme_disabled"
  }
}
```

## What is implemented

| Scheme | Country | Maturity | What is verified | Network calls |
|---|---|---|---|---|
| UPI deep links | IN | Stable | URI, required payee address, decimal amount, INR | None |
| EMVCo MPM | Global | Stable | bounded TLV structure, required format, CRC-16 | None |
| Pix / BR Code | BR | Stable | EMV structure/CRC, Pix GUI, key or dynamic URL fields | None |
| Bitcoin BIP-21 | Global | Stable | URI, address encoding/checksum, exact BTC amount, required params | None |
| Ethereum ERC-681 | Global | Stable | target, chain ID, native amount, ERC-20 transfer shape | None |
| Solana Pay | Global | Stable | recipient/mint/reference public keys, exact decimal amount | None |
| PromptPay | TH | Beta | EMV structure/CRC, confirmed GUID `A000000677010111` in a merchant sub-template | None |
| VietQR | VN | Beta | EMV structure/CRC, confirmed GUID `A000000727`, bank BIN/account shape | None |
| PayNow | SG | Beta | EMV structure/CRC, confirmed GUID `SG.PAYNOW` | None |
| DuitNow QR | MY | Beta | EMV structure/CRC, confirmed GUID `A0000006150001` | None |
| QRIS | ID | Beta | EMV structure/CRC, confirmed GUID `ID.CO.QRIS.WWW` | None |
| QR Ph | PH | Beta | EMV structure/CRC, confirmed GUID `PH.INSTAPAY.ME` | None |
| HKQR | HK | Beta | EMV structure/CRC, confirmed GUID `HK.COM.HKICL` | None |
| NepalPay QR | NP | Beta | EMV structure/CRC, confirmed GUID `np.gov.nrb` | None |
| KHQR | KH | Community | EMV structure/CRC, generic country-tag detection (GUID not yet confirmed) | None |
| LankaQR | LK | Community | EMV structure/CRC, generic country-tag detection (GUID not yet confirmed) | None |
| Bangla QR | BD | Community | EMV structure/CRC, generic country-tag detection (GUID not yet confirmed) | None |
| Raast QR | PK | Community | EMV structure/CRC, generic country-tag detection (GUID not yet confirmed) | None |
| MMQR | MM | Community | EMV structure/CRC, generic country-tag detection (GUID not yet confirmed) | None |
| Lao QR | LA | Community | EMV structure/CRC, generic country-tag detection (GUID not yet confirmed) | None |
| JPQR | JP | Community | EMV structure/CRC, generic country-tag detection (GUID not yet confirmed) | None |
| TWQR | TW | Community | EMV structure/CRC, generic country-tag detection (GUID not yet confirmed) | None |
| ZeroPay | KR | Community | EMV structure/CRC, generic country-tag detection (GUID not yet confirmed) | None |
| Mercado Pago | AR | Community | EMV structure/CRC, generic country-tag detection (BCRA mandates EMVCo; GUID not yet confirmed) | None |
| PayPal | Global | Beta | public link shape (`paypal_me`, `invoice_qr` subtypes) | None |
| Venmo | US | Beta | public profile link shape (`/u/<handle>` only, to avoid misreading other venmo.com pages) | None |
| Cash App | US, GB | Beta | public `$cashtag` link shape | None |
| Lightning BOLT-11 | Global | Experimental | recognizable invoice prefix and bounds only | None |
| Alipay | CN | Experimental | link shape only; proprietary, `PROPRIETARY_FORMAT` | None |
| WeChat Pay | CN | Experimental | link shape only; proprietary, `PROPRIETARY_FORMAT` | None |
| TRON (TRC-20) | Global | Beta | Base58Check address, double-SHA256 checksum | None |
| TON | Global | Beta | user-friendly address, CRC16/XMODEM checksum | None |
| XRP Ledger | Global | Beta | classic address, XRPL base58 alphabet, checksum | None |
| Stellar | Global | Beta | strkey ed25519 public key, CRC16/XMODEM checksum | None |
| WalletConnect | Global | Beta | pairing URI shape; recognized, flagged `NOT_PAYMENT_QR` | None |
| Authenticator (OTP) setup | Global | Beta | `otpauth://` shape; secret never extracted or stored | None |
| Generic URL | Global | Beta | any other well-formed `http(s)://` link; flagged `NOT_PAYMENT_QR` | None |
| SEPA / EPC QR (Girocode) | EU/SEPA | Beta | EPC069-12 line format, real IBAN MOD-97 checksum | None |
| Swiss QR-bill | CH, LI | Experimental | SPC v2.x format, IBAN checksum; debtor-block field count and the QRR reference's own check digit not independently confirmed | None |

"Valid" means structurally valid. It never means that a recipient is trustworthy or that a
payment is safe. "Community" maturity means the national standard and its EMVCo basis are
confirmed from a public source, but the exact scheme GUID hasn't been pinned yet - contributions
welcome, see [adding a scheme](docs/adding-a-scheme.md).

Several schemes above are structural families rather than one shape: `PaymentIntent.subtype`
distinguishes them (e.g. `paypal` + `paypal_me` vs `paypal` + `invoice_qr`). See
[schema](docs/schema.md).

WalletConnect, Authenticator setup, and Generic URL are deliberately not payment schemes. A
scanner that can only say "unknown QR" for a wallet-connection request or a 2FA setup code is
less trustworthy than one that says what it actually found - see
[architecture](docs/architecture.md).

Bhutan, Brunei, Mongolia (QPay), Kazakhstan (Unified QR), Kyrgyzstan (ELQR), and Zelle (no public
payload spec exists at all - confirmed bank-proprietary, no public API or URI scheme) are not yet
covered: research for this release did not surface a usable public payload specification for any
of them, and this project does not ship a parser it cannot verify against one. Standalone TWINT
(distinct from a Swiss QR-bill a TWINT app can pay) and PayPal's seller/payment-link QR subtypes
are similarly deferred pending a confirmed URL pattern.

## Other languages and platforms

Every binding below calls the exact same Rust core (`crates/core`) - no payment logic is ever
reimplemented. "Verified" means real tests ran against a real build in this repository's own
development session, not just that the code compiles by inspection.

| Platform | Package | Status |
|---|---|---|
| Rust | `crates/core` (crates.io-ready) | Verified - 66 unit tests, fuzzed, benchmarked |
| Browser / Node | [`@universal-payment-qr/core`](packages/core) (WASM) | Verified - live in the playground |
| Python | [`bindings/python`](bindings/python) | Verified - 10/10 pytest, real built wheel |
| Kotlin / Android | [`bindings/android`](bindings/android) | Verified - 9/9 JVM unit tests, real `.aar` built |
| Flutter / Dart | [`bindings/flutter`](bindings/flutter) | Verified - 8/8 `flutter test`, Android packaging not yet built |
| Swift / iOS | [`bindings/swift`](bindings/swift) | Written, **not compiled** - this repo was built on Windows; needs a Mac |
| React Native | [`bindings/react-native`](bindings/react-native) | Interface only - see its README for why (Hermes has no WebAssembly) |
| Go (hosted API) | [`services/api-go`](services/api-go) | Verified - delegates to the Rust CLI, same output |

## Repository map

```text
crates/core/           Rust registry, policy, parsers, validation, CLI
crates/wasm/            wasm-bindgen wrapper over the Rust core
crates/ffi/             C ABI over the core, for Swift/Kotlin/anything else that isn't WASM or Rust
services/api-go/       Optional stateless HTTP service; delegates to the Rust CLI
bindings/python/       PyO3 bindings (pip install universal-payment-qr)
bindings/android/      JNI bindings (Kotlin, Maven/Gradle)
bindings/flutter/      dart:ffi bindings over crates/ffi
bindings/swift/        Swift Package Manager wrapper over crates/ffi (uncompiled - needs a Mac)
bindings/react-native/ Interface + architecture notes (native module wiring not built)
packages/wasm/         Generated WASM package wrapper
packages/core/         TypeScript SDK
packages/scanner/      Browser camera and image decoding (ZXing)
packages/react/        Accessible ready-made scanner and headless hook
apps/playground/       Local-first developer playground
test-vectors/          Public/synthetic valid and malicious fixtures
fuzz/                  cargo-fuzz targets
docs/                  Architecture, schema, adapters, security, versioning
```

## Quick start

Prerequisites: Rust 1.92+, Node 24+, npm 11+, and `wasm-pack` 0.15+.

For an application consuming a published release:

```bash
npm install @universal-payment-qr/core
```

For this repository:

```bash
npm install
npm run build
npm run dev
```

Rust:

```rust
use universal_payment_qr_core::{parse_payment_qr, ErrorCode};

let result = parse_payment_qr("upi://pay?pa=merchant%40bank&am=499.00&cu=INR");
assert!(result.recognized);
```

Go API:

```bash
cargo build --release -p universal-payment-qr-core
cd services/api-go
UPQR_CORE_BIN=../../target/release/upqr-core go run ./cmd/server
```

```bash
curl -s http://localhost:8080/v1/parse \
  -H 'content-type: application/json' \
  -d '{"payload":"bitcoin:1BoatSLRHtKNngkdXEeobR76b53LETtpyT?amount=0.001"}'
```

React:

```tsx
import { PaymentQRScanner } from "@universal-payment-qr/react";
import "@universal-payment-qr/react/styles.css";

<PaymentQRScanner
  enabledSchemes={["upi", "pix", "bitcoin", "ethereum"]}
  onDetected={(intent) => console.log(intent)}
  onUnsupported={(intent) => console.log(intent.support.reason)}
  onError={(error) => console.error(error)}
/>
```

## Public APIs

The TypeScript SDK exports `detectPaymentQR`, `parsePaymentQR`, `validatePaymentQR`,
`normalizePaymentQR`, `createScanner`, `getSupportedSchemes`, `getCapabilities`, and `preset`.
The Rust crate exports equivalent free functions plus `Scanner` and `CapabilityPolicy`.

See [architecture](docs/architecture.md), [schema](docs/schema.md), [adding a scheme](docs/adding-a-scheme.md),
[mobile integration](docs/mobile-integration.md), [deployment](docs/deployment.md),
[benchmarks](benchmarks/results.md), [security](SECURITY.md), and [contributing](CONTRIBUTING.md).

## Development

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd services/api-go && go test ./...
npm run typecheck && npm test && npm run build
```

No credentials are required for the core, WASM SDK, scanner, or playground. The Go service only
uses `UPQR_CORE_BIN` and `UPQR_LISTEN_ADDR`.

## License

Licensed under the MIT License.
