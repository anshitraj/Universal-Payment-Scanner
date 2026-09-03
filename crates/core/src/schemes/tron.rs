use sha2::{Digest, Sha256};

use crate::registry::PaymentScheme;
use crate::{
    ActionType, Asset, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, Recipient,
    RecommendedAction, SchemeMetadata,
};

/// TRON addresses use the same Base58Check construction as Bitcoin (double-SHA256 checksum,
/// same alphabet), just with version byte 0x41 instead of Bitcoin's {0,5,111,196} and no
/// alternate (bech32-style) encoding. Wallets show the bare "T..." address in receive QR codes;
/// there is no widely-adopted `tron:` URI scheme confirmed public, so only the bare form is
/// recognized here.
pub struct Tron;

impl PaymentScheme for Tron {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "tron".into(),
            display_name: "TRON".into(),
            countries: vec![],
            category: Category::Crypto,
            standard: "TRON Base58Check address".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Beta,
            static_supported: true,
            dynamic_supported: false,
            features: vec!["address-checksum".into()],
            references: vec![
                "https://tronprotocol.github.io/documentation-en/mechanism-algorithm/account/"
                    .into(),
            ],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        if payload.starts_with('T')
            && (25..=40).contains(&payload.len())
            && validate_tron_address(payload).is_ok()
        {
            85
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        validate_tron_address(payload).map_err(|_| {
            ParseError::new(
                ErrorCode::InvalidRecipient,
                "TRON address checksum or encoding is invalid.",
            )
        })?;
        let mut intent = PaymentIntent::recognized("tron", Category::Crypto);
        intent.standard = Some("TRON Base58Check address".into());
        intent.network = Some("tron".into());
        intent.recipient = Some(Recipient {
            address: Some(payload.into()),
            ..Recipient::default()
        });
        intent.asset = Some(Asset {
            symbol: Some("TRX".into()),
            decimals: Some(6),
            ..Asset::default()
        });
        intent.dynamic = Some(false);
        intent.validation.warnings.push(Issue::error(
            ErrorCode::UnverifiedRecipient,
            "Address encoding is valid; ownership is not verified. A TRC-20 token (e.g. USDT) may \
             be the intended asset instead of native TRX; the bare address alone doesn't say which.",
        ));
        intent.recommended_action = Some(RecommendedAction {
            kind: ActionType::Wallet,
            uri: Some(payload.into()),
            requires_user_confirmation: true,
        });
        Ok(intent)
    }
}

fn validate_tron_address(address: &str) -> Result<(), ()> {
    let decoded = bs58::decode(address).into_vec().map_err(|_| ())?;
    if decoded.len() != 25 || decoded[0] != 0x41 {
        return Err(());
    }
    let first = Sha256::digest(&decoded[..21]);
    let second = Sha256::digest(first);
    (second[..4] == decoded[21..]).then_some(()).ok_or(())
}

#[cfg(test)]
mod tests {
    use crate::parse_payment_qr;

    #[test]
    fn recognizes_a_bare_tron_address() {
        // Public, widely-documented example address (USDT-TRC20 contract deployer), not a
        // real customer account.
        let intent = parse_payment_qr("TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t");
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "tron");
        assert_eq!(
            intent.recipient.unwrap().address.as_deref(),
            Some("TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t")
        );
    }

    #[test]
    fn rejects_bad_checksum() {
        assert!(
            !parse_payment_qr("TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6X")
                .validation
                .valid
        );
    }
}
