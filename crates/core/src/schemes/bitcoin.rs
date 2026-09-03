use sha2::{Digest, Sha256};
use url::Url;

use crate::decimal::validate_decimal;
use crate::registry::PaymentScheme;
use crate::{
    ActionType, Asset, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, Recipient,
    RecommendedAction, SchemeMetadata,
};

pub struct Bitcoin;

impl PaymentScheme for Bitcoin {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "bitcoin".into(),
            display_name: "Bitcoin".into(),
            countries: vec![],
            category: Category::Crypto,
            standard: "BIP-21".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Stable,
            static_supported: true,
            dynamic_supported: false,
            features: vec![
                "amount".into(),
                "label".into(),
                "message".into(),
                "address-checksum".into(),
            ],
            references: vec!["https://bips.dev/21/".into()],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        if payload
            .get(..8)
            .is_some_and(|p| p.eq_ignore_ascii_case("bitcoin:"))
        {
            100
        } else if (14..=90).contains(&payload.len()) && validate_bitcoin_address(payload).is_ok() {
            // Most wallet "receive" screens QR-encode the bare address, not a `bitcoin:` BIP-21
            // URI - that form is mostly used for payment *requests* with a specific amount.
            85
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        if !payload.contains(':') {
            return parse_bare_address(payload);
        }
        let (_, body) = payload
            .split_once(':')
            .ok_or_else(|| ParseError::new(ErrorCode::MalformedPayload, "Invalid BIP-21 URI."))?;
        let normalized_uri = if body.starts_with("//") {
            format!("bitcoin:{body}")
        } else {
            format!("bitcoin://{body}")
        };
        let url = Url::parse(&normalized_uri)
            .map_err(|_| ParseError::new(ErrorCode::MalformedPayload, "Invalid BIP-21 URI."))?;
        let address = url
            .host_str()
            .filter(|v| !v.is_empty())
            .or_else(|| {
                let path = url.path().trim_start_matches('/');
                (!path.is_empty()).then_some(path)
            })
            .ok_or_else(|| {
                ParseError::new(ErrorCode::InvalidRecipient, "Bitcoin address is required.")
            })?;
        validate_bitcoin_address(address).map_err(|_| {
            ParseError::new(
                ErrorCode::InvalidRecipient,
                "Bitcoin address checksum or encoding is invalid.",
            )
        })?;
        let mut amount = None;
        let mut label = None;
        let mut message = None;
        for (key, value) in url.query_pairs() {
            if key.starts_with("req-") {
                return Err(ParseError::new(
                    ErrorCode::SchemeUnsupported,
                    format!("Unsupported required BIP-21 parameter: {key}."),
                ));
            }
            match key.as_ref() {
                "amount" => amount = Some(validate_decimal(&value, 8)?),
                "label" => label = Some(value.into_owned()),
                "message" => message = Some(value.into_owned()),
                _ => {}
            }
        }
        let mut intent = PaymentIntent::recognized("bitcoin", Category::Crypto);
        intent.standard = Some("BIP-21".into());
        intent.network = Some("bitcoin".into());
        intent.recipient = Some(Recipient {
            address: Some(address.into()),
            name: label,
            ..Recipient::default()
        });
        intent.amount = amount;
        intent.asset = Some(Asset {
            symbol: Some("BTC".into()),
            decimals: Some(8),
            ..Asset::default()
        });
        intent.description = message;
        intent.dynamic = Some(false);
        intent.validation.warnings.push(Issue::error(
            ErrorCode::UnverifiedRecipient,
            "Address encoding is valid; ownership is not verified.",
        ));
        intent.recommended_action = Some(RecommendedAction {
            kind: ActionType::Wallet,
            uri: Some(payload.into()),
            requires_user_confirmation: true,
        });
        Ok(intent)
    }
}

/// A bare address with no `bitcoin:` scheme and no amount - what most wallet apps put in a
/// "receive" QR. Normalized the same as `bitcoin:<address>` with no query parameters.
fn parse_bare_address(payload: &str) -> Result<PaymentIntent, ParseError> {
    validate_bitcoin_address(payload).map_err(|_| {
        ParseError::new(
            ErrorCode::InvalidRecipient,
            "Bitcoin address checksum or encoding is invalid.",
        )
    })?;
    let mut intent = PaymentIntent::recognized("bitcoin", Category::Crypto);
    intent.standard = Some("BIP-21".into());
    intent.network = Some("bitcoin".into());
    intent.recipient = Some(Recipient {
        address: Some(payload.into()),
        ..Recipient::default()
    });
    intent.asset = Some(Asset {
        symbol: Some("BTC".into()),
        decimals: Some(8),
        ..Asset::default()
    });
    intent.dynamic = Some(false);
    intent.validation.warnings.push(Issue::error(
        ErrorCode::UnverifiedRecipient,
        "Address encoding is valid; ownership is not verified.",
    ));
    intent.recommended_action = Some(RecommendedAction {
        kind: ActionType::Wallet,
        uri: Some(payload.into()),
        requires_user_confirmation: true,
    });
    Ok(intent)
}

