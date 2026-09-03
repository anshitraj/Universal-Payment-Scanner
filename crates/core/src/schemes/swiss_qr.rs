use crate::checksums::validate_iban;
use crate::decimal::validate_decimal;
use crate::registry::PaymentScheme;
use crate::{
    ActionType, Asset, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, Recipient,
    RecommendedAction, SchemeMetadata,
};

/// Swiss QR-bill (SIX Group's "Swiss Payments Code", SPC): plain UTF-8 text, one field per line,
/// current version (v2.x - the only version in real-world circulation; v1.0 with its extra "due
/// date" line was retired years ago).
///
/// Header, creditor block, and amount/currency (lines 0-17) are validated by fixed position -
/// well corroborated across sources. The debtor address block that follows has a line count this
/// crate could not fully confirm against a primary source, so reference type/reference/message
/// are instead anchored from the *back*, off the position of the literal "EPD" trailer, which is
/// robust regardless of exactly how many debtor-address lines came before it. Given that residual
/// uncertainty, this stays `Experimental` rather than `Beta`: detection and the creditor/amount
/// fields are solid, but full field-by-field normalization hasn't been checked against a real
/// bill. The QRR reference's own check-digit algorithm (distinct from IBAN's) is not verified
/// here - only its digit shape is.
pub struct SwissQr;

impl PaymentScheme for SwissQr {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "swiss_qr_bill".into(),
            display_name: "Swiss QR-bill".into(),
            countries: vec!["CH".into(), "LI".into()],
            category: Category::BankTransfer,
            standard: "SIX Swiss Implementation Guidelines QR-bill v2.x".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Experimental,
            static_supported: true,
            dynamic_supported: false,
            features: vec!["iban-checksum".into(), "amount".into()],
            references: vec!["https://www.six-group.com/en/products-services/banking-services/payment-standardization/standards/qr-bill.html".into()],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        if payload.lines().next() == Some("SPC") {
            80
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        let lines: Vec<&str> = payload.lines().collect();
        if lines.len() < 22 {
            return Err(ParseError::new(
                ErrorCode::MalformedPayload,
                "Swiss QR-bill payload is too short for the v2.x field set.",
            ));
        }
        if lines[0] != "SPC" {
            return Err(ParseError::new(
                ErrorCode::MalformedPayload,
                "Swiss QR-bill header must be SPC.",
            ));
        }
        if !lines[1].starts_with("02") {
            return Err(ParseError::new(
                ErrorCode::SchemeUnsupported,
                "Only Swiss QR-bill v2.x is supported (v1.0 is retired).",
            ));
        }
        if lines[2] != "1" {
            return Err(ParseError::new(
                ErrorCode::MalformedPayload,
                "Swiss QR-bill coding type must be 1 (UTF-8).",
            ));
        }
        let iban = lines[3];
        validate_iban(iban).map_err(|_| {
            ParseError::new(ErrorCode::InvalidRecipient, "IBAN checksum is invalid.")
        })?;
        let creditor_name = lines[4];
        if creditor_name.is_empty() {
            return Err(ParseError::new(
                ErrorCode::InvalidRecipient,
                "Swiss QR-bill creditor name is missing.",
            ));
        }
        let currency = lines[17];
        if currency != "CHF" && currency != "EUR" {
            return Err(ParseError::new(
                ErrorCode::InvalidCurrency,
                "Swiss QR-bill currency must be CHF or EUR.",
            ));
        }
        let epd_index = lines
            .iter()
            .rposition(|line| *line == "EPD")
            .ok_or_else(|| {
                ParseError::new(
                    ErrorCode::MalformedPayload,
                    "Swiss QR-bill trailer (EPD) was not found.",
                )
            })?;
        if epd_index < 21 {
            return Err(ParseError::new(
                ErrorCode::MalformedPayload,
                "Swiss QR-bill trailer appears before the required fields.",
            ));
        }
        let reference_type = lines[epd_index - 3];
        let reference = lines[epd_index - 2];
        if !matches!(reference_type, "QRR" | "SCOR" | "NON") {
            return Err(ParseError::new(
                ErrorCode::MalformedPayload,
                "Swiss QR-bill reference type must be QRR, SCOR, or NON.",
            ));
        }
        if reference_type == "QRR" && !reference.bytes().all(|b| b.is_ascii_digit()) {
            return Err(ParseError::new(
                ErrorCode::MalformedPayload,
                "Swiss QR-bill QRR reference must be numeric.",
            ));
        }

        let mut intent = PaymentIntent::recognized("swiss_qr_bill", Category::BankTransfer);
        intent.standard = Some("SIX Swiss Implementation Guidelines QR-bill v2.x".into());
        intent.network = Some("Swiss QR-bill".into());
        intent.country = Some(iban[..2].to_owned());
        intent.currency = Some(currency.into());
        intent.recipient = Some(Recipient {
            name: Some(creditor_name.into()),
            address: Some(iban.into()),
            ..Recipient::default()
        });
        if let Some(amount) = lines.get(16).filter(|v| !v.is_empty()) {
            intent.amount = Some(validate_decimal(amount, 2)?);
        }
        intent.asset = Some(Asset {
            symbol: Some(currency.into()),
            decimals: Some(2),
            ..Asset::default()
        });
        if !reference.is_empty() {
            intent.reference = Some(reference.into());
        }
        intent
            .metadata
            .insert("referenceType".into(), reference_type.into());
        if let Some(message) = lines
            .get(epd_index.saturating_sub(1))
            .filter(|v| !v.is_empty())
        {
            intent.description = Some((*message).into());
        }
        intent.dynamic = Some(false);
        intent.validation.warnings.push(Issue::error(
            ErrorCode::UnverifiedRecipient,
            "IBAN checksum is valid; account ownership is not verified. Debtor address fields and \
             the QRR reference's own check digit are not independently verified in this release.",
        ));
        intent.recommended_action = Some(RecommendedAction {
            kind: ActionType::Handoff,
            uri: None,
            requires_user_confirmation: true,
        });
        Ok(intent)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_payment_qr;

    fn sample(reference_type: &str, reference: &str) -> String {
        [
            "SPC",
            "0200",
            "1",
            "DE89370400440532013000",
            "Musterfrau AG",
            "Bahnhofstrasse",
            "1",
            "8001",
            "Zurich",
            "CH",
            "",
            "",
            "",
            "",
            "",
            "",
            "100.00",
            "CHF",
            "",
            "",
            "",
            "",
            "",
            "",
            reference_type,
            reference,
            "Order 42",
            "EPD",
        ]
        .join("\n")
    }

    #[test]
    fn parses_a_v2_bill_with_a_scor_reference() {
        let intent = parse_payment_qr(&sample("SCOR", "RF18539007547034"));
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "swiss_qr_bill");
        assert_eq!(intent.amount.as_deref(), Some("100.00"));
        assert_eq!(intent.currency.as_deref(), Some("CHF"));
        assert_eq!(intent.reference.as_deref(), Some("RF18539007547034"));
    }

    #[test]
    fn rejects_a_non_numeric_qrr_reference() {
        assert!(
            !parse_payment_qr(&sample("QRR", "not-numeric"))
                .validation
                .valid
        );
    }

    #[test]
    fn rejects_v1_payloads() {
        let v1 = sample("NON", "").replacen("0200", "0100", 1);
        assert!(!parse_payment_qr(&v1).validation.valid);
    }
}
