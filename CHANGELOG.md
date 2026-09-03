# Changelog

## 0.1.0 - Unreleased

- Fixed a WASM/native parity bug found while manually checking a new scheme's output in-browser:
  `crates/wasm` serialized `PaymentIntent` with `serde_wasm_bindgen::to_value`'s default
  serializer, which represents a Rust map as a JS `Map` instance rather than a plain object. A JS
  `Map` has no enumerable own properties, so `JSON.stringify` - on it, or on anything containing
  it - silently renders `{}`. `PaymentIntent.metadata` is exactly that kind of field, so every
  scheme that populates it (EMVCo merchant fields, Pix's dynamic URL, TON's bounceable flag, Swiss
  QR's reference type, WalletConnect's protocol version, ...) was silently losing that field for
  every consumer of the WASM/JS SDK - the browser playground included - while the native Rust and
  Go paths kept it intact. Nothing in the existing test suite exercised WASM output through actual
  JSON serialization (the vitest tests use an injected engine; `vectors.rs` only runs native Rust),
  so this shipped undetected until scanning a Swiss QR-bill payload live. Fixed by switching to
  `serde_wasm_bindgen::Serializer::json_compatible()`, which serializes maps as plain objects.
- Added `PaymentIntent.subtype` (schema 1.1.0, additive/minor) for schemes with more than one
  meaningfully different flow. Restructured `paypal` onto it: `paypal_me` and `invoice_qr`
  subtypes, both from public link patterns alone (no PayPal API access needed or used).
- Added Venmo (`venmo`, public `/u/<handle>` profile links only - deliberately not the bare-handle
  business form, which is indistinguishable from any other venmo.com page) and Cash App
  (`cash_app`, public `$cashtag` links).
- Added Mercado Pago / Transferencias 3.0 (`mercado_pago`, AR) as a `community`-maturity national
  EMVCo overlay - Argentina's central bank mandates EMVCo QRCPS for interoperable QR.
