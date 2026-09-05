use url::Url;

use crate::decimal::validate_decimal;
use crate::registry::PaymentScheme;
use crate::{
    ActionType, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, Recipient,
    RecommendedAction, SchemeMetadata,
};

/// PayPal has more than one public QR/link shape (paypal.me handles, shareable invoice links,
/// and others not yet confirmed against a stable URL pattern). All of them are recognized under
/// one `paypal` scheme with a `subtype` distinguishing the shape - see `PaymentIntent::subtype`.
/// None of this needs PayPal API access: every subtype here is identified from a public link
/// pattern alone, the same way `paypal.me` already was.
pub struct PayPal;

const PAYPAL_ME_HOST: &str = "paypal.me";
const PAYPAL_HOSTS: [&str; 2] = ["paypal.com", "www.paypal.com"];

impl PaymentScheme for PayPal {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "paypal".into(),
            display_name: "PayPal".into(),
            countries: vec![],
            category: Category::PaymentLink,
            standard: "Public PayPal link patterns (paypal.me, invoice)".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Beta,
            static_supported: true,
            dynamic_supported: false,
            features: vec![
                "paypal-me-handle".into(),
                "amount".into(),
                "invoice-link".into(),
                "subtype".into(),
            ],
            references: vec![
                "https://www.paypal.com/paypalme/".into(),
                "https://www.paypal.com/invoice/p/".into(),
            ],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        let Ok(url) = Url::parse(payload) else {
            return 0;
        };
        let Some(host) = url.host_str() else {
            return 0;
        };
        if host.eq_ignore_ascii_case(PAYPAL_ME_HOST) {
            95
        } else if PAYPAL_HOSTS.iter().any(|h| host.eq_ignore_ascii_case(h))
            && url.path().starts_with("/invoice/")
        {
            85
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        let url = Url::parse(payload)
            .map_err(|_| ParseError::new(ErrorCode::UnsafeUri, "Invalid PayPal URL."))?;
        if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
            return Err(ParseError::new(
                ErrorCode::UnsafeUri,
                "PayPal links must use https without embedded credentials.",
            ));
        }
        let host = url
            .host_str()
            .ok_or_else(|| ParseError::new(ErrorCode::UnsafeUri, "PayPal URL has no host."))?;
        if host.eq_ignore_ascii_case(PAYPAL_ME_HOST) {
            parse_paypal_me(payload, &url)
        } else if PAYPAL_HOSTS.iter().any(|h| host.eq_ignore_ascii_case(h))
            && url.path().starts_with("/invoice/")
        {
            parse_invoice_link(payload, &url)
        } else {
            Err(ParseError::new(
                ErrorCode::ProprietaryFormat,
                "Recognized as a PayPal link, but not a shape this parser can classify yet.",
            ))
        }
    }
}

fn parse_paypal_me(payload: &str, url: &Url) -> Result<PaymentIntent, ParseError> {
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
    intent.subtype = Some("paypal_me".into());
    intent.standard = Some("PayPal.Me public link".into());
    intent.network = Some("PayPal".into());
    intent.recipient = Some(Recipient {
        id: Some((*handle).into()),
        ..Recipient::default()
    });
    intent.amount = amount;
    intent.dynamic = Some(false);
    intent.validation.warnings.push(Issue::error(
        ErrorCode::ProprietaryFormat,
        "Link structure is public, but recipient and payment details require PayPal verification.",
    ));
    intent.recommended_action = Some(RecommendedAction::new(
        ActionType::Redirect,
        Some(payload.into()),
        true,
    ));
    Ok(intent)
}

fn parse_invoice_link(payload: &str, url: &Url) -> Result<PaymentIntent, ParseError> {
    let has_id = url
        .path_segments()
        .into_iter()
        .flatten()
        .any(|s| !s.is_empty() && s != "invoice" && s != "p");
    let has_fragment_id = url.fragment().is_some_and(|f| !f.is_empty());
    if !has_id && !has_fragment_id {
        return Err(ParseError::new(
            ErrorCode::InvalidRecipient,
            "PayPal invoice link is missing an invoice identifier.",
        ));
    }
    let mut intent = PaymentIntent::recognized("paypal", Category::PaymentLink);
    intent.subtype = Some("invoice_qr".into());
    intent.standard = Some("PayPal shareable invoice link".into());
    intent.network = Some("PayPal".into());
    intent.dynamic = Some(true);
    intent.validation.warnings.push(Issue::error(
        ErrorCode::ProprietaryFormat,
        "Link structure is public, but invoice amount, currency, and payee require PayPal verification.",
    ));
    intent.recommended_action = Some(RecommendedAction::new(
        ActionType::Redirect,
        Some(payload.into()),
        true,
    ));
    Ok(intent)
}

#[cfg(test)]
mod tests {
    use crate::parse_payment_qr;

    #[test]
    fn parses_paypal_me_with_subtype() {
        let intent = parse_payment_qr("https://paypal.me/exampleuser/25.00");
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "paypal");
        assert_eq!(intent.subtype.as_deref(), Some("paypal_me"));
        assert_eq!(intent.amount.as_deref(), Some("25.00"));
    }

    #[test]
    fn parses_invoice_link_with_subtype() {
        let intent = parse_payment_qr("https://www.paypal.com/invoice/p/#INV2-ABCD-1234-EFGH");
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "paypal");
        assert_eq!(intent.subtype.as_deref(), Some("invoice_qr"));
    }

    #[test]
    fn rejects_credentials_embedded_in_the_url() {
        assert!(
            !parse_payment_qr("https://user:pass@paypal.me/exampleuser")
                .validation
                .valid
        );
    }
}
