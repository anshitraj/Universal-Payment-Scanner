# universal-payment-qr (Python)

PyO3 bindings over the same Rust core every SDK in this project uses (`crates/core`) - no
parsing logic is reimplemented here. Local, deterministic, no network calls, no credentials.

```bash
pip install universal-payment-qr
```

```python
from universal_payment_qr import parse_payment_qr

result = parse_payment_qr("upi://pay?pa=merchant%40bank&am=499.00&cu=INR")
print(result["scheme"], result["amount"])  # upi 499.00
```

Selectively enable schemes the same way the JS/Rust SDKs do:

```python
from universal_payment_qr import create_scanner

scanner = create_scanner(schemes={"upi": True, "bitcoin": False})
scanner.scan("bitcoin:1BoatSLRHtKNngkdXEeobR76b53LETtpyT")["supported"]  # False, still recognized
```

See the [repository root README](../../README.md) for the full scheme list, the normalized
schema, and the security/privacy model - all identical here, since this binding calls the exact
same Rust engine.

## Development

```bash
pip install maturin
maturin develop  # builds the native extension and installs it into your active venv
pytest tests/
```
