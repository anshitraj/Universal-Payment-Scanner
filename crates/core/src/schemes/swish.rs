use url::Url;

use crate::registry::PaymentScheme;
use crate::{
    ActionType, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent,
    RecommendedAction, SchemeMetadata,
};

/// Swish (Sweden) merchant/MCommerce QR and deep-link payments resolve through Getswish AB's own
/// universal link, `https://app.swish.nu/1/p/sw/...`, carrying an opaque, Swish-issued payment
/// token in the query string. The token encodes payee, amount, and message server-side; nothing
/// in the link itself is independently verifiable, so - like Zelle and Interac - this recognizes
/// the link shape and hands off instead of guessing at its contents.
pub struct Swish;

impl PaymentScheme for Swish {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "swish".into(),
            display_name: "Swish".into(),
            countries: vec!["SE".into()],
            category: Category::PaymentLink,
            standard: "Swish universal payment link".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Beta,
            static_supported: true,
            dynamic_supported: true,
            features: vec!["payment-token".into()],
            references: vec!["https://developer.swish.nu/".into()],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        let Ok(url) = Url::parse(payload) else {
            return 0;
        };
        let is_swish = url
            .host_str()
            .is_some_and(|h| h.eq_ignore_ascii_case("app.swish.nu"));
        if is_swish && url.path().starts_with("/1/p/") {
            90
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        let url = Url::parse(payload)
            .map_err(|_| ParseError::new(ErrorCode::UnsafeUri, "Invalid Swish URL."))?;
        if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
            return Err(ParseError::new(
                ErrorCode::UnsafeUri,
                "Swish links must use https without embedded credentials.",
            ));
        }
        let has_token = url.query().is_some_and(|q| !q.is_empty() && q.len() <= 4096);
        if !has_token {
            return Err(ParseError::new(
                ErrorCode::InvalidRecipient,
                "Swish payment link is missing its payment token.",
            ));
        }
        let mut intent = PaymentIntent::recognized("swish", Category::PaymentLink);
        intent.subtype = Some("payment_request".into());
        intent.standard = Some("Swish universal payment link".into());
        intent.country = Some("SE".into());
        intent.network = Some("Swish".into());
        intent.dynamic = Some(true);
        intent.validation.warnings.push(Issue::error(
            ErrorCode::ProprietaryFormat,
            "Link structure is public, but the payment token is opaque and requires Swish verification.",
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
    fn parses_swish_payment_link() {
        let intent = parse_payment_qr("https://app.swish.nu/1/p/sw/?sw=AbCdEf123456");
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "swish");
        assert_eq!(intent.country.as_deref(), Some("SE"));
    }

    #[test]
    fn rejects_swish_link_without_token() {
        assert!(!parse_payment_qr("https://app.swish.nu/1/p/sw/").validation.valid);
    }
}
