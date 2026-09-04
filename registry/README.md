# UPS Scheme Registry

A machine-readable catalogue of every payment (and explicitly non-payment) QR/address format
Universal Payment QR recognizes: one JSON file per scheme, plus a generated aggregate, generated
directly from the live Rust core - never hand-written, never able to silently drift from what the
parser actually does.

```
registry/
  schema/scheme.schema.json   JSON Schema (draft 2020-12) for one entry - validate against this
  schemes/<id>.json           One file per scheme, e.g. schemes/upi.json, schemes/promptpay.json
  schemes.json                All of the above, as one array, with a generation manifest
```

## Fetching it

Every file here is plain JSON on GitHub, so it's fetchable with nothing but an HTTP client - no
SDK, no auth, no rate limit beyond GitHub's own:

```bash
curl -s https://raw.githubusercontent.com/anshitraj/Universal-Payment-Scanner/main/registry/schemes.json
curl -s https://raw.githubusercontent.com/anshitraj/Universal-Payment-Scanner/main/registry/schemes/upi.json
```

(jsDelivr's GitHub CDN mirrors the same paths under `cdn.jsdelivr.net/gh/anshitraj/Universal-Payment-Scanner@main/registry/...`
if you want caching in front of it.)

## Example entry

```json
{
  "scheme": "upi",
  "displayName": "UPI",
  "country": ["IN"],
  "category": "bank_transfer",
  "standard": "UPI deep link v1",
  "parserVersion": "0.1.0",
  "maturity": "stable",
  "identification": "exact",
  "supported": true,
  "nonPayment": false,
  "lastVerified": "2026-09-05",
  "supports": { "amount": true, "merchant": true, "currency": true, "dynamic": true },
  "references": [
    "https://www.npci.org.in/PDF/npci/upi/circular/2017/Circular18_BankCompliances_to_enbaleUPIMerchantecosystem_0.pdf"
  ]
}
```

## Field reference

| Field | Meaning |
|---|---|
| `scheme` | Stable machine id. Matches `PaymentIntent.scheme` in the runtime API exactly - this is the join key between a registry entry and a live parse result. |
| `displayName` | Human-readable name. |
| `country` | ISO 3166-1 alpha-2 codes this scheme is issued in. Empty for schemes with no single issuing country (generic EMVCo MPM, global crypto/link schemes). |
| `category` | `bank_transfer` \| `wallet` \| `card` \| `crypto` \| `payment_link` \| `unknown`. |
| `standard` | The named specification or governing document this scheme implements. |
| `parserVersion` | The `universal-payment-qr-core` crate version that produced this entry (semver). |
| `maturity` | `stable` \| `beta` \| `community` \| `experimental` \| `deprecated`. See the main [README](../README.md#what-is-implemented) for what each tier actually requires - this registry doesn't redefine them, only reports them. |
| `identification` | `exact` \| `generic` \| `heuristic` - see below. |
| `supported` | `false` only for a scheme that's detected but never fully parsed because no public payload spec exists (Alipay, WeChat Pay today). |
| `nonPayment` | `true` for the 3 entries that are deliberately not payment schemes (WalletConnect pairing, `otpauth://` setup, generic URL) but are still identified by name instead of falling into "unknown". |
| `lastVerified` | The date this entry was last regenerated from, and cross-checked against, the live parser and its test suite. This is a snapshot/regeneration date, **not** the date the scheme was first implemented - check `CHANGELOG.md` or `git log` on the scheme's source file for that. |
| `supports.amount` | The wire format has a field for a specific transfer amount. |
| `supports.merchant` | The wire format has a field for a recipient/merchant/beneficiary name. |
| `supports.currency` | The wire format has an explicit currency field, as opposed to an implied one (a crypto network's native asset isn't a "currency field"). |
| `supports.dynamic` | The scheme supports a dynamic QR - one that resolves the actual payment details from a URL/reference at scan time rather than encoding them statically. |
| `references` | Public source(s) this entry's identification method was verified against. |

### `identification`, and how it relates to the runtime API

This is the one field in the registry that isn't a direct pass-through of an existing
`SchemeMetadata` value - it's a 3-tier classification of **how confidently a match actually
identifies this specific scheme**, as opposed to a family it belongs to:

- **`exact`** - a scheme-specific signature (a confirmed GUID, a verified checksum algorithm, or a
  scheme-unique URI/domain) positively identifies this exact standard.
- **`generic`** - matched via a shared envelope plus a weaker signal (the generic EMVCo envelope
  plus a country tag) that cannot rule out an unrelated standard from the same family/country.
- **`heuristic`** - matched by shape or prefix only, with no structural or checksum verification.

For the national-EMVCo-overlay family specifically, this lines up exactly with the runtime
`PaymentIntent.identification` field added in schema 2.0.0: `GUID_MATCH` → `exact`,
`COUNTRY_LEVEL_INFERENCE` → `generic` (see `docs/schema.md`). That runtime field doesn't (yet)
cover non-EMVCo scheme types - a Bitcoin address's checksum verification and a bare Alipay link's
shape-only match don't get an explicit runtime `identification` value today - so this registry
field is deliberately a little broader: it's derived from each scheme's `maturity` (`stable`/`beta`
→ `exact`, `community` → `generic`, `experimental` → `heuristic`), with exactly one documented
override where maturity alone would understate a scheme's identification confidence. That override
- and the reasoning for it - is a comment directly in `scripts/generate-registry.mjs`, not hidden
in this doc; check there for the current, authoritative list of exceptions.

## Regenerating

```bash
npm run registry:generate   # rebuilds the Rust core, then rewrites registry/schemes/*.json + schemes.json
npm run registry:check      # same, but only reports drift (exit 1) instead of writing - CI-friendly
```

The generator (`scripts/generate-registry.mjs`) never invents a fact: every field is either a
direct pass-through of `Scanner::schemes()` - the same metadata already served to every language
binding - or a small, documented derivation from it (see the script's own comments). If a new
scheme is added to the Rust core, running `registry:generate` picks it up automatically; nothing
here needs to be hand-maintained per scheme.

## Contributing a correction

Most useful contribution: pinning one of the `community`-maturity schemes' scheme-specific GUID
(see the main README's "researched, not yet confirmed" list) - that's a change to the Rust core
itself (`crates/core/src/schemes/national_emv.rs`) followed by `npm run registry:generate`, not a
hand-edit to a file under `registry/schemes/`. A PR that edits a file in `registry/schemes/`
directly without a corresponding core change will fail `npm run registry:check` in CI.
