use crate::schemes;
use crate::{
    ActionType, CapabilityPolicy, Category, Detection, ErrorCode, Issue, ParseError, PaymentIntent,
    RecommendedAction, SchemeMetadata, Support,
};

pub trait PaymentScheme: Send + Sync {
    fn metadata(&self) -> SchemeMetadata;
    fn detect(&self, payload: &str) -> u8;
    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError>;
}

pub fn default_registry() -> Vec<Box<dyn PaymentScheme>> {
    let mut registry: Vec<Box<dyn PaymentScheme>> = vec![
        Box::new(schemes::Upi),
        Box::new(schemes::Pix),
        Box::new(schemes::EmvCo),
        Box::new(schemes::Bitcoin),
        Box::new(schemes::Ethereum),
        Box::new(schemes::SolanaPay),
        Box::new(schemes::PayPal),
        Box::new(schemes::Lightning),
        Box::new(schemes::Alipay),
        Box::new(schemes::WeChatPay),
        Box::new(schemes::Venmo),
        Box::new(schemes::CashApp),
        Box::new(schemes::Zelle),
        Box::new(schemes::Revolut),
        Box::new(schemes::Wise),
        Box::new(schemes::Monzo),
        Box::new(schemes::Interac),
        Box::new(schemes::Swish),
        Box::new(schemes::Vipps),
        Box::new(schemes::UnionPay),
        Box::new(schemes::Tron),
        Box::new(schemes::Ton),
        Box::new(schemes::Xrp),
        Box::new(schemes::Stellar),
        Box::new(schemes::Monero),
        Box::new(schemes::Zcash),
        Box::new(schemes::WalletConnect),
        Box::new(schemes::Authenticator),
        Box::new(schemes::EpcQr),
        Box::new(schemes::SwissQr),
    ];
    registry.extend(
        schemes::OVERLAYS
            .iter()
            .map(|config| Box::new(schemes::NationalOverlay(config)) as Box<dyn PaymentScheme>),
    );
    // GenericUrl is the lowest-confidence detector in the registry (see its own doc comment) -
    // registered last so every specific http(s) scheme above gets first refusal.
    registry.push(Box::new(schemes::GenericUrl));
    registry
}

pub struct Scanner {
    policy: CapabilityPolicy,
    schemes: Vec<Box<dyn PaymentScheme>>,
}

impl Default for Scanner {
    fn default() -> Self {
        Self::new(CapabilityPolicy::default())
    }
}

impl Scanner {
    pub fn new(policy: CapabilityPolicy) -> Self {
        Self {
            policy,
            schemes: default_registry(),
        }
    }

    pub fn with_registry(policy: CapabilityPolicy, schemes: Vec<Box<dyn PaymentScheme>>) -> Self {
        Self { policy, schemes }
    }

    pub fn schemes(&self) -> Vec<SchemeMetadata> {
        self.schemes
            .iter()
            .map(|scheme| scheme.metadata())
            .collect()
    }

    fn match_scheme(&self, payload: &str) -> Option<(&dyn PaymentScheme, u8)> {
        self.schemes
            .iter()
            .map(|scheme| (scheme.as_ref(), scheme.detect(payload)))
            .filter(|(_, confidence)| *confidence >= 70)
            .max_by_key(|(_, confidence)| *confidence)
    }

    pub fn detect(&self, payload: &str) -> Detection {
        if payload.len() > self.policy.max_payload_bytes {
            return Detection {
                recognized: false,
                scheme: None,
                confidence: 0,
            };
        }
        match self.match_scheme(payload) {
            Some((scheme, confidence)) => Detection {
                recognized: true,
                scheme: Some(scheme.metadata().id),
                confidence,
            },
            None => Detection {
                recognized: false,
                scheme: None,
                confidence: 0,
            },
        }
    }

    pub fn scan(&self, payload: &str) -> PaymentIntent {
        if payload.len() > self.policy.max_payload_bytes {
            return PaymentIntent::unknown(Issue::error(
                ErrorCode::PayloadTooLarge,
                format!(
                    "Payload exceeds the configured {} byte limit.",
                    self.policy.max_payload_bytes
                ),
            ));
        }
        if payload.contains('\0') {
            return PaymentIntent::unknown(Issue::error(
                ErrorCode::MalformedPayload,
                "Payload contains a null byte.",
            ));
        }
        let Some((scheme, _)) = self.match_scheme(payload) else {
            return PaymentIntent::unknown(Issue::error(
                ErrorCode::NotPaymentQr,
                "The QR payload is not a recognized payment request.",
            ));
        };
        let metadata = scheme.metadata();
        let mut intent = match scheme.parse(payload) {
            Ok(intent) => intent,
            Err(error) => {
                let mut intent =
                    PaymentIntent::recognized(metadata.id.clone(), metadata.category.clone());
                intent.standard = Some(metadata.standard.clone());
                intent.country = metadata.countries.first().cloned();
                intent.invalidate(Issue::error(error.code, error.message));
                intent
            }
        };
        // Fill action-type-specific detail generically from fields the scheme parser already
        // populated, so no scheme adapter has to know about `scheme`/`network`/`provider` itself -
        // see `RecommendedAction`'s doc comment.
        if let Some(action) = intent.recommended_action.as_mut() {
            match action.kind {
                ActionType::Wallet if action.network.is_none() => {
                    action.network = intent.network.clone();
                }
                ActionType::Redirect if action.provider.is_none() => {
                    action.provider = Some(intent.scheme.clone());
                }
                ActionType::Handoff | ActionType::Deeplink if action.scheme.is_none() => {
                    action.scheme = action
                        .uri
                        .as_deref()
                        .and_then(|uri| uri.split_once(':'))
                        .map(|(scheme, _)| scheme.to_ascii_lowercase());
                }
                _ => {}
            }
        }
        if !self.policy.is_enabled(&metadata) && intent.supported {
            intent.supported = false;
            intent.support = Support {
                enabled: false,
                reason: Some(ErrorCode::SchemeDisabled),
                message: Some(format!(
                    "{} payments are not supported by this application.",
                    metadata.display_name
                )),
                message_key: Some("scanner.scheme_disabled".into()),
                details: None,
            };
            intent.recommended_action =
                Some(RecommendedAction::new(ActionType::Unsupported, None, true));
        }
        if intent.supported
            && metadata.category == Category::Crypto
            && let Some(network) = intent.network.clone()
        {
            let symbol = intent.asset.as_ref().and_then(|a| a.symbol.clone());
            let contract = intent.asset.as_ref().and_then(|a| a.contract.clone());
            if let Err(rejection) =
                self.policy
                    .crypto_check(&network, symbol.as_deref(), contract.as_deref())
            {
                intent.supported = false;
                intent.support = rejection.to_support();
                intent.recommended_action =
                    Some(RecommendedAction::new(ActionType::Unsupported, None, true));
            }
        }
        intent
    }
}
