use gloo::console::warn;
use js_sys::{Function, Promise, Reflect};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

pub(crate) fn rget(obj: &JsValue, key: &str) -> JsValue {
    Reflect::get(obj, &key.into()).unwrap()
}

pub(crate) fn get_function(obj: &JsValue, name: &str) -> Function {
    Reflect::get(obj, &name.into())
        .unwrap()
        .dyn_into::<Function>()
        .unwrap()
}

pub(crate) async fn resolve_promise(p: JsValue) -> Option<JsValue> {
    let p = match p.dyn_into::<Promise>() {
        Ok(p) => p,
        Err(e) => {
            warn!("Failed to resolve promise:", e);
            return None;
        }
    };
    match JsFuture::from(p).await {
        Ok(x) => Some(x),
        Err(e) => {
            warn!("Failed to resolve promise:", e);
            return None;
        }
    }
}
