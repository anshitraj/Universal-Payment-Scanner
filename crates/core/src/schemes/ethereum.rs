use url::form_urlencoded;

use crate::decimal::integer_scientific_to_decimal;
use crate::registry::PaymentScheme;
use crate::{
    ActionType, Asset, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, Recipient,
    RecommendedAction, SchemeMetadata,
};

pub struct Ethereum;

impl PaymentScheme for Ethereum {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "ethereum".into(),
            display_name: "Ethereum / ERC-681".into(),
            countries: vec![],
            category: Category::Crypto,
            standard: "ERC-681".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Stable,
            static_supported: true,
            dynamic_supported: false,
            features: vec![
                "native-transfer".into(),
                "erc-20-transfer".into(),
                "chain-id".into(),
                "atomic-amounts".into(),
            ],
            references: vec!["https://eips.ethereum.org/EIPS/eip-681".into()],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        if payload
            .get(..9)
            .is_some_and(|p| p.eq_ignore_ascii_case("ethereum:"))
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
            .ok_or_else(|| ParseError::new(ErrorCode::MalformedPayload, "Invalid ERC-681 URI."))?;
        let body = body.strip_prefix("pay-").unwrap_or(body);
        let (path, query) = body.split_once('?').unwrap_or((body, ""));
        let (target_chain, function) = path.split_once('/').unwrap_or((path, ""));
        let (target, chain_id) = target_chain.split_once('@').unwrap_or((target_chain, ""));
        validate_eth_target(target)?;
        if !chain_id.is_empty()
            && (chain_id.len() > 20 || !chain_id.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err(ParseError::new(
                ErrorCode::InvalidNetwork,
                "ERC-681 chain ID must be an unsigned decimal integer.",
            ));
        }
        let parameters: std::collections::BTreeMap<_, _> = form_urlencoded::parse(query.as_bytes())
            .into_owned()
            .collect();
        let is_token = function == "transfer";
        if !function.is_empty() && !is_token {
            return Err(ParseError::new(
                ErrorCode::SchemeUnsupported,
                "Only native and ERC-20 transfer intents are supported.",
            ));
        }
        let mut intent = PaymentIntent::recognized("ethereum", Category::Crypto);
        intent.standard = Some("ERC-681".into());
        intent.network = Some(chain_name(chain_id).into());
        intent.dynamic = Some(false);
        if is_token {
            let recipient = parameters.get("address").ok_or_else(|| {
                ParseError::new(
                    ErrorCode::InvalidRecipient,
                    "ERC-20 transfer requires an address parameter.",
                )
            })?;
            validate_hex_address(recipient)?;
            let atomic = parameters
                .get("uint256")
                .map(|v| parse_atomic_integer(v))
                .transpose()?;
            intent.recipient = Some(Recipient {
                address: Some(recipient.clone()),
                ..Recipient::default()
            });
            intent.asset = Some(Asset {
                contract: Some(target.into()),
                amount_atomic: atomic,
                ..Asset::default()
            });
            intent
                .metadata
                .insert("tokenDecimalsKnown".into(), false.into());
        } else {
            intent.recipient = Some(Recipient {
                address: Some(target.into()),
                ..Recipient::default()
            });
            if let Some(value) = parameters.get("value") {
                intent.amount = Some(integer_scientific_to_decimal(value, 18)?);
                intent.asset = Some(Asset {
                    symbol: Some("ETH".into()),
                    decimals: Some(18),
                    amount_atomic: Some(parse_atomic_integer(value)?),
                    ..Asset::default()
                });
            } else {
                intent.asset = Some(Asset {
                    symbol: Some("ETH".into()),
                    decimals: Some(18),
                    ..Asset::default()
                });
            }
        }
        intent.validation.warnings.push(Issue::error(
            ErrorCode::UnverifiedRecipient,
            "ERC-681 syntax is valid; address ownership and chain state are not verified.",
        ));
        intent.recommended_action = Some(RecommendedAction {
            kind: ActionType::Wallet,
            uri: Some(payload.into()),
            requires_user_confirmation: true,
        });
        Ok(intent)
    }
}

fn validate_eth_target(value: &str) -> Result<(), ParseError> {
    if value.starts_with("0x") {
        validate_hex_address(value)
    } else if value.len() <= 255
        && value.contains('.')
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-".contains(&b))
    {
        Ok(())
    } else {
        Err(ParseError::new(
            ErrorCode::InvalidRecipient,
            "Ethereum target must be a hexadecimal address or ENS name.",
        ))
    }
}

fn validate_hex_address(value: &str) -> Result<(), ParseError> {
    if value.len() == 42
        && value.starts_with("0x")
        && value[2..].bytes().all(|b| b.is_ascii_hexdigit())
    {
        Ok(())
    } else {
        Err(ParseError::new(
            ErrorCode::InvalidRecipient,
            "Ethereum address must contain 20 hexadecimal bytes.",
        ))
    }
}

fn parse_atomic_integer(value: &str) -> Result<String, ParseError> {
    let decimal = integer_scientific_to_decimal(value, 0)?;
    if decimal.contains('.') {
        Err(ParseError::new(
            ErrorCode::InvalidAmount,
            "Atomic amount must be an integer.",
        ))
    } else {
        Ok(decimal)
    }
}

fn chain_name(id: &str) -> &'static str {
    match id {
        "" | "1" => "ethereum",
        "10" => "optimism",
        "137" => "polygon",
        "42161" => "arbitrum",
        "8453" => "base",
        _ => "evm",
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_payment_qr;

    #[test]
    fn parses_native_erc681_value_exactly() {
        let intent = parse_payment_qr(
            "ethereum:0xfb6916095ca1df60bb79Ce92ce3ea74c37c5d359@1?value=2.014e18",
        );
        assert!(intent.validation.valid);
        assert_eq!(intent.amount.as_deref(), Some("2.014"));
    }

    #[test]
    fn parses_erc20_transfer_without_guessing_decimals() {
        let intent = parse_payment_qr(
            "ethereum:0x89205A3A3b2A69De6Dbf7f01ED13B2108B2c43e7/transfer?address=0x8e23ee67d1332ad560396262c48ffbb01f93d052&uint256=1",
        );
        assert!(intent.validation.valid);
        assert_eq!(intent.asset.unwrap().amount_atomic.as_deref(), Some("1"));
        assert!(intent.amount.is_none());
    }
}
