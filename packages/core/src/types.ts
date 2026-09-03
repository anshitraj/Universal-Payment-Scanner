export type PaymentCategory =
  | "bank_transfer"
  | "wallet"
  | "card"
  | "crypto"
  | "payment_link"
  | "unknown";

export type ErrorCode =
  | "UNKNOWN_QR"
  | "NOT_PAYMENT_QR"
  | "MALFORMED_PAYLOAD"
  | "INVALID_CHECKSUM"
  | "INVALID_RECIPIENT"
  | "INVALID_AMOUNT"
  | "INVALID_CURRENCY"
  | "INVALID_NETWORK"
  | "SCHEME_DISABLED"
  | "SCHEME_UNSUPPORTED"
  | "PROPRIETARY_FORMAT"
  | "EXPIRED"
  | "PAYLOAD_TOO_LARGE"
  | "UNSAFE_URI"
  | "PARSER_ERROR"
  | "UNVERIFIED_RECIPIENT"
  | "SUSPICIOUS";

export interface Issue {
  code: ErrorCode;
  message: string;
  messageKey: string;
}

export interface PaymentIntent {
  schemaVersion: string;
  recognized: boolean;
  supported: boolean;
  category: PaymentCategory;
  scheme: string;
  subtype?: string;
  standard?: string;
  country?: string;
  network?: string;
  recipient?: { id?: string; address?: string; name?: string; merchantId?: string };
  amount?: string;
  currency?: string;
  asset?: { symbol?: string; contract?: string; decimals?: number; amountAtomic?: string };
  reference?: string;
  description?: string;
  dynamic?: boolean;
  expiresAt?: string;
  metadata: Record<string, unknown>;
  validation: { valid: boolean; errors: Issue[]; warnings: Issue[] };
  support: {
    enabled: boolean;
    reason?: ErrorCode;
    message?: string;
    messageKey?: string;
  };
  recommendedAction?: {
    type: "handoff" | "deeplink" | "wallet" | "redirect" | "display_only" | "unsupported";
    uri?: string;
    requiresUserConfirmation: boolean;
  };
}

export interface Detection {
  recognized: boolean;
  scheme?: string;
  confidence: number;
}

export interface SchemeMetadata {
  id: string;
  displayName: string;
  countries: string[];
  category: PaymentCategory;
  standard: string;
  parserVersion: string;
  maturity: "stable" | "beta" | "experimental" | "community" | "deprecated";
  staticSupported: boolean;
  dynamicSupported: boolean;
  features: string[];
  references: string[];
}

export type Preset = "india" | "asia-pacific" | "banking-only" | "crypto-only" | "all-stable" | "all";

/**
 * Every scheme id the Rust registry knows about (`crates/core/src/registry.rs`). Single source of
 * truth for building an explicit `schemes: {}` allow/deny map from a UI's enabled-list, so a
 * newly-added scheme can't silently fall through an SDK-side list that forgot to mention it.
 */
export const ALL_SCHEME_IDS = [
  "upi",
  "pix",
  "emvco_mpm",
  "bitcoin",
  "ethereum",
  "solana_pay",
  "paypal",
  "lightning",
  "alipay",
  "wechat_pay",
  "promptpay",
  "vietqr",
  "paynow",
  "duitnow_qr",
  "qris",
  "qr_ph",
  "hkqr",
  "nepalpay_qr",
  "khqr",
  "lankaqr",
  "bangla_qr",
  "raast_qr",
  "mmqr",
  "lao_qr",
  "jpqr",
  "twqr",
  "zeropay",
  "mercado_pago",
  "venmo",
  "cash_app",
  "tron",
  "ton",
  "xrp",
  "stellar",
  "walletconnect",
  "otp_setup",
  "url",
  "epc_qr",
  "swiss_qr_bill",
] as const;

export interface ScannerOptions {
  preset?: Preset;
  schemes?: Record<string, boolean>;
  categories?: Partial<Record<PaymentCategory, boolean>>;
  countries?: Record<string, Record<string, boolean>>;
  maxPayloadBytes?: number;
  engine?: PaymentEngine;
}

export interface PaymentEngine {
  scan(payload: string): PaymentIntent | Promise<PaymentIntent>;
  detect(payload: string): Detection | Promise<Detection>;
  schemes(): SchemeMetadata[] | Promise<SchemeMetadata[]>;
}
