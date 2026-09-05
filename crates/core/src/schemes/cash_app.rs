use url::Url;

use crate::decimal::validate_decimal;
use crate::registry::PaymentScheme;
use crate::{
    ActionType, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, Recipient,
    RecommendedAction, SchemeMetadata,
};

/// Cash App `$cashtag` links (`cash.app/$<cashtag>[/<amount>]`) are public. The leading `$` in
/// the path segment is unambiguous - no other cash.app page uses that shape - so unlike Venmo's
/// bare-handle form this needs no allowlist to detect safely.
pub struct CashApp;

impl PaymentScheme for CashApp {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "cash_app".into(),
            display_name: "Cash App".into(),
            countries: vec!["US".into(), "GB".into()],
            category: Category::PaymentLink,
            standard: "Public Cash App $cashtag link".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Beta,
            static_supported: true,
            dynamic_supported: false,
            features: vec!["cashtag".into(), "amount".into(), "subtype".into()],
            references: vec!["https://cash.app/help/us/en-us/3123-cashtags".into()],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        let Ok(url) = Url::parse(payload) else {
            return 0;
        };
        let is_cash_app = url
            .host_str()
            .is_some_and(|h| h.eq_ignore_ascii_case("cash.app"));
        if is_cash_app && url.path().starts_with("/$") {
            95
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        let url = Url::parse(payload)
            .map_err(|_| ParseError::new(ErrorCode::UnsafeUri, "Invalid Cash App URL."))?;
        if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
            return Err(ParseError::new(
                ErrorCode::UnsafeUri,
                "Cash App links must use https without embedded credentials.",
            ));
        }
        let segments: Vec<_> = url
            .path_segments()
            .into_iter()
            .flatten()
            .filter(|s| !s.is_empty())
            .collect();
        let cashtag = segments
            .first()
            .and_then(|s| s.strip_prefix('$'))
            .filter(|tag| {
                !tag.is_empty() && tag.len() <= 20 && tag.bytes().all(|b| b.is_ascii_alphanumeric())
            })
            .ok_or_else(|| {
                ParseError::new(
                    ErrorCode::InvalidRecipient,
                    "Cash App $cashtag is missing or malformed.",
                )
            })?;
        let amount = segments
            .get(1)
            .map(|value| validate_decimal(value, 2))
            .transpose()?;
        let mut intent = PaymentIntent::recognized("cash_app", Category::PaymentLink);
        intent.subtype = Some("cashtag".into());
        intent.standard = Some("Public Cash App $cashtag link".into());
        intent.network = Some("Cash App".into());
        intent.recipient = Some(Recipient {
            id: Some(format!("${cashtag}")),
            ..Recipient::default()
        });
        intent.amount = amount;
        intent.dynamic = Some(false);
        intent.validation.warnings.push(Issue::error(
            ErrorCode::ProprietaryFormat,
            "Link structure is public, but recipient and payment details require Cash App verification.",
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
    fn parses_cashtag_link_with_amount() {
        let intent = parse_payment_qr("https://cash.app/$exampletag/12.50");
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "cash_app");
        assert_eq!(intent.subtype.as_deref(), Some("cashtag"));
        assert_eq!(intent.recipient.unwrap().id.as_deref(), Some("$exampletag"));
        assert_eq!(intent.amount.as_deref(), Some("12.50"));
    }
}
