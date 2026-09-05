# PaymentIntent schema v1

`PaymentIntent` is the cross-runtime contract. JSON uses camelCase for intent fields and stable
SCREAMING_SNAKE_CASE error codes.

Important semantics:

- `subtype`: a finer shape within `scheme`, when one payload family covers meaningfully different
  flows a developer would want to branch on - e.g. `paypal`'s `paypal_me` vs `invoice_qr`. Omitted
  when the scheme has no subtypes. Not a closed enum on the wire: new subtype strings can appear
  as detection for a scheme's other shapes improves, without a schema version bump.
- `possibleScheme` / `identification` / `confidence`: `scheme` is only ever a standard the parser
  actually confirmed - it is never the library's best guess. Some national EMVCo overlays (KHQR,
  LankaQR, Bangla QR, Raast QR, MMQR, Lao QR, JPQR, TWQR, ZeroPay, Mercado Pago) currently have no
  publicly confirmed scheme-specific GUID, only a country tag on top of the generic EMVCo MPM
  envelope. A country match alone can't rule out some other, unrelated EMVCo QR issued in that
  country, so those results report `scheme: "emvco_mpm"` (what's actually confirmed),
  `possibleScheme` (the specific standard it's most likely to be), `identification:
  "COUNTRY_LEVEL_INFERENCE"`, and `confidence: "medium"`. Once a GUID is confirmed for one of
  these, it moves to `scheme` directly with `identification: "GUID_MATCH"` and `confidence:
  "high"`, the same as every other GUID-confirmed national overlay already gets. All three fields
  are omitted for schemes whose detection is unambiguous by construction (a URI scheme prefix, a
  checksum-valid address format, a confirmed GUID, ...) - never invented where a scheme's adapter
  doesn't actually have graded confidence.
- `recognized`: a registered adapter confidently identified the payload family.
- `validation.valid`: the parser verified the implemented structural rules.
- `supported`: application policy allows the recognized scheme and the adapter can normalize it.
- `amount`: exact decimal text. It is never parsed through IEEE-754 floating point.
- `asset.amountAtomic`: exact base-unit integer when the standard supplies one.
- `validation.warnings`: limitations such as unverified recipient identity.
- `support.details`: structured, reason-specific detail a host UI can act on without parsing
  `support.message` - e.g. `{"network":"ethereum","allowedNetworks":["base","bnb","solana"]}` for
  `NETWORK_UNSUPPORTED`. Shape depends on `support.reason`; omitted when `enabled` is true or the
  reason needs no detail beyond the message.
- `recommendedAction`: descriptive only. The library never executes it. Exactly one of
  `scheme`/`network`/`provider` is populated, matching `type`: `scheme` for `handoff`/`deeplink`
  (the URI scheme to hand off to, e.g. `"upi"`), `network` for `wallet` (the chain a crypto wallet
  should handle), `provider` for `redirect` (the web provider, e.g. `"paypal"`).
- `ScannerOptions.accept.crypto`: network id -> allowed asset symbols, for accepting the `crypto`
  category down to specific chains/tokens (e.g. `{"base":["USDC"]}`). Omitted applies no
  crypto-specific restriction. An asset whose identity can't be confirmed from the QR alone (e.g.
  an ERC-20 transfer where only the contract address is known) is rejected with
  `ASSET_UNVERIFIABLE` rather than guessed - see `Asset.symbol`'s doc comment on not inferring
  token identity from a bare contract address.

These axes are deliberately independent. A result can be recognized and valid while unsupported
because the application disabled it. It can also be recognized and malformed.

The canonical Rust definition is in `crates/core/src/model.rs`; TypeScript declarations mirror its
serialized shape in `packages/core/src/types.ts`.

