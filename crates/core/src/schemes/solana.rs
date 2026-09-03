use url::Url;

use crate::decimal::validate_decimal;
use crate::registry::PaymentScheme;
use crate::{
    ActionType, Asset, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, Recipient,
    RecommendedAction, SchemeMetadata,
};

pub struct SolanaPay;

impl PaymentScheme for SolanaPay {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "solana_pay".into(),
            display_name: "Solana Pay".into(),
            countries: vec![],
            category: Category::Crypto,
            standard: "Solana Pay transfer request 1.0".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Stable,
            static_supported: true,
            dynamic_supported: true,
            features: vec![
                "amount".into(),
                "spl-token".into(),
                "references".into(),
                "memo".into(),
                "transaction-request-detection".into(),
            ],
            references: vec!["https://docs.solanapay.com/spec".into()],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        if payload
            .get(..7)
            .is_some_and(|p| p.eq_ignore_ascii_case("solana:"))
        {
            100
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        let body = payload
            .split_once(':')
            .map(|(_, body)| body)
            .ok_or_else(|| {
                ParseError::new(ErrorCode::MalformedPayload, "Invalid Solana Pay URI.")
            })?;
        if body.starts_with("http%3A") || body.starts_with("https%3A") {
            let mut intent = PaymentIntent::recognized("solana_pay", Category::Crypto);
            intent.standard = Some("Solana Pay transaction request 1.0".into());
            intent.network = Some("solana".into());
            intent.dynamic = Some(true);
            intent.validation.warnings.push(Issue::error(
                ErrorCode::UnverifiedRecipient,
                "Transaction request URL is not fetched or verified.",
            ));
            intent.recommended_action = Some(RecommendedAction {
                kind: ActionType::Wallet,
                uri: Some(payload.into()),
                requires_user_confirmation: true,
            });
            return Ok(intent);
        }
        let normalized = format!("solana://{}", body.trim_start_matches("//"));
        let url = Url::parse(&normalized)
            .map_err(|_| ParseError::new(ErrorCode::MalformedPayload, "Invalid Solana Pay URI."))?;
        let recipient = url.host_str().ok_or_else(|| {
            ParseError::new(ErrorCode::InvalidRecipient, "Solana recipient is required.")
        })?;
        validate_pubkey(recipient)?;
        let mut amount = None;
        let mut token = None;
        let mut label = None;
        let mut message = None;
        let mut memo = None;
        let mut references = Vec::new();
        for (key, value) in url.query_pairs() {
            match key.as_ref() {
                "amount" => amount = Some(validate_decimal(&value, 18)?),
                "spl-token" => {
                    validate_pubkey(&value)?;
                    token = Some(value.into_owned());
                }
                "reference" => {
                    validate_pubkey(&value)?;
                    references.push(value.into_owned());
                }
                "label" => label = Some(value.into_owned()),
                "message" => message = Some(value.into_owned()),
                "memo" => memo = Some(value.into_owned()),
                _ => {}
            }
        }
        let mut intent = PaymentIntent::recognized("solana_pay", Category::Crypto);
        intent.standard = Some("Solana Pay transfer request 1.0".into());
        intent.network = Some("solana".into());
        intent.recipient = Some(Recipient {
            address: Some(recipient.into()),
            name: label,
            ..Recipient::default()
        });
        intent.amount = amount;
        intent.asset = Some(Asset {
            symbol: token.is_none().then(|| "SOL".into()),
            contract: token,
            decimals: None,
            amount_atomic: None,
        });
        intent.description = message;
        intent.dynamic = Some(false);
        intent
            .metadata
            .insert("references".into(), serde_json::json!(references));
        if let Some(memo) = memo {
            intent.metadata.insert("memo".into(), memo.into());
        }
        intent.validation.warnings.push(Issue::error(ErrorCode::UnverifiedRecipient, "Solana address encoding is valid; account ownership and mint metadata are not verified."));
        intent.recommended_action = Some(RecommendedAction {
            kind: ActionType::Wallet,
            uri: Some(payload.into()),
            requires_user_confirmation: true,
        });
        Ok(intent)
    }
}

fn validate_pubkey(value: &str) -> Result<(), ParseError> {
    let decoded = bs58::decode(value).into_vec().map_err(|_| {
        ParseError::new(ErrorCode::InvalidRecipient, "Solana address is not base58.")
    })?;
    if decoded.len() == 32 {
        Ok(())
    } else {
        Err(ParseError::new(
            ErrorCode::InvalidRecipient,
            "Solana address must decode to 32 bytes.",
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_payment_qr;

    #[test]
    fn parses_solana_transfer_request() {
        let intent = parse_payment_qr(
            "solana:9xQeWvG816bUx9EPfEZi4q3G44E2sA6v7a6D9k5GmRse?amount=1.25&label=Store",
        );
        assert!(intent.validation.valid);
        assert_eq!(intent.amount.as_deref(), Some("1.25"));
    }
}
