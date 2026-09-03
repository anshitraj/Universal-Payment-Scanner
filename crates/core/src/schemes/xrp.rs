use sha2::{Digest, Sha256};

use crate::registry::PaymentScheme;
use crate::{
    ActionType, Asset, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, Recipient,
    RecommendedAction, SchemeMetadata,
};

/// XRP Ledger classic addresses: version byte 0x00 + 20-byte account id + 4-byte double-SHA256
/// checksum, Base58Check - structurally identical to Bitcoin's scheme, but XRPL uses its own
/// base58 alphabet (`bs58::Alphabet::RIPPLE`), confirmed against `ripple-address-codec`. Only the
/// bare classic ("r...") address is recognized; the newer X-address format (which can embed a
/// destination tag) is a documented future addition once its exact byte layout is verified here.
pub struct Xrp;

impl PaymentScheme for Xrp {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "xrp".into(),
            display_name: "XRP Ledger".into(),
            countries: vec![],
            category: Category::Crypto,
            standard: "XRPL classic address".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Beta,
            static_supported: true,
            dynamic_supported: false,
            features: vec!["address-checksum".into()],
            references: vec!["https://xrpl.org/docs/concepts/accounts/addresses".into()],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        if payload.starts_with('r')
            && (25..=35).contains(&payload.len())
            && validate_xrp_address(payload).is_ok()
        {
            85
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        validate_xrp_address(payload).map_err(|_| {
            ParseError::new(
                ErrorCode::InvalidRecipient,
                "XRP Ledger address checksum or encoding is invalid.",
            )
        })?;
        let mut intent = PaymentIntent::recognized("xrp", Category::Crypto);
        intent.standard = Some("XRPL classic address".into());
        intent.network = Some("xrpl".into());
        intent.recipient = Some(Recipient {
            address: Some(payload.into()),
            ..Recipient::default()
        });
        intent.asset = Some(Asset {
            symbol: Some("XRP".into()),
            decimals: Some(6),
            ..Asset::default()
        });
        intent.dynamic = Some(false);
        intent.validation.warnings.push(Issue::error(
            ErrorCode::UnverifiedRecipient,
            "Address encoding is valid; ownership is not verified. Exchanges often require a \
             destination tag alongside this address; a bare classic address does not encode one.",
        ));
        intent.recommended_action = Some(RecommendedAction {
            kind: ActionType::Wallet,
            uri: Some(payload.into()),
            requires_user_confirmation: true,
        });
        Ok(intent)
    }
}

fn validate_xrp_address(address: &str) -> Result<(), ()> {
    let decoded = bs58::decode(address)
        .with_alphabet(bs58::Alphabet::RIPPLE)
        .into_vec()
        .map_err(|_| ())?;
    if decoded.len() != 25 || decoded[0] != 0x00 {
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
    fn recognizes_a_bare_xrp_address() {
        // The well-known XRPL genesis account address, published throughout XRPL documentation.
        let intent = parse_payment_qr("rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh");
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "xrp");
    }

    #[test]
    fn rejects_bad_checksum() {
        assert!(
            !parse_payment_qr("rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTX")
                .validation
                .valid
        );
    }
}
