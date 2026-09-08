mod alipay;
mod bitcoin;
mod cash_app;
mod epc_qr;
mod ethereum;
mod interac;
mod lightning;
mod monero;
mod monzo;
pub mod national_emv;
mod nonpayment;
mod paypal;
mod revolut;
mod solana;
mod stellar;
mod swiss_qr;
mod swish;
mod ton;
mod tron;
mod unionpay;
mod upi;
mod venmo;
mod vipps;
mod wechat_pay;
mod wise;
mod xrp;
mod zcash;
mod zelle;

pub use alipay::Alipay;
pub use bitcoin::Bitcoin;
pub use cash_app::CashApp;
pub use epc_qr::EpcQr;
pub use ethereum::Ethereum;
pub use interac::Interac;
pub use lightning::Lightning;
pub use monero::Monero;
pub use monzo::Monzo;
pub use national_emv::{NationalOverlay, OVERLAYS};
pub use nonpayment::{Authenticator, GenericUrl, WalletConnect};
pub use paypal::PayPal;
pub use revolut::Revolut;
pub use solana::SolanaPay;
pub use stellar::Stellar;
pub use swiss_qr::SwissQr;
pub use swish::Swish;
pub use ton::Ton;
pub use tron::Tron;
pub use unionpay::UnionPay;
pub use upi::Upi;
pub use venmo::Venmo;
pub use vipps::Vipps;
pub use wechat_pay::WeChatPay;
pub use wise::Wise;
pub use xrp::Xrp;
pub use zcash::Zcash;
pub use zelle::Zelle;

pub struct EmvCo;
pub struct Pix;

