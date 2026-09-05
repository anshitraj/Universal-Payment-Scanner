use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const SCHEMA_VERSION: &str = "2.0.0";
pub const DEFAULT_MAX_PAYLOAD_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    BankTransfer,
    Wallet,
    Card,
    Crypto,
    PaymentLink,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    UnknownQr,
    NotPaymentQr,
    MalformedPayload,
    InvalidChecksum,
    InvalidRecipient,
    InvalidAmount,
    InvalidCurrency,
    InvalidNetwork,
    SchemeDisabled,
    SchemeUnsupported,
    NetworkUnsupported,
    AssetUnsupported,
    AssetUnverifiable,
    ProprietaryFormat,
    Expired,
    PayloadTooLarge,
    UnsafeUri,
    ParserError,
    UnverifiedRecipient,
    Suspicious,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub code: ErrorCode,
    pub message: String,
    pub message_key: String,
}

impl Issue {
    pub fn error(code: ErrorCode, message: impl Into<String>) -> Self {
        let key = match code {
            ErrorCode::SchemeDisabled => "scanner.scheme_disabled",
            ErrorCode::SchemeUnsupported | ErrorCode::ProprietaryFormat => {
                "scanner.unsupported_scheme"
            }
            ErrorCode::NotPaymentQr | ErrorCode::UnknownQr => "scanner.not_payment_qr",
            ErrorCode::PayloadTooLarge => "scanner.payload_too_large",
            ErrorCode::UnverifiedRecipient => "scanner.unverified_recipient",
            _ => "scanner.invalid_qr",
        };
        Self {
            code,
            message: message.into(),
            message_key: key.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Validation {
    pub valid: bool,
    pub errors: Vec<Issue>,
    pub warnings: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Recipient {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merchant_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    /// Only ever set when the payload itself names the asset (a native coin like ETH/SOL, or an
    /// explicit ticker in the URI). A contract address alone is never enough to fill this in - a
    /// contract's symbol is not part of the payload and guessing it would misattribute an asset
    /// the payload never actually claimed. `CryptoRejection::AssetUnverifiable` in `policy.rs`
    /// depends on this staying honest: an accept-policy asset check fails closed exactly when
    /// `symbol` is `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decimals: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_atomic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    Handoff,
    Deeplink,
    Wallet,
    Redirect,
    DisplayOnly,
    Unsupported,
}

/// Action-type-specific detail, filled in generically by `Scanner::scan` from fields the scheme
/// parser already populated on the intent (`uri`'s scheme prefix, `network`, `scheme` id) - a
/// scheme adapter constructs a `RecommendedAction` with `new()` and never sets these itself, so
/// host apps get a consistent, typed instruction ("what UI should I show?") instead of having to
/// parse `uri` themselves. Exactly one is meaningful per `ActionType`:
/// - `scheme` - the URI scheme to hand off to an external app (`Handoff`/`Deeplink`, e.g. `"upi"`)
/// - `network` - the chain/network a crypto wallet should handle (`Wallet`, e.g. `"solana"`)
/// - `provider` - the web provider to redirect to (`Redirect`, e.g. `"paypal"`)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecommendedAction {
    #[serde(rename = "type")]
    pub kind: ActionType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    pub requires_user_confirmation: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

impl RecommendedAction {
    pub fn new(kind: ActionType, uri: Option<String>, requires_user_confirmation: bool) -> Self {
        Self {
            kind,
            uri,
            requires_user_confirmation,
            scheme: None,
            network: None,
            provider: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Support {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<ErrorCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_key: Option<String>,
    /// Structured, reason-specific detail a host UI can act on without parsing `message` - e.g.
    /// `{"network":"ethereum","allowedNetworks":["base","bnb","solana"]}` for
    /// `NETWORK_UNSUPPORTED`. Shape depends on `reason`; absent when `reason` needs no detail
    /// beyond the message (or when `enabled` is true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl Default for Support {
    fn default() -> Self {
        Self {
            enabled: true,
            reason: None,
            message: None,
            message_key: None,
            details: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Identification {
    /// A scheme-specific GUID (tag 00 inside a merchant account sub-template) was matched
    /// against a value confirmed from an official/authoritative source. `scheme` names that
    /// exact standard.
    GuidMatch,
    /// No scheme-specific GUID has been verified yet. The payload matched the generic EMVCo
    /// envelope plus a country tag only - `scheme` stays the generic identity and the specific
    /// guess moves to `possibleScheme`, because a country match alone cannot rule out some other,
    /// unrelated EMVCo QR issued in that country.
    CountryLevelInference,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    High,
    Medium,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentIntent {
    pub schema_version: String,
    pub recognized: bool,
    pub supported: bool,
    pub category: Category,
    pub scheme: String,
    /// A finer-grained shape within `scheme` when one payload family covers meaningfully
    /// different flows - e.g. `paypal`'s `paypal_me` vs `unknown_paypal`, or a bare crypto
    /// address vs a full payment-request URI. Omitted when the scheme has no subtypes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>,
    /// Set only when `scheme` had to fall back to a generic identity (e.g. `emvco_mpm`) because
    /// `identification` is no stronger than `COUNTRY_LEVEL_INFERENCE` - see `identification`.
    /// The specific standard this payload is most likely to be, not yet confirmed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub possible_scheme: Option<String>,
    /// How `scheme` (or `possibleScheme`) was determined. Omitted when a scheme's own detection
    /// is unambiguous by construction (a URI scheme prefix, a checksum-valid address format, ...)
    /// rather than inferred. Never invents confidence a scheme's adapter doesn't actually have.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identification: Option<Identification>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standard: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipient: Option<Recipient>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset: Option<Asset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    pub metadata: Map<String, Value>,
    pub validation: Validation,
    pub support: Support,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_action: Option<RecommendedAction>,
}

impl PaymentIntent {
    pub fn recognized(scheme: impl Into<String>, category: Category) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.into(),
            recognized: true,
            supported: true,
            category,
            scheme: scheme.into(),
            subtype: None,
            possible_scheme: None,
            identification: None,
            confidence: None,
            standard: None,
            country: None,
            network: None,
            recipient: None,
            amount: None,
            currency: None,
            asset: None,
            reference: None,
            description: None,
            dynamic: None,
            expires_at: None,
            metadata: Map::new(),
            validation: Validation {
                valid: true,
                ..Validation::default()
            },
            support: Support::default(),
            recommended_action: None,
        }
    }

    pub fn unknown(issue: Issue) -> Self {
        let mut result = Self::recognized("unknown", Category::Unknown);
        result.recognized = false;
        result.supported = false;
        result.validation.valid = false;
        result.validation.errors.push(issue);
        result.support = Support {
            enabled: false,
            reason: Some(ErrorCode::SchemeUnsupported),
            message: Some("No supported payment scheme recognized this payload.".into()),
            message_key: Some("scanner.not_payment_qr".into()),
            details: None,
        };
        result.recommended_action =
            Some(RecommendedAction::new(ActionType::Unsupported, None, true));
        result
    }

    pub fn invalidate(&mut self, issue: Issue) {
        self.validation.valid = false;
        self.validation.errors.push(issue);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Detection {
    pub recognized: bool,
    pub scheme: Option<String>,
    pub confidence: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Maturity {
    Stable,
    Beta,
    Experimental,
    Community,
    Deprecated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SchemeMetadata {
    pub id: String,
    pub display_name: String,
    pub countries: Vec<String>,
    pub category: Category,
    pub standard: String,
    pub parser_version: String,
    pub maturity: Maturity,
    pub static_supported: bool,
    pub dynamic_supported: bool,
    pub features: Vec<String>,
    pub references: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub code: ErrorCode,
    pub message: String,
}

impl ParseError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
