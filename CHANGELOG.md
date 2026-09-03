# Changelog

## 0.1.0 - Unreleased

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

