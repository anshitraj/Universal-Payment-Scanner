//! Deterministic payment payload intelligence.
//!
//! This crate parses data that has already been decoded from a QR image. It never performs
//! network requests, opens URLs, or initiates transactions.

mod checksums;
mod decimal;
mod emv;
mod model;
mod policy;
mod registry;
mod schemes;

pub use model::*;
pub use policy::*;
pub use registry::{Scanner, default_registry};

/// Parse a payment payload using the default registry and policy.
pub fn parse_payment_qr(payload: &str) -> PaymentIntent {
    Scanner::default().scan(payload)
}

/// Detect a payment payload without changing its normalized representation.
pub fn detect_payment_qr(payload: &str) -> Detection {
    Scanner::default().detect(payload)
}

/// Validate a payment payload. Validation is syntactic and does not verify recipient identity.
pub fn validate_payment_qr(payload: &str) -> Validation {
    parse_payment_qr(payload).validation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truly_unrecognizable_input_is_not_recognized() {
        let result = parse_payment_qr("just some plain text with no structure at all");
        assert!(!result.recognized);
        assert_eq!(result.validation.errors[0].code, ErrorCode::NotPaymentQr);
    }

    #[test]
    fn a_generic_url_is_recognized_as_a_url_but_flagged_as_not_a_payment() {
        // A website link is a *known* shape - it should read as "recognized, not a payment"
        // (scheme "url", NOT_PAYMENT_QR) rather than the same "we have no idea" fallback that a
        // truly unstructured payload gets. See schemes::nonpayment::GenericUrl.
        let result = parse_payment_qr("https://example.com/docs");
        assert!(result.recognized);
        assert_eq!(result.scheme, "url");
        assert!(!result.supported);
        assert_eq!(result.support.reason, Some(ErrorCode::NotPaymentQr));
    }

    #[test]
    fn disabled_scheme_stays_recognized() {
        let mut policy = CapabilityPolicy::default();
        policy.schemes.insert("solana_pay".into(), false);
        let scanner = Scanner::new(policy);
        let result =
            scanner.scan("solana:9xQeWvG816bUx9EPfEZi4q3G44E2sA6v7a6D9k5GmRse?amount=1.25");
        assert!(result.recognized);
        assert!(!result.supported);
        assert_eq!(result.support.reason, Some(ErrorCode::SchemeDisabled));
    }

    #[test]
    fn rejects_oversized_payloads_before_detection() {
        let result = parse_payment_qr(&"x".repeat(DEFAULT_MAX_PAYLOAD_BYTES + 1));
        assert!(!result.recognized);
        assert_eq!(result.validation.errors[0].code, ErrorCode::PayloadTooLarge);
    }
}
