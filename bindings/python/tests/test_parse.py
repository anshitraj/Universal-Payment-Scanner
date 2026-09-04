from universal_payment_qr import (
    create_scanner,
    detect_payment_qr,
    get_capabilities,
    get_supported_schemes,
    parse_payment_qr,
)


def test_parses_upi_without_floating_point_drift():
    result = parse_payment_qr("upi://pay?pa=merchant%40bank&pn=Example&am=499.00&cu=INR")
    assert result["recognized"] is True
    assert result["scheme"] == "upi"
    assert result["amount"] == "499.00"
    assert result["validation"]["valid"] is True


def test_recognizes_a_bare_bitcoin_address_with_no_uri_scheme():
    result = parse_payment_qr("1BoatSLRHtKNngkdXEeobR76b53LETtpyT")
    assert result["recognized"] is True
    assert result["scheme"] == "bitcoin"


def test_non_payment_qr_is_recognized_but_flagged():
    result = parse_payment_qr("https://example.com/docs")
    assert result["recognized"] is True
    assert result["scheme"] == "url"
    assert result["supported"] is False
    assert result["support"]["reason"] == "NOT_PAYMENT_QR"


def test_truly_unstructured_input_is_not_recognized():
    result = parse_payment_qr("just some plain text with no structure at all")
    assert result["recognized"] is False


def test_detect_is_cheaper_than_parse_and_agrees_on_the_scheme():
    detection = detect_payment_qr("upi://pay?pa=a%40b&am=1&cu=INR")
    assert detection["recognized"] is True
    assert detection["scheme"] == "upi"


def test_disabled_scheme_stays_recognized_but_unsupported():
    scanner = create_scanner(schemes={"solana_pay": False})
    result = scanner.scan("solana:9xQeWvG816bUx9EPfEZi4q3G44E2sA6v7a6D9k5GmRse?amount=1.25")
    assert result["recognized"] is True
    assert result["supported"] is False
    assert result["support"]["reason"] == "SCHEME_DISABLED"


def test_preset_scopes_to_india_only():
    scanner = create_scanner(preset="india")
    upi = scanner.scan("upi://pay?pa=merchant%40bank&am=1&cu=INR")
    assert upi["supported"] is True
    bitcoin = scanner.scan("1BoatSLRHtKNngkdXEeobR76b53LETtpyT")
    assert bitcoin["recognized"] is True
    assert bitcoin["supported"] is False


def test_capabilities_lists_every_registered_scheme():
    capabilities = get_capabilities()
    ids = {scheme["id"] for scheme in capabilities}
    assert "upi" in ids
    assert "pix" in ids
    assert len(capabilities) > 30


def test_supported_schemes_excludes_experimental():
    supported = get_supported_schemes()
    assert all(scheme.get("maturity") != "experimental" for scheme in supported)


def test_oversized_payload_is_rejected_not_crashed():
    result = parse_payment_qr("x" * 200_000)
    assert result["recognized"] is False
    assert result["validation"]["errors"][0]["code"] == "PAYLOAD_TOO_LARGE"
