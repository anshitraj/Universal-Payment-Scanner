# PaymentIntent schema v1

`PaymentIntent` is the cross-runtime contract. JSON uses camelCase for intent fields and stable
SCREAMING_SNAKE_CASE error codes.

Important semantics:

- `subtype`: a finer shape within `scheme`, when one payload family covers meaningfully different
  flows a developer would want to branch on - e.g. `paypal`'s `paypal_me` vs `invoice_qr`. Omitted
  when the scheme has no subtypes. Not a closed enum on the wire: new subtype strings can appear
  as detection for a scheme's other shapes improves, without a schema version bump.
- `recognized`: a registered adapter confidently identified the payload family.
- `validation.valid`: the parser verified the implemented structural rules.
- `supported`: application policy allows the recognized scheme and the adapter can normalize it.
- `amount`: exact decimal text. It is never parsed through IEEE-754 floating point.
- `asset.amountAtomic`: exact base-unit integer when the standard supplies one.
- `validation.warnings`: limitations such as unverified recipient identity.
- `recommendedAction`: descriptive only. The library never executes it.

These axes are deliberately independent. A result can be recognized and valid while unsupported
because the application disabled it. It can also be recognized and malformed.

The canonical Rust definition is in `crates/core/src/model.rs`; TypeScript declarations mirror its
serialized shape in `packages/core/src/types.ts`.

