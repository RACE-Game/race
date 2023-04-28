#![cfg(target_arch = "wasm32")]

use gloo::console::error;
use wasm_bindgen::JsValue;

use crate::error::TransportError;

impl From<JsValue> for TransportError {
    fn from(value: JsValue) -> Self {
        error!(value);
        TransportError::InteropError("An error occurred in transport".into())
    }
}
