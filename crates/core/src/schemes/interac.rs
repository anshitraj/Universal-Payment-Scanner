use url::Url;

use crate::registry::PaymentScheme;
use crate::{
    ActionType, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent,
    RecommendedAction, SchemeMetadata,
};

/// Interac e-Transfer QR codes (Canada) encode a redirect through the interac.ca domain -
/// `https://etransfer.interac.ca/...` - carrying an opaque, Interac-issued reference. Like Zelle's
/// enrollment token, nothing in the payload verifies whose transfer this is, so this recognizes
/// the link structure and hands off rather than inferring a payee.
pub struct Interac;

impl PaymentScheme for Interac {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "interac".into(),
            display_name: "Interac e-Transfer".into(),
            countries: vec!["CA".into()],
            category: Category::PaymentLink,
            standard: "Interac e-Transfer QR link".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Beta,
            static_supported: true,
            dynamic_supported: false,
            features: vec!["reference-token".into()],
            references: vec!["https://www.interac.ca/en/interac-e-transfer/".into()],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        let Ok(url) = Url::parse(payload) else {
            return 0;
        };
        let is_interac = url
            .host_str()
            .is_some_and(|h| h.eq_ignore_ascii_case("etransfer.interac.ca"));
        if is_interac && !url.path().trim_matches('/').is_empty() {
            90
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        let url = Url::parse(payload)
            .map_err(|_| ParseError::new(ErrorCode::UnsafeUri, "Invalid Interac URL."))?;
        if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
            return Err(ParseError::new(
                ErrorCode::UnsafeUri,
                "Interac links must use https without embedded credentials.",
            ));
        }
        if url.path().trim_matches('/').is_empty() {
            return Err(ParseError::new(
                ErrorCode::InvalidRecipient,
                "Interac e-Transfer link is missing its reference.",
            ));
        }
        let mut intent = PaymentIntent::recognized("interac", Category::PaymentLink);
        intent.subtype = Some("etransfer_qr".into());
        intent.standard = Some("Interac e-Transfer QR link".into());
        intent.country = Some("CA".into());
        intent.network = Some("Interac".into());
        intent.dynamic = Some(false);
        intent.validation.warnings.push(Issue::error(
            ErrorCode::ProprietaryFormat,
            "Link structure is public, but the reference token is opaque and requires Interac verification.",
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
    fn parses_interac_etransfer_link() {
        let intent = parse_payment_qr("https://etransfer.interac.ca/qr/AbCdEf123456");
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "interac");
        assert_eq!(intent.country.as_deref(), Some("CA"));
    }

    #[test]
    fn rejects_credentials_embedded_in_the_url() {
        assert!(
            !parse_payment_qr("https://user:pass@etransfer.interac.ca/qr/AbCdEf")
                .validation
                .valid
        );
    }
}
