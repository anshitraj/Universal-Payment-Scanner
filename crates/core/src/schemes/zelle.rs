use url::Url;

use crate::registry::PaymentScheme;
use crate::{
    ActionType, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent,
    RecommendedAction, SchemeMetadata,
};

/// Zelle personal QR codes encode an enrollment URL of the form
/// `https://enroll.zellepay.com/qr-codes?data=<token>` (bare `zellepay.com` host is accepted too).
/// The `data` parameter is an opaque, Zelle-issued token - it is deliberately NOT decoded or
/// treated as recipient identity here: nothing in the payload verifies who the token belongs to,
/// so this recognizes the link structure and hands off, without claiming a payee.
pub struct Zelle;

impl PaymentScheme for Zelle {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "zelle".into(),
            display_name: "Zelle".into(),
            countries: vec!["US".into()],
            category: Category::PaymentLink,
            standard: "Zelle personal QR enrollment link".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Beta,
            static_supported: true,
            dynamic_supported: false,
            features: vec!["enroll-token".into(), "subtype".into()],
            references: vec!["https://www.zellepay.com/how-it-works".into()],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        let Ok(url) = Url::parse(payload) else {
            return 0;
        };
        let is_zelle = url.host_str().is_some_and(|h| {
            h.eq_ignore_ascii_case("zellepay.com") || h.eq_ignore_ascii_case("enroll.zellepay.com")
        });
        if is_zelle && url.path().starts_with("/qr-codes") {
            95
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        let url = Url::parse(payload)
            .map_err(|_| ParseError::new(ErrorCode::UnsafeUri, "Invalid Zelle URL."))?;
        if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
            return Err(ParseError::new(
                ErrorCode::UnsafeUri,
                "Zelle links must use https without embedded credentials.",
            ));
        }
        let has_token = url
            .query_pairs()
            .any(|(k, v)| k == "data" && !v.is_empty() && v.len() <= 4096);
        if !has_token {
            return Err(ParseError::new(
                ErrorCode::InvalidRecipient,
                "Zelle QR link is missing its enrollment token.",
            ));
        }
        let mut intent = PaymentIntent::recognized("zelle", Category::PaymentLink);
        intent.subtype = Some("enroll_qr".into());
        intent.standard = Some("Zelle personal QR enrollment link".into());
        intent.country = Some("US".into());
        intent.network = Some("Zelle".into());
        intent.dynamic = Some(false);
        intent.validation.warnings.push(Issue::error(
            ErrorCode::ProprietaryFormat,
            "Link structure is public, but the recipient token is opaque and requires Zelle verification.",
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
    fn parses_zelle_qr_link() {
        let intent = parse_payment_qr("https://enroll.zellepay.com/qr-codes?data=AbCdEf123");
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "zelle");
        assert_eq!(intent.subtype.as_deref(), Some("enroll_qr"));
    }

    #[test]
    fn rejects_zelle_qr_link_without_token() {
        let intent = parse_payment_qr("https://enroll.zellepay.com/qr-codes");
        assert!(!intent.validation.valid);
    }
}
