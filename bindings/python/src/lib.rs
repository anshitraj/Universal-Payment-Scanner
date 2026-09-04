//! PyO3 native extension backing the `universal_payment_qr` Python package. Every function here
//! returns a JSON string; the pure-Python wrapper in `universal_payment_qr/__init__.py` does
//! `json.loads()` on it. That keeps this crate a thin, boring translation layer with the parsing
//! logic living in exactly one place (`universal-payment-qr-core`), and avoids a direct
//! Rust-struct-to-Python-object conversion dependency for something this simple.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use universal_payment_qr_core::{CapabilityPolicy, Scanner};

fn scanner_from_policy_json(policy_json: Option<&str>) -> PyResult<Scanner> {
    match policy_json {
        None => Ok(Scanner::default()),
        Some(json) => {
            let policy: CapabilityPolicy = serde_json::from_str(json)
                .map_err(|e| PyValueError::new_err(format!("invalid policy JSON: {e}")))?;
            Ok(Scanner::new(policy))
        }
    }
}

/// Parses `payload` with the default policy, or with `policy_json` (a JSON-encoded
/// `CapabilityPolicy`) if given. Returns a JSON-encoded `PaymentIntent`.
#[pyfunction]
#[pyo3(signature = (payload, policy_json=None))]
fn parse(payload: &str, policy_json: Option<&str>) -> PyResult<String> {
    let scanner = scanner_from_policy_json(policy_json)?;
    serde_json::to_string(&scanner.scan(payload)).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Detects `payload` without full parsing. Returns a JSON-encoded `Detection`.
#[pyfunction]
#[pyo3(signature = (payload, policy_json=None))]
fn detect(payload: &str, policy_json: Option<&str>) -> PyResult<String> {
    let scanner = scanner_from_policy_json(policy_json)?;
    serde_json::to_string(&scanner.detect(payload))
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Every registered scheme's metadata (id, maturity, features, ...), independent of which are
/// enabled by `policy_json`. Returns a JSON-encoded array.
#[pyfunction]
#[pyo3(signature = (policy_json=None))]
fn schemes(policy_json: Option<&str>) -> PyResult<String> {
    let scanner = scanner_from_policy_json(policy_json)?;
    serde_json::to_string(&scanner.schemes()).map_err(|e| PyValueError::new_err(e.to_string()))
}

#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(detect, m)?)?;
    m.add_function(wrap_pyfunction!(schemes, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    Ok(())
}
