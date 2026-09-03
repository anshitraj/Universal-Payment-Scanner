use serde::Serialize;
use universal_payment_qr_core::{CapabilityPolicy, Scanner};
use wasm_bindgen::prelude::*;

/// `serde_wasm_bindgen::to_value`'s default serializer represents a Rust map (`PaymentIntent`'s
/// `metadata: serde_json::Map<String, Value>`) as a JS `Map` instance rather than a plain object.
/// A JS `Map` has no enumerable own properties, so `JSON.stringify` on it - or on anything
/// containing it - silently renders `{}`: every scheme that populates `metadata` (EMVCo merchant
/// fields, Pix's dynamic URL, TON's bounceable flag, Swiss QR's reference type, ...) would lose
/// that field for every consumer of the WASM/JS SDK while the native Rust and Go paths keep it
/// intact. `json_compatible()` makes maps serialize as plain objects, matching what `serde_json`
/// already does and what every other field in this schema already looks like on the wire.
fn to_json_compatible_value<T: Serialize + ?Sized>(value: &T) -> Result<JsValue, JsValue> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

#[wasm_bindgen]
pub struct WasmScanner {
    scanner: Scanner,
}

#[wasm_bindgen]
impl WasmScanner {
    #[wasm_bindgen(constructor)]
    pub fn new(config: JsValue) -> Result<WasmScanner, JsValue> {
        let policy = if config.is_null() || config.is_undefined() {
            CapabilityPolicy::default()
        } else {
            serde_wasm_bindgen::from_value(config)
                .map_err(|error| JsValue::from_str(&error.to_string()))?
        };
        Ok(Self {
            scanner: Scanner::new(policy),
        })
    }

    pub fn scan(&self, payload: &str) -> Result<JsValue, JsValue> {
        to_json_compatible_value(&self.scanner.scan(payload))
    }

    pub fn detect(&self, payload: &str) -> Result<JsValue, JsValue> {
        to_json_compatible_value(&self.scanner.detect(payload))
    }

    pub fn schemes(&self) -> Result<JsValue, JsValue> {
        to_json_compatible_value(&self.scanner.schemes())
    }
}
