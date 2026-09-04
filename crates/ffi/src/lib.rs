//! C ABI over `universal-payment-qr-core`, for platforms without a native Rust or WASM story:
//! Python (via PyO3, which can link this directly instead), Swift (via a C header + XCFramework),
//! and Kotlin/Android (via JNI, loading this as a `.so`). Every exported function returns a JSON
//! string shaped exactly like the CLI/WASM output - one wire format, three more front doors.
//!
//! Every function here is a panic boundary: a Rust panic unwinding across an `extern "C"` call is
//! undefined behavior in the caller, so each entry point is wrapped in `catch_unwind` even though
//! the core itself is already panic-free (see the crate-level docs on `universal_payment_qr_core`)
//! - this is defense in depth at the FFI boundary specifically, not a substitute for that.
//!
//! Every function taking a raw pointer is `unsafe`: the caller must uphold that it's either null
//! or a valid pointer to a NUL-terminated UTF-8 buffer that stays valid and unmodified for the
//! duration of the call. Memory contract for the strings this module hands back: every function
//! returning `*mut c_char` gives the caller an owned, heap-allocated string that MUST be passed to
//! `upqr_free_string` exactly once when done - never call `free()` on it directly, never free it
//! twice, and never touch a pointer this module didn't allocate.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::panic::catch_unwind;

use universal_payment_qr_core::{CapabilityPolicy, Scanner};

fn error_json(message: &str) -> String {
    serde_json::json!({ "error": { "code": "FFI_ERROR", "message": message } }).to_string()
}

fn to_c_string(value: String) -> *mut c_char {
    match CString::new(value) {
        Ok(s) => s.into_raw(),
        Err(_) => {
            // Never happens in practice (JSON strings escape NUL), but a raw unwrap here would
            // be exactly the kind of FFI-boundary panic this module exists to prevent.
            CString::new(error_json(
                "internal: result contained an embedded NUL byte",
            ))
            .expect("a fixed ASCII literal never contains an embedded NUL")
            .into_raw()
        }
    }
}

/// Reads a caller-supplied C string. Caller must uphold the pointer contract described in the
/// module docs. Private helper - not itself a public FFI entry point, so it isn't `unsafe fn`;
/// every public function that calls it performs its own safety check first.
fn read_c_str<'a>(ptr: *const c_char) -> Result<Option<&'a str>, String> {
    if ptr.is_null() {
        return Ok(None);
    }
    let bytes = unsafe { CStr::from_ptr(ptr) };
    bytes
        .to_str()
        .map(Some)
        .map_err(|_| "argument is not valid UTF-8".to_string())
}

fn scanner_from_policy(policy_json: *const c_char) -> Result<Scanner, String> {
    match read_c_str(policy_json)? {
        None | Some("") => Ok(Scanner::default()),
        Some(json) => {
            let policy: CapabilityPolicy =
                serde_json::from_str(json).map_err(|e| format!("invalid policy JSON: {e}"))?;
            Ok(Scanner::new(policy))
        }
    }
}

fn run(f: impl FnOnce() -> Result<String, String> + std::panic::UnwindSafe) -> *mut c_char {
    let outcome = catch_unwind(f);
    let json = match outcome {
        Ok(Ok(json)) => json,
        Ok(Err(message)) => error_json(&message),
        Err(_) => error_json(
            "internal panic during parsing - this should never happen; please file an issue",
        ),
    };
    to_c_string(json)
}

/// Parses a payload with the default policy (`policy_json` null) or a JSON-encoded
/// `CapabilityPolicy` override, returning a JSON-encoded `PaymentIntent`.
///
/// # Safety
/// `payload` and `policy_json` must each be either null or a valid pointer to a NUL-terminated
/// UTF-8 buffer, valid and unmodified for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn upqr_parse(
    payload: *const c_char,
    policy_json: *const c_char,
) -> *mut c_char {
    run(move || {
        let scanner = scanner_from_policy(policy_json)?;
        let payload = read_c_str(payload)?.ok_or_else(|| "payload must not be null".to_string())?;
        serde_json::to_string(&scanner.scan(payload)).map_err(|e| e.to_string())
    })
}

/// Detects a payload without full parsing, returning a JSON-encoded `Detection`.
///
/// # Safety
/// Same pointer contract as [`upqr_parse`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn upqr_detect(
    payload: *const c_char,
    policy_json: *const c_char,
) -> *mut c_char {
    run(move || {
        let scanner = scanner_from_policy(policy_json)?;
        let payload = read_c_str(payload)?.ok_or_else(|| "payload must not be null".to_string())?;
        serde_json::to_string(&scanner.detect(payload)).map_err(|e| e.to_string())
    })
}

