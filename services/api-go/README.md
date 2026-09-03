# Go API service

This stateless service delegates every payment decision to the Rust `upqr-core` binary. It does
not log request bodies and makes no network calls to payment providers.

```bash
cargo build --release -p universal-payment-qr-core
$env:UPQR_CORE_BIN = "../../target/release/upqr-core.exe"
go run ./cmd/server
```

Environment variables:

- `UPQR_CORE_BIN` — path to the Rust CLI (default: `upqr-core` on `PATH`)
- `UPQR_LISTEN_ADDR` — listen address (default: `:8080`)

To run the Rust/Go HTTP parity test (`internal/httpapi/parity_test.go`, skipped by default), set
`UPQR_CORE_BIN` to an absolute path: `go test ./...` runs each package with that package's own
directory as its working directory, not the module root, so a path relative to `services/api-go`
(as used above for `go run`) will not resolve from inside `internal/httpapi`.

