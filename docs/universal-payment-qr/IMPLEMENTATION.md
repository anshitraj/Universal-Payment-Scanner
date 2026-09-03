# Universal Payment QR implementation plan

## Phase 1: deterministic core

- [x] Versioned schema and stable errors
- [x] Registry and independent capability policy
- [x] Bounded URI/TLV parsing and exact decimal handling

## Phase 2: verified first-release adapters

- [x] UPI, EMVCo MPM, and Pix
- [x] Bitcoin BIP-21, Ethereum ERC-681, and Solana Pay
- [x] PayPal.Me beta and Lightning experimental identification

## Phase 3: runtime boundaries

- [x] WASM wrapper and TypeScript SDK
- [x] Stateless Go HTTP API delegating to Rust
- [x] Browser QR decoder package

## Phase 4: product surface

- [x] React component and headless hook
- [x] Camera, upload, drag/drop, paste, policy controls, and normalized JSON
- [x] Accessible mobile-first playground

## Phase 5: assurance and release readiness

- [x] Unit, fixture, Go, TypeScript, malformed-input, and disabled-scheme tests
- [x] Fuzz target (now wired into CI as a 60s smoke run), reproducible parser benchmark, CI,
      security and contributor docs
- [ ] Add full BOLT-11 validation before promoting Lightning

## Phase 6: national EMVCo overlay expansion (top 25 countries)

- [x] Config-driven `national_emv.rs` adapter so a country is a config entry, not a new parser
- [x] PromptPay, VietQR, PayNow, DuitNow QR, QRIS, QR Ph, HKQR, NepalPay QR - confirmed GUID, beta
- [x] KHQR, LankaQR, Bangla QR, Raast QR, MMQR, Lao QR, JPQR, TWQR, ZeroPay - country-tag detection
      pending a confirmed GUID, community maturity
- [x] Alipay, WeChat Pay - proprietary detect-only stubs, same pattern as Lightning
- [ ] Confirm exact scheme GUIDs for the 9 community-maturity overlays above, promote to beta
- [ ] Bhutan, Brunei, Mongolia (QPay), Kazakhstan (Unified QR), Kyrgyzstan (ELQR): no public
      payload specification found in this round's research - revisit if one is published
- [ ] Round 2 (deferred by request): UAE, Bahrain, Qatar, Turkey, Egypt, and the Africa/Europe/
      Latin America countries from the original wishlist

