use url::Url;

use crate::registry::PaymentScheme;
use crate::{
    ActionType, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent,
    RecommendedAction, SchemeMetadata,
};

/// Vipps MobilePay (Norway) QR stickers and payment requests resolve through their own redirect
/// domain, `https://qr.vipps.no/<code>`, carrying an opaque, provider-issued code. Same shape as
/// Zelle/Interac/Swish: recognize the link, don't guess at what the code resolves to.
pub struct Vipps;

impl PaymentScheme for Vipps {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "vipps".into(),
            display_name: "Vipps".into(),
            countries: vec!["NO".into()],
            category: Category::PaymentLink,
            standard: "Vipps QR redirect link".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Beta,
            static_supported: true,
            dynamic_supported: true,
            features: vec!["qr-code-token".into()],
            references: vec!["https://developer.vippsmobilepay.com/".into()],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        let Ok(url) = Url::parse(payload) else {
            return 0;
        };
        let is_vipps = url
            .host_str()
            .is_some_and(|h| h.eq_ignore_ascii_case("qr.vipps.no"));
        if is_vipps && !url.path().trim_matches('/').is_empty() {
            90
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        let url = Url::parse(payload)
            .map_err(|_| ParseError::new(ErrorCode::UnsafeUri, "Invalid Vipps URL."))?;
        if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
            return Err(ParseError::new(
                ErrorCode::UnsafeUri,
                "Vipps links must use https without embedded credentials.",
            ));
        }
        if url.path().trim_matches('/').is_empty() {
            return Err(ParseError::new(
                ErrorCode::InvalidRecipient,
                "Vipps QR link is missing its code.",
            ));
        }
        let mut intent = PaymentIntent::recognized("vipps", Category::PaymentLink);
        intent.subtype = Some("qr_code".into());
        intent.standard = Some("Vipps QR redirect link".into());
        intent.country = Some("NO".into());
        intent.network = Some("Vipps".into());
        intent.dynamic = Some(true);
        intent.validation.warnings.push(Issue::error(
            ErrorCode::ProprietaryFormat,
            "Link structure is public, but the code is opaque and requires Vipps verification.",
        ));
        intent.recommended_action = Some(RecommendedAction::new(
            ActionType::Redirect,
            Some(payload.into()),
            true,
        ));
        Ok(intent)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_payment_qr;

    #[test]
    fn parses_vipps_qr_link() {
        let intent = parse_payment_qr("https://qr.vipps.no/AbCdEf123456");
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "vipps");
        assert_eq!(intent.country.as_deref(), Some("NO"));
    }

    #[test]
    fn rejects_credentials_embedded_in_the_url() {
        assert!(
            !parse_payment_qr("https://user:pass@qr.vipps.no/AbCdEf123456")
                .validation
                .valid
        );
    }
}
