#![cfg(target_arch = "wasm32")]
use js_sys::{Function, Object, Reflect, Uint8Array, WebAssembly};
use jsonrpsee::wasm_client::WasmClientBuilder;
use jsonrpsee::{core::client::ClientT, rpc_params};
use race_core::context::{GameContext, SecretContext};
use race_crypto::secret::{apply, decrypt, encrypt, export_rsa_pubkey, gen_chacha20, gen_rsa};
use race_core::types::{GameBundle, GetGameBundleParams};

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace=console)]
    pub fn log(s: &str);

    #[wasm_bindgen(js_namespace=window)]
    pub fn postMessage(s: &str, domain: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[derive(Debug)]
pub enum Error {
    FromJsValue(JsValue),
    FromCoreError(race_core::error::Error),
    RpcError(jsonrpsee::core::Error),
}

impl From<JsValue> for Error {
    fn from(e: JsValue) -> Self {
        Self::FromJsValue(e)
    }
}

impl From<race_core::error::Error> for Error {
    fn from(e: race_core::error::Error) -> Self {
        Self::FromCoreError(e)
    }
}

impl From<jsonrpsee::core::Error> for Error {
    fn from(e: jsonrpsee::core::Error) -> Self {
        Self::RpcError(e)
    }
}

pub async fn load_game_internal(addr: &str) -> Result<WebAssembly::Instance, Error> {
    let url = "ws://localhost:12002";
    let client = WasmClientBuilder::default().build(url);
    let client = client.await;
    let client = client.unwrap();
    let params = rpc_params![GetGameBundleParams { addr: addr.into() }];
    let mut bundle: GameBundle = client.request("get_game_bundle", params).await?;
    let wasm = bundle.data.as_mut_slice();
    let a = JsFuture::from(WebAssembly::instantiate_buffer(wasm, &Object::new())).await?;
    let ins: WebAssembly::Instance = Reflect::get(&a, &"instance".into())?.dyn_into()?;
    Ok(ins)
}

pub async fn start_event_loop(instance: WebAssembly::Instance) -> Result<(), Error> {
    let exports = instance.exports();
    postMessage(&format!("exports: {:?}", exports), "http://localhost:8000");
    Ok(())
}

#[wasm_bindgen(start)]
pub async fn main() {
    let instance = load_game_internal("facade-program-addr").await.unwrap();
    start_event_loop(instance).await.unwrap();
}

// #[wasm_bindgen(start)]
// pub async fn main() {
//     let (privkey, pubkey) = gen_rsa().expect("Rsa gen failed");
//     let message = b"hello world";
//     let encoded = encrypt(&pubkey, message).expect("");
//     let decoded = decrypt(&privkey, encoded.as_ref()).expect("");
//     assert_eq!(decoded, message);
//     let pubkey_export = export_rsa_pubkey(&pubkey).expect("");
//     log(format!("pubkey: {:?}", pubkey_export).as_str());

//     let mut cipher1 = gen_chacha20();
//     let mut cipher2 = gen_chacha20();

//     let mut buffer = message.clone();
//     apply(&mut cipher1, &mut buffer);
//     apply(&mut cipher2, &mut buffer);
//     apply(&mut cipher2, &mut buffer);
//     apply(&mut cipher1, &mut buffer);

//     assert_eq!(&buffer, message);

//     log(format!("chacha20 decoded: {:?}", buffer).as_str());
// }
