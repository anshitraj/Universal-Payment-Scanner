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
  | "NETWORK_UNSUPPORTED"
  | "ASSET_UNSUPPORTED"
  | "ASSET_UNVERIFIABLE"
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

/** How `scheme` (or `possibleScheme`) was determined. Omitted when a scheme's detection is
 * unambiguous by construction (a URI scheme prefix, a checksum-valid address format, ...) rather
 * than inferred. */
export type Identification = "GUID_MATCH" | "COUNTRY_LEVEL_INFERENCE";

export type Confidence = "high" | "medium";

export interface PaymentIntent {
  schemaVersion: string;
  recognized: boolean;
  supported: boolean;
  category: PaymentCategory;
  scheme: string;
  subtype?: string;
  /** Set only when `scheme` had to fall back to a generic identity (e.g. `emvco_mpm`) because
   * `identification` is no stronger than `COUNTRY_LEVEL_INFERENCE` - the specific standard this
   * payload is most likely to be, not yet confirmed. */
  possibleScheme?: string;
  identification?: Identification;
  confidence?: Confidence;
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
    /** Structured, reason-specific detail a host UI can act on without parsing `message` - e.g.
     * `{ network: "ethereum", allowedNetworks: ["base", "bnb", "solana"] }` for
     * `NETWORK_UNSUPPORTED`. Shape depends on `reason`. */
    details?: Record<string, unknown>;
  };
  /** What UI a host app should show next, and the one piece of detail that action needs - never
   * performed automatically. Exactly one of `scheme`/`network`/`provider` is populated, matching
   * `type`: `scheme` for `handoff`/`deeplink` (the URI scheme to hand off to, e.g. `"upi"`),
   * `network` for `wallet` (the chain a crypto wallet should handle, e.g. `"solana"`), `provider`
   * for `redirect` (the web provider, e.g. `"paypal"`). */
  recommendedAction?: {
    type: "handoff" | "deeplink" | "wallet" | "redirect" | "display_only" | "unsupported";
    uri?: string;
    requiresUserConfirmation: boolean;
    scheme?: string;
    network?: string;
    provider?: string;
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
  /**
   * Fine-grained accept policy below the scheme/category level - today just crypto network/asset
   * allow-listing, since a host that accepts the `crypto` category still needs to say *which*
   * chains and tokens it can actually settle. Keyed by network id (as reported in
   * `PaymentIntent.network`, e.g. `"base"`) -> allowed asset symbols on that network; an empty
   * array allows any asset on that network. Omitted (the default) applies no crypto-specific
   * restriction - crypto intents are governed only by `schemes`/`categories`/`countries`, same as
   * before this option existed.
   *
   * An asset whose identity can't be confirmed from the QR alone (e.g. an ERC-20 transfer where
   * only the contract address is known) is rejected with `ASSET_UNVERIFIABLE` rather than guessed
   * - see `support.details` on the returned intent.
   */
  accept?: {
    crypto?: Record<string, string[]>;
  };
  engine?: PaymentEngine;
}

export interface PaymentEngine {
  scan(payload: string): PaymentIntent | Promise<PaymentIntent>;
  detect(payload: string): Detection | Promise<Detection>;
  schemes(): SchemeMetadata[] | Promise<SchemeMetadata[]>;
}
