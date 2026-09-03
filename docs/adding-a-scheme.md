# Adding a payment scheme

## Fast path: a national EMVCo overlay

Many national QR standards (PromptPay, DuitNow, QRIS, VietQR, PayNow, ...) are not their own wire
format - they are the same EMVCo Merchant-Presented Mode envelope `crates/core/src/emv.rs` already
parses, with a country-specific GUID registered inside the merchant account information fields
(tags 26-51). For one of these, do not write a new parser module: add one config entry to
`OVERLAYS` in `crates/core/src/schemes/national_emv.rs` via the `overlay!` macro.

- If you have a public source confirming the exact GUID string (an official spec, or an
  independent open-source parser/SDK you can cite), pass it in `guids` and use `Maturity::Beta`.
  `parse` will verify that GUID actually tags a merchant account sub-template, not just that the
  string appears somewhere in the payload.
- If you've confirmed the national standard is real and EMVCo-based but haven't pinned the exact
  GUID, pass an empty `guids` slice and use `Maturity::Community`. Detection falls back to the
  generic envelope plus the country field (tag 58) - honest, but say so in the maturity level.
- Add the currency's ISO 4217 numeric code to `emv::currency_from_numeric` if it isn't there yet.
- Add a positive test vector (and a negative one - wrong country, bad CRC) built the same way the
  existing ones were: construct the TLV fields programmatically (tag + 2-digit length + value) and
  compute the real CRC-16/CCITT-FALSE trailer against `crc16_ccitt_false` rather than hand-splicing
  a string - a single off-by-one in a hand-typed length byte produces a payload that looks
  plausible but parses wrong.

For anything that isn't an EMVCo overlay (a bespoke URI scheme, a proprietary format with no
public spec), follow the general process below.

1. Create one module in `crates/core/src/schemes/`.
2. Implement `PaymentScheme`: cheap `detect`, bounded `parse`, and honest `metadata`.
3. Register the adapter in `default_registry`. No orchestration changes are needed.
4. Add public or synthetic fixtures to `test-vectors/`; never use customer payment data.
5. Test valid, malformed, oversized, confusable, unknown-version, and disabled-policy cases.
6. Add the scheme to the compatibility table only after parser tests pass.

Rules:

- Use public/legal specifications and record the primary source URL in metadata.
- Do not fetch provider endpoints inside an adapter.
- Do not infer identity, asset decimals, expiry, or “safety” without evidence in the payload.
- Bound field counts, nesting, decoded sizes, and numeric length before allocating.
- Keep amounts as strings or integers. Never use floating point.
- Unknown mandatory extensions must fail deterministically.
- Detection must be narrower than “this is a URL.”

Maturity meanings:

- `stable`: required fields and integrity checks have tested coverage.
- `beta`: useful public structure is parsed but provider-side meaning remains limited.
- `experimental`: confidently detected, not fully validated or normalized.
- `community`: externally maintained adapter with documented ownership.
- `deprecated`: kept only for compatibility.

