package org.universalpaymentqr

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Runs on the host JVM against a host-native build of the JNI crate - see
 * `build.gradle.kts`'s `java.library.path` wiring and README.md "Local unit tests" for how that
 * build gets produced. The real Android-targeted `.so` files under `src/main/jniLibs` are only
 * exercised on an actual device/emulator.
 */
class UniversalPaymentQRTest {
    @Test
    fun `parses UPI without floating point drift`() {
        val result = UniversalPaymentQR.parse("upi://pay?pa=merchant%40bank&pn=Example&am=499.00&cu=INR")
        assertTrue(result.getBoolean("recognized"))
        assertEquals("upi", result.getString("scheme"))
        assertEquals("499.00", result.getString("amount"))
    }

    @Test
    fun `recognizes a bare bitcoin address with no uri scheme`() {
        val result = UniversalPaymentQR.parse("1BoatSLRHtKNngkdXEeobR76b53LETtpyT")
        assertTrue(result.getBoolean("recognized"))
        assertEquals("bitcoin", result.getString("scheme"))
    }

    @Test
    fun `non-payment QR is recognized but flagged, not left fully unrecognized`() {
        val result = UniversalPaymentQR.parse("https://example.com/docs")
        assertTrue(result.getBoolean("recognized"))
        assertEquals("url", result.getString("scheme"))
        assertFalse(result.getBoolean("supported"))
        assertEquals("NOT_PAYMENT_QR", result.getJSONObject("support").getString("reason"))
    }

    @Test
    fun `disabled scheme stays recognized but unsupported`() {
        val scanner = createScanner(schemes = mapOf("solana_pay" to false))
        val result = scanner.scan("solana:9xQeWvG816bUx9EPfEZi4q3G44E2sA6v7a6D9k5GmRse?amount=1.25")
        assertTrue(result.getBoolean("recognized"))
        assertFalse(result.getBoolean("supported"))
        assertEquals("SCHEME_DISABLED", result.getJSONObject("support").getString("reason"))
    }

    @Test
    fun `preset scopes to india only`() {
        val scanner = createScanner(preset = "india")
        val upi = scanner.scan("upi://pay?pa=merchant%40bank&am=1&cu=INR")
        assertTrue(upi.getBoolean("supported"))
        val bitcoin = scanner.scan("1BoatSLRHtKNngkdXEeobR76b53LETtpyT")
        assertTrue(bitcoin.getBoolean("recognized"))
        assertFalse(bitcoin.getBoolean("supported"))
    }

    @Test
    fun `raw policy JSONObject overload works directly`() {
        val scanner = createScanner(JSONObject().put("preset", "crypto-only"))
        val result = scanner.scan("1BoatSLRHtKNngkdXEeobR76b53LETtpyT")
        assertTrue(result.getBoolean("supported"))
    }

    @Test
    fun `capabilities lists every registered scheme`() {
        val ids = (0 until UniversalPaymentQR.capabilities().length())
            .map { UniversalPaymentQR.capabilities().getJSONObject(it).getString("id") }
            .toSet()
        assertTrue(ids.contains("upi"))
        assertTrue(ids.contains("pix"))
        assertTrue(ids.size > 30)
    }

    @Test
    fun `supported schemes excludes experimental`() {
        val schemes = UniversalPaymentQR.supportedSchemes()
        for (i in 0 until schemes.length()) {
            assertFalse(schemes.getJSONObject(i).optString("maturity") == "experimental")
        }
    }

    @Test
    fun `version is non-empty`() {
        assertTrue(UniversalPaymentQR.version.isNotEmpty())
    }
}
