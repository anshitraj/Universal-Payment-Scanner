use crate::checksums::crc16_xmodem;
use crate::registry::PaymentScheme;
use crate::{
    ActionType, Asset, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, Recipient,
    RecommendedAction, SchemeMetadata,
};

/// TON's "user-friendly" address: a 36-byte structure (1 flags + 1 workchain + 32 account id + 2
/// CRC16/XMODEM checksum) encoded as 48 characters of unpadded base64 or base64url. Confirmed
/// against `@ton/core`'s own address and crc16 source, not assumed from the "CRC16-CCITT" name
/// (see `checksums::crc16_xmodem`'s doc comment for why that distinction mattered here).
pub struct Ton;

const VALID_FLAGS: [u8; 4] = [0x11, 0x51, 0x91, 0xD1];

impl PaymentScheme for Ton {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "ton".into(),
            display_name: "TON".into(),
            countries: vec![],
            category: Category::Crypto,
            standard: "TON user-friendly address".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Beta,
            static_supported: true,
            dynamic_supported: false,
            features: vec!["address-checksum".into()],
            references: vec![
                "https://docs.ton.org/v3/documentation/smart-contracts/addresses/address-formats"
                    .into(),
            ],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        if payload.len() == 48 && decode_ton_address(payload).is_some() {
            80
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        let bytes = decode_ton_address(payload).ok_or_else(|| {
            ParseError::new(
                ErrorCode::InvalidRecipient,
                "TON address checksum or encoding is invalid.",
            )
        })?;
        let bounceable = bytes[0] & 0x40 == 0;
        let mut intent = PaymentIntent::recognized("ton", Category::Crypto);
        intent.standard = Some("TON user-friendly address".into());
        intent.network = Some("ton".into());
        intent.recipient = Some(Recipient {
            address: Some(payload.into()),
            ..Recipient::default()
        });
        intent.asset = Some(Asset {
            symbol: Some("TON".into()),
            decimals: Some(9),
            ..Asset::default()
        });
        intent.dynamic = Some(false);
        intent
            .metadata
            .insert("bounceable".into(), bounceable.into());
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

fn decode_ton_address(payload: &str) -> Option<[u8; 36]> {
    let raw = base64_decode(payload)?;
    if raw.len() != 36 || !VALID_FLAGS.contains(&raw[0]) {
        return None;
    }
    let checksum = crc16_xmodem(&raw[..34]);
    if raw[34] != (checksum >> 8) as u8 || raw[35] != (checksum & 0xff) as u8 {
        return None;
    }
    let mut out = [0u8; 36];
    out.copy_from_slice(&raw);
    Some(out)
}

/// Unpadded standard or URL-safe base64, whichever the input uses. No external dependency: the
/// project already hand-rolls small encoding algorithms (see bech32 in `schemes::bitcoin`) rather
/// than pull in a crate for something this size.
fn base64_decode(input: &str) -> Option<Vec<u8>> {
    if input.len() != 48 {
        return None;
    }
    let value_of = |byte: u8| -> Option<u8> {
        match byte {
            b'A'..=b'Z' => Some(byte - b'A'),
            b'a'..=b'z' => Some(byte - b'a' + 26),
            b'0'..=b'9' => Some(byte - b'0' + 52),
            b'+' | b'-' => Some(62),
            b'/' | b'_' => Some(63),
            _ => None,
        }
    };
    let mut accumulator = 0u32;
    let mut bits = 0u32;
    let mut out = Vec::with_capacity(36);
    for byte in input.bytes() {
        let value = value_of(byte)?;
        accumulator = (accumulator << 6) | value as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((accumulator >> bits) & 0xff) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::base64_decode;
    use crate::checksums::crc16_xmodem;
    use crate::parse_payment_qr;

    fn build_address(flags: u8, workchain: u8, account_id: [u8; 32]) -> String {
        let mut raw = Vec::with_capacity(36);
        raw.push(flags);
        raw.push(workchain);
        raw.extend_from_slice(&account_id);
        let crc = crc16_xmodem(&raw);
        raw.push((crc >> 8) as u8);
        raw.push((crc & 0xff) as u8);
        let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut out = String::with_capacity(48);
        let mut accumulator = 0u32;
        let mut bits = 0u32;
        for byte in raw {
            accumulator = (accumulator << 8) | byte as u32;
            bits += 8;
            while bits >= 6 {
                bits -= 6;
                out.push(alphabet[((accumulator >> bits) & 0x3f) as usize] as char);
            }
        }
        out
    }

    #[test]
    fn base64_decode_round_trips_with_the_test_builder() {
        let address = build_address(0x11, 0, [7u8; 32]);
        let decoded = base64_decode(&address).unwrap();
        assert_eq!(decoded.len(), 36);
        assert_eq!(decoded[0], 0x11);
    }

    #[test]
    fn recognizes_a_bare_ton_address_with_valid_checksum() {
        let address = build_address(0x11, 0, [1u8; 32]);
        let intent = parse_payment_qr(&address);
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "ton");
    }

    #[test]
    fn rejects_a_tampered_checksum() {
        let mut address = build_address(0x11, 0, [1u8; 32]);
        address.replace_range(47..48, if address.ends_with('A') { "B" } else { "A" });
        assert!(!parse_payment_qr(&address).validation.valid);
    }
}
