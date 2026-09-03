# Universal Payment QR research

## Overview

The project needs a deterministic interpretation layer between generic QR decoders and payment
applications. Detection and application acceptance must remain independent.

## Recommended approach

Use a memory-safe Rust registry as the sole parser implementation. Compile it to WASM for browsers,
expose a small CLI boundary for the stateless Go service, and keep image decoding in a separate web
package. Model exact amounts as strings and atomic integers. Plugins are compile-time Rust trait
implementations for predictable performance and auditability.

## Primary references

- EMVCo QRCPS Merchant-Presented Mode v1.1 public resources
- NPCI UPI merchant deep-link circular
- Banco Central do Brasil Pix initiation manual 2.9.0
- BIP-21, ERC-681, and Solana Pay transfer-request specifications

## Risks

National EMV overlays differ and proprietary providers may expose only opaque identifiers. Such
formats must stay unsupported or experimental until public specifications and conformance vectors
exist. Syntax validation cannot establish recipient identity.

