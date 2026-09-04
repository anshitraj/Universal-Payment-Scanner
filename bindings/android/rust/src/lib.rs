//! JNI bindings backing the `org.universalpaymentqr` Kotlin package. Every native function
//! returns a JSON `String`; the Kotlin wrapper in `UniversalPaymentQR.kt` does its own
//! `org.json`/kotlinx.serialization parsing on the caller's side. Parsing logic lives in exactly
//! one place (`universal-payment-qr-core`) - this crate is a thin, boring JNI translation layer.
//!
//! String extraction from `JString` happens *before* the panic boundary, and the panic-guarded
//! closure is pure Rust with no live `JNIEnv` reference inside it - crossing a panic boundary
//! while holding a JNI environment reference is exactly the kind of thing to avoid, even though
//! the core itself is already panic-free (see `universal_payment_qr_core`'s crate docs). Defense
//! in depth at the FFI boundary, not a substitute for that.

use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::jstring;
use universal_payment_qr_core::{CapabilityPolicy, Scanner};

fn error_json(message: &str) -> String {
    serde_json::json!({ "error": { "code": "FFI_ERROR", "message": message } }).to_string()
}

fn run_parse(payload: &str, policy_json: Option<&str>) -> Result<String, String> {
    let scanner = scanner_from_policy(policy_json)?;
    serde_json::to_string(&scanner.scan(payload)).map_err(|e| e.to_string())
}

fn run_detect(payload: &str, policy_json: Option<&str>) -> Result<String, String> {
    let scanner = scanner_from_policy(policy_json)?;
    serde_json::to_string(&scanner.detect(payload)).map_err(|e| e.to_string())
}

fn run_schemes(policy_json: Option<&str>) -> Result<String, String> {
    let scanner = scanner_from_policy(policy_json)?;
    serde_json::to_string(&scanner.schemes()).map_err(|e| e.to_string())
}

fn scanner_from_policy(policy_json: Option<&str>) -> Result<Scanner, String> {
    match policy_json {
        None => Ok(Scanner::default()),
        Some(json) => {
            let policy: CapabilityPolicy =
                serde_json::from_str(json).map_err(|e| format!("invalid policy JSON: {e}"))?;
            Ok(Scanner::new(policy))
        }
    }
}

/// Reads a nullable Java string into an owned Rust `String`, or `None` for a Java `null` /
/// JNI error. Must run before any `catch_unwind` boundary - see the module docs.
fn read_nullable_jstring(env: &mut JNIEnv, value: &JString) -> Option<String> {
    if value.is_null() {
        return None;
    }
    env.get_string(value).ok().map(|s| {
        let owned: String = s.into();
        owned
    })
}

fn respond(env: &mut JNIEnv, outcome: std::thread::Result<Result<String, String>>) -> jstring {
    let json = match outcome {
        Ok(Ok(json)) => json,
        Ok(Err(message)) => error_json(&message),
        Err(_) => error_json(
            "internal panic during parsing - this should never happen; please file an issue",
        ),
    };
    env.new_string(json)
        .expect("JSON output is always valid UTF-8")
        .into_raw()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_universalpaymentqr_NativeBridge_nativeParse<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    payload: JString<'local>,
    policy_json: JString<'local>,
) -> jstring {
    let Some(payload) = read_nullable_jstring(&mut env, &payload) else {
        return respond(&mut env, Ok(Err("payload must not be null".into())));
    };
    let policy_json = read_nullable_jstring(&mut env, &policy_json);
    let outcome = std::panic::catch_unwind(move || run_parse(&payload, policy_json.as_deref()));
    respond(&mut env, outcome)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_universalpaymentqr_NativeBridge_nativeDetect<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    payload: JString<'local>,
    policy_json: JString<'local>,
) -> jstring {
    let Some(payload) = read_nullable_jstring(&mut env, &payload) else {
        return respond(&mut env, Ok(Err("payload must not be null".into())));
    };
    let policy_json = read_nullable_jstring(&mut env, &policy_json);
    let outcome = std::panic::catch_unwind(move || run_detect(&payload, policy_json.as_deref()));
    respond(&mut env, outcome)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_universalpaymentqr_NativeBridge_nativeSchemes<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    policy_json: JString<'local>,
) -> jstring {
    let policy_json = read_nullable_jstring(&mut env, &policy_json);
    let outcome = std::panic::catch_unwind(move || run_schemes(policy_json.as_deref()));
    respond(&mut env, outcome)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_universalpaymentqr_NativeBridge_nativeVersion<'local>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jstring {
    env.new_string(env!("CARGO_PKG_VERSION"))
        .expect("crate version is always valid UTF-8")
        .into_raw()
}
