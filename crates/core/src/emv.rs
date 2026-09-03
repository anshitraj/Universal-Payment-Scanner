use std::collections::BTreeMap;

use crate::{ErrorCode, ParseError};

pub type TlvMap = BTreeMap<String, String>;

pub fn parse_tlv(payload: &str, max_fields: usize) -> Result<TlvMap, ParseError> {
    if !payload.is_ascii() {
        return Err(ParseError::new(
            ErrorCode::MalformedPayload,
            "EMV payload must be ASCII.",
        ));
    }
    let mut fields = TlvMap::new();
    let bytes = payload.as_bytes();
    let mut cursor = 0usize;
    let mut count = 0usize;
    while cursor < bytes.len() {
        if count >= max_fields || bytes.len() - cursor < 4 {
            return Err(ParseError::new(
                ErrorCode::MalformedPayload,
                "Invalid or excessive EMV TLV fields.",
            ));
        }
        let tag = std::str::from_utf8(&bytes[cursor..cursor + 2])
            .map_err(|_| ParseError::new(ErrorCode::MalformedPayload, "Invalid EMV tag."))?;
        let length_text = std::str::from_utf8(&bytes[cursor + 2..cursor + 4])
            .map_err(|_| ParseError::new(ErrorCode::MalformedPayload, "Invalid EMV length."))?;
        if !tag.bytes().all(|b| b.is_ascii_digit())
            || !length_text.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(ParseError::new(
                ErrorCode::MalformedPayload,
                "EMV tag and length must be decimal digits.",
            ));
        }
        let length: usize = length_text
            .parse()
            .map_err(|_| ParseError::new(ErrorCode::MalformedPayload, "Invalid EMV length."))?;
        cursor += 4;
        let end = cursor
            .checked_add(length)
            .ok_or_else(|| ParseError::new(ErrorCode::MalformedPayload, "EMV length overflow."))?;
        if end > bytes.len() {
            return Err(ParseError::new(
                ErrorCode::MalformedPayload,
                "EMV field exceeds payload boundary.",
            ));
        }
        let value = std::str::from_utf8(&bytes[cursor..end])
            .map_err(|_| ParseError::new(ErrorCode::MalformedPayload, "Invalid EMV value."))?;
        if fields.insert(tag.to_owned(), value.to_owned()).is_some() {
            return Err(ParseError::new(
                ErrorCode::MalformedPayload,
                format!("Duplicate EMV tag {tag}."),
            ));
        }
        cursor = end;
        count += 1;
    }
    Ok(fields)
}

/// Cheap, panic-free check that a payload has the EMVCo MPM envelope shape: starts with the
/// payload-format-indicator tag and ends with a `6304` CRC marker. Slices `.as_bytes()`, never
/// `payload` itself, so a multi-byte UTF-8 character near the tail cannot land on a non-boundary
/// offset (see `schemes::mod::EmvCo::detect`'s history for why that distinction matters).
pub fn has_mpm_envelope(payload: &str) -> bool {
    let bytes = payload.as_bytes();
    payload.starts_with("000201")
        && bytes.len() >= 12
        && bytes[bytes.len() - 8..].starts_with(b"6304")
}

