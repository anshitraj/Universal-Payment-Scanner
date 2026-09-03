//! Config-driven adapter for national EMVCo Merchant-Presented Mode overlays.
//!
//! Many national QR standards (PromptPay, DuitNow, QRIS, VietQR, ...) are not their own wire
//! format: they are the same EMVCo MPM envelope already implemented in `emv.rs`/`EmvCo`/`Pix`,
//! with a country-specific Globally Unique Identifier registered inside the merchant account
//! information fields (tags 26-51). Adding a country here is a config entry, not a new parser.
//!
//! Two confidence tiers, reflected honestly in `maturity` and in `detect()`'s confidence score:
//! - **Tier 1** (`guids` non-empty): a specific GUID string was confirmed against a public
//!   central-bank/scheme-operator source. `parse` verifies that GUID actually tags a merchant
//!   account sub-template, not just that the string appears somewhere in the payload.
//! - **Tier 2** (`guids` empty): the national standard is confirmed real and EMVCo-based, but no
//!   specific GUID has been verified yet. Detection and normalization fall back to the generic
//!   EMVCo envelope plus the country field (tag 58), which is honest but less precise - reflected
//!   in `Maturity::Community` rather than `Beta`.

use crate::emv::{self, TlvMap};
use crate::registry::PaymentScheme;
use crate::{
    ActionType, Asset, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, Recipient,
    RecommendedAction, SchemeMetadata,
};

pub struct NationalOverlayConfig {
    pub id: &'static str,
    pub display_name: &'static str,
    pub network: &'static str,
    /// ISO 3166-1 alpha-2, matched against EMV tag 58.
    pub country: &'static str,
    /// (ISO 4217 numeric, ISO 4217 alpha) matched against EMV tag 53.
    pub currency: (&'static str, &'static str),
    /// Confirmed GUID(s) for tag 00 inside a tag 26-51 sub-template. Empty means Tier 2.
    pub guids: &'static [&'static str],
    pub standard: &'static str,
    pub maturity: Maturity,
    pub references: &'static [&'static str],
}

