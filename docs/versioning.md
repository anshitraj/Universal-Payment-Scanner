# Versioning policy

Packages follow semantic versioning. `PaymentIntent.schemaVersion` follows semantic versioning
separately:

- Patch: clarification with identical serialized behavior.
- Minor: optional fields, error codes, or scheme IDs may be added.
- Major: field removal, renamed values, or changed meaning.

Adapters state their parser version and target standard in capability metadata. Adding a new scheme
does not change existing policy defaults except that `all` opts into it; `all-stable` only opts into
stable/beta adapters.

