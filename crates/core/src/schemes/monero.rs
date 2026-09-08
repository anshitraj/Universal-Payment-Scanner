use sha3::{Digest, Keccak256};
use url::Url;

use crate::decimal::validate_decimal;
use crate::registry::PaymentScheme;
use crate::{
    ActionType, Asset, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, Recipient,
    RecommendedAction, SchemeMetadata,
};

/// Monero addresses use CryptoNote's own Base58 variant - NOT Bitcoin-style Base58Check. Data is
/// encoded in fixed 8-byte blocks (11 base58 chars each), with a shorter, table-driven encoding
/// for the final partial block; the checksum is the first 4 bytes of Keccak-256 (not SHA-256)
/// over the preceding bytes. Both the block table and this construction are confirmed against the
/// Monero reference client (`src/common/base58.cpp`, `src/cryptonote_config.h`).
///
/// Recognizes the three mainnet address kinds - standard ("4..."), integrated ("4...", 8-byte
/// payment ID embedded), and subaddress ("8...") - plus the documented `monero:` payment URI.
pub struct Monero;

const NETWORK_STANDARD: u64 = 18;
const NETWORK_INTEGRATED: u64 = 19;
const NETWORK_SUBADDRESS: u64 = 42;

impl PaymentScheme for Monero {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "monero".into(),
            display_name: "Monero".into(),
            countries: vec![],
            category: Category::Crypto,
            standard: "CryptoNote Base58 address (Monero)".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Beta,
            static_supported: true,
            dynamic_supported: false,
            features: vec![
                "address-checksum".into(),
                "subaddress".into(),
                "integrated-address".into(),
                "amount".into(),
            ],
            references: vec![
                "https://www.getmonero.org/resources/developer-guides/wallet-rpc.html".into(),
                "https://github.com/monero-project/monero/blob/master/src/common/base58.cpp"
                    .into(),
            ],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        if payload
            .get(..7)
            .is_some_and(|p| p.eq_ignore_ascii_case("monero:"))
        {
            100
        } else if (95..=106).contains(&payload.len()) && decode_address(payload).is_some() {
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
            .ok_or_else(|| ParseError::new(ErrorCode::MalformedPayload, "Invalid monero: URI."))?;
        let normalized_uri = if body.starts_with("//") {
            format!("monero:{body}")
        } else {
            format!("monero://{body}")
        };
        let url = Url::parse(&normalized_uri)
            .map_err(|_| ParseError::new(ErrorCode::MalformedPayload, "Invalid monero: URI."))?;
        let address = url
            .host_str()
            .filter(|v| !v.is_empty())
            .or_else(|| {
                let path = url.path().trim_start_matches('/');
                (!path.is_empty()).then_some(path)
            })
            .ok_or_else(|| {
                ParseError::new(ErrorCode::InvalidRecipient, "Monero address is required.")
            })?;
        let subtype = decode_address(address).ok_or_else(|| {
            ParseError::new(
                ErrorCode::InvalidRecipient,
                "Monero address checksum or encoding is invalid.",
            )
        })?;
        let mut amount = None;
        let mut description = None;
        for (key, value) in url.query_pairs() {
            match key.as_ref() {
                "tx_amount" => amount = Some(validate_decimal(&value, 12)?),
                "tx_description" if !value.is_empty() && value.len() <= 256 => {
                    description = Some(value.into_owned())
                }
                _ => {}
            }
        }
        let mut intent = PaymentIntent::recognized("monero", Category::Crypto);
        intent.subtype = Some(subtype.into());
        intent.standard = Some("Monero URI".into());
        intent.network = Some("monero".into());
        intent.recipient = Some(Recipient {
            address: Some(address.into()),
            ..Recipient::default()
        });
        intent.amount = amount;
        intent.asset = Some(Asset {
            symbol: Some("XMR".into()),
            decimals: Some(12),
            ..Asset::default()
        });
        intent.description = description;
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
    let subtype = decode_address(payload).ok_or_else(|| {
        ParseError::new(
            ErrorCode::InvalidRecipient,
            "Monero address checksum or encoding is invalid.",
        )
    })?;
    let mut intent = PaymentIntent::recognized("monero", Category::Crypto);
    intent.subtype = Some(subtype.into());
    intent.standard = Some("CryptoNote Base58 address (Monero)".into());
    intent.network = Some("monero".into());
    intent.recipient = Some(Recipient {
        address: Some(payload.into()),
        ..Recipient::default()
    });
    intent.asset = Some(Asset {
        symbol: Some("XMR".into()),
        decimals: Some(12),
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

/// Decodes and fully validates a Monero address (network byte + checksum). Returns the address
/// subtype on success.
fn decode_address(address: &str) -> Option<&'static str> {
    let raw = monero_base58_decode(address)?;
    let (network, prefix_len) = read_varint(&raw)?;
    let expected_len = match network {
        NETWORK_STANDARD | NETWORK_SUBADDRESS => prefix_len + 32 + 32 + 4,
        NETWORK_INTEGRATED => prefix_len + 32 + 32 + 8 + 4,
        _ => return None,
    };
    if raw.len() != expected_len {
        return None;
    }
    let (body, checksum) = raw.split_at(raw.len() - 4);
    let hash = Keccak256::digest(body);
    if hash[..4] != *checksum {
        return None;
    }
    Some(match network {
        NETWORK_STANDARD => "standard",
        NETWORK_INTEGRATED => "integrated",
        _ => "subaddress",
    })
}

/// Monero network IDs are small enough to always fit the mainnet values checked above in one
/// byte, but the wire format is technically a Boost-style base-128 varint (7 bits per byte, MSB
/// continuation flag) - decoded properly rather than just reading `raw[0]` so a malformed or
/// testnet/stagenet-only prefix is rejected instead of silently misread.
fn read_varint(raw: &[u8]) -> Option<(u64, usize)> {
    let mut value: u64 = 0;
    for (i, byte) in raw.iter().enumerate().take(9) {
        value |= ((byte & 0x7f) as u64) << (7 * i);
        if byte & 0x80 == 0 {
            return Some((value, i + 1));
        }
    }
    None
}

const ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const FULL_ENCODED_BLOCK_SIZE: usize = 11;
// encoded char count for a final partial block of N raw bytes, N = 0..=8 (index = N).
const ENCODED_BLOCK_SIZES: [usize; 9] = [0, 2, 3, 5, 6, 7, 9, 10, 11];

fn monero_base58_decode(input: &str) -> Option<Vec<u8>> {
    if !input.is_ascii() || input.is_empty() {
        return None;
    }
    let bytes = input.as_bytes();
    let full_blocks = bytes.len() / FULL_ENCODED_BLOCK_SIZE;
    let remainder = bytes.len() % FULL_ENCODED_BLOCK_SIZE;
    let last_block_raw = if remainder == 0 {
        0
    } else {
        ENCODED_BLOCK_SIZES
            .iter()
            .position(|&size| size == remainder)?
    };
    let mut out = Vec::with_capacity(full_blocks * 8 + last_block_raw);
    for i in 0..full_blocks {
        let chunk = &bytes[i * FULL_ENCODED_BLOCK_SIZE..(i + 1) * FULL_ENCODED_BLOCK_SIZE];
        out.extend(decode_block(chunk, 8)?);
    }
    if remainder > 0 {
        let chunk = &bytes[full_blocks * FULL_ENCODED_BLOCK_SIZE..];
        out.extend(decode_block(chunk, last_block_raw)?);
    }
    Some(out)
}

/// Big-endian base58 decode of one block into exactly `raw_size` bytes. `digit` overflowing past
/// the fixed-size accumulator (a nonzero carry after the top byte) means the block's numeric
/// value doesn't fit in `raw_size` bytes - an invalid encoding, not a panic.
fn decode_block(chars: &[u8], raw_size: usize) -> Option<Vec<u8>> {
    let mut result = vec![0u8; raw_size];
    for &c in chars {
        let mut digit = ALPHABET.iter().position(|&a| a == c)? as u32;
        for byte in result.iter_mut().rev() {
            let x = (*byte as u32) * 58 + digit;
            *byte = (x & 0xff) as u8;
            digit = x >> 8;
        }
        if digit != 0 {
            return None;
        }
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use sha3::Digest;

    use super::monero_base58_decode;
    use crate::parse_payment_qr;

    /// Builds a real, checksum-correct standard Monero address from fixed 32-byte spend/view
    /// keys, using the same block table and Keccak-256 checksum the parser verifies against -
    /// not a hand-typed string.
    fn build_standard_address(spend: [u8; 32], view: [u8; 32]) -> String {
        let mut raw = vec![18u8]; // NETWORK_STANDARD, fits one varint byte
        raw.extend_from_slice(&spend);
        raw.extend_from_slice(&view);
        let hash = super::Keccak256::digest(&raw);
        raw.extend_from_slice(&hash[..4]);
        monero_base58_encode(&raw)
    }

    fn monero_base58_encode(raw: &[u8]) -> String {
        use super::{ALPHABET, ENCODED_BLOCK_SIZES};
        let mut out = String::new();
        for chunk in raw.chunks(8) {
            let encoded_len = ENCODED_BLOCK_SIZES[chunk.len()];
            let mut digits = vec![0u8; encoded_len];
            let mut num: Vec<u8> = chunk.to_vec();
            for slot in digits.iter_mut().rev() {
                // divide `num` (big-endian) by 58, remainder is this digit
                let mut remainder: u32 = 0;
                for byte in num.iter_mut() {
                    let acc = remainder * 256 + *byte as u32;
                    *byte = (acc / 58) as u8;
                    remainder = acc % 58;
                }
                *slot = ALPHABET[remainder as usize];
            }
            out.push_str(std::str::from_utf8(&digits).unwrap());
        }
        out
    }

    #[test]
    fn base58_round_trips_a_full_block() {
        let raw = [7u8; 8];
        let encoded = monero_base58_encode(&raw);
        assert_eq!(encoded.len(), 11);
        assert_eq!(monero_base58_decode(&encoded).unwrap(), raw);
    }

    #[test]
    fn recognizes_a_standard_address_with_valid_checksum() {
        let address = build_standard_address([1u8; 32], [2u8; 32]);
        assert_eq!(address.len(), 95);
        let intent = parse_payment_qr(&address);
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "monero");
        assert_eq!(intent.subtype.as_deref(), Some("standard"));
    }

    #[test]
    fn rejects_a_tampered_checksum() {
        let mut address = build_standard_address([1u8; 32], [2u8; 32]);
        let last = address.pop().unwrap();
        address.push(if last == 'A' { 'B' } else { 'A' });
        assert!(!parse_payment_qr(&address).validation.valid);
    }

    #[test]
    fn parses_the_real_public_monero_project_donation_address() {
        // getmonero.org's own published donation address - a real, independently-verifiable
        // checksum, not something we generated ourselves like the other tests here.
        let address = "44AFFq5kSiGBoZ4NMDwYtN18obc8AemS33DBLWs3H7otXft3XjrpDtQGv7SqSsaBYBb98uNbr2VBBEt7f2wfn3RVGQBEP3A";
        let intent = parse_payment_qr(address);
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "monero");
        assert_eq!(intent.subtype.as_deref(), Some("standard"));
    }

    #[test]
    fn parses_monero_uri_with_amount() {
        let address = build_standard_address([3u8; 32], [4u8; 32]);
        let intent = parse_payment_qr(&format!("monero:{address}?tx_amount=1.25"));
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.amount.as_deref(), Some("1.25"));
    }
}