pub struct NationalOverlay(pub &'static NationalOverlayConfig);

impl PaymentScheme for NationalOverlay {
    fn metadata(&self) -> SchemeMetadata {
        let config = self.0;
        SchemeMetadata {
            id: config.id.into(),
            display_name: config.display_name.into(),
            countries: vec![config.country.into()],
            category: Category::BankTransfer,
            standard: config.standard.into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: config.maturity.clone(),
            static_supported: true,
            dynamic_supported: true,
            features: if config.guids.is_empty() {
                vec![
                    "crc16".into(),
                    "merchant".into(),
                    "amount".into(),
                    "currency".into(),
                    "country-tag-match".into(),
                ]
            } else {
                vec![
                    "crc16".into(),
                    "merchant".into(),
                    "amount".into(),
                    "currency".into(),
                    "guid-match".into(),
                ]
            },
            references: config.references.iter().map(|r| (*r).into()).collect(),
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        let config = self.0;
        if !emv::has_mpm_envelope(payload) {
            return 0;
        }
        if config.guids.is_empty() {
            let country_tag = format!("5802{}", config.country);
            if payload.contains(&country_tag) {
                82
            } else {
                0
            }
        } else if config.guids.iter().any(|guid| payload.contains(guid)) {
            90
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        let config = self.0;
        emv::validate_crc(payload)?;
        let fields = emv::parse_tlv(payload, 100)?;
        if fields.get("00").map(String::as_str) != Some("01") {
            return Err(ParseError::new(
                ErrorCode::MalformedPayload,
                "EMV payload format indicator must be 01.",
            ));
        }
        emv::validate_required_fields(&fields)?;

        if !config.guids.is_empty() && !merchant_account_has_guid(&fields, config.guids) {
            return Err(ParseError::new(
                ErrorCode::InvalidRecipient,
                format!(
                    "{} GUID was not found in a merchant account sub-template.",
                    config.display_name
                ),
            ));
        }
        if fields.get("58").map(String::as_str) != Some(config.country) {
            return Err(ParseError::new(
                ErrorCode::InvalidNetwork,
                format!(
                    "{} requires country {}.",
                    config.display_name, config.country
                ),
            ));
        }
        if fields.get("53").map(String::as_str) != Some(config.currency.0) {
            return Err(ParseError::new(
                ErrorCode::InvalidCurrency,
                format!(
                    "{} requires currency {} ({}).",
                    config.display_name, config.currency.1, config.currency.0
                ),
            ));
        }

        let mut intent = PaymentIntent::recognized(config.id, Category::BankTransfer);
        intent.standard = Some(config.standard.into());
        intent.country = Some(config.country.into());
        intent.network = Some(config.network.into());
        intent.currency = emv::currency_from_numeric(config.currency.0).map(str::to_owned);
        if let Some(value) = fields.get("54") {
            intent.amount = Some(crate::decimal::validate_decimal(value, 12)?);
        }
        intent.dynamic = Some(fields.get("01").map(String::as_str) == Some("12"));
        intent.recipient = Some(Recipient {
            name: fields.get("59").cloned(),
            ..Recipient::default()
        });
        intent.asset = intent.currency.clone().map(|symbol| Asset {
            symbol: Some(symbol),
            ..Asset::default()
        });
        intent.metadata.insert(
            "merchantCategoryCode".into(),
            fields.get("52").cloned().unwrap_or_default().into(),
        );
        intent.validation.warnings.push(Issue::error(
            ErrorCode::UnverifiedRecipient,
            "Checksum and structure are valid; merchant identity is not verified.",
        ));
        intent.recommended_action = Some(RecommendedAction {
            kind: ActionType::Handoff,
            uri: None,
            requires_user_confirmation: true,
        });
        Ok(intent)
    }
}

/// Searches merchant account sub-templates (tags 26-51) for a nested GUID (tag 00) matching one
/// of `guids`. Bounded the same way Pix's own account sub-parse is bounded.
fn merchant_account_has_guid(fields: &TlvMap, guids: &[&str]) -> bool {
    (2..=51).any(|tag| {
        fields
            .get(&format!("{tag:02}"))
            .and_then(|value| emv::parse_tlv(value, 16).ok())
            .and_then(|sub| sub.get("00").cloned())
            .is_some_and(|found| guids.iter().any(|guid| found.eq_ignore_ascii_case(guid)))
    })
}

macro_rules! overlay {
    ($name:ident, $id:literal, $display:literal, $network:literal, $country:literal, $currency:expr, $guids:expr, $standard:literal, $maturity:expr, $refs:expr) => {
        static $name: NationalOverlayConfig = NationalOverlayConfig {
            id: $id,
            display_name: $display,
            network: $network,
            country: $country,
            currency: $currency,
            guids: $guids,
            standard: $standard,
            maturity: $maturity,
            references: $refs,
        };
    };
}

// Tier 1: confirmed GUID, cross-validated against an official/authoritative source plus at least
// one independent open-source implementation. `Maturity::Beta` - real structural validation, not
// yet promoted to `Stable` pending a byte-exact official reference test vector.
overlay!(
    PROMPTPAY,
    "promptpay",
    "PromptPay",
    "PromptPay",
    "TH",
    ("764", "THB"),
    &["A000000677010111"],
    "Bank of Thailand Standardised Thai QR Code for Payment Transactions",
    Maturity::Beta,
    &["https://www.bot.or.th/content/dam/bot/fipcs/documents/FPG/2562/EngPDF/25620084.pdf"]
);
overlay!(
    VIETQR,
    "vietqr",
    "VietQR",
    "VietQR",
    "VN",
    ("704", "VND"),
    &["A000000727"],
    "NAPAS QR Code Technical Specification",
    Maturity::Beta,
    &["https://www.napas.com.vn"]
);
overlay!(
    PAYNOW,
    "paynow",
    "PayNow",
    "PayNow",
    "SG",
    ("702", "SGD"),
    &["SG.PAYNOW"],
    "MAS Singapore Quick Response Code (SGQR) - PayNow",
    Maturity::Beta,
    &["https://www.mas.gov.sg/development/e-payments/sgqr"]
);
overlay!(
    DUITNOW_QR,
    "duitnow_qr",
    "DuitNow QR",
    "DuitNow",
    "MY",
    ("458", "MYR"),
    &["A0000006150001", "MY.PAYNET.DUITNOW"],
    "PayNet DuitNow QR Merchant-Presented Mode specification",
    Maturity::Beta,
    &["https://docs.developer.paynet.my/docs/duitNow-QR/introduction/overview"]
);
overlay!(
    QRIS,
    "qris",
    "QRIS",
    "QRIS",
    "ID",
    ("360", "IDR"),
    &["ID.CO.QRIS.WWW"],
    "Bank Indonesia / ASPI Quick Response Code Indonesian Standard",
    Maturity::Beta,
    &[
        "https://www.bi.go.id/en/fungsi-utama/sistem-pembayaran/ritel/kanal-layanan/qris/default.aspx"
    ]
);
overlay!(
    QR_PH,
    "qr_ph",
    "QR Ph",
    "InstaPay",
    "PH",
    ("608", "PHP"),
    &["PH.INSTAPAY.ME"],
    "Bangko Sentral ng Pilipinas QR Ph P2M specification",
    Maturity::Beta,
    &["https://www.bsp.gov.ph/Media_and_Research/Primers%20Faqs/QR_Ph_P2M_FAQs.pdf"]
);
overlay!(
    HKQR,
    "hkqr",
    "HKQR",
    "FPS",
    "HK",
    ("344", "HKD"),
    &["HK.COM.HKICL"],
    "HKMA Common QR Code Specification for Retail Payments in Hong Kong",
    Maturity::Beta,
    &[
        "https://www.hkma.gov.hk/media/eng/doc/key-functions/financial-infrastructure/infrastructure/retail-payment-initiatives/Common_QR_Code_Specification.pdf"
    ]
);
overlay!(
    NEPALPAY_QR,
    "nepalpay_qr",
    "NepalPay QR",
    "NepalPay",
    "NP",
    ("524", "NPR"),
    &["np.gov.nrb"],
    "Nepal Rastra Bank NepalQR Guidelines and Framework",
    Maturity::Beta,
    &[
        "https://www.nrb.org.np/contents/uploads/2021/01/QR-Code-Guidelines-and-Framework-and-Specifications.pdf"
    ]
);

// Tier 2: the national standard and its EMVCo basis are confirmed from a public source, but no
// specific GUID has been verified yet. Detected via the generic envelope plus the country field
// only, so `Maturity::Community` rather than `Beta` - honestly less precise than Tier 1.
overlay!(
    KHQR,
    "khqr",
    "KHQR",
    "Bakong",
    "KH",
    ("116", "KHR"),
    &[],
    "National Bank of Cambodia Bakong KHQR specification",
    Maturity::Community,
    &["https://bakong.nbc.gov.kh/download/KHQR/integration/KHQR%20Content%20Guideline%20v.1.3.pdf"]
);
overlay!(
    LANKAQR,
    "lankaqr",
    "LankaQR",
    "LankaQR",
    "LK",
    ("144", "LKR"),
    &[],
    "Central Bank of Sri Lanka LankaQR specification",
    Maturity::Community,
    &["https://www.cbsl.gov.lk/en/LANKAQR"]
);
overlay!(
    BANGLA_QR,
    "bangla_qr",
    "Bangla QR",
    "Bangla QR",
    "BD",
    ("050", "BDT"),
    &[],
    "Bangladesh Bank Bangla QR guidelines",
    Maturity::Community,
    &["https://www.bb.org.bd"]
);
overlay!(
    RAAST_QR,
    "raast_qr",
    "Raast QR",
    "Raast",
    "PK",
    ("586", "PKR"),
    &[],
    "State Bank of Pakistan Raast QR specification",
    Maturity::Community,
    &["https://www.sbp.org.pk/PS/Raast.html"]
);
overlay!(
    MMQR,
    "mmqr",
    "MMQR",
    "MyanmarPay",
    "MM",
    ("104", "MMK"),
    &[],
    "Myanmar QR Code Specification for Retail Payments",
    Maturity::Community,
    &["https://myanmarpay.com.mm/frontend/assets/files/MyanmarQRSpecification.pdf"]
);
overlay!(
    LAO_QR,
    "lao_qr",
    "Lao QR",
    "Lao QR",
    "LA",
    ("418", "LAK"),
    &[],
    "Bank of the Lao PDR unified QR specification",
    Maturity::Community,
    &["https://www.bol.gov.la"]
);
overlay!(
    JPQR,
    "jpqr",
    "JPQR",
    "JPQR",
    "JP",
    ("392", "JPY"),
    &[],
    "Payments Japan Association unified QR code (JPQR)",
    Maturity::Community,
    &["https://www.paymentsjapan.or.jp"]
);
overlay!(
    TWQR,
    "twqr",
    "TWQR",
    "TWQR",
    "TW",
    ("901", "TWD"),
    &[],
    "Taiwan EMVCo-based common QR code",
    Maturity::Community,
    &["https://www.emvco.com/emv-technologies/qr-codes/"]
);
overlay!(
    ZEROPAY,
    "zeropay",
    "ZeroPay",
    "ZeroPay",
    "KR",
    ("410", "KRW"),
    &[],
    "Seoul Metropolitan Government ZeroPay common QR",
    Maturity::Community,
    &["https://www.zeropay.or.kr"]
);

pub const OVERLAYS: &[&NationalOverlayConfig] = &[
    &PROMPTPAY,
    &VIETQR,
    &PAYNOW,
    &DUITNOW_QR,
    &QRIS,
    &QR_PH,
    &HKQR,
    &NEPALPAY_QR,
    &KHQR,
    &LANKAQR,
    &BANGLA_QR,
    &RAAST_QR,
    &MMQR,
    &LAO_QR,
    &JPQR,
    &TWQR,
    &ZEROPAY,
];

#[cfg(test)]
mod tests {
    use crate::parse_payment_qr;

    // Built field-by-field against the EMVCo TLV grammar (tag+2-digit-length+value) by a script
    // that also computed the real CRC-16/CCITT-FALSE trailer, then verified against this crate's
    // own `crc16_ccitt_false` - not hand-spliced, not copied from a real merchant. Structurally
    // representative of what each spec publicly documents; not a byte-exact official sample.
    const PROMPTPAY: &str = "00020101021129370016A0000006770101110113006689123456752046011530376454041.005802TH5913TEST MERCHANT6007BANGKOK63047594";
    const PROMPTPAY_WRONG_COUNTRY: &str = "00020101021129370016A0000006770101110113006689123456752046011530376454041.005802ID5913TEST MERCHANT6007BANGKOK630472E4";
    const VIETQR: &str = "00020101021138540010A00000072701240006970436011000118860200208QRIBFTTA5204599953037045802VN5912NGUYEN VAN A6005HANOI63042FCB";
    const KHQR: &str = "00020101021129300010kh.gov.nbc0112someone@wing52045999530311654041.005802KH5913TEST MERCHANT6010PHNOM PENH6304D6A2";
    const DUITNOW_QR: &str = "00020101021126350014A00000061500010113016612345678952045999530345854041.005802MY5913TEST MERCHANT6012KUALA LUMPUR63049BF8";
    const DUITNOW_QR_BAD_CRC: &str = "00020101021126350014A00000061500010113016612345678952045999530345854041.005802MY5913TEST MERCHANT6012KUALA LUMPUR63040000";
    const QRIS: &str = "00020101021126400014ID.CO.QRIS.WWW01189360001412345678905204599953033605802ID5913TEST MERCHANT6007JAKARTA630403AF";
    const PAYNOW: &str = "00020101021126380009SG.PAYNOW010100211+65912345670301152045999530370254041.005802SG5913TEST MERCHANT6009SINGAPORE63046B0D";
    const QR_PH: &str = "00020101021126380014PH.INSTAPAY.ME0316123456789012345652045999530360854041.005802PH5913TEST MERCHANT6006MANILA63044481";
    const HKQR: &str = "00020101021126370012HK.COM.HKICL010100212+8529123456752045999530334454041.005802HK5913TEST MERCHANT6009HONG KONG630488EB";
    const NEPALPAY_QR: &str = "00020101021126300010np.gov.nrb0112NCHL0000123452045999530352454041.005802NP5913TEST MERCHANT6009KATHMANDU6304A6A5";
    const LANKAQR: &str = "00020101021129320011lk.gov.cbsl0113123456789012352045999530314454041.005802LK5913TEST MERCHANT6007COLOMBO630440A5";
    const MMQR: &str = "00020101021129290011mm.com.mmqr0110091234567852045999530310454041.005802MM5913TEST MERCHANT6006YANGON630483CE";

    #[test]
    fn promptpay_guid_is_recognized_and_normalized() {
        let intent = parse_payment_qr(PROMPTPAY);
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "promptpay");
        assert_eq!(intent.currency.as_deref(), Some("THB"));
        assert_eq!(intent.amount.as_deref(), Some("1.00"));
    }

    #[test]
    fn vietqr_guid_is_recognized() {
        let intent = parse_payment_qr(VIETQR);
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "vietqr");
        assert_eq!(intent.currency.as_deref(), Some("VND"));
    }

    #[test]
    fn tier1_rejects_wrong_country_even_with_matching_guid() {
        let intent = parse_payment_qr(PROMPTPAY_WRONG_COUNTRY);
        assert_eq!(intent.scheme, "promptpay");
        assert!(!intent.validation.valid);
        assert_eq!(
            intent.validation.errors[0].code,
            crate::ErrorCode::InvalidNetwork
        );
    }

    #[test]
    fn tier2_khqr_is_recognized_via_country_tag_without_a_confirmed_guid() {
        let intent = parse_payment_qr(KHQR);
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "khqr");
        assert_eq!(intent.currency.as_deref(), Some("KHR"));
    }

    #[test]
    fn duitnow_qr_guid_is_recognized() {
        let intent = parse_payment_qr(DUITNOW_QR);
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "duitnow_qr");
        assert_eq!(intent.currency.as_deref(), Some("MYR"));
    }

    #[test]
    fn duitnow_qr_rejects_bad_crc() {
        let intent = parse_payment_qr(DUITNOW_QR_BAD_CRC);
        assert!(!intent.validation.valid);
        assert_eq!(
            intent.validation.errors[0].code,
            crate::ErrorCode::InvalidChecksum
        );
    }

    #[test]
    fn qris_guid_is_recognized() {
        let intent = parse_payment_qr(QRIS);
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "qris");
        assert_eq!(intent.currency.as_deref(), Some("IDR"));
    }

    #[test]
    fn paynow_guid_is_recognized() {
        let intent = parse_payment_qr(PAYNOW);
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "paynow");
        assert_eq!(intent.currency.as_deref(), Some("SGD"));
    }

    #[test]
    fn qr_ph_guid_is_recognized() {
        let intent = parse_payment_qr(QR_PH);
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "qr_ph");
        assert_eq!(intent.currency.as_deref(), Some("PHP"));
    }

    #[test]
    fn hkqr_guid_is_recognized() {
        let intent = parse_payment_qr(HKQR);
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "hkqr");
        assert_eq!(intent.currency.as_deref(), Some("HKD"));
    }

    #[test]
    fn nepalpay_qr_guid_is_recognized() {
        let intent = parse_payment_qr(NEPALPAY_QR);
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "nepalpay_qr");
        assert_eq!(intent.currency.as_deref(), Some("NPR"));
    }

    #[test]
    fn lankaqr_is_recognized_via_country_tag() {
        let intent = parse_payment_qr(LANKAQR);
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "lankaqr");
        assert_eq!(intent.currency.as_deref(), Some("LKR"));
    }

    #[test]
    fn mmqr_is_recognized_via_country_tag() {
        let intent = parse_payment_qr(MMQR);
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert_eq!(intent.scheme, "mmqr");
        assert_eq!(intent.currency.as_deref(), Some("MMK"));
    }

    #[test]
    fn disabled_national_overlay_stays_recognized() {
        use crate::{CapabilityPolicy, Scanner};
        let mut policy = CapabilityPolicy::default();
        policy.schemes.insert("promptpay".into(), false);
        let scanner = Scanner::new(policy);
        let intent = scanner.scan(PROMPTPAY);
        assert!(intent.recognized);
        assert!(!intent.supported);
        assert_eq!(
            intent.support.reason,
            Some(crate::ErrorCode::SchemeDisabled)
        );
    }
}
