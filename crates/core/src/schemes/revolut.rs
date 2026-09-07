use url::Url;

use crate::decimal::validate_decimal;
use crate::registry::PaymentScheme;
use crate::{
    ActionType, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, Recipient,
    RecommendedAction, SchemeMetadata,
};

/// Revolut payment links (`revolut.me/<username>`) are public. An optional `?amount=` and
/// `?currency=` pair is documented on the Revolut.Me share flow and parsed when present; the
/// username itself is a public handle, not verified identity.
pub struct Revolut;

impl PaymentScheme for Revolut {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "revolut".into(),
            display_name: "Revolut".into(),
            countries: vec!["GB".into()],
            category: Category::PaymentLink,
            standard: "Public Revolut.Me link".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Beta,
            static_supported: true,
            dynamic_supported: false,
            features: vec![
                "profile-handle".into(),
                "amount".into(),
                "currency".into(),
                "subtype".into(),
            ],
            references: vec!["https://www.revolut.com/revolut-me/".into()],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        let Ok(url) = Url::parse(payload) else {
            return 0;
        };
        let is_revolut = url
            .host_str()
            .is_some_and(|h| h.eq_ignore_ascii_case("revolut.me"));
        if is_revolut && !url.path().trim_matches('/').is_empty() {
            95
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        let url = Url::parse(payload)
            .map_err(|_| ParseError::new(ErrorCode::UnsafeUri, "Invalid Revolut URL."))?;
        if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
            return Err(ParseError::new(
                ErrorCode::UnsafeUri,
                "Revolut links must use https without embedded credentials.",
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
                    "Revolut handle is missing or malformed.",
                )
            })?;
        let mut amount = None;
        let mut currency = None;
        for (key, value) in url.query_pairs() {
            match key.as_ref() {
                "amount" => amount = Some(validate_decimal(&value, 2)?),
                "currency" => {
                    if value.len() == 3 && value.bytes().all(|b| b.is_ascii_alphabetic()) {
                        currency = Some(value.to_ascii_uppercase());
                    }
                }
                _ => {}
            }
        }
        let mut intent = PaymentIntent::recognized("revolut", Category::PaymentLink);
        intent.subtype = Some("revolut_me".into());
        intent.standard = Some("Public Revolut.Me link".into());
        intent.country = Some("GB".into());
        intent.network = Some("Revolut".into());
        intent.recipient = Some(Recipient {
            id: Some(handle.into()),
            ..Recipient::default()
        });
        intent.amount = amount;
        intent.currency = currency;
        intent.dynamic = Some(false);
        intent.validation.warnings.push(Issue::error(
            ErrorCode::ProprietaryFormat,
            "Link structure is public, but recipient and payment details require Revolut verification.",
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
    fn parses_revolut_link_with_amount_and_currency() {
        let intent = parse_payment_qr("https://revolut.me/exampleuser?amount=12.50&currency=gbp");
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "revolut");
        assert_eq!(intent.recipient.unwrap().id.as_deref(), Some("exampleuser"));
        assert_eq!(intent.amount.as_deref(), Some("12.50"));
        assert_eq!(intent.currency.as_deref(), Some("GBP"));
    }

    #[test]
    fn parses_bare_revolut_handle() {
        let intent = parse_payment_qr("https://revolut.me/exampleuser");
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "revolut");
    }
}
