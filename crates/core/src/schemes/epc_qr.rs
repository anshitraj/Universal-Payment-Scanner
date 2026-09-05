use crate::checksums::validate_iban;
use crate::decimal::validate_decimal;
use crate::registry::PaymentScheme;
use crate::{
    ActionType, Asset, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, Recipient,
    RecommendedAction, SchemeMetadata,
};

/// EPC069-12 "Girocode": the European Payments Council's QR code for initiating a SEPA Credit
/// Transfer. Plain UTF-8 text, one field per line (LF or CRLF), not EMVCo TLV. Field order and
/// semantics confirmed against the EPC's own EPC069-12 v2.1 guidelines and cross-checked against
/// two independent open-source implementations (segno, eu-payment-qr).
pub struct EpcQr;

impl PaymentScheme for EpcQr {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "epc_qr".into(),
            display_name: "SEPA / EPC QR (Girocode)".into(),
            countries: vec![],
            category: Category::BankTransfer,
            standard: "EPC069-12 v2.1".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Beta,
            static_supported: true,
            dynamic_supported: false,
            features: vec![
                "iban-checksum".into(),
                "amount".into(),
                "merchant".into(),
                "currency".into(),
                "structured-reference".into(),
            ],
            references: vec!["https://www.europeanpaymentscouncil.eu/document-library/guidance-documents/quick-response-code-guidelines-enable-data-capture-initiation".into()],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        if payload.lines().next() == Some("BCD") {
            90
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        let lines: Vec<&str> = payload.lines().collect();
        if lines.len() < 7 || lines.len() > 12 {
            return Err(ParseError::new(
                ErrorCode::MalformedPayload,
                "EPC QR must have between 7 and 12 lines.",
            ));
        }
        if lines[0] != "BCD" {
            return Err(ParseError::new(
                ErrorCode::MalformedPayload,
                "EPC QR service tag must be BCD.",
            ));
        }
        if lines[1] != "001" && lines[1] != "002" {
            return Err(ParseError::new(
                ErrorCode::MalformedPayload,
                "EPC QR version must be 001 or 002.",
            ));
        }
        if lines[3] != "SCT" {
            return Err(ParseError::new(
                ErrorCode::MalformedPayload,
                "EPC QR identification must be SCT (SEPA Credit Transfer).",
            ));
        }
        let name = lines[5];
        if name.is_empty() || name.chars().count() > 70 {
            return Err(ParseError::new(
                ErrorCode::InvalidRecipient,
                "EPC QR beneficiary name is missing or exceeds 70 characters.",
            ));
        }
        let iban = lines[6];
        validate_iban(iban).map_err(|_| {
            ParseError::new(ErrorCode::InvalidRecipient, "IBAN checksum is invalid.")
        })?;

        let mut intent = PaymentIntent::recognized("epc_qr", Category::BankTransfer);
        intent.standard = Some("EPC069-12 v2.1".into());
        intent.network = Some("SEPA".into());
        intent.country = Some(iban[..2].to_owned());
        intent.currency = Some("EUR".into());
        intent.recipient = Some(Recipient {
            name: Some(name.into()),
            address: Some(iban.into()),
            ..Recipient::default()
        });
        if let Some(amount_field) = lines.get(7).filter(|v| !v.is_empty()) {
            let value = amount_field.strip_prefix("EUR").ok_or_else(|| {
                ParseError::new(
                    ErrorCode::InvalidCurrency,
                    "EPC QR amount must be EUR-denominated (SEPA Credit Transfer is EUR-only).",
                )
            })?;
            intent.amount = Some(validate_decimal(value, 2)?);
        }
        intent.asset = Some(Asset {
            symbol: Some("EUR".into()),
            decimals: Some(2),
            ..Asset::default()
        });
        if let Some(purpose) = lines.get(8).filter(|v| !v.is_empty()) {
            intent.metadata.insert("purpose".into(), (*purpose).into());
        }
        if let Some(structured_reference) = lines.get(9).filter(|v| !v.is_empty()) {
            intent.reference = Some((*structured_reference).into());
        } else if let Some(unstructured) = lines.get(10).filter(|v| !v.is_empty()) {
            intent.reference = Some((*unstructured).into());
        }
        if let Some(info) = lines.get(11).filter(|v| !v.is_empty()) {
            intent.description = Some((*info).into());
        }
        intent.dynamic = Some(false);
        intent.validation.warnings.push(Issue::error(
            ErrorCode::UnverifiedRecipient,
            "IBAN checksum is valid; account ownership is not verified.",
        ));
        intent.recommended_action = Some(RecommendedAction::new(ActionType::Handoff, None, true));
        Ok(intent)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_payment_qr;

    #[test]
    fn parses_a_minimal_girocode() {
        let payload = "BCD\n001\n1\nSCT\nBANKDEFFXXX\nMax Mustermann\nDE89370400440532013000\nEUR12.30\n\n\nInvoice 123";
        let intent = parse_payment_qr(payload);
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "epc_qr");
        assert_eq!(intent.amount.as_deref(), Some("12.30"));
        assert_eq!(intent.currency.as_deref(), Some("EUR"));
    }

    #[test]
    fn open_amount_girocode_is_still_valid() {
        let payload = "BCD\n002\n1\nSCT\n\nMax Mustermann\nDE89370400440532013000\n\n\n\n";
        let intent = parse_payment_qr(payload);
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert!(intent.amount.is_none());
    }
}