/// Checks the required-field shape every EMVCo Merchant-Presented Mode adapter needs: the six
/// mandatory tags are present, some merchant account information exists in 02-51, and the
/// merchant category code / currency / country fields have the lengths and character classes the
/// standard specifies. Shared by the generic `EmvCo`/`Pix` adapters and every national overlay in
/// `national_emv.rs` so the shape check has exactly one implementation.
pub fn validate_required_fields(fields: &TlvMap) -> Result<(), ParseError> {
    for required in ["52", "53", "58", "59", "60", "63"] {
        if !fields.contains_key(required) {
            return Err(ParseError::new(
                ErrorCode::MalformedPayload,
                format!("Required EMV field {required} is missing."),
            ));
        }
    }
    if !(2..=51).any(|tag| fields.contains_key(&format!("{tag:02}"))) {
        return Err(ParseError::new(
            ErrorCode::InvalidRecipient,
            "EMV merchant account information is missing.",
        ));
    }
    if !fields
        .get("52")
        .is_some_and(|value| value.len() == 4 && value.bytes().all(|byte| byte.is_ascii_digit()))
        || !fields.get("53").is_some_and(|value| {
            value.len() == 3 && value.bytes().all(|byte| byte.is_ascii_digit())
        })
        || !fields.get("58").is_some_and(|value| {
            value.len() == 2 && value.bytes().all(|byte| byte.is_ascii_uppercase())
        })
    {
        return Err(ParseError::new(
            ErrorCode::MalformedPayload,
            "EMV merchant category, currency, or country field is malformed.",
        ));
    }
    Ok(())
}

pub fn validate_crc(payload: &str) -> Result<(), ParseError> {
    // Slice `.as_bytes()` rather than `payload` itself: byte-length arithmetic on a `&str`
    // can land on a non-UTF-8-boundary offset (e.g. a multi-byte char near the tail) and
    // panic. Byte slices have no such boundary constraint.
    let bytes = payload.as_bytes();
    if bytes.len() < 8 {
        return Err(ParseError::new(
            ErrorCode::MalformedPayload,
            "EMV payload is too short.",
        ));
    }
    let crc_marker = &bytes[bytes.len() - 8..bytes.len() - 4];
    if crc_marker != b"6304" {
        return Err(ParseError::new(
            ErrorCode::MalformedPayload,
            "EMV CRC field must be final.",
        ));
    }
    let expected = &bytes[bytes.len() - 4..];
    if !expected.iter().all(|b| b.is_ascii_hexdigit()) {
        return Err(ParseError::new(
            ErrorCode::InvalidChecksum,
            "EMV CRC is not hexadecimal.",
        ));
    }
    let actual = crc16_ccitt_false(&bytes[..bytes.len() - 4]);
    let actual_hex = format!("{actual:04X}");
    let matches = actual_hex
        .as_bytes()
        .iter()
        .zip(expected)
        .all(|(a, b)| *a == b.to_ascii_uppercase());
    if !matches {
        return Err(ParseError::new(
            ErrorCode::InvalidChecksum,
            "EMV CRC-16/CCITT-FALSE checksum mismatch.",
        ));
    }
    Ok(())
}

pub fn crc16_ccitt_false(input: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;
    for byte in input {
        crc ^= (*byte as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

pub fn currency_from_numeric(code: &str) -> Option<&'static str> {
    match code {
        "356" => Some("INR"),
        "702" => Some("SGD"),
        "764" => Some("THB"),
        "840" => Some("USD"),
        "978" => Some("EUR"),
        "986" => Some("BRL"),
        "360" => Some("IDR"),
        "458" => Some("MYR"),
        "704" => Some("VND"),
        "608" => Some("PHP"),
        "116" => Some("KHR"),
        "144" => Some("LKR"),
        "050" => Some("BDT"),
        "586" => Some("PKR"),
        "524" => Some("NPR"),
        "104" => Some("MMK"),
        "418" => Some("LAK"),
        "392" => Some("JPY"),
        "901" => Some("TWD"),
        "410" => Some("KRW"),
        "344" => Some("HKD"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_matches_pix_reference_payload() {
        let payload = "00020126360014BR.GOV.BCB.PIX0114+55119999999995204000053039865802BR5913FULANO DE TAL6008BRASILIA62070503***6304";
        let crc = crc16_ccitt_false(payload.as_bytes());
        assert_eq!(format!("{crc:04X}"), "C23A");
    }

    #[test]
    fn tlv_parser_is_bounded() {
        assert!(parse_tlv("000201", 1).is_ok());
        assert!(parse_tlv("0002010101A", 1).is_err());
    }
}
