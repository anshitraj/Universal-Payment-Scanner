from pathlib import Path
from playwright.sync_api import sync_playwright
import qrcode

root = Path(__file__).resolve().parents[2]
artifact_dir = root / "tests" / "artifacts"
artifact_dir.mkdir(parents=True, exist_ok=True)
upload_fixture = artifact_dir / "bitcoin-test.png"
qrcode.make("bitcoin:1BoatSLRHtKNngkdXEeobR76b53LETtpyT?amount=0.001").save(upload_fixture)

with sync_playwright() as playwright:
    browser = playwright.chromium.launch(headless=True)
    page = browser.new_page(viewport={"width": 1440, "height": 1000}, device_scale_factor=1)
    console_errors = []
    page.on("console", lambda message: console_errors.append(message.text) if message.type == "error" else None)

    page.goto("http://127.0.0.1:4173")
    page.wait_for_load_state("networkidle")
    assert page.get_by_role("heading", name="One scanner for every payment QR.").is_visible()

    payload = page.get_by_label("Raw QR payload")
    payload.fill("upi://pay?pa=merchant%40bank&pn=Example&am=499.00&cu=INR")
    page.get_by_role("button", name="Parse intent").click()
    page.wait_for_timeout(1000)
    page.get_by_text("upi recognized", exact=True).wait_for()
    assert '"scheme": "upi"' in page.locator(".json-panel pre").inner_text()
    assert '"amount": "499.00"' in page.locator(".json-panel pre").inner_text()

    page.locator(".scheme-list label", has_text="Solana Pay").get_by_role("checkbox").uncheck()
    payload.fill("solana:9xQeWvG816bUx9EPfEZi4q3G44E2sA6v7a6D9k5GmRse?amount=1.25")
    page.get_by_role("button", name="Parse intent").click()
    page.get_by_text("Solana Pay payments are not supported by this application.", exact=True).wait_for()
    assert '"reason": "SCHEME_DISABLED"' in page.locator(".json-panel pre").inner_text()

    # A generic website link is recognized (scheme "url") and flagged NOT_PAYMENT_QR, not left
    # fully unrecognized - see schemes::nonpayment::GenericUrl.
    payload.fill("https://example.com/docs")
    page.get_by_role("button", name="Parse intent").click()
    page.get_by_text("That QR is a website link, not a payment request.", exact=True).wait_for()
    assert '"scheme": "url"' in page.locator(".json-panel pre").inner_text()

    page.get_by_role("button", name="upload", exact=True).click()
    page.locator('input[type="file"]').set_input_files(str(upload_fixture))
    page.get_by_text("bitcoin recognized", exact=True).wait_for()
    assert '"scheme": "bitcoin"' in page.locator(".json-panel pre").inner_text()

    # Regression guard for a WASM/native parity bug: serde_wasm_bindgen's default serializer
    # represented PaymentIntent.metadata (a Rust map) as a JS Map, which JSON.stringify silently
    # renders as "{}". Nothing exercised metadata through real WASM+JSON.stringify until this was
    # added, so it shipped undetected - see crates/wasm/src/lib.rs and CHANGELOG.md.
    page.get_by_role("button", name="paste", exact=True).click()
    payload.fill(
        "00020126360014BR.GOV.BCB.PIX0114+55119999999995204000053039865802BR5913FULANO DE TAL6008BRASILIA62070503***6304C23A"
    )
    page.get_by_role("button", name="Parse intent").click()
    page.get_by_text("pix recognized", exact=True).wait_for()
    metadata_text = page.locator(".json-panel pre").inner_text()
    assert '"merchantCategoryCode": "0000"' in metadata_text
    assert '"merchantCity": "BRASILIA"' in metadata_text

    page.screenshot(path=str(artifact_dir / "playground.png"), full_page=True)
    page.set_viewport_size({"width": 390, "height": 844})
    assert page.evaluate("document.documentElement.scrollWidth <= window.innerWidth")
    page.screenshot(path=str(artifact_dir / "playground-mobile.png"), full_page=True)
    assert not console_errors, f"browser console errors: {console_errors}"
    browser.close()
