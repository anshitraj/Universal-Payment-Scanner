import CUniversalPaymentQR
import Foundation

/// Deterministic, local-first payment QR/address parsing - UPI, Pix, EMVCo, Bitcoin, Ethereum,
/// and more, one normalized result. Backed by the same Rust core every SDK in this project uses
/// (`crates/core`), via the C ABI in `crates/ffi` - no parsing logic is reimplemented here. No
/// network calls, no credentials, no payment execution.
///
/// ```swift
/// let result = try UniversalPaymentQR.parse("upi://pay?pa=merchant%40bank&am=499.00&cu=INR")
/// result["scheme"] as? String // "upi"
/// ```
public enum UniversalPaymentQR {
    /// The crate version backing this build (`CARGO_PKG_VERSION`), kept in one place.
    public static var version: String {
        guard let ptr = upqr_version() else { return "unknown" }
        defer { upqr_free_string(ptr) }
        return String(cString: ptr)
    }

    /// Parses `payload` (a string already decoded from a QR image, or pasted/typed directly) with
    /// the default policy, or with `policy` (matching `CapabilityPolicy`'s JSON shape - see
    /// `crates/core/src/policy.rs`) if given. Returns a normalized PaymentIntent dictionary - see
    /// docs/schema.md in the repository.
    public static func parse(_ payload: String, policy: [String: Any]? = nil) throws -> [String: Any] {
        try call(payload: payload, policy: policy, native: upqr_parse)
    }

    /// Cheap detection only (scheme id + confidence), without full parsing/validation.
    public static func detect(_ payload: String, policy: [String: Any]? = nil) throws -> [String: Any] {
        try call(payload: payload, policy: policy, native: upqr_detect)
    }

    /// Every registered scheme's metadata (id, maturity, features, references, ...), independent
    /// of any capability policy. Use this to build a scheme-toggle UI.
    public static func capabilities(policy: [String: Any]? = nil) throws -> [[String: Any]] {
        let json = try withPolicyJSON(policy) { policyPtr in
            try invoke { upqr_schemes(policyPtr) }
        }
        guard let array = try JSONSerialization.jsonObject(with: json) as? [[String: Any]] else {
            throw UPQRError.unexpectedShape
        }
        return array
    }

    /// `capabilities()` filtered to schemes at `stable` or `beta` maturity.
    public static func supportedSchemes(policy: [String: Any]? = nil) throws -> [[String: Any]] {
        try capabilities(policy: policy).filter { ($0["maturity"] as? String) != "experimental" }
    }

    private static func call(
        payload: String,
        policy: [String: Any]?,
        native: (UnsafePointer<CChar>?, UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?
    ) throws -> [String: Any] {
        let json = try withPolicyJSON(policy) { policyPtr in
            try payload.withCString { payloadPtr in
                try invoke { native(payloadPtr, policyPtr) }
            }
        }
        guard let dict = try JSONSerialization.jsonObject(with: json) as? [String: Any] else {
            throw UPQRError.unexpectedShape
        }
        return dict
    }

    /// Encodes `policy` to a C string for the duration of `body`, or passes `nil` through
    /// unchanged - matching every `upqr_*` function's "null policy_json means default policy"
    /// contract.
    private static func withPolicyJSON<T>(
        _ policy: [String: Any]?,
        _ body: (UnsafePointer<CChar>?) throws -> T
    ) throws -> T {
        guard let policy else { return try body(nil) }
        let data = try JSONSerialization.data(withJSONObject: policy)
        guard let text = String(data: data, encoding: .utf8) else { throw UPQRError.unexpectedShape }
        return try text.withCString(body)
    }

    /// Calls a native function returning an owned C string, decodes it as UTF-8 `Data`, and frees
    /// it - the shared tail of every `upqr_*` call above.
    private static func invoke(_ native: () -> UnsafeMutablePointer<CChar>?) throws -> Data {
        guard let ptr = native() else { throw UPQRError.nativeCallFailed }
        defer { upqr_free_string(ptr) }
        return Data(String(cString: ptr).utf8)
    }
}

public enum UPQRError: Error {
    case nativeCallFailed
    case unexpectedShape
}

/// A parser bound to one fixed capability policy - construct via `createScanner()`, not directly.
public struct Scanner {
    fileprivate let policy: [String: Any]?

    public func scan(_ payload: String) throws -> [String: Any] {
        try UniversalPaymentQR.parse(payload, policy: policy)
    }

    public func detect(_ payload: String) throws -> [String: Any] {
        try UniversalPaymentQR.detect(payload, policy: policy)
    }

    public func schemes() throws -> [[String: Any]] {
        try UniversalPaymentQR.capabilities(policy: policy)
    }
}

/// Builds a `Scanner` bound to a capability policy. All arguments are optional and match the
/// JS/Python/Kotlin/Dart SDKs' `createScanner({...})` shape - e.g. `preset: "india"`, or
/// `schemes: ["upi": true, "bitcoin": false]`. Detection always stays global; this only controls
/// what your application accepts as `supported`.
public func createScanner(
    preset: String? = nil,
    schemes: [String: Bool]? = nil,
    categories: [String: Bool]? = nil,
    countries: [String: [String: Bool]]? = nil
) -> Scanner {
    var policy: [String: Any] = [:]
    if let preset { policy["preset"] = preset }
    if let schemes { policy["schemes"] = schemes }
    if let categories { policy["categories"] = categories }
    if let countries { policy["countries"] = countries }
    return Scanner(policy: policy.isEmpty ? nil : policy)
}
