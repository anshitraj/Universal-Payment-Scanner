# Testing and test vectors

Unit tests sit beside each parser. `test-vectors/v1.json` provides cross-runtime inputs and expected
scheme/support outcomes. Fixtures are synthetic or taken from public specification examples.

Security regressions cover oversized payloads, null bytes, malformed TLV lengths, duplicate tags,
invalid CRCs, invalid recipients, decimal scale, unknown required BIP-21 fields, and disabled
schemes. The fuzz target passes arbitrary UTF-8 lossily to the registry and asserts only that the
process returns without panic.

Parity is enforced architecturally because WASM compiles the Rust crate and Go delegates to the
Rust CLI. Release CI builds and tests every boundary.

