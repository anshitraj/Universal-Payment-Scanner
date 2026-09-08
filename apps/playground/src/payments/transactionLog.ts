// A local, client-side-only record of payment attempts made from this browser. UniPayScan never
// holds funds or brokers a transfer - this exists purely so the demo can show "here's what you
// just tried to do," the same way a browser's own download history works. Nothing here is sent
// anywhere; it lives in localStorage and never leaves this device.

export interface PaymentAttempt {
  id: string;
  at: string; // ISO timestamp
  scheme: string;
  wallet: string; // e.g. "Phantom", "PhonePe", "Venmo"
  amount?: string | undefined;
  currency?: string | undefined;
  recipient?: string | undefined;
  outcome: "opened" | "signed" | "not_installed" | "failed";
  detail?: string | undefined; // e.g. a Solana signature, or an error message
}

const STORAGE_KEY = "unipayscan.playground.paymentLog.v1";
const MAX_ENTRIES = 25;

function readAll(): PaymentAttempt[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

export function listPaymentAttempts(): PaymentAttempt[] {
  return readAll();
}

export function recordPaymentAttempt(entry: Omit<PaymentAttempt, "id" | "at">): PaymentAttempt {
  const full: PaymentAttempt = {
    ...entry,
    id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    at: new Date().toISOString(),
  };
  try {
    const next = [full, ...readAll()].slice(0, MAX_ENTRIES);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
  } catch {
    // Private browsing / storage disabled - the attempt still happened, it just won't be logged.
  }
  return full;
}

export function clearPaymentAttempts(): void {
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch {
    // Nothing to do if storage isn't available.
  }
}
