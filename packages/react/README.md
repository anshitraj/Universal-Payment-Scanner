# @universal-payment-qr/react

React components and hooks for scanning and parsing payment QR codes with UniPayScan — camera,
paste, and image-upload input out of the box.

```tsx
import { PaymentQRScanner } from "@universal-payment-qr/react";

<PaymentQRScanner onDetected={(intent) => console.log(intent)} />;
```

Need just the state and no UI? Use the underlying hook:

```tsx
import { usePaymentQRScanner } from "@universal-payment-qr/react";

const { scan, intent, state } = usePaymentQRScanner({ enabledSchemes: ["upi", "pix"] });
```

Repo: https://github.com/anshitraj/Universal-Payment-Scanner
Live playground: https://unipayscan.xyz

MIT licensed.
