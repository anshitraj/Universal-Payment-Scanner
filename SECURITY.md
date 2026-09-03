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

Known limitations are tracked in scheme metadata. In v0.1, Lightning is detection-only, PayPal.Me
is public-link parsing only, Alipay/WeChat Pay are proprietary-format detect-only stubs, and nine
national EMVCo overlays (`community` maturity - see README) are detected via the generic envelope
and country field rather than a confirmed scheme GUID.

