//! Checksum algorithms shared by more than one scheme adapter, kept in one place so a mistake in
//! the math only has to be fixed (and tested) once. `emv::crc16_ccitt_false` stays in `emv.rs`
//! since only EMVCo-family adapters need it.

/// CRC-16/XMODEM: polynomial 0x1021, initial value 0x0000, no input/output reflection. Used by
/// TON's "user-friendly" address checksum and Stellar's strkey checksum - confirmed against each
/// project's own reference implementation, not assumed from the "CRC16-CCITT" name alone (which
/// ambiguously refers to several different-init variants in the wild).
pub fn crc16_xmodem(data: &[u8]) -> u16 {
    let mut crc = 0u16;
    for byte in data {
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

/// ISO 7064 MOD 97-10, the standard IBAN check-digit algorithm: move the first 4 characters to
/// the end, map letters to two-digit numbers (A=10..Z=35), and verify the result is 1 mod 97.
/// Streams the digits rather than building a big number, since a 34-character IBAN can expand to
/// roughly 70 decimal digits. Used by both `epc_qr` and `swiss_qr`.
pub fn validate_iban(iban: &str) -> Result<(), ()> {
    if iban.len() < 15 || iban.len() > 34 || !iban.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err(());
    }
    let (head, tail) = iban.split_at(4);
    let mut remainder = 0u32;
    for byte in tail.bytes().chain(head.bytes()) {
        match byte {
            b'0'..=b'9' => {
                remainder = (remainder * 10 + (byte - b'0') as u32) % 97;
            }
            b'A'..=b'Z' => {
                let value = (byte - b'A') as u32 + 10; // A=10 .. Z=35, always two digits
                remainder = (remainder * 10 + value / 10) % 97;
                remainder = (remainder * 10 + value % 10) % 97;
            }
            _ => return Err(()),
        }
    }
    (remainder == 1).then_some(()).ok_or(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_known_xmodem_test_vector() {
        // "123456789" -> 0x31C3 is the standard CRC-16/XMODEM check value.
        assert_eq!(crc16_xmodem(b"123456789"), 0x31C3);
    }

    #[test]
    fn validates_a_real_published_iban_checksum() {
        // Germany's own EPC069-12 guidance document example IBAN.
        assert!(validate_iban("DE89370400440532013000").is_ok());
    }

    #[test]
    fn rejects_a_tampered_iban_checksum() {
        assert!(validate_iban("DE89370400440532013001").is_err());
    }
}
