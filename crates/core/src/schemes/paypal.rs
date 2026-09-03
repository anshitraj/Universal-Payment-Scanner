use url::Url;

use crate::decimal::validate_decimal;
use crate::registry::PaymentScheme;
use crate::{
    ActionType, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, Recipient,
    RecommendedAction, SchemeMetadata,
};

pub struct PayPal;

impl PaymentScheme for PayPal {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "paypal".into(),
            display_name: "PayPal.Me".into(),
            countries: vec![],
            category: Category::PaymentLink,
            standard: "PayPal.Me public link".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Beta,
            static_supported: true,
            dynamic_supported: false,
            features: vec!["recipient-handle".into(), "amount".into()],
            references: vec!["https://www.paypal.com/paypalme/".into()],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        Url::parse(payload)
            .ok()
            .and_then(|url| url.host_str().map(|h| h.eq_ignore_ascii_case("paypal.me")))
            .filter(|v| *v)
            .map(|_| 95)
            .unwrap_or(0)
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        let url = Url::parse(payload)
            .map_err(|_| ParseError::new(ErrorCode::UnsafeUri, "Invalid PayPal URL."))?;
        if url.scheme() != "https"
            || !url.username().is_empty()
            || url.password().is_some()
            || url.host_str() != Some("paypal.me")
        {
            return Err(ParseError::new(
                ErrorCode::UnsafeUri,
                "PayPal links must use https://paypal.me without credentials.",
            ));
        }
        let segments: Vec<_> = url
            .path_segments()
            .into_iter()
            .flatten()
            .filter(|s| !s.is_empty())
            .collect();
        let handle = segments
            .first()
            .filter(|h| {
                h.len() <= 64
                    && h.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            })
            .ok_or_else(|| {
                ParseError::new(
                    ErrorCode::InvalidRecipient,
                    "PayPal.Me handle is missing or malformed.",
                )
            })?;
        let amount = segments
            .get(1)
            .map(|value| validate_decimal(value, 2))
            .transpose()?;
        let mut intent = PaymentIntent::recognized("paypal", Category::PaymentLink);
        intent.standard = Some("PayPal.Me public link".into());
        intent.network = Some("PayPal".into());
        intent.recipient = Some(Recipient {
            id: Some((*handle).into()),
            ..Recipient::default()
        });
        intent.amount = amount;
        intent.dynamic = Some(false);
        intent.validation.warnings.push(Issue::error(ErrorCode::ProprietaryFormat, "Link structure is public, but recipient and payment details require PayPal verification."));
        intent.recommended_action = Some(RecommendedAction {
            kind: ActionType::Redirect,
            uri: Some(payload.into()),
            requires_user_confirmation: true,
        });
        Ok(intent)
    }
}
