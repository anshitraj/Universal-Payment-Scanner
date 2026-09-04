import XCTest
@testable import UniversalPaymentQR

// Mirrors the equivalent test suites in bindings/python, bindings/android, and bindings/flutter -
// same assertions, same payloads, one per binding, so a scheme's behavior can't quietly diverge
// between platforms. NOT run in this repository's development session (no macOS/Xcode available
// there) - see README.md "What wasn't verified here".
final class UniversalPaymentQRTests: XCTestCase {
    func testParsesUPIWithoutFloatingPointDrift() throws {
        let result = try UniversalPaymentQR.parse("upi://pay?pa=merchant%40bank&pn=Example&am=499.00&cu=INR")
        XCTAssertEqual(result["recognized"] as? Bool, true)
        XCTAssertEqual(result["scheme"] as? String, "upi")
        XCTAssertEqual(result["amount"] as? String, "499.00")
    }

    func testRecognizesABareBitcoinAddressWithNoURIScheme() throws {
        let result = try UniversalPaymentQR.parse("1BoatSLRHtKNngkdXEeobR76b53LETtpyT")
        XCTAssertEqual(result["recognized"] as? Bool, true)
        XCTAssertEqual(result["scheme"] as? String, "bitcoin")
    }

    func testNonPaymentQRIsRecognizedButFlagged() throws {
        let result = try UniversalPaymentQR.parse("https://example.com/docs")
        XCTAssertEqual(result["recognized"] as? Bool, true)
        XCTAssertEqual(result["scheme"] as? String, "url")
        XCTAssertEqual(result["supported"] as? Bool, false)
        let support = result["support"] as? [String: Any]
        XCTAssertEqual(support?["reason"] as? String, "NOT_PAYMENT_QR")
    }

    func testDisabledSchemeStaysRecognizedButUnsupported() throws {
        let scanner = createScanner(schemes: ["solana_pay": false])
        let result = try scanner.scan("solana:9xQeWvG816bUx9EPfEZi4q3G44E2sA6v7a6D9k5GmRse?amount=1.25")
        XCTAssertEqual(result["recognized"] as? Bool, true)
        XCTAssertEqual(result["supported"] as? Bool, false)
        let support = result["support"] as? [String: Any]
        XCTAssertEqual(support?["reason"] as? String, "SCHEME_DISABLED")
    }

    func testPresetScopesToIndiaOnly() throws {
        let scanner = createScanner(preset: "india")
        let upi = try scanner.scan("upi://pay?pa=merchant%40bank&am=1&cu=INR")
        XCTAssertEqual(upi["supported"] as? Bool, true)
        let bitcoin = try scanner.scan("1BoatSLRHtKNngkdXEeobR76b53LETtpyT")
        XCTAssertEqual(bitcoin["recognized"] as? Bool, true)
        XCTAssertEqual(bitcoin["supported"] as? Bool, false)
    }

    func testCapabilitiesListsEveryRegisteredScheme() throws {
        let capabilities = try UniversalPaymentQR.capabilities()
        let ids = Set(capabilities.compactMap { $0["id"] as? String })
        XCTAssertTrue(ids.contains("upi"))
        XCTAssertTrue(ids.contains("pix"))
        XCTAssertGreaterThan(capabilities.count, 30)
    }

    func testSupportedSchemesExcludesExperimental() throws {
        let supported = try UniversalPaymentQR.supportedSchemes()
        XCTAssertTrue(supported.allSatisfy { ($0["maturity"] as? String) != "experimental" })
    }

    func testVersionIsNonEmpty() {
        XCTAssertFalse(UniversalPaymentQR.version.isEmpty)
    }
}