- Added four crypto address schemes recognizing a bare, checksum-valid address (no URI scheme,
  matching how wallet apps' receive screens actually show them): TRON (`tron`, Base58Check,
  double-SHA256), TON (`ton`, user-friendly address, CRC16/XMODEM), XRP Ledger (`xrp`, classic
  address, XRPL's own base58 alphabet), and Stellar (`stellar`, strkey ed25519 key, CRC16/XMODEM).
  Every checksum algorithm was confirmed against each network's own reference implementation
  source before being implemented, not assumed from a spec name alone - TON's and Stellar's CRC16
  in particular, since "CRC16-CCITT" ambiguously refers to several different-init variants.
- Added non-payment QR recognition: WalletConnect pairing URIs (`walletconnect`) and `otpauth://`
  2FA setup codes (`otp_setup`, secret never extracted) are now identified by name rather than
  falling into the generic "unknown" bucket. Added a generic-URL fallback (`url`) so an ordinary
  website link reads as "recognized, not a payment" (`NOT_PAYMENT_QR`) instead of "unrecognized" -
  this is a deliberate behavior change from earlier 0.1.0 snapshots, where a plain `https://` link
  was fully unrecognized; see `docs/architecture.md`.
- Added SEPA/EPC QR - "Girocode" (`epc_qr`, EPC069-12 v2.1) and Swiss QR-bill (`swiss_qr_bill`,
  SIX's v2.x SPC format), both plain-text line-based formats rather than EMVCo TLV. Both validate
  the IBAN with a real ISO 7064 MOD 97-10 check-digit implementation (`checksums::validate_iban`),
  not just a shape check. Swiss QR-bill's debtor-address block has a line count this project could
  not fully confirm against a primary source, so its reference/message fields are anchored from
  the back off the literal "EPD" trailer rather than assumed fixed positions; kept at
  `experimental` rather than `beta` to reflect that residual uncertainty honestly, and the QRR
  reference's own check-digit algorithm (distinct from IBAN's) is not independently verified yet.
- Researched and explicitly did not implement: Zelle (confirmed no public payload spec, no
  `zelle://` scheme, no public API - bank-proprietary by design) and standalone TWINT (no payload
  shape confirmed distinct from a Swiss QR-bill a TWINT app can pay). Also deferred: PayPal's
  seller/payment-link QR subtypes, pending a confirmed URL pattern.
- Initial Rust registry, normalized schema, application capability policy, and stable error model.
- UPI, EMVCo MPM, Pix, Bitcoin BIP-21, Ethereum ERC-681, Solana Pay, and PayPal.Me adapters.
- Experimental Lightning identification.
- WASM, TypeScript, React, browser scanner, Go API, playground, fixtures, fuzzing, and CI.
- Fixed: `EmvCo::detect` and `emv::validate_crc` indexed the raw payload string by byte-length
  arithmetic instead of on `.as_bytes()`, so a multi-byte UTF-8 character positioned near the
  payload's tail (an emoji or accented character in an otherwise EMV-shaped string) panicked
  instead of failing detection. Every scheme's `detect` runs on every payload, so this was
  reachable from arbitrary input, not just malformed EMV QR codes. Fixed to slice bytes
  throughout; covered by two Rust regression tests and two shared `test-vectors/v1.json` cases.
- Added a `messageKey` for `UNVERIFIED_RECIPIENT` (`scanner.unverified_recipient`) instead of
  folding it into the generic `scanner.invalid_qr` key; it is a warning on an otherwise-valid
  parse, not an invalid-QR condition.
- Added a CI job that runs a 60-second `cargo fuzz` smoke test against the `parse_payload`
  target on every push/PR, closing the gap between the fuzz target existing and it ever running.
- Added a config-driven national EMVCo overlay adapter (`crates/core/src/schemes/national_emv.rs`)
  and 17 country configs built on it: PromptPay (TH), VietQR (VN), PayNow (SG), DuitNow QR (MY),
  QRIS (ID), QR Ph (PH), HKQR (HK), and NepalPay QR (NP) with a confirmed scheme GUID (`beta`);
  KHQR (KH), LankaQR (LK), Bangla QR (BD), Raast QR (PK), MMQR (MM), Lao QR (LA), JPQR (JP), TWQR
  (TW), and ZeroPay (KR) via generic country-tag detection pending a confirmed GUID (`community`).
  Added detect-only proprietary stubs for Alipay and WeChat Pay (`experimental`, unsupported,
  `PROPRIETARY_FORMAT`) - same pattern as Lightning. Bhutan, Brunei, Mongolia (QPay), Kazakhstan
  (Unified QR), and Kyrgyzstan (ELQR) were researched and explicitly not implemented: no public
  payload specification surfaced for any of them.
- Added a `preset("asia-pacific")` convenience preset.
- Fixed: `Scanner::scan` unconditionally overwrote a scheme's own unsupported reason (e.g.
  `PROPRIETARY_FORMAT`, or Lightning's `SCHEME_UNSUPPORTED`) with a generic `SCHEME_DISABLED`
  whenever the active policy also happened to exclude that scheme's maturity level, collapsing two
  genuinely different, spec-required error codes into one. Found via the first test exercising a
  detect-only-unsupported scheme (Alipay) under the default policy - Lightning had no such test
  before. Now only overrides when the scheme's own `parse` considered it supported.
- Added `ALL_SCHEME_IDS` to `@universal-payment-qr/core` as the single source of truth for scheme
  ids, fixing a latent bug where `@universal-payment-qr/react`'s `enabledSchemes` prop silently
  ignored any scheme not in its own separately-hardcoded id list (would have affected every new
  national overlay scheme).
- Fixed (found via real-device testing): Bitcoin, Ethereum, and Solana Pay only recognized their
  `bitcoin:`/`ethereum:`/`solana:` URI form, not a bare address - which is what most wallet apps
  (Coinbase, Phantom, etc.) actually put in a "receive" QR. All three now also recognize a bare,
  checksum-valid address with no amount.
- Fixed (found via a real scanned VietQR): `validate_required_fields` hard-required merchant
  category code (52), merchant name (59), and merchant city (60) on every EMVCo-family payload.
  Real-world personal/P2P transfers (confirmed against a real VietQR) omit these merchant-only
  fields even though the base EMVCo spec calls them mandatory. Now validated when present, not
  required; currency (53), country (58), and the CRC remain mandatory.

