package org.universalpaymentqr

/**
 * Raw JNI declarations matching `bindings/android/rust/src/lib.rs` exactly - the mangled method
 * names below (`Java_org_universalpaymentqr_NativeBridge_native*`) are derived from this class's
 * fully-qualified name and method names, so renaming anything here means updating the Rust side
 * (or vice versa) in lockstep. Every function returns a raw JSON string; [UniversalPaymentQR] is
 * the public API that parses it into Kotlin data.
 */
internal object NativeBridge {
    init {
        System.loadLibrary("universal_payment_qr_android")
    }

    external fun nativeParse(payload: String, policyJson: String?): String
    external fun nativeDetect(payload: String, policyJson: String?): String
    external fun nativeSchemes(policyJson: String?): String
    external fun nativeVersion(): String
}
