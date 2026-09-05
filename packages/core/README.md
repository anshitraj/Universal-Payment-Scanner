# unipayscan

Turn a scanned or pasted payment QR into structured data — UPI, Pix, EMVCo, PayPal, Bitcoin,
Ethereum, Solana Pay, and 30+ more formats. All the parsing, checksum validation, and scheme
detection happens inside the package. Zero network calls, no custody, no auto-navigation.

```ts
import { parsePaymentQR } from "unipayscan";

const intent = await parsePaymentQR(qrPayload);
```

That's the integration. `intent` comes back normalized, regardless of which payment rail the QR
was from:

```json
{
  "recognized": true,
  "scheme": "upi",
  "amount": "499.00",
  "currency": "INR",
  "recipient": { "id": "merchant@bank", "name": "Example" },
  "validation": { "valid": true, "errors": [], "warnings": [] }
}
```

## Restricting which payment methods your app accepts

Detection always stays global — `createScanner` only controls what your app is willing to act
on:

```ts
import { createScanner } from "unipayscan";

const scanner = createScanner({ schemes: { upi: true, pix: true, bitcoin: true } });
const intent = await scanner.scan(qrPayload);
```

A disabled scheme stays recognized instead of turning into a flat "invalid QR":

```json
{ "recognized": true, "scheme": "solana_pay", "supported": false }
```

## More

- Full scheme coverage, maturity levels, and per-platform SDKs:
  https://github.com/anshitraj/Universal-Payment-Scanner
- Live playground: https://unipayscan.xyz
- React? See [`@universal-payment-qr/react`](https://www.npmjs.com/package/@universal-payment-qr/react)
  for camera/paste/upload components built on top of this.

MIT licensed.
