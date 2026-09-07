use url::Url;

use crate::registry::PaymentScheme;
use crate::{
    ActionType, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, Recipient,
    RecommendedAction, SchemeMetadata,
};

/// Wise payment links (`wise.com/pay/me/<slug>` for personal, `wise.com/pay/business/<slug>` for
/// business) are public request-money links. The `/pay/me/` and `/pay/business/` path prefixes are
/// unambiguous - no other wise.com page uses them - so detection needs no allowlist. The slug is a
/// public link identifier, not verified payee identity.
pub struct Wise;

const WISE_HOSTS: [&str; 2] = ["wise.com", "www.wise.com"];

impl PaymentScheme for Wise {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "wise".into(),
            display_name: "Wise".into(),
            countries: vec![],
            category: Category::PaymentLink,
            standard: "Public Wise pay link".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Beta,
            static_supported: true,
            dynamic_supported: false,
            features: vec!["pay-link-slug".into(), "subtype".into()],
            references: vec!["https://wise.com/help/articles/2978066/how-do-i-request-money".into()],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        let Ok(url) = Url::parse(payload) else {
            return 0;
        };
        let is_wise = url
            .host_str()
            .is_some_and(|h| WISE_HOSTS.iter().any(|w| h.eq_ignore_ascii_case(w)));
        if is_wise && (url.path().starts_with("/pay/me/") || url.path().starts_with("/pay/business/"))
        {
            95
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        let url = Url::parse(payload)
            .map_err(|_| ParseError::new(ErrorCode::UnsafeUri, "Invalid Wise URL."))?;
        if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
            return Err(ParseError::new(
                ErrorCode::UnsafeUri,
                "Wise links must use https without embedded credentials.",
            ));
        }
        let segments: Vec<_> = url
            .path_segments()
            .into_iter()
            .flatten()
            .filter(|s| !s.is_empty())
            .collect();
        // segments == ["pay", "me"|"business", <slug>, ...]
        let kind = segments.get(1).copied().unwrap_or_default();
        let slug = segments
            .get(2)
            .filter(|s| {
                s.len() <= 64
                    && s.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            })
            .ok_or_else(|| {
                ParseError::new(
                    ErrorCode::InvalidRecipient,
                    "Wise pay-link slug is missing or malformed.",
                )
            })?;
        let mut intent = PaymentIntent::recognized("wise", Category::PaymentLink);
        intent.subtype = Some(if kind == "business" { "pay_business" } else { "pay_me" }.into());
        intent.standard = Some("Public Wise pay link".into());
        intent.network = Some("Wise".into());
        intent.recipient = Some(Recipient {
            id: Some((*slug).into()),
            ..Recipient::default()
        });
        intent.dynamic = Some(false);
        intent.validation.warnings.push(Issue::error(
            ErrorCode::ProprietaryFormat,
            "Link structure is public, but recipient and payment details require Wise verification.",
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
    fn parses_wise_personal_pay_link() {
        let intent = parse_payment_qr("https://wise.com/pay/me/exampleuser");
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "wise");
        assert_eq!(intent.subtype.as_deref(), Some("pay_me"));
        assert_eq!(intent.recipient.unwrap().id.as_deref(), Some("exampleuser"));
    }

    #[test]
    fn parses_wise_business_pay_link() {
        let intent = parse_payment_qr("https://wise.com/pay/business/acme-co");
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.subtype.as_deref(), Some("pay_business"));
    }
}
