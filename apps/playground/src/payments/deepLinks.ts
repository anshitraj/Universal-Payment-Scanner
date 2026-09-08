// Deep-link handoff for schemes with no browser-extension equivalent to Solana's Wallet Standard
// (UPI, Venmo): construct the right URI, hand it to the OS, and get out of the way. UniPayScan
// never sees the user's UPI PIN or Venmo credentials - the destination app owns that entirely.

import type { PaymentIntent } from "unipayscan";

// --- UPI ------------------------------------------------------------------------------------
// Generic `upi://pay` is NPCI's own intent format - every UPI app registers as a handler for it,
// so the OS shows its own chooser. Confirmed against this project's own UPI parser
// (crates/core/src/schemes/upi.rs) and NPCI's published UPI Linking Specification.

function upiParams(intent: PaymentIntent): URLSearchParams {
  const params = new URLSearchParams();
  const payeeAddress = intent.recipient?.id ?? intent.recipient?.address;
  if (payeeAddress) params.set("pa", payeeAddress);
  if (intent.recipient?.name) params.set("pn", intent.recipient.name);
  if (intent.amount) params.set("am", intent.amount);
  params.set("cu", intent.currency ?? "INR");
  if (intent.reference) params.set("tr", intent.reference);
  if (intent.description) params.set("tn", intent.description);
  return params;
}

export function genericUpiLink(intent: PaymentIntent): string {
  const original = intent.recommendedAction?.uri;
  if (original?.startsWith("upi://")) return original;
  return `upi://pay?${upiParams(intent).toString()}`;
}

export interface UpiAppOption {
  id: string;
  name: string;
  logo: string;
  /** Android package name, for the intent:// package-targeted form. There is no equivalent
   * mechanism on iOS - a per-app custom scheme exists for some of these apps, but is not
   * consistently documented or stable enough to build against here, so iOS/desktop fall back to
   * the generic upi://pay link and let the OS/browser pick. */
  androidPackage: string;
}

export const UPI_APPS: readonly UpiAppOption[] = [
  { id: "gpay", name: "Google Pay", logo: "/logos/googlepay.svg", androidPackage: "com.google.android.apps.nbu.paisa.user" },
  { id: "phonepe", name: "PhonePe", logo: "/logos/phonepe.svg", androidPackage: "com.phonepe.app" },
  { id: "paytm", name: "Paytm", logo: "/logos/paytm.svg", androidPackage: "net.one97.paytm" },
];

/** Android's `intent://` syntax targets one specific app by package name, bypassing the OS
 * chooser, with `S.browser_fallback_url` as where the browser sends the user if that package
 * isn't installed. Android/Chrome only - see `androidPackage`'s doc comment for why there's no
 * iOS equivalent here. */
export function upiAndroidIntentLink(app: UpiAppOption, intent: PaymentIntent): string {
  const params = upiParams(intent).toString();
  const fallback = encodeURIComponent(genericUpiLink(intent));
  return `intent://pay?${params}#Intent;scheme=upi;package=${app.androidPackage};S.browser_fallback_url=${fallback};end;`;
}

export function isAndroid(): boolean {
  return typeof navigator !== "undefined" && /android/i.test(navigator.userAgent);
}

// --- Venmo ------------------------------------------------------------------------------------
// venmo://paycharge is not an app scheme Venmo documents publicly - this is reverse-engineered by
// the developer community and has been stable for years, but treat it as best-effort, not a
// contract. The venmo.com web URL is Venmo's own public page and is the more reliable fallback:
// it handles the "open in app" handoff on mobile itself, and works standalone on desktop.

export function venmoLinks(intent: PaymentIntent): { app: string; web: string } | null {
  const handle = intent.recipient?.id;
  if (!handle) return null;
  const params = new URLSearchParams({ txn: "pay" });
  if (intent.amount) params.set("amount", intent.amount);
  if (intent.description) params.set("note", intent.description);
  const query = params.toString();
  return {
    app: `venmo://paycharge?recipients=${encodeURIComponent(handle)}&${query}`,
    web: `https://venmo.com/${encodeURIComponent(handle)}?${query}`,
  };
}

// --- Generic "did the app open" heuristic --------------------------------------------------
// Browsers deliberately don't expose "is app X installed" to a web page. Every site that deep
// links to a native app (Venmo and Cash App's own web buttons included) uses the same imperfect
// workaround: fire the link, and if the page is still visible after a short timeout, assume
// nothing opened. A real app launch backgrounds the browser tab (blur/visibilitychange fires)
// before the timeout; a failed one doesn't. Not 100% reliable - a user switching tabs for an
// unrelated reason during the window would read as "not installed" - but it's the industry
// standard given the platform constraint, not a corner this project is cutting alone.

export function attemptAppLaunch(uri: string, onNotInstalled: () => void, timeoutMs = 1500): void {
  let left = false;
  const markLeft = () => {
    left = true;
  };
  window.addEventListener("blur", markLeft, { once: true });
  document.addEventListener("visibilitychange", markLeft, { once: true });
  window.location.href = uri;
  window.setTimeout(() => {
    window.removeEventListener("blur", markLeft);
    document.removeEventListener("visibilitychange", markLeft);
    if (!left && !document.hidden) onNotInstalled();
  }, timeoutMs);
}
