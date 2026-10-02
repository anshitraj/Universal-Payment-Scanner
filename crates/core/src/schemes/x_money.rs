use url::Url;

use crate::registry::PaymentScheme;
use crate::{
    ActionType, Category, ErrorCode, Issue, Maturity, ParseError, PaymentIntent, Recipient,
    RecommendedAction, SchemeMetadata,
};

/// An X account's profile link (`x.com/<handle>`), which is what the QR code in the X app encodes
/// ("Scanning an X QR code will bring up the account's profile", per X's own help page). X Money
/// is sent to people from inside the X app, starting from that profile - there is no published
/// payment-link, QR payload, or API for it, so this scheme recognizes the *recipient account* and
/// hands the user off to the profile. It never sees, builds, or claims an amount or a payment
/// request.
///
/// `Beta` because the link shape it claims is confirmed by X's own documentation, the same bar
/// `venmo.com/u/<handle>` meets. What the maturity does not say is that every `x.com/<handle>`
/// can receive money: the URL is the ordinary social profile link for every X account, so whether
/// a given account is set up for X Money cannot be known from the QR alone. The intent carries an
/// `UnverifiedRecipient` warning that says exactly that rather than guessing.
///
/// Detection is intentionally narrow: `https://x.com/<handle>` or `https://www.x.com/<handle>`
/// with exactly one path segment that is a valid handle and not one of X's own top-level pages
/// (`/home`, `/explore`, ...). Legacy `twitter.com` links predate X Money and are not claimed;
/// a post, a `/<handle>/followers` page, or any other deeper path is a generic URL.
pub struct XMoney;

/// Top-level `x.com` paths that are pages of the site, not profiles. Lowercase and sorted. This
/// is a best-effort denylist, not an authoritative one: X can add routes at any time, and a
/// missed one is low-harm because the redirect target is always the canonicalized URL of the
/// scanned link, never anything else.
const RESERVED_ROUTES: &[&str] = &[
    "about",
    "account",
    "analytics",
    "api",
    "apps",
    "auth",
    "bookmarks",
    "business",
    "communities",
    "compose",
    "contact",
    "download",
    "explore",
    "followers",
    "following",
    "grok",
    "hashtag",
    "help",
    "home",
    "i",
    "intent",
    "jobs",
    "lists",
    "login",
    "logout",
    "messages",
    "money",
    "notifications",
    "oauth",
    "premium",
    "privacy",
    "search",
    "settings",
    "share",
    "signup",
    "support",
    "tos",
    "verified",
    "x_money",
];

/// X handles are 1-15 characters of `A-Z a-z 0-9 _`: X's own account-creation rules, not an
/// assumption made here.
fn is_valid_handle(handle: &str) -> bool {
    (1..=15).contains(&handle.len())
        && handle
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && !RESERVED_ROUTES
            .iter()
            .any(|route| route.eq_ignore_ascii_case(handle))
}

/// The single path segment of a profile link (`/handle` or `/handle/`), or `None` when the path
/// is empty, deeper, or contains an empty segment (`//handle`).
fn profile_segment(url: &Url) -> Option<&str> {
    let mut segments: Vec<&str> = url.path_segments()?.collect();
    if segments.last() == Some(&"") {
        segments.pop();
    }
    match segments.as_slice() {
        [handle] => Some(handle),
        _ => None,
    }
}

fn is_x_host(url: &Url) -> bool {
    url.port().is_none()
        && url
            .host_str()
            .is_some_and(|h| h.eq_ignore_ascii_case("x.com") || h.eq_ignore_ascii_case("www.x.com"))
}

impl PaymentScheme for XMoney {
    fn metadata(&self) -> SchemeMetadata {
        SchemeMetadata {
            id: "x_money".into(),
            display_name: "X Money".into(),
            // X Money's availability by country is not documented anywhere checkable, so no
            // country is claimed.
            countries: vec![],
            category: Category::PaymentLink,
            standard: "Public X profile link (account handle only)".into(),
            parser_version: env!("CARGO_PKG_VERSION").into(),
            maturity: Maturity::Beta,
            static_supported: true,
            dynamic_supported: false,
            features: vec!["profile-handle".into(), "subtype".into()],
            references: vec!["https://help.x.com/en/using-x/qr-codes".into()],
        }
    }

    fn detect(&self, payload: &str) -> u8 {
        let Ok(url) = Url::parse(payload) else {
            return 0;
        };
        if is_x_host(&url) && profile_segment(&url).is_some_and(is_valid_handle) {
            95
        } else {
            0
        }
    }

    fn parse(&self, payload: &str) -> Result<PaymentIntent, ParseError> {
        let url = Url::parse(payload)
            .map_err(|_| ParseError::new(ErrorCode::UnsafeUri, "Invalid X profile URL."))?;
        if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
            return Err(ParseError::new(
                ErrorCode::UnsafeUri,
                "X profile links must use https without embedded credentials.",
            ));
        }
        let handle = profile_segment(&url)
            .filter(|handle| is_valid_handle(handle))
            .ok_or_else(|| {
                ParseError::new(
                    ErrorCode::InvalidRecipient,
                    "X profile handle is missing or malformed.",
                )
            })?;
        let mut intent = PaymentIntent::recognized("x_money", Category::PaymentLink);
        intent.subtype = Some("profile".into());
        intent.standard = Some("Public X profile link (account handle only)".into());
        intent.network = Some("X".into());
        intent.recipient = Some(Recipient {
            id: Some(handle.into()),
            ..Recipient::default()
        });
        intent.dynamic = Some(false);
        intent.validation.warnings.push(Issue::error(
            ErrorCode::UnverifiedRecipient,
            format!(
                "This is the X profile @{handle}. The QR names an account only: whether it can receive X Money, and whether it belongs to the person you mean, cannot be verified here. Confirm both in the X app before sending anything."
            ),
        ));
        // The redirect target is rebuilt from the validated handle rather than echoing the
        // scanned string, so a tracking query, fragment, or any other attacker-supplied suffix on
        // the scanned link never reaches the handoff.
        intent.recommended_action = Some(RecommendedAction::new(
            ActionType::Redirect,
            Some(format!("https://x.com/{handle}")),
            true,
        ));
        Ok(intent)
    }
}

