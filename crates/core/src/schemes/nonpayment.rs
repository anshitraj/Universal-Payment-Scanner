//! Recognized-but-not-a-payment QR types. A scanner that only ever says "unknown QR" for a
//! WalletConnect pairing code, a 2FA setup code, or a plain website link is less trustworthy than
//! one that says what it actually found. All three here share the same shape: `recognized: true`,
//! `category: Unknown`, `supported: false` with `support.reason: NOT_PAYMENT_QR` - reusing the
//! existing error code rather than adding a new one, since the meaning ("recognized, but this
//! isn't a payment") is exactly what `NotPaymentQr` already means; only the `scheme` differs from
//! the fully-unrecognized fallback.

use crate::registry::PaymentScheme;
use crate::{
    ActionType, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, RecommendedAction,
    SchemeMetadata,
};

fn not_a_payment(
    scheme: &'static str,
    standard: &'static str,
    message: &'static str,
) -> PaymentIntent {
    let mut intent = PaymentIntent::recognized(scheme, Category::Unknown);
    intent.standard = Some(standard.into());
    intent.supported = false;
    intent.support.enabled = false;
    intent.support.reason = Some(ErrorCode::NotPaymentQr);
    intent.support.message = Some(message.into());
    intent.support.message_key = Some("scanner.not_payment_qr".into());
    intent
        .validation
        .warnings
        .push(Issue::error(ErrorCode::NotPaymentQr, message));
    intent.recommended_action = Some(RecommendedAction {
        kind: ActionType::DisplayOnly,
        uri: None,
        requires_user_confirmation: false,
    });
    intent
}

/// WalletConnect pairing URIs (`wc:<topic>@<version>?...`), per ERC-1328 / the WalletConnect
/// specs. A wallet-connection request, not a payment - scanning one should never be silently
/// treated as an unknown QR.
pub struct WalletConnect;

impl PaymentScheme for WalletConnect {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "walletconnect".into(),
            display_name: "WalletConnect pairing request".into(),
            countries: vec![],
            category: Category::Unknown,
            standard: "WalletConnect pairing URI (ERC-1328 / WalletConnect specs)".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Beta,
            static_supported: true,
            dynamic_supported: true,
            features: vec!["not-a-payment".into()],
            references: vec![
                "https://specs.walletconnect.com/2.0/specs/clients/core/pairing/pairing-uri".into(),
            ],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        if payload
            .get(..3)
            .is_some_and(|p| p.eq_ignore_ascii_case("wc:"))
        {
            95
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        let body = &payload[3..];
        let (topic, rest) = body.split_once('@').unwrap_or((body, ""));
        if topic.is_empty() {
            return Err(ParseError::new(
                ErrorCode::MalformedPayload,
                "WalletConnect URI is missing a pairing topic.",
            ));
        }
        let version = rest.split_once('?').map_or(rest, |(v, _)| v);
        let mut intent = not_a_payment(
            "walletconnect",
            "WalletConnect pairing URI",
            "This is a wallet-connection request, not a payment. Approving it lets an app see wallet addresses and request signatures - it does not move funds by itself.",
        );
        intent.network = Some("walletconnect".into());
        if !version.is_empty() {
            intent
                .metadata
                .insert("protocolVersion".into(), version.into());
        }
        Ok(intent)
    }
}

/// TOTP/HOTP setup QR codes (`otpauth://totp/...` or `otpauth://hotp/...`), the Google
/// Authenticator Key URI Format used by essentially every 2FA app. Deliberately never extracts or
/// stores the `secret` parameter: that's a live authentication credential, and echoing it back
/// (even locally) works against the reason someone scans this in the first place.
pub struct Authenticator;

