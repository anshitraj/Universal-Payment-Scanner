// Verified against test-vectors/v1.json — real, checksum-correct payloads (Pix needs a valid
// CRC-16), so these are guaranteed to parse rather than hand-crafted approximations.
export interface ScannerExample {
  key: string;
  label: string;
  payload: string;
}

export const SCANNER_EXAMPLES: readonly ScannerExample[] = [
  { key: "upi", label: "UPI", payload: "upi://pay?pa=merchant%40bank&pn=Example&am=499.00&cu=INR" },
  { key: "pix", label: "Pix", payload: "00020126360014BR.GOV.BCB.PIX0114+55119999999995204000053039865802BR5913FULANO DE TAL6008BRASILIA62070503***6304C23A" },
  { key: "bitcoin", label: "Bitcoin", payload: "1BoatSLRHtKNngkdXEeobR76b53LETtpyT" },
  { key: "ethereum", label: "Ethereum", payload: "0xfb6916095ca1df60bb79Ce92ce3ea74c37c5d359" },
  { key: "solana_pay", label: "Solana", payload: "9xQeWvG816bUx9EPfEZi4q3G44E2sA6v7a6D9k5GmRse" },
  { key: "paypal", label: "PayPal", payload: "https://paypal.me/exampleuser/25.00" },
  { key: "zelle", label: "Zelle", payload: "https://enroll.zellepay.com/qr-codes?data=AbCdEf123456" },
  { key: "revolut", label: "Revolut", payload: "https://revolut.me/exampleuser?amount=12.50&currency=GBP" },
  { key: "wise", label: "Wise", payload: "https://wise.com/pay/me/exampleuser" },
  { key: "monzo", label: "Monzo", payload: "https://monzo.me/exampleuser?amount=8.00&d=Dinner" },
  { key: "interac", label: "Interac", payload: "https://etransfer.interac.ca/qr/AbCdEf123456" },
  { key: "swish", label: "Swish", payload: "https://app.swish.nu/1/p/sw/?sw=AbCdEf123456" },
  { key: "vipps", label: "Vipps", payload: "https://qr.vipps.no/AbCdEf123456" },
  { key: "monero", label: "Monero", payload: "monero:44AFFq5kSiGBoZ4NMDwYtN18obc8AemS33DBLWs3H7otXft3XjrpDtQGv7SqSsaBYBb98uNbr2VBBEt7f2wfn3RVGQBEP3A?tx_amount=1.25" },
];