/// Returns a JSON-encoded array of every registered scheme's metadata (id, maturity, features,
/// ...), independent of which are enabled by `policy_json`.
///
/// # Safety
/// `policy_json` must be either null or a valid pointer to a NUL-terminated UTF-8 buffer, valid
/// and unmodified for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn upqr_schemes(policy_json: *const c_char) -> *mut c_char {
    run(move || {
        let scanner = scanner_from_policy(policy_json)?;
        serde_json::to_string(&scanner.schemes()).map_err(|e| e.to_string())
    })
}

/// The crate version (`CARGO_PKG_VERSION`), for bindings that want to surface it without their
/// own separate version string to keep in sync. Takes no pointer arguments, so unlike its
/// siblings this one is safe to call directly.
#[unsafe(no_mangle)]
pub extern "C" fn upqr_version() -> *mut c_char {
    to_c_string(env!("CARGO_PKG_VERSION").to_string())
}

/// Frees a string previously returned by any `upqr_*` function. Safe to call with null (no-op).
///
/// # Safety
/// `ptr` must be either null or a pointer this module previously returned that has not already
/// been freed. Passing anything else - a pointer this module didn't allocate, or one already
/// freed - is undefined behavior, same as `free()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn upqr_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    drop(unsafe { CString::from_raw(ptr) });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_and_free(ptr: *mut c_char) -> String {
        assert!(!ptr.is_null());
        let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap().to_owned();
        unsafe { upqr_free_string(ptr) };
        s
    }

    #[test]
    fn parses_upi_through_the_c_abi() {
        let payload = CString::new("upi://pay?pa=merchant%40bank&am=499.00&cu=INR").unwrap();
        let result = read_and_free(unsafe { upqr_parse(payload.as_ptr(), std::ptr::null()) });
        assert!(result.contains("\"scheme\":\"upi\""));
        assert!(result.contains("\"amount\":\"499.00\""));
    }

    #[test]
    fn null_payload_returns_an_error_json_rather_than_crashing() {
        let result = read_and_free(unsafe { upqr_parse(std::ptr::null(), std::ptr::null()) });
        assert!(result.contains("FFI_ERROR"));
    }

    #[test]
    fn invalid_utf8_payload_returns_an_error_json_rather_than_crashing() {
        let invalid = [0x66u8, 0x6f, 0xff, 0x6f, 0x00]; // "fo\xFFo\0" - 0xFF is invalid UTF-8
        let ptr = invalid.as_ptr() as *const c_char;
        let result = read_and_free(unsafe { upqr_parse(ptr, std::ptr::null()) });
        assert!(result.contains("FFI_ERROR"));
        assert!(result.contains("UTF-8"));
    }

    #[test]
    fn policy_disables_a_scheme_through_the_c_abi() {
        let payload =
            CString::new("solana:9xQeWvG816bUx9EPfEZi4q3G44E2sA6v7a6D9k5GmRse?amount=1.25")
                .unwrap();
        let policy = CString::new(r#"{"schemes":{"solana_pay":false}}"#).unwrap();
        let result = read_and_free(unsafe { upqr_parse(payload.as_ptr(), policy.as_ptr()) });
        assert!(result.contains("\"recognized\":true"));
        assert!(result.contains("\"supported\":false"));
        assert!(result.contains("SCHEME_DISABLED"));
    }

    #[test]
    fn malformed_policy_json_returns_an_error_rather_than_panicking() {
        let payload = CString::new("upi://pay?pa=a%40b").unwrap();
        let bad_policy = CString::new("not json").unwrap();
        let result = read_and_free(unsafe { upqr_parse(payload.as_ptr(), bad_policy.as_ptr()) });
        assert!(result.contains("FFI_ERROR"));
        assert!(result.contains("invalid policy JSON"));
    }

    #[test]
    fn schemes_lists_every_registered_scheme() {
        let result = read_and_free(unsafe { upqr_schemes(std::ptr::null()) });
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed.as_array().unwrap().len() > 30);
    }

    #[test]
    fn version_matches_the_crate_version() {
        let result = read_and_free(upqr_version());
        assert_eq!(result, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn freeing_null_is_a_safe_no_op() {
        unsafe { upqr_free_string(std::ptr::null_mut()) };
    }
}
