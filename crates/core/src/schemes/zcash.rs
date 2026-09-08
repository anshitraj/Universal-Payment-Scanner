use sha2::{Digest, Sha256};
use url::Url;

use crate::decimal::validate_decimal;
use crate::registry::PaymentScheme;
use crate::{
    ActionType, Asset, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, Recipient,
    RecommendedAction, SchemeMetadata,
};

/// Zcash transparent ("t-") addresses use the identical Base58Check construction as Bitcoin
/// (double-SHA256 checksum, same alphabet) but with a 2-byte version prefix instead of Bitcoin's
/// 1 byte: 0x1CB8 for P2PKH ("t1...") and 0x1CBD for P2SH ("t3..."), confirmed against the zcashd
/// reference client's `chainparams.cpp`. Shielded ("z-", "u-") addresses use a different encoding
/// (Bech32/Bech32m over a Sapling/Orchard payload) not implemented here - only transparent
/// addresses and the ZIP-321 `zcash:` payment URI wrapping one are recognized.
pub struct Zcash;

impl PaymentScheme for Zcash {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "zcash".into(),
            display_name: "Zcash".into(),
            countries: vec![],
            category: Category::Crypto,
            standard: "Zcash transparent (t-addr) Base58Check".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Beta,
            static_supported: true,
            dynamic_supported: false,
            features: vec!["address-checksum".into(), "amount".into()],
            references: vec![
                "https://zips.z.cash/protocol/protocol.pdf".into(),
                "https://zips.z.cash/zip-0321".into(),
            ],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        if payload
            .get(..6)
            .is_some_and(|p| p.eq_ignore_ascii_case("zcash:"))
        {
            100
        } else if (35..=36).contains(&payload.len()) && validate_t_address(payload).is_ok() {
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
            .ok_or_else(|| ParseError::new(ErrorCode::MalformedPayload, "Invalid zcash: URI."))?;
        let normalized_uri = if body.starts_with("//") {
            format!("zcash:{body}")
        } else {
            format!("zcash://{body}")
        };
        let url = Url::parse(&normalized_uri)
            .map_err(|_| ParseError::new(ErrorCode::MalformedPayload, "Invalid zcash: URI."))?;
        let address = url
            .host_str()
            .filter(|v| !v.is_empty())
            .or_else(|| {
                let path = url.path().trim_start_matches('/');
                (!path.is_empty()).then_some(path)
            })
            .ok_or_else(|| {
                ParseError::new(ErrorCode::InvalidRecipient, "Zcash address is required.")
            })?;
        validate_t_address(address).map_err(|_| {
            ParseError::new(
                ErrorCode::InvalidRecipient,
                "Zcash address checksum or encoding is invalid.",
            )
        })?;
        let mut amount = None;
        let mut message = None;
        for (key, value) in url.query_pairs() {
            match key.as_ref() {
                "amount" => amount = Some(validate_decimal(&value, 8)?),
                "message" => message = Some(value.into_owned()),
                _ => {}
            }
        }
        let mut intent = PaymentIntent::recognized("zcash", Category::Crypto);
        intent.standard = Some("ZIP-321 payment URI".into());
        intent.network = Some("zcash".into());
        intent.recipient = Some(Recipient {
            address: Some(address.into()),
            ..Recipient::default()
        });
        intent.amount = amount;
        intent.asset = Some(Asset {
            symbol: Some("ZEC".into()),
            decimals: Some(8),
            ..Asset::default()
        });
        intent.description = message;
        intent.dynamic = Some(false);
        intent.validation.warnings.push(Issue::error(
            ErrorCode::UnverifiedRecipient,
            "Address encoding is valid; ownership is not verified.",
        ));
        intent.recommended_action = Some(RecommendedAction::new(
            ActionType::Wallet,
            Some(payload.into()),
            true,
        ));
        Ok(intent)
    }
}

fn parse_bare_address(payload: &str) -> Result<PaymentIntent, ParseError> {
    validate_t_address(payload).map_err(|_| {
        ParseError::new(
            ErrorCode::InvalidRecipient,
            "Zcash address checksum or encoding is invalid.",
        )
    })?;
    let mut intent = PaymentIntent::recognized("zcash", Category::Crypto);
    intent.standard = Some("Zcash transparent (t-addr) Base58Check".into());
    intent.network = Some("zcash".into());
    intent.recipient = Some(Recipient {
        address: Some(payload.into()),
        ..Recipient::default()
    });
    intent.asset = Some(Asset {
        symbol: Some("ZEC".into()),
        decimals: Some(8),
        ..Asset::default()
    });
    intent.dynamic = Some(false);
    intent.validation.warnings.push(Issue::error(
        ErrorCode::UnverifiedRecipient,
        "Address encoding is valid; ownership is not verified.",
    ));
    intent.recommended_action = Some(RecommendedAction::new(
        ActionType::Wallet,
        Some(payload.into()),
        true,
    ));
    Ok(intent)
}

fn validate_t_address(address: &str) -> Result<(), ()> {
    if !(address.starts_with("t1") || address.starts_with("t3")) {
        return Err(());
    }
    let decoded = bs58::decode(address).into_vec().map_err(|_| ())?;
    if decoded.len() != 26 {
        return Err(());
    }
    let version = [decoded[0], decoded[1]];
    if version != [0x1C, 0xB8] && version != [0x1C, 0xBD] {
        return Err(());
    }
    let first = Sha256::digest(&decoded[..22]);
    let second = Sha256::digest(first);
    (second[..4] == decoded[22..]).then_some(()).ok_or(())
}

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    use crate::parse_payment_qr;

    /// Builds a real, checksum-correct t1 address from a fixed 20-byte hash160, the same way the
    /// production encoder would, rather than hand-typing a base58 string and hoping it's valid.
    fn build_t1_address(hash160: [u8; 20]) -> String {
        let mut raw = vec![0x1C, 0xB8];
        raw.extend_from_slice(&hash160);
        let first = Sha256::digest(&raw);
        let second = Sha256::digest(first);
        raw.extend_from_slice(&second[..4]);
        bs58::encode(raw).into_string()
    }

    #[test]
    fn recognizes_a_bare_transparent_address() {
        let address = build_t1_address([7u8; 20]);
        let intent = parse_payment_qr(&address);
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "zcash");
        assert_eq!(intent.recipient.unwrap().address.as_deref(), Some(address.as_str()));
    }

    #[test]
    fn rejects_bad_checksum() {
        let mut address = build_t1_address([7u8; 20]);
        let last = address.pop().unwrap();
        address.push(if last == 'A' { 'B' } else { 'A' });
        assert!(!parse_payment_qr(&address).validation.valid);
    }

    #[test]
    fn parses_zcash_uri_with_amount() {
        let address = build_t1_address([9u8; 20]);
        let intent = parse_payment_qr(&format!("zcash:{address}?amount=1.5"));
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.amount.as_deref(), Some("1.5"));
    }
}
