use gloo::console::{error, warn};
use js_sys::{Function, Promise, Reflect};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

use crate::error::SdkError;

pub(crate) fn rget(obj: &JsValue, key: &str) -> JsValue {
    Reflect::get(obj, &key.into()).unwrap()
}

pub(crate) fn get_function(obj: &JsValue, name: &str) -> Result<Function, SdkError> {
    Reflect::get(obj, &name.into())
        .map_err(|_e| {
            error!("Object:", obj);
            error!("Property:", name);
            SdkError::InteropError("Function does not exist".into())
        })?
        .dyn_into::<Function>()
        .map_err(|_e| {
            error!("Object:", obj);
            error!("Property:", name);
            SdkError::InteropError("Property is not a function".into())
        })
}

pub(crate) async fn resolve_promise(p: JsValue) -> Result<JsValue, SdkError> {
    let p = match p.dyn_into::<Promise>() {
        Ok(p) => p,
        Err(e) => {
            warn!("Unexpected type of JsValue(Promise was expected):", e);
            return Err(SdkError::InteropError("Unexpected type of JsValue".into()));
        }
    };
    match JsFuture::from(p).await {
        Ok(x) => Ok(x),
        Err(e) => {
            warn!("Failed to resolve promise:", e);
            return Err(SdkError::InteropError("Failed to resolve a promise".into()));
        }
    }
}
