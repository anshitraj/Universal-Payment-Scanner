use url::Url;

use crate::decimal::validate_decimal;
use crate::registry::PaymentScheme;
use crate::{
    ActionType, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, Recipient,
    RecommendedAction, SchemeMetadata,
};

/// Monzo.Me links (`monzo.me/<username>`) are public request-money links. The share flow appends
/// an optional `?amount=` (in pounds) and `?d=` (description); both are parsed when present. The
/// username is a public handle, not verified identity.
pub struct Monzo;

impl PaymentScheme for Monzo {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "monzo".into(),
            display_name: "Monzo".into(),
            countries: vec!["GB".into()],
            category: Category::PaymentLink,
            standard: "Public Monzo.Me link".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Beta,
            static_supported: true,
            dynamic_supported: false,
            features: vec![
                "profile-handle".into(),
                "amount".into(),
                "reference".into(),
                "subtype".into(),
            ],
            references: vec!["https://monzo.com/monzo-me/".into()],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        let Ok(url) = Url::parse(payload) else {
            return 0;
        };
        let is_monzo = url
            .host_str()
            .is_some_and(|h| h.eq_ignore_ascii_case("monzo.me"));
        if is_monzo && !url.path().trim_matches('/').is_empty() {
            95
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        let url = Url::parse(payload)
            .map_err(|_| ParseError::new(ErrorCode::UnsafeUri, "Invalid Monzo URL."))?;
        if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
            return Err(ParseError::new(
                ErrorCode::UnsafeUri,
                "Monzo links must use https without embedded credentials.",
            ));
        }
        let handle = url
            .path_segments()
            .into_iter()
            .flatten()
            .next()
            .filter(|h| {
                !h.is_empty()
                    && h.len() <= 64
                    && h.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            })
            .ok_or_else(|| {
                ParseError::new(
                    ErrorCode::InvalidRecipient,
                    "Monzo handle is missing or malformed.",
                )
            })?;
        let mut amount = None;
        let mut reference = None;
        for (key, value) in url.query_pairs() {
            match key.as_ref() {
                "amount" => amount = Some(validate_decimal(&value, 2)?),
                "d" if !value.is_empty() && value.len() <= 256 => {
                    reference = Some(value.into_owned())
                }
                _ => {}
            }
        }
        let mut intent = PaymentIntent::recognized("monzo", Category::PaymentLink);
        intent.subtype = Some("monzo_me".into());
        intent.standard = Some("Public Monzo.Me link".into());
        intent.country = Some("GB".into());
        intent.network = Some("Monzo".into());
        intent.recipient = Some(Recipient {
            id: Some(handle.into()),
            ..Recipient::default()
        });
        intent.amount = amount;
        intent.reference = reference;
        intent.dynamic = Some(false);
        intent.validation.warnings.push(Issue::error(
            ErrorCode::ProprietaryFormat,
            "Link structure is public, but recipient and payment details require Monzo verification.",
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
    fn parses_monzo_link_with_amount_and_reference() {
        let intent = parse_payment_qr("https://monzo.me/exampleuser?amount=8.00&d=Dinner");
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "monzo");
        assert_eq!(intent.recipient.unwrap().id.as_deref(), Some("exampleuser"));
        assert_eq!(intent.amount.as_deref(), Some("8.00"));
        assert_eq!(intent.reference.as_deref(), Some("Dinner"));
    }
}
