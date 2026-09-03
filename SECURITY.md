# Security policy

Please report vulnerabilities privately through the repository security advisory feature. Do not
include a live customer QR, private key, or payment credential; use a minimized synthetic payload.

## Security model

Every payload is hostile. The core imposes an 8 KiB default limit, rejects null bytes, bounds TLV
field counts and numeric sizes, validates checksums/addresses where the public standard permits,
and makes no network calls. It never navigates, signs, broadcasts, pays, or labels a recipient safe.

The Go service caps request bodies, uses short timeouts, disables response caching, omits payloads
from logs, and returns generic core failures. Integrators should add authentication and deployment
rate limiting at their own edge if exposing it publicly.

Known limitations are tracked in scheme metadata. In v0.1, Lightning is detection-only, PayPal
link parsing covers `paypal_me`/`invoice_qr` only, Alipay/WeChat Pay are proprietary-format
detect-only stubs, and ten national EMVCo overlays (`community` maturity - see README) are
detected via the generic envelope and country field rather than a confirmed scheme GUID.

The `otp_setup` adapter (`otpauth://` 2FA setup codes) intentionally never reads or stores the
`secret` query parameter - only the OTP type (`totp`/`hotp`) is extracted. That parameter is a
live authentication credential; the adapter's job is to say "this is a 2FA setup code," not to
handle the credential inside it.