fn validate_bitcoin_address(address: &str) -> Result<(), ()> {
    if address.starts_with('1')
        || address.starts_with('3')
        || address.starts_with('m')
        || address.starts_with('n')
        || address.starts_with('2')
    {
        let decoded = bs58::decode(address).into_vec().map_err(|_| ())?;
        if decoded.len() != 25 {
            return Err(());
        }
        let version = decoded[0];
        if !matches!(version, 0 | 5 | 111 | 196) {
            return Err(());
        }
        let first = Sha256::digest(&decoded[..21]);
        let second = Sha256::digest(first);
        return (second[..4] == decoded[21..]).then_some(()).ok_or(());
    }
    validate_segwit_address(address)
}

fn validate_segwit_address(address: &str) -> Result<(), ()> {
    if address.len() < 14
        || address.len() > 90
        || address.to_ascii_lowercase() != address && address.to_ascii_uppercase() != address
    {
        return Err(());
    }
    let lower = address.to_ascii_lowercase();
    let separator = lower.rfind('1').ok_or(())?;
    let hrp = &lower[..separator];
    if !matches!(hrp, "bc" | "tb" | "bcrt") || lower.len() - separator - 1 < 7 {
        return Err(());
    }
    let charset = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
    let values: Vec<u8> = lower[separator + 1..]
        .bytes()
        .map(|byte| {
            charset
                .iter()
                .position(|candidate| *candidate == byte)
                .map(|value| value as u8)
                .ok_or(())
        })
        .collect::<Result<_, _>>()?;
    let mut expanded: Vec<u8> = hrp.bytes().map(|byte| byte >> 5).collect();
    expanded.push(0);
    expanded.extend(hrp.bytes().map(|byte| byte & 31));
    expanded.extend(&values);
    let checksum = bech32_polymod(&expanded);
    let witness_version = *values.first().ok_or(())?;
    let expected = if witness_version == 0 { 1 } else { 0x2bc830a3 };
    if checksum != expected || witness_version > 16 {
        return Err(());
    }
    let program = convert_bits(&values[1..values.len() - 6], 5, 8, false)?;
    if !(2..=40).contains(&program.len())
        || witness_version == 0 && !matches!(program.len(), 20 | 32)
    {
        return Err(());
    }
    Ok(())
}

fn bech32_polymod(values: &[u8]) -> u32 {
    const GENERATORS: [u32; 5] = [0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3];
    let mut checksum = 1u32;
    for value in values {
        let top = checksum >> 25;
        checksum = (checksum & 0x1ffffff) << 5 ^ *value as u32;
        for (index, generator) in GENERATORS.iter().enumerate() {
            if (top >> index) & 1 == 1 {
                checksum ^= generator;
            }
        }
    }
    checksum
}

fn convert_bits(data: &[u8], from: u32, to: u32, pad: bool) -> Result<Vec<u8>, ()> {
    let mut accumulator = 0u32;
    let mut bits = 0u32;
    let mut result = Vec::with_capacity(data.len());
    let max_value = (1u32 << to) - 1;
    for value in data {
        if (*value as u32) >> from != 0 {
            return Err(());
        }
        accumulator = (accumulator << from) | *value as u32;
        bits += from;
        while bits >= to {
            bits -= to;
            result.push(((accumulator >> bits) & max_value) as u8);
        }
    }
    if pad && bits > 0 {
        result.push(((accumulator << (to - bits)) & max_value) as u8);
    } else if bits >= from || (accumulator << (to - bits)) & max_value != 0 {
        return Err(());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use crate::parse_payment_qr;

    #[test]
    fn parses_bip21() {
        let intent = parse_payment_qr(
            "bitcoin:1BoatSLRHtKNngkdXEeobR76b53LETtpyT?amount=0.00100000&label=Donation",
        );
        assert!(intent.validation.valid);
        assert_eq!(intent.amount.as_deref(), Some("0.00100000"));
    }

    #[test]
    fn validates_address_checksum() {
        assert!(
            !parse_payment_qr("bitcoin:1BoatSLRHtKNngkdXEeobR76b53LETtpyX")
                .validation
                .valid
        );
    }

    #[test]
    fn recognizes_a_bare_address_with_no_uri_scheme() {
        let intent = parse_payment_qr("1BoatSLRHtKNngkdXEeobR76b53LETtpyT");
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "bitcoin");
        assert_eq!(
            intent.recipient.unwrap().address.as_deref(),
            Some("1BoatSLRHtKNngkdXEeobR76b53LETtpyT")
        );
        assert!(intent.amount.is_none());
    }
}
