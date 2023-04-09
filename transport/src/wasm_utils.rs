#![cfg(target_arch = "wasm32")]

use gloo::console::warn;
use js_sys::{Function, Object, Reflect, Promise};
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

pub(crate) fn create_object(entries: &[(&str, &JsValue)]) -> Object {
    let obj = Object::new();
    for (k, v) in entries.into_iter() {
        Reflect::set(&obj, &(k.clone().into()), &v).unwrap();
    }
    obj
}

pub(crate) fn construct(ctor: &Function, vargs: &[&JsValue]) -> JsValue {
    let args = js_sys::Array::new();
    for arg in vargs.iter() {
        args.push(&arg);
    }
    Reflect::construct(ctor, &args).unwrap()
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
