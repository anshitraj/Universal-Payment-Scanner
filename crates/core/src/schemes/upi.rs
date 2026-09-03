use url::Url;

use crate::decimal::validate_decimal;
use crate::registry::PaymentScheme;
use crate::{
    ActionType, Asset, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, Recipient,
    RecommendedAction, SchemeMetadata,
};

pub struct Upi;

impl PaymentScheme for Upi {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "upi".into(),
            display_name: "UPI".into(),
            countries: vec!["IN".into()],
            category: Category::BankTransfer,
            standard: "UPI deep link v1".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Stable,
            static_supported: true,
            dynamic_supported: true,
            features: vec!["amount".into(), "currency".into(), "reference".into(), "merchant".into()],
            references: vec!["https://www.npci.org.in/PDF/npci/upi/circular/2017/Circular18_BankCompliances_to_enbaleUPIMerchantecosystem_0.pdf".into()],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        if payload
            .get(..10)
            .is_some_and(|p| p.eq_ignore_ascii_case("upi://pay?"))
        {
            100
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        let url = Url::parse(payload)
            .map_err(|_| ParseError::new(ErrorCode::MalformedPayload, "Invalid UPI URI."))?;
        if !url.scheme().eq_ignore_ascii_case("upi") || url.host_str() != Some("pay") {
            return Err(ParseError::new(
                ErrorCode::MalformedPayload,
                "UPI URI must use upi://pay.",
            ));
        }
        let mut pa = None;
        let mut pn = None;
        let mut amount = None;
        let mut currency = None;
        let mut reference = None;
        let mut description = None;
        let mut mode = None;
        for (key, value) in url.query_pairs() {
            if value.len() > 256 {
                return Err(ParseError::new(
                    ErrorCode::MalformedPayload,
                    "UPI parameter exceeds 256 characters.",
                ));
            }
            match key.as_ref() {
                "pa" => pa = Some(value.into_owned()),
                "pn" => pn = Some(value.into_owned()),
                "am" => amount = Some(validate_decimal(&value, 2)?),
                "cu" => currency = Some(value.into_owned()),
                "tr" | "tid" => reference = Some(value.into_owned()),
                "tn" => description = Some(value.into_owned()),
                "mode" => mode = Some(value.into_owned()),
                _ => {}
            }
        }
        let pa = pa.ok_or_else(|| {
            ParseError::new(
                ErrorCode::InvalidRecipient,
                "UPI payee address (pa) is required.",
            )
        })?;
        if !valid_vpa(&pa) {
            return Err(ParseError::new(
                ErrorCode::InvalidRecipient,
                "UPI payee address is malformed.",
            ));
        }
        let currency = currency.unwrap_or_else(|| "INR".into());
        if currency != "INR" {
            return Err(ParseError::new(
                ErrorCode::InvalidCurrency,
                "UPI currency must be INR.",
            ));
        }
        let mut intent = PaymentIntent::recognized("upi", Category::BankTransfer);
        intent.standard = Some("UPI deep link v1".into());
        intent.country = Some("IN".into());
        intent.network = Some("UPI".into());
        intent.recipient = Some(Recipient {
            id: Some(pa),
            name: pn,
            ..Recipient::default()
        });
        intent.amount = amount;
        intent.currency = Some(currency);
        intent.asset = Some(Asset {
            symbol: Some("INR".into()),
            decimals: Some(2),
            ..Asset::default()
        });
        intent.reference = reference;
        intent.description = description;
        intent.dynamic = Some(mode.as_deref() == Some("02"));
        intent.validation.warnings.push(Issue::error(
            ErrorCode::UnverifiedRecipient,
            "UPI syntax is valid; recipient identity is not verified.",
        ));
        intent.recommended_action = Some(RecommendedAction {
            kind: ActionType::Deeplink,
            uri: Some(payload.into()),
            requires_user_confirmation: true,
        });
        Ok(intent)
    }
}

fn valid_vpa(value: &str) -> bool {
    let Some((local, handle)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !handle.is_empty()
        && value.len() <= 255
        && local
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-_".contains(&b))
        && handle
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-".contains(&b))
}

#[cfg(test)]
mod tests {
    use crate::parse_payment_qr;

    #[test]
    fn parses_upi_without_using_floats() {
        let intent =
            parse_payment_qr("upi://pay?pa=merchant%40bank&pn=Acme&am=499.00&cu=INR&tr=INV-7");
        assert!(intent.validation.valid);
        assert_eq!(intent.scheme, "upi");
        assert_eq!(intent.amount.as_deref(), Some("499.00"));
        assert_eq!(
            intent.recipient.unwrap().id.as_deref(),
            Some("merchant@bank")
        );
    }

    #[test]
    fn rejects_invalid_vpa() {
        assert!(
            !parse_payment_qr("upi://pay?pa=not-a-vpa&am=1")
                .validation
                .valid
        );
    }
}
