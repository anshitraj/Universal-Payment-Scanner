# Architecture

## Boundary

Image decoding and payment interpretation are separate. Camera or image input is decoded to an
opaque string by `packages/scanner`; the same raw string can be supplied directly.

```text
camera / image / string
          |
          v
      QR decoder        (browser concern; optional)
          |
          v
      Rust registry     (detection winner, bounded input)
          |
          v
      scheme adapter    (parse + validate + normalize)
          |
          v
   capability policy    (support decision after recognition)
          |
          v
   PaymentIntent v1
```

The Rust crate is the only payment parser. WASM invokes it in browsers. The Go service invokes its
CLI protocol and can later switch that transport to a native ABI without changing HTTP behavior.

## Determinism and privacy

Adapters make no network calls, read no clock, and store no state. The same payload, registry
version, and capability policy produce the same JSON. Raw payloads are not logged by the Go API.

## Adapter contract

`PaymentScheme` has three operations:

- `metadata`: immutable scheme identity, maturity, features, and public references.
- `detect`: cheap confidence score without applying application policy.
- `parse`: bounded parsing, validation, and normalized intent construction.

The registry selects the highest detector score above 70. Policy is applied only after parsing,
which guarantees that a disabled scheme cannot be mislabeled as an invalid QR.

## Versioning

`schemaVersion` versions normalized output independently from package versions. Scheme metadata
holds the standard and parser versions. New optional fields and new scheme IDs are additive.

