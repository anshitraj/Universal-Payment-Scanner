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

    #[test]
    fn crypto_accept_policy_is_permissive_by_default() {
        // No `accept.crypto` configured - the exact scenario every crypto test before this one
        // already relies on: crypto intents pass through ungated by network or asset.
        let result = parse_payment_qr(
            "ethereum:0xfb6916095ca1df60bb79Ce92ce3ea74c37c5d359@1?value=2.014e18",
        );
        assert!(result.supported);
    }

    #[test]
    fn crypto_accept_policy_rejects_a_network_not_on_the_allow_list() {
        let mut policy = CapabilityPolicy::default();
        policy
            .accept
            .crypto
            .insert("base".into(), vec!["USDC".into()]);
        policy.accept.crypto.insert("solana".into(), vec![]);
        let scanner = Scanner::new(policy);
        // Chain id 1 -> "ethereum" (see schemes::ethereum::chain_name), not in the allow-list.
        let result =
            scanner.scan("ethereum:0xfb6916095ca1df60bb79Ce92ce3ea74c37c5d359@1?value=2.014e18");
        assert!(result.recognized);
        assert!(!result.supported);
        assert_eq!(result.support.reason, Some(ErrorCode::NetworkUnsupported));
        let details = result.support.details.expect("details");
        assert_eq!(details["network"], "ethereum");
        let mut allowed = details["allowedNetworks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>();
        allowed.sort_unstable();
        assert_eq!(allowed, ["base", "solana"]);
        assert_eq!(
            result.recommended_action.unwrap().kind,
            ActionType::Unsupported
        );
    }

    #[test]
    fn crypto_accept_policy_allows_any_asset_when_the_network_has_no_asset_list() {
        let mut policy = CapabilityPolicy::default();
        policy.accept.crypto.insert("ethereum".into(), vec![]);
        let scanner = Scanner::new(policy);
        let result =
            scanner.scan("ethereum:0xfb6916095ca1df60bb79Ce92ce3ea74c37c5d359@1?value=2.014e18");
        assert!(result.supported);
    }

    #[test]
    fn crypto_accept_policy_rejects_a_native_asset_not_on_its_networks_allow_list() {
        let mut policy = CapabilityPolicy::default();
        policy
            .accept
            .crypto
            .insert("ethereum".into(), vec!["USDC".into()]);
        let scanner = Scanner::new(policy);
        // Native ETH transfer - asset.symbol is the known value "ETH", not "USDC".
        let result =
            scanner.scan("ethereum:0xfb6916095ca1df60bb79Ce92ce3ea74c37c5d359@1?value=2.014e18");
        assert!(!result.supported);
        assert_eq!(result.support.reason, Some(ErrorCode::AssetUnsupported));
        assert_eq!(result.support.details.unwrap()["asset"], "ETH");
    }

    #[test]
    fn crypto_accept_policy_fails_closed_when_asset_identity_is_unverifiable() {
        let mut policy = CapabilityPolicy::default();
        policy
            .accept
            .crypto
            .insert("ethereum".into(), vec!["USDT".into()]);
        let scanner = Scanner::new(policy);
        // ERC-20 transfer: only a contract address is known, never guessed as a symbol (see
        // schemes::ethereum::tests::parses_erc20_transfer_without_guessing_decimals) - so an
        // asset-restricted policy can't confirm it belongs on the allow-list either way.
        let result = scanner.scan(
            "ethereum:0x89205A3A3b2A69De6Dbf7f01ED13B2108B2c43e7/transfer?address=0x8e23ee67d1332ad560396262c48ffbb01f93d052&uint256=1",
        );
        assert!(!result.supported);
        assert_eq!(result.support.reason, Some(ErrorCode::AssetUnverifiable));
        assert_eq!(
            result.support.details.unwrap()["contract"],
            "0x89205A3A3b2A69De6Dbf7f01ED13B2108B2c43e7"
        );
    }

    #[test]
    fn wallet_action_carries_the_network_it_should_settle_on() {
        let result =
            parse_payment_qr("solana:9xQeWvG816bUx9EPfEZi4q3G44E2sA6v7a6D9k5GmRse?amount=1.25");
        let action = result.recommended_action.unwrap();
        assert_eq!(action.kind, ActionType::Wallet);
        assert_eq!(action.network.as_deref(), result.network.as_deref());
        assert_eq!(action.network.as_deref(), Some("solana"));
    }

    #[test]
    fn redirect_action_carries_the_web_provider() {
        let result = parse_payment_qr("https://paypal.me/exampleuser/25.00");
        let action = result.recommended_action.unwrap();
        assert_eq!(action.kind, ActionType::Redirect);
        assert_eq!(action.provider.as_deref(), Some("paypal"));
    }

    #[test]
    fn deeplink_action_carries_the_uri_scheme_for_external_app_handoff() {
        let result = parse_payment_qr("upi://pay?pa=merchant%40bank&am=499.00&cu=INR");
        let action = result.recommended_action.unwrap();
        assert_eq!(action.kind, ActionType::Deeplink);
        assert_eq!(action.scheme.as_deref(), Some("upi"));
    }
}
