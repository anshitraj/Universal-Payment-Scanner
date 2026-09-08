use url::Url;

use crate::registry::PaymentScheme;
use crate::{
    ActionType, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, RecommendedAction,
    SchemeMetadata,
};

/// UnionPay QR payloads redirect through UnionPay International's own domain
/// (`https://qr.95516.com/...` - 95516 is UnionPay's customer-service number, reused as their QR
/// gateway host). Like Alipay/WeChat Pay, there is no public payload specification: this adapter
/// only recognizes the link shape and marks it `PROPRIETARY_FORMAT`, never attempting to decode
/// merchant, amount, or account details.
pub struct UnionPay;

impl PaymentScheme for UnionPay {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "unionpay".into(),
            display_name: "UnionPay QR".into(),
            countries: vec!["CN".into()],
            category: Category::Card,
            standard: "Proprietary (UnionPay QR)".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Experimental,
            static_supported: false,
            dynamic_supported: false,
            features: vec!["link-shape-detection".into()],
            references: vec!["https://www.unionpayintl.com/".into()],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        let Ok(url) = Url::parse(payload) else {
            return 0;
        };
        let is_unionpay = url
            .host_str()
            .is_some_and(|h| h.eq_ignore_ascii_case("qr.95516.com"));
        if is_unionpay {
            90
        } else {
            0
        }
    }

    fn parse(&self, _payload: &str) -> Result<PaymentIntent, ParseError> {
        let mut intent = PaymentIntent::recognized("unionpay", Category::Card);
        intent.standard = Some("Proprietary (UnionPay QR)".into());
        intent.country = Some("CN".into());
        intent.network = Some("UnionPay".into());
        intent.supported = false;
        intent.support.enabled = false;
        intent.support.reason = Some(ErrorCode::ProprietaryFormat);
        intent.support.message = Some(
            "UnionPay QR payloads are a proprietary format with no public specification; only the link shape is recognized.".into(),
        );
        intent.support.message_key = Some("scanner.unsupported_scheme".into());
        intent.validation.warnings.push(Issue::error(
            ErrorCode::ProprietaryFormat,
            "UnionPay QR link shape detected; payload contents are not publicly documented.",
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
    fn recognizes_unionpay_link_shape_as_proprietary_and_unsupported() {
        let intent = parse_payment_qr("https://qr.95516.com/00010000/abc123XYZ");
        assert!(intent.recognized);
        assert!(!intent.supported);
        assert_eq!(intent.scheme, "unionpay");
        assert_eq!(
            intent.support.reason,
            Some(crate::ErrorCode::ProprietaryFormat)
        );
    }
}
