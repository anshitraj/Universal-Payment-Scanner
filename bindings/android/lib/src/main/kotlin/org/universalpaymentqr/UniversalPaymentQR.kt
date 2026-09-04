package org.universalpaymentqr

import org.json.JSONArray
import org.json.JSONObject

/**
 * Deterministic, local-first payment QR/address parsing - UPI, Pix, EMVCo, Bitcoin, Ethereum,
 * and more, one normalized result. Backed by the same Rust core every SDK in this project uses
 * (`crates/core`), via JNI - no parsing logic is reimplemented here. No network calls, no
 * credentials, no payment execution.
 *
 * ```kotlin
 * val result = UniversalPaymentQR.parse("upi://pay?pa=merchant%40bank&am=499.00&cu=INR")
 * result.getString("scheme") // "upi"
 * ```
 */
object UniversalPaymentQR {
    /** The crate version backing this build (`CARGO_PKG_VERSION`), kept in one place. */
    val version: String
        get() = NativeBridge.nativeVersion()

    /**
     * Parses [payload] (a string already decoded from a QR image, or pasted/typed directly) with
     * the default policy - every `stable`/`beta` scheme enabled. Returns a normalized
     * PaymentIntent - see docs/schema.md in the repository. A recognized-but-disabled scheme is
     * still returned with `recognized: true` and `supported: false`, never silently dropped.
     */
    @JvmStatic
    fun parse(payload: String): JSONObject = JSONObject(NativeBridge.nativeParse(payload, null))

    /** Cheap detection only (scheme id + confidence), without full parsing/validation. */
    @JvmStatic
    fun detect(payload: String): JSONObject = JSONObject(NativeBridge.nativeDetect(payload, null))

    /**
     * Every registered scheme's metadata (id, maturity, features, references, ...), independent
     * of any capability policy. Use this to build a scheme-toggle UI.
     */
    @JvmStatic
    fun capabilities(): JSONArray = JSONArray(NativeBridge.nativeSchemes(null))

    /** [capabilities] filtered to schemes at `stable` or `beta` maturity. */
    @JvmStatic
    fun supportedSchemes(): JSONArray {
        val result = JSONArray()
        val all = capabilities()
        for (i in 0 until all.length()) {
            val scheme = all.getJSONObject(i)
            if (scheme.optString("maturity") != "experimental") result.put(scheme)
        }
        return result
    }
}

/** A parser bound to one fixed capability policy - construct via [createScanner], not directly. */
class Scanner internal constructor(private val policyJson: String?) {
    fun scan(payload: String): JSONObject = JSONObject(NativeBridge.nativeParse(payload, policyJson))
    fun detect(payload: String): JSONObject = JSONObject(NativeBridge.nativeDetect(payload, policyJson))
    fun schemes(): JSONArray = JSONArray(NativeBridge.nativeSchemes(policyJson))
}

/**
 * Builds a [Scanner] from a raw policy object matching `CapabilityPolicy`'s JSON shape
 * (`crates/core/src/policy.rs`) directly - e.g. `{"preset": "india"}` or
 * `{"schemes": {"upi": true, "bitcoin": false}}`.
 */
fun createScanner(policy: JSONObject): Scanner = Scanner(policy.toString())

/**
 * Builds a [Scanner] from the two most common policy shapes: a named [preset] (e.g. `"india"`,
 * `"all-stable"`, `"crypto-only"`) and/or per-scheme overrides in [schemes]. Detection always
 * stays global; this only controls what your application accepts as `supported`.
 *
 * ```kotlin
 * val scanner = createScanner(schemes = mapOf("upi" to true, "bitcoin" to false))
 * scanner.scan(payload)
 * ```
 */
fun createScanner(preset: String? = null, schemes: Map<String, Boolean> = emptyMap()): Scanner {
    val policy = JSONObject()
    if (preset != null) policy.put("preset", preset)
    if (schemes.isNotEmpty()) {
        val schemesJson = JSONObject()
        schemes.forEach { (id, enabled) -> schemesJson.put(id, enabled) }
        policy.put("schemes", schemesJson)
    }
    return Scanner(policy.toString())
}
