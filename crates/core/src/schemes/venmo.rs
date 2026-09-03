use url::Url;

use crate::registry::PaymentScheme;
use crate::{
    ActionType, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, Recipient,
    RecommendedAction, SchemeMetadata,
};

/// Venmo profile links (`venmo.com/u/<handle>`) are public and unambiguous - no other Venmo page
/// uses the `/u/` path prefix. The bare-handle business-profile form (`venmo.com/<handle>`, no
/// `/u/`) is documented too, but is deliberately not detected here: it's indistinguishable from
/// any other single-segment page on venmo.com (help, legal, ...) without a confirmed allowlist,
/// and a false positive there is worse than under-detecting.
pub struct Venmo;

impl PaymentScheme for Venmo {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "venmo".into(),
            display_name: "Venmo".into(),
            countries: vec!["US".into()],
            category: Category::PaymentLink,
            standard: "Public Venmo profile link".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Beta,
            static_supported: true,
            dynamic_supported: false,
            features: vec!["profile-handle".into(), "subtype".into()],
            references: vec![
                "https://help.venmo.com/cs/articles/personal-qr-codes-on-venmo-faq-vhel316".into(),
            ],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        let Ok(url) = Url::parse(payload) else {
            return 0;
        };
        let is_venmo = url.host_str().is_some_and(|h| {
            h.eq_ignore_ascii_case("venmo.com") || h.eq_ignore_ascii_case("www.venmo.com")
        });
        if is_venmo && url.path().starts_with("/u/") {
            95
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        let url = Url::parse(payload)
            .map_err(|_| ParseError::new(ErrorCode::UnsafeUri, "Invalid Venmo URL."))?;
        if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
            return Err(ParseError::new(
                ErrorCode::UnsafeUri,
                "Venmo links must use https without embedded credentials.",
            ));
        }
        let handle = url
            .path_segments()
            .into_iter()
            .flatten()
            .nth(1)
            .filter(|h| {
                !h.is_empty()
                    && h.len() <= 64
                    && h.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            })
            .ok_or_else(|| {
                ParseError::new(
                    ErrorCode::InvalidRecipient,
                    "Venmo profile handle is missing or malformed.",
                )
            })?;
        let mut intent = PaymentIntent::recognized("venmo", Category::PaymentLink);
        intent.subtype = Some("profile".into());
        intent.standard = Some("Public Venmo profile link".into());
        intent.country = Some("US".into());
        intent.network = Some("Venmo".into());
        intent.recipient = Some(Recipient {
            id: Some(handle.into()),
            ..Recipient::default()
        });
        intent.dynamic = Some(false);
        intent.validation.warnings.push(Issue::error(
            ErrorCode::ProprietaryFormat,
            "Link structure is public, but recipient and payment details require Venmo verification.",
        ));
        intent.recommended_action = Some(RecommendedAction {
            kind: ActionType::Redirect,
            uri: Some(payload.into()),
            requires_user_confirmation: true,
        });
        Ok(intent)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_payment_qr;

    #[test]
    fn parses_venmo_profile_link() {
        let intent = parse_payment_qr("https://venmo.com/u/Example-User");
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "venmo");
        assert_eq!(intent.subtype.as_deref(), Some("profile"));
        assert_eq!(
            intent.recipient.unwrap().id.as_deref(),
            Some("Example-User")
        );
    }

    #[test]
    fn does_not_claim_an_unrelated_venmo_com_page_as_a_venmo_profile() {
        // Still recognized (falls through to the generic-URL detector), just not misclassified
        // as a Venmo profile/payment link.
        assert_ne!(
            parse_payment_qr("https://venmo.com/legal/us-user-agreement").scheme,
            "venmo"
        );
    }
}