#[cfg(test)]
mod tests {
    use super::RESERVED_ROUTES;
    use crate::parse_payment_qr;
    use crate::{ActionType, ErrorCode};

    #[test]
    fn parses_x_profile_link() {
        let intent = parse_payment_qr("https://x.com/Example_User");
        assert!(intent.validation.valid, "{:?}", intent.validation.errors);
        assert!(intent.supported);
        assert_eq!(intent.scheme, "x_money");
        assert_eq!(intent.subtype.as_deref(), Some("profile"));
        assert_eq!(intent.network.as_deref(), Some("X"));
        assert_eq!(
            intent.recipient.unwrap().id.as_deref(),
            Some("Example_User")
        );
        // A profile link carries no payment request, so nothing amount-shaped is ever invented.
        assert!(intent.amount.is_none());
        assert!(intent.currency.is_none());
        let action = intent.recommended_action.unwrap();
        assert_eq!(action.kind, ActionType::Redirect);
        assert_eq!(action.provider.as_deref(), Some("x_money"));
        assert_eq!(action.uri.as_deref(), Some("https://x.com/Example_User"));
        assert!(action.requires_user_confirmation);
    }

    #[test]
    fn warns_that_the_recipient_is_unverified() {
        let intent = parse_payment_qr("https://x.com/Example_User");
        assert!(
            intent.validation.warnings.iter().any(|w| {
                w.code == ErrorCode::UnverifiedRecipient && w.message.contains("@Example_User")
            }),
            "{:?}",
            intent.validation.warnings
        );
    }

    #[test]
    fn accepts_www_trailing_slash_and_tracking_query() {
        for link in [
            "https://www.x.com/example",
            "https://x.com/example/",
            "https://x.com/example?s=21&t=tracking-token",
            "https://x.com/example#section",
            "HTTPS://X.COM/example",
        ] {
            let intent = parse_payment_qr(link);
            assert_eq!(intent.scheme, "x_money", "{link}");
            assert!(
                intent.validation.valid,
                "{link}: {:?}",
                intent.validation.errors
            );
        }
    }

    #[test]
    fn redirect_target_is_canonical_and_drops_query_and_fragment() {
        let intent =
            parse_payment_qr("https://www.x.com/example?s=21&redirect=https://evil.test#x");
        let action = intent.recommended_action.unwrap();
        assert_eq!(action.uri.as_deref(), Some("https://x.com/example"));
    }

    #[test]
    fn site_pages_are_not_claimed_as_profiles() {
        for route in RESERVED_ROUTES {
            let link = format!("https://x.com/{route}");
            assert_ne!(parse_payment_qr(&link).scheme, "x_money", "{link}");
        }
        assert_eq!(parse_payment_qr("https://x.com/home").scheme, "url");
        assert_eq!(parse_payment_qr("https://x.com/").scheme, "url");
        assert_eq!(parse_payment_qr("https://x.com").scheme, "url");
    }

    #[test]
    fn reserved_routes_are_sorted_lowercase_and_unique() {
        for pair in RESERVED_ROUTES.windows(2) {
            assert!(
                pair[0] < pair[1],
                "{} must sort before {}",
                pair[0],
                pair[1]
            );
        }
        for route in RESERVED_ROUTES {
            assert_eq!(*route, route.to_ascii_lowercase());
        }
    }

    #[test]
    fn deeper_paths_are_not_profiles() {
        for link in [
            "https://x.com/example/status/1234567890",
            "https://x.com/example/followers",
            "https://x.com/i/flow/login",
            "https://x.com//example",
        ] {
            assert_ne!(parse_payment_qr(link).scheme, "x_money", "{link}");
        }
    }

    #[test]
    fn rejects_malformed_handles() {
        for link in [
            // 16 characters: one over X's limit.
            "https://x.com/abcdefghijklmnop",
            "https://x.com/with-dash",
            "https://x.com/with.dot",
            "https://x.com/@example",
            "https://x.com/%40example",
        ] {
            assert_ne!(parse_payment_qr(link).scheme, "x_money", "{link}");
        }
    }

    #[test]
    fn lookalike_hosts_are_not_x() {
        for link in [
            "https://x.com.evil.test/example",
            "https://evilx.com/example",
            "https://x.com@evil.test/example",
            "https://fakex.com/example",
            "https://x.co/example",
            "https://x.com:8443/example",
            // Legacy domain: predates X Money, deliberately not claimed.
            "https://twitter.com/example",
        ] {
            assert_ne!(parse_payment_qr(link).scheme, "x_money", "{link}");
        }
    }

    #[test]
    fn plain_http_and_embedded_credentials_are_unsafe() {
        for link in ["http://x.com/example", "https://user:secret@x.com/example"] {
            let intent = parse_payment_qr(link);
            assert_eq!(intent.scheme, "x_money", "{link}");
            assert!(!intent.validation.valid, "{link}");
            assert_eq!(intent.validation.errors[0].code, ErrorCode::UnsafeUri);
        }
    }

    #[test]
    fn a_non_x_https_link_still_resolves_to_the_generic_url_detector() {
        assert_eq!(
            parse_payment_qr("https://example.com/example").scheme,
            "url"
        );
    }
}
