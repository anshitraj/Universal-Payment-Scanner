# Baseline results

Measured on 2026-09-04 with Rust 1.92.0 on the local Windows x64 development host, release profile:

| Measurement | Result |
|---|---:|
| Mixed UPI/BIP-21/ERC-681/Solana parse loop | 1.846 µs / payload |
| Iterations | 100,000 |
| Browser WASM (uncompressed) | 430.15 kB |
| Browser WASM (gzip) | 164.25 kB |
| Playground initial JS (gzip) | 68.50 kB |
| Lazy QR decoder JS (gzip) | 107.68 kB |

Run `cargo bench -p universal-payment-qr-core --bench parse` to reproduce parser latency. Results
are a baseline, not a cross-device guarantee. Peak memory and camera startup instrumentation remain
future benchmark work.

