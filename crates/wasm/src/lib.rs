use universal_payment_qr_core::{CapabilityPolicy, Scanner};
use wasm_bindgen::prelude::*;

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
        serde_wasm_bindgen::to_value(&self.scanner.scan(payload))
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    pub fn detect(&self, payload: &str) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.scanner.detect(payload))
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    pub fn schemes(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.scanner.schemes())
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }
}
