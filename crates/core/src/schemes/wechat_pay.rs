use crate::registry::PaymentScheme;
use crate::{
    ActionType, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, RecommendedAction,
    SchemeMetadata,
};

/// WeChat Pay QR payloads (`weixin://wxpay/bizpayurl?...`, `wxp://...`) have no public payload
/// specification: the encoded data is a proprietary, provider-resolved identifier. This adapter
/// only recognizes the shape and marks it `PROPRIETARY_FORMAT` - it never attempts to decode
/// merchant, amount, or account details, because there is nothing public to verify them against.
pub struct WeChatPay;

impl PaymentScheme for WeChatPay {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "wechat_pay".into(),
            display_name: "WeChat Pay".into(),
            countries: vec!["CN".into()],
            category: Category::Wallet,
            standard: "Proprietary (WeChat Pay)".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Experimental,
            static_supported: false,
            dynamic_supported: false,
            features: vec!["link-shape-detection".into()],
            references: vec!["https://pay.weixin.qq.com/".into()],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        let p = payload.to_ascii_lowercase();
        if p.starts_with("weixin://wxpay/") || p.starts_with("wxp://") {
            90
        } else {
            0
        }
    }

    fn parse(&self, _payload: &str) -> Result<PaymentIntent, ParseError> {
        let mut intent = PaymentIntent::recognized("wechat_pay", Category::Wallet);
        intent.standard = Some("Proprietary (WeChat Pay)".into());
        intent.network = Some("WeChat Pay".into());
        intent.supported = false;
        intent.support.enabled = false;
        intent.support.reason = Some(ErrorCode::ProprietaryFormat);
        intent.support.message = Some(
            "WeChat Pay QR payloads are a proprietary format with no public specification; only the link shape is recognized.".into(),
        );
        intent.support.message_key = Some("scanner.unsupported_scheme".into());
        intent.validation.warnings.push(Issue::error(
            ErrorCode::ProprietaryFormat,
            "WeChat Pay link shape detected; payload contents are not publicly documented.",
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
    fn recognizes_wechat_pay_link_shape_as_proprietary_and_unsupported() {
        let intent = parse_payment_qr("weixin://wxpay/bizpayurl?pr=abc123XYZ");
        assert!(intent.recognized);
        assert!(!intent.supported);
        assert_eq!(intent.scheme, "wechat_pay");
        assert_eq!(
            intent.support.reason,
            Some(crate::ErrorCode::ProprietaryFormat)
        );
    }
}