impl crate::registry::PaymentScheme for EmvCo {
    fn metadata(&self) -> crate::SchemeMetadata {
        crate::SchemeMetadata {
            id: "emvco_mpm".into(),
            display_name: "EMVCo Merchant-Presented QR".into(),
            countries: vec![],
            category: crate::Category::Card,
            standard: "EMV QRCPS MPM v1.1".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: crate::Maturity::Stable,
            static_supported: true,
            dynamic_supported: true,
            features: vec![
                "crc16".into(),
                "merchant".into(),
                "amount".into(),
                "currency".into(),
                "merchant-accounts".into(),
            ],
            references: vec!["https://www.emvco.com/emv-technologies/qr-codes/".into()],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        if crate::emv::has_mpm_envelope(payload) {
            80
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<crate::PaymentIntent, crate::ParseError> {
        parse_emv_intent(payload, false)
    }
}

impl crate::registry::PaymentScheme for Pix {
    fn metadata(&self) -> crate::SchemeMetadata {
        crate::SchemeMetadata {
            id: "pix".into(), display_name: "Pix / BR Code".into(), countries: vec!["BR".into()], category: crate::Category::BankTransfer,
            standard: "Pix initiation manual 2.9.0 / BR Code".into(), parser_version: env!("CARGO_PKG_VERSION").into(), maturity: crate::Maturity::Stable,
            static_supported: true, dynamic_supported: true,
            features: vec!["crc16".into(), "pix-key".into(), "dynamic-url".into(), "txid".into(), "amount".into(), "merchant".into(), "currency".into()],
            references: vec!["https://www.bcb.gov.br/content/estabilidadefinanceira/pix/Regulamento_Pix/II_ManualdePadroesparaIniciacaodoPix.pdf".into()],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        if payload.starts_with("000201") && payload.to_ascii_lowercase().contains("br.gov.bcb.pix")
        {
            100
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<crate::PaymentIntent, crate::ParseError> {
        parse_emv_intent(payload, true)
    }
}

fn parse_emv_intent(payload: &str, pix: bool) -> Result<crate::PaymentIntent, crate::ParseError> {
    use crate::{
        ActionType, Asset, Category, ErrorCode, Issue, PaymentIntent, Recipient, RecommendedAction,
    };
    crate::emv::validate_crc(payload)?;
    let fields = crate::emv::parse_tlv(payload, 100)?;
    if fields.get("00").map(String::as_str) != Some("01") {
        return Err(crate::ParseError::new(
            ErrorCode::MalformedPayload,
            "EMV payload format indicator must be 01.",
        ));
    }
    crate::emv::validate_required_fields(&fields)?;
    let scheme = if pix { "pix" } else { "emvco_mpm" };
    let category = if pix {
        Category::BankTransfer
    } else {
        Category::Card
    };
    let mut intent = PaymentIntent::recognized(scheme, category);
    intent.standard = Some(
        if pix {
            "Pix initiation manual 2.9.0 / BR Code"
        } else {
            "EMV QRCPS MPM v1.1"
        }
        .into(),
    );
    intent.country = fields.get("58").cloned();
    intent.network = Some(if pix { "Pix" } else { "EMVCo" }.into());
    intent.currency = fields
        .get("53")
        .and_then(|c| crate::emv::currency_from_numeric(c))
        .map(str::to_owned);
    if let Some(value) = fields.get("54") {
        intent.amount = Some(crate::decimal::validate_decimal(value, 12)?);
    }
    intent.dynamic = Some(fields.get("01").map(String::as_str) == Some("12"));
    intent.recipient = Some(Recipient {
        name: fields.get("59").cloned(),
        merchant_id: None,
        id: None,
        address: None,
    });
    intent.asset = intent.currency.clone().map(|symbol| Asset {
        symbol: Some(symbol),
        ..Asset::default()
    });
    if let Some(mcc) = fields.get("52") {
        intent
            .metadata
            .insert("merchantCategoryCode".into(), mcc.clone().into());
    }
    if let Some(city) = fields.get("60") {
        intent
            .metadata
            .insert("merchantCity".into(), city.clone().into());
    }
    if pix {
        let account = (26..=51)
            .find_map(|tag| fields.get(&format!("{tag:02}")))
            .ok_or_else(|| {
                crate::ParseError::new(
                    ErrorCode::InvalidRecipient,
                    "Pix merchant account information is missing.",
                )
            })?;
        let pix_fields = crate::emv::parse_tlv(account, 32)?;
        if !pix_fields
            .get("00")
            .is_some_and(|v| v.eq_ignore_ascii_case("br.gov.bcb.pix"))
        {
            return Err(crate::ParseError::new(
                ErrorCode::MalformedPayload,
                "Pix GUI is missing.",
            ));
        }
        if !pix_fields.contains_key("01") && !pix_fields.contains_key("25") {
            return Err(crate::ParseError::new(
                ErrorCode::InvalidRecipient,
                "Pix merchant information requires a key or dynamic URL.",
            ));
        }
        if fields.get("53").map(String::as_str) != Some("986")
            || fields.get("58").map(String::as_str) != Some("BR")
        {
            return Err(crate::ParseError::new(
                ErrorCode::InvalidCurrency,
                "Pix BR Code must use BRL (986) and country BR.",
            ));
        }
        if let Some(recipient) = intent.recipient.as_mut() {
            recipient.id = pix_fields.get("01").cloned();
        }
        if let Some(url) = pix_fields.get("25") {
            intent
                .metadata
                .insert("dynamicUrl".into(), url.clone().into());
            intent.dynamic = Some(true);
        }
        if let Some(extra) = fields.get("62") {
            let extra_fields = crate::emv::parse_tlv(extra, 32)?;
            intent.reference = extra_fields
                .get("05")
                .filter(|v| v.as_str() != "***")
                .cloned();
        }
    }
    intent.validation.warnings.push(Issue::error(
        ErrorCode::UnverifiedRecipient,
        "Checksum and structure are valid; merchant identity is not verified.",
    ));
    intent.recommended_action = Some(RecommendedAction::new(ActionType::Handoff, None, true));
    Ok(intent)
}

#[cfg(test)]
mod emv_tests {
    use crate::parse_payment_qr;

    #[test]
    fn parses_pix_reference_vector_and_checks_crc() {
        let payload = "00020126360014BR.GOV.BCB.PIX0114+55119999999995204000053039865802BR5913FULANO DE TAL6008BRASILIA62070503***6304C23A";
        let intent = parse_payment_qr(payload);
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "pix");
        assert_eq!(intent.currency.as_deref(), Some("BRL"));
    }

    #[test]
    fn rejects_bad_emv_crc() {
        let payload = "00020126360014BR.GOV.BCB.PIX0114+55119999999995204000053039865802BR5913FULANO DE TAL6008BRASILIA62070503***63040000";
        assert_eq!(
            parse_payment_qr(payload).validation.errors[0].code,
            crate::ErrorCode::InvalidChecksum
        );
    }

    /// Regression: a multi-byte UTF-8 character positioned so a fixed byte offset from the
    /// end lands mid-character used to panic (`byte index N is not a char boundary`) instead
    /// of failing detection, because EmvCo::detect indexed `payload` (a `&str`) at an offset
    /// derived only from `payload.len()`, with no relationship to that offset being a char
    /// boundary. Every scheme's `detect` runs on every payload, so this was reachable from
    /// any input, not just EMV-shaped ones.
    #[test]
    fn does_not_panic_on_multibyte_unicode_near_the_emv_tail() {
        let intent = parse_payment_qr("000201€€€");
        assert!(!intent.recognized);
    }

    /// Regression: same char-boundary panic, reachable through `emv::validate_crc` once a
    /// payload is confidently detected (here, via the Pix GUI substring match) but the fixed
    /// trailing-CRC slice lands inside a multi-byte character instead of on `6304`.
    #[test]
    fn does_not_panic_on_multibyte_unicode_reaching_pix_crc_validation() {
        let intent = parse_payment_qr("000201br.gov.bcb.pix€€€");
        assert!(intent.recognized);
        assert!(!intent.validation.valid);
        assert_eq!(
            intent.validation.errors[0].code,
            crate::ErrorCode::MalformedPayload
        );
    }
}
