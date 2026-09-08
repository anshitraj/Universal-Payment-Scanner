// Brand marks for schemes/platforms we can legally show a real logo for.
// Most are Simple Icons (CC0). upi.svg is NPCI's own official UPI mark (public domain per
// Wikimedia Commons - simple geometric shapes/text don't meet the threshold of originality for
// copyright - but still trademarked; shown here only to indicate format compatibility, the same
// nominative use as every other brand mark on this page), cropped to drop the "UNIFIED PAYMENTS
// INTERFACE" tagline so it reads as a compact mark like the others. Wider than the square icons
// (it's a wordmark+glyph lockup, not a glyph alone) - see the `.scheme-icon--wide` rule in app.css.
export const SCHEME_LOGOS: Record<string, string> = {
  upi: "/logos/upi.svg",
  pix: "/logos/pix.svg",
  bitcoin: "/logos/bitcoin.svg",
  ethereum: "/logos/ethereum.svg",
  solana_pay: "/logos/solana.svg",
  paypal: "/logos/paypal.svg",
  walletconnect: "/logos/walletconnect.svg",
  zelle: "/logos/zelle.svg",
  revolut: "/logos/revolut.svg",
  wise: "/logos/wise.svg",
  monzo: "/logos/monzo.svg",
  monero: "/logos/monero.svg",
  zcash: "/logos/zcash.svg",
  venmo: "/logos/venmo.svg",
};

// Wallet-app brand marks used by the payment-launcher buttons (PaymentActionPanel), keyed by
// the wallet id each launcher defines - not scheme ids, since several wallets can serve one
// scheme (three different apps all handle "upi").
export const WALLET_LOGOS: Record<string, string> = {
  googlepay: "/logos/googlepay.svg",
  phonepe: "/logos/phonepe.svg",
  paytm: "/logos/paytm.svg",
  venmo: "/logos/venmo.svg",
};

// Scheme ids whose mark is a wide wordmark+glyph lockup rather than a square glyph.
export const WIDE_LOGOS = new Set(["upi"]);
