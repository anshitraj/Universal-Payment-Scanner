# Universal Payment QR progress

## Status: country expansion in progress, 2026-09-04

Scheme count grew from 8 to 27 in one session: 17 national EMVCo overlays plus 2 proprietary
detect-only stubs, covering 20 of the 25 requested countries honestly (5 had no public spec surface
and were deliberately left undone rather than fabricated - see IMPLEMENTATION.md Phase 6 and
README's compatibility table). Full verification (below) re-ran clean after the expansion, including
a second real bug found by the new tests: `Scanner::scan` was collapsing a scheme's own specific
unsupported reason into a generic `SCHEME_DISABLED` whenever policy also excluded its maturity
level - fixed, see CHANGELOG.md.

## Status: verified 2026-09-04 (pre-expansion baseline)

Fresh `cargo fmt`/`clippy`/`test`, `go vet`/`test`, `npm run typecheck`/`test`/`build`, `npm audit`,
and `cargo audit` all pass. The full workspace build (WASM + all TypeScript packages + playground)
was rebuilt from a clean state, and the Go API was exercised end to end against the release Rust
CLI binary. The playground was driven in a real browser (desktop and mobile viewports) against the
production WASM build: recognized/valid, recognized/disabled, and not-a-payment-QR states all
render correctly with zero console errors.

That verification pass found and fixed one real bug: `EmvCo::detect` and `emv::validate_crc`
indexed the payload string using byte-length arithmetic instead of on `.as_bytes()`, so a payload
with a multi-byte UTF-8 character near its tail panicked instead of returning a parse error —
reachable from arbitrary input because every scheme's `detect` runs on every payload. See
CHANGELOG.md. Fixed, covered by two regression tests plus two shared test-vector cases, and
re-verified against the rebuilt release binary and the rebuilt browser WASM.

## Architectural decisions

- Rust is the only payment parser; downstream runtimes adapt it.
- Capability policy runs after detection and parsing.
- No network calls, automatic navigation, signing, or payment execution.
- Exact decimal strings and bounded fields are part of the public contract.
- Scheme maturity is explicit and incomplete formats are not promoted.

