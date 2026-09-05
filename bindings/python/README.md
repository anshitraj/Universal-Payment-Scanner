# unipayscan (Python)

[![PyPI](https://img.shields.io/pypi/v/unipayscan.svg)](https://pypi.org/project/unipayscan/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://github.com/anshitraj/Universal-Payment-Scanner/blob/main/LICENSE)
[![Python versions](https://img.shields.io/pypi/pyversions/unipayscan.svg)](https://pypi.org/project/unipayscan/)

Turn a scanned or pasted payment QR into structured data — UPI, Pix, EMVCo, PayPal, Bitcoin,
Ethereum, Solana Pay, and 30+ more formats. PyO3 bindings over the same Rust core every SDK in
this project uses (`crates/core`) — no parsing logic is reimplemented here. Local, deterministic,
no network calls, no credentials.

```bash
pip install unipayscan
```

```python
from unipayscan import parse_payment_qr

result = parse_payment_qr("upi://pay?pa=merchant%40bank&am=499.00&cu=INR")
print(result["scheme"], result["amount"])  # upi 499.00
```

That's the integration. Everything else — QR parsing, checksum validation, scheme detection,
normalization — happens inside the package.

## Restricting which payment methods your app accepts

Detection always stays global; `create_scanner` only controls what your app is willing to act on,
the same way the JS/Rust SDKs work:

```python
from unipayscan import create_scanner

scanner = create_scanner(schemes={"upi": True, "bitcoin": False})
scanner.scan("bitcoin:1BoatSLRHtKNngkdXEeobR76b53LETtpyT")["supported"]  # False, still recognized
```

See the [repository root README](../../README.md) for the full scheme list, the normalized
schema, and the security/privacy model — all identical here, since this binding calls the exact
same Rust engine.

## Development

```bash
pip install maturin
maturin develop --release  # builds the native extension and installs it into your active venv
pytest tests/
```

## License

MIT © [anshitraj](https://github.com/anshitraj) — see [LICENSE](https://github.com/anshitraj/Universal-Payment-Scanner/blob/main/LICENSE).