impl PaymentScheme for Authenticator {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "otp_setup".into(),
            display_name: "Authenticator (OTP) setup".into(),
            countries: vec![],
            category: Category::Unknown,
            standard: "Google Authenticator Key URI Format".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Beta,
            static_supported: true,
            dynamic_supported: false,
            features: vec!["not-a-payment".into(), "secret-never-extracted".into()],
            references: vec![
                "https://github.com/google/google-authenticator/wiki/Key-Uri-Format".into(),
            ],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        if payload
            .get(..10)
            .is_some_and(|p| p.eq_ignore_ascii_case("otpauth://"))
        {
            95
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        let body = payload
            .get(10..)
            .ok_or_else(|| ParseError::new(ErrorCode::MalformedPayload, "Invalid otpauth URI."))?;
        let otp_type = body.split_once('/').map_or(body, |(t, _)| t);
        let otp_type = otp_type.split('?').next().unwrap_or(otp_type);
        if !otp_type.eq_ignore_ascii_case("totp") && !otp_type.eq_ignore_ascii_case("hotp") {
            return Err(ParseError::new(
                ErrorCode::MalformedPayload,
                "otpauth URI must be totp or hotp.",
            ));
        }
        let mut intent = not_a_payment(
            "otp_setup",
            "Google Authenticator Key URI Format",
            "This is a two-factor authentication setup code, not a payment. Its secret is not read or stored.",
        );
        intent
            .metadata
            .insert("otpType".into(), otp_type.to_ascii_lowercase().into());
        Ok(intent)
    }
}

/// A well-formed `http(s)://` URL that no more specific scheme claimed. Lowest confidence of any
/// detector in the registry (just above the recognition threshold) so any real payment scheme's
/// own host/path match always wins - this only fires when genuinely nothing else recognized the
/// link.
pub struct GenericUrl;

impl PaymentScheme for GenericUrl {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "url".into(),
            display_name: "Generic URL".into(),
            countries: vec![],
            category: Category::Unknown,
            standard: "RFC 3986 URI".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Beta,
            static_supported: true,
            dynamic_supported: false,
            features: vec!["not-a-payment".into()],
            references: vec!["https://www.rfc-editor.org/rfc/rfc3986".into()],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        let lower_prefix = payload.get(..8).map(|p| p.to_ascii_lowercase());
        if payload
            .get(..7)
            .is_some_and(|p| p.eq_ignore_ascii_case("http://"))
            || lower_prefix.as_deref() == Some("https://")
        {
            71
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        let url = url::Url::parse(payload)
            .map_err(|_| ParseError::new(ErrorCode::MalformedPayload, "Invalid URL."))?;
        let mut intent = not_a_payment(
            "url",
            "RFC 3986 URI",
            "That QR is a website link, not a payment request.",
        );
        if let Some(host) = url.host_str() {
            intent.metadata.insert("host".into(), host.into());
        }
        Ok(intent)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_payment_qr;

    #[test]
    fn walletconnect_uri_is_recognized_but_not_a_payment() {
        let intent = parse_payment_qr(
            "wc:7f6e504bfad60b485450578e05678ed3e8e8c4751d3c6160be17160d63ec90f9@2?relay-protocol=irn&symKey=abc",
        );
        assert!(intent.recognized);
        assert!(!intent.supported);
        assert_eq!(intent.scheme, "walletconnect");
        assert_eq!(intent.support.reason, Some(crate::ErrorCode::NotPaymentQr));
    }

    #[test]
    fn otpauth_uri_is_recognized_and_never_exposes_the_secret() {
        let payload =
            "otpauth://totp/Example:alice@example.com?secret=JBSWY3DPEHPK3PXP&issuer=Example";
        let intent = parse_payment_qr(payload);
        assert!(intent.recognized);
        assert!(!intent.supported);
        assert_eq!(intent.scheme, "otp_setup");
        let serialized = serde_json::to_string(&intent).unwrap();
        assert!(!serialized.contains("JBSWY3DPEHPK3PXP"));
    }

    #[test]
    fn generic_url_is_recognized_as_not_a_payment() {
        let intent = parse_payment_qr("https://example.com/docs");
        assert!(intent.recognized);
        assert!(!intent.supported);
        assert_eq!(intent.scheme, "url");
        assert_eq!(intent.support.reason, Some(crate::ErrorCode::NotPaymentQr));
    }

    #[test]
    fn a_specific_payment_link_still_wins_over_the_generic_url_fallback() {
        let intent = parse_payment_qr("https://paypal.me/exampleuser/10.00");
        assert_eq!(intent.scheme, "paypal");
    }
}
