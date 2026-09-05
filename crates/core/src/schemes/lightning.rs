use crate::registry::PaymentScheme;
use crate::{
    ActionType, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, RecommendedAction,
    SchemeMetadata,
};

pub struct Lightning;

impl PaymentScheme for Lightning {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "lightning".into(),
            display_name: "Lightning invoice".into(),
            countries: vec![],
            category: Category::Crypto,
            standard: "BOLT-11 identification".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Experimental,
            static_supported: true,
            dynamic_supported: false,
            features: vec!["invoice-identification".into()],
            references: vec![
                "https://github.com/lightning/bolts/blob/master/11-payment-encoding.md".into(),
            ],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        let p = payload
            .strip_prefix("lightning:")
            .unwrap_or(payload)
            .to_ascii_lowercase();
        if p.starts_with("lnbc1") || p.starts_with("lntb1") || p.starts_with("lnbcrt1") {
            90
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        let invoice = payload.strip_prefix("lightning:").unwrap_or(payload);
        if invoice.len() < 20
            || invoice.len() > 4096
            || !invoice.bytes().all(|b| b.is_ascii_alphanumeric())
        {
            return Err(ParseError::new(
                ErrorCode::MalformedPayload,
                "Malformed Lightning invoice.",
            ));
        }
        let mut intent = PaymentIntent::recognized("lightning", Category::Crypto);
        intent.standard = Some("BOLT-11 identification".into());
        intent.network = Some(
            if invoice.to_ascii_lowercase().starts_with("lntb") {
                "bitcoin-testnet"
            } else {
                "bitcoin-lightning"
            }
            .into(),
        );
        intent.supported = false;
        intent.support.enabled = false;
        intent.support.reason = Some(ErrorCode::SchemeUnsupported);
        intent.support.message = Some(
            "Invoice signature, amount, and expiry validation are not implemented in this release."
                .into(),
        );
        intent.support.message_key = Some("scanner.unsupported_scheme".into());
        intent.validation.warnings.push(Issue::error(
            ErrorCode::SchemeUnsupported,
            "BOLT-11 invoice detected but not fully validated.",
        ));
        intent.recommended_action =
            Some(RecommendedAction::new(ActionType::Unsupported, None, true));
        Ok(intent)
    }
}
