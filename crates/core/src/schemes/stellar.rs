use crate::checksums::crc16_xmodem;
use crate::registry::PaymentScheme;
use crate::{
    ActionType, Asset, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, Recipient,
    RecommendedAction, SchemeMetadata,
};

/// Stellar "strkey" public keys: version byte 0x30 (ed25519 public key, the "G..." prefix) + 32
/// byte key + 2-byte CRC16/XMODEM checksum over the preceding 33 bytes, unpadded RFC4648 base32.
/// Confirmed against Stellar's own `strkey` reference implementations. Only the bare address is
/// recognized; the SEP-0007 `web+stellar:pay?...` payment-request URI is a documented future
/// addition once its parameter set is verified here.
pub struct Stellar;

const PUBLIC_KEY_VERSION: u8 = 6 << 3; // 0x30

impl PaymentScheme for Stellar {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "stellar".into(),
            display_name: "Stellar".into(),
            countries: vec![],
            category: Category::Crypto,
            standard: "Stellar strkey (ed25519 public key)".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Beta,
            static_supported: true,
            dynamic_supported: false,
            features: vec!["address-checksum".into()],
            references: vec!["https://developers.stellar.org/docs/learn/encyclopedia/security/signatures-multisig".into()],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        if payload.starts_with('G') && payload.len() == 56 && decode_stellar_key(payload).is_some()
        {
            85
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        decode_stellar_key(payload).ok_or_else(|| {
            ParseError::new(
                ErrorCode::InvalidRecipient,
                "Stellar address checksum or encoding is invalid.",
            )
        })?;
        let mut intent = PaymentIntent::recognized("stellar", Category::Crypto);
        intent.standard = Some("Stellar strkey (ed25519 public key)".into());
        intent.network = Some("stellar".into());
        intent.recipient = Some(Recipient {
            address: Some(payload.into()),
            ..Recipient::default()
        });
        intent.asset = Some(Asset {
            symbol: Some("XLM".into()),
            decimals: Some(7),
            ..Asset::default()
        });
        intent.dynamic = Some(false);
        intent.validation.warnings.push(Issue::error(
            ErrorCode::UnverifiedRecipient,
            "Address encoding is valid; ownership is not verified. Exchanges often require a \
             memo alongside this address; a bare public key does not encode one.",
        ));
        intent.recommended_action = Some(RecommendedAction::new(
            ActionType::Wallet,
            Some(payload.into()),
            true,
        ));
        Ok(intent)
    }
}

fn decode_stellar_key(payload: &str) -> Option<[u8; 32]> {
    let raw = base32_decode(payload)?;
    if raw.len() != 35 || raw[0] != PUBLIC_KEY_VERSION {
        return None;
    }
    let checksum = crc16_xmodem(&raw[..33]);
    if raw[33] != (checksum & 0xff) as u8 || raw[34] != (checksum >> 8) as u8 {
        // Stellar strkey stores the checksum little-endian, unlike TON's big-endian trailer.
        return None;
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&raw[1..33]);
    Some(key)
}

/// Unpadded RFC 4648 base32. Hand-rolled for the same reason `schemes::ton`'s base64 decoder is:
/// this is small enough that a dependency isn't worth it, and the project already hand-rolls
/// bech32 (see `schemes::bitcoin`) for the same class of algorithm.
fn base32_decode(input: &str) -> Option<Vec<u8>> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut accumulator = 0u32;
    let mut bits = 0u32;
    let mut out = Vec::with_capacity(input.len() * 5 / 8);
    for byte in input.bytes() {
        let value = ALPHABET
            .iter()
            .position(|c| *c == byte.to_ascii_uppercase())? as u32;
        accumulator = (accumulator << 5) | value;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(((accumulator >> bits) & 0xff) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::base32_decode;
    use crate::checksums::crc16_xmodem;
    use crate::parse_payment_qr;

    fn build_address(key: [u8; 32]) -> String {
        let mut raw = Vec::with_capacity(35);
        raw.push(super::PUBLIC_KEY_VERSION);
        raw.extend_from_slice(&key);
        let crc = crc16_xmodem(&raw);
        raw.push((crc & 0xff) as u8);
        raw.push((crc >> 8) as u8);
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
        let mut out = String::with_capacity(56);
        let mut accumulator = 0u32;
        let mut bits = 0u32;
        for byte in raw {
            accumulator = (accumulator << 8) | byte as u32;
            bits += 8;
            while bits >= 5 {
                bits -= 5;
                out.push(ALPHABET[((accumulator >> bits) & 0x1f) as usize] as char);
            }
        }
        out
    }

    #[test]
    fn base32_decode_round_trips_with_the_test_builder() {
        let address = build_address([9u8; 32]);
        assert_eq!(address.len(), 56);
        assert!(address.starts_with('G'));
        assert_eq!(base32_decode(&address).unwrap().len(), 35);
    }

    #[test]
    fn recognizes_a_bare_stellar_address_with_valid_checksum() {
        let address = build_address([3u8; 32]);
        let intent = parse_payment_qr(&address);
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "stellar");
    }

    #[test]
    fn rejects_a_tampered_checksum() {
        let mut address = build_address([3u8; 32]);
        address.replace_range(55..56, if address.ends_with('A') { "B" } else { "A" });
        assert!(!parse_payment_qr(&address).validation.valid);
    }
}
