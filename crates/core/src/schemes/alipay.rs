use crate::registry::PaymentScheme;
use crate::{
    ActionType, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, RecommendedAction,
    SchemeMetadata,
};

/// Alipay QR payloads (`https://qr.alipay.com/...`, `alipays://platformapi/...`) have no public
/// payload specification: the encoded data is a proprietary, provider-resolved identifier. This
/// adapter only recognizes the shape and marks it `PROPRIETARY_FORMAT` - it never attempts to
/// decode merchant, amount, or account details, because there is nothing public to verify them
/// against.
pub struct Alipay;

impl PaymentScheme for Alipay {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "alipay".into(),
            display_name: "Alipay".into(),
            countries: vec!["CN".into()],
            category: Category::Wallet,
            standard: "Proprietary (Alipay)".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Experimental,
            static_supported: false,
            dynamic_supported: false,
            features: vec!["link-shape-detection".into()],
            references: vec!["https://global.alipay.com/".into()],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        let p = payload.to_ascii_lowercase();
        if p.starts_with("https://qr.alipay.com/") || p.starts_with("alipays://") {
            90
        } else {
            0
        }
    }

    fn parse(&self, _payload: &str) -> Result<PaymentIntent, ParseError> {
        let mut intent = PaymentIntent::recognized("alipay", Category::Wallet);
        intent.standard = Some("Proprietary (Alipay)".into());
        intent.network = Some("Alipay".into());
        intent.supported = false;
        intent.support.enabled = false;
        intent.support.reason = Some(ErrorCode::ProprietaryFormat);
        intent.support.message = Some(
            "Alipay QR payloads are a proprietary format with no public specification; only the link shape is recognized.".into(),
        );
        intent.support.message_key = Some("scanner.unsupported_scheme".into());
        intent.validation.warnings.push(Issue::error(
            ErrorCode::ProprietaryFormat,
            "Alipay link shape detected; payload contents are not publicly documented.",
        ));
        intent.recommended_action =
            Some(RecommendedAction::new(ActionType::Unsupported, None, true));
        Ok(intent)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_payment_qr;

    #[test]
    fn recognizes_alipay_link_shape_as_proprietary_and_unsupported() {
        let intent = parse_payment_qr("https://qr.alipay.com/fkx01234567890abcdef");
        assert!(intent.recognized);
        assert!(!intent.supported);
        assert_eq!(intent.scheme, "alipay");
        assert_eq!(
            intent.support.reason,
            Some(crate::ErrorCode::ProprietaryFormat)
        );
    }
}
