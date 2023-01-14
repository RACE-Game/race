#![target_arch="wasm32"]

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace=console)]
    pub fn log(s: &str);

    #[wasm_bindgen(js_namespace=window)]
    pub fn postMessage(s: &str, domain: &str);
}
