#![cfg(target_arch = "wasm32")]

use borsh::{BorshDeserialize, BorshSerialize};
use js_sys::WebAssembly::{Instance, Memory};
use js_sys::{Function, Object, Reflect, Uint8Array, WebAssembly, JSON};
use jsonrpsee::wasm_client::WasmClientBuilder;
use jsonrpsee::{core::client::ClientT, rpc_params};
use race_core::context::{DispatchEvent, GameContext, SecretContext};
use race_core::event::Event;
use race_core::types::{GameBundle, GetGameBundleParams};
use race_crypto::secret::{apply, decrypt, encrypt, export_rsa_pubkey, gen_chacha20, gen_rsa};

use wasm_bindgen::convert::IntoWasmAbi;
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

impl From<Error> for JsValue {
    fn from(e: Error) -> Self {
        JsError::new(&format!("{:?}", e)).into()
    }
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

#[wasm_bindgen]
pub struct RaceClient {
    addr: String,
    instance: Instance,
    context: GameContext,
}

#[wasm_bindgen]
impl RaceClient {
    #[wasm_bindgen]
    pub async fn init(addr: &str) -> Result<RaceClient, Error> {
        let context = GameContext::default();
        let url = "ws://localhost:12002";
        let client = WasmClientBuilder::default().build(url);
        let client = client.await;
        let client = client.unwrap();
        let params = rpc_params![GetGameBundleParams { addr: addr.into() }];
        let mut bundle: GameBundle = client.request("get_game_bundle", params).await?;
        console_log!("Bundle size: {:?}", bundle.data.len());
        let wasm = bundle.data.as_mut_slice();
        let mem_descriptor = Object::new();
        Reflect::set(&mem_descriptor, &"shared".into(), &true.into()).unwrap();
        // 64k * 1000, TODO, use fewer memory
        Reflect::set(&mem_descriptor, &"maximum".into(), &1000.into()).unwrap();
        Reflect::set(&mem_descriptor, &"initial".into(), &1000.into()).unwrap();
        console_log!("Initializing linear memory...");
        let memory = WebAssembly::Memory::new(&mem_descriptor).unwrap();
        console_log!("Linear memory created");
        let import_obj = Object::new();
        Reflect::set(&import_obj, &"memory".into(), &memory).unwrap();
        let a = JsFuture::from(WebAssembly::instantiate_buffer(wasm, &import_obj)).await?;
        let instance: WebAssembly::Instance = Reflect::get(&a, &"instance".into())?.dyn_into()?;
        console_log!("Game bundle loaded");
        Ok(RaceClient {
            addr: addr.into(),
            instance,
            context,
        })
    }

    #[wasm_bindgen]
    pub fn get_addr(&self) -> String {
        self.addr.to_owned()
    }

    #[wasm_bindgen]
    pub fn get_json_state(&self) -> Option<String> {
        console_log!("state: {:?}", self.context.state_json);
        self.context.state_json.clone()
    }

    #[wasm_bindgen]
    pub async fn dispatch_json_event(&mut self, event: &str) {
        let event: Event = serde_json::from_str(event).unwrap();
        self.dispatch_event(event).await;
    }

    #[wasm_bindgen]
    pub async fn dispatch_json_custom_event(&mut self, custom_event: &str) {
        self.dispatch_event(Event::Custom(custom_event.into())).await;
    }

    #[wasm_bindgen]
    pub async fn dispatch_custom_event(&mut self, custom_event: JsValue) {
        let custom: String = JSON::stringify(&custom_event).unwrap().into();
        self.dispatch_event(Event::Custom(custom)).await;
    }

    #[wasm_bindgen]
    pub async fn dispatch_raw_event(&mut self, event: Uint8Array) {
        let event = Event::try_from_slice(&event.to_vec()).unwrap();
        self.dispatch_event(event).await;
    }

    async fn dispatch_event(&mut self, event: Event) {
        console_log!("Dispatch event: {:?}", event);
        let exports = self.instance.exports();
        let mem = Reflect::get(exports.as_ref(), &"memory".into())
            .unwrap()
            .dyn_into::<Memory>()
            .expect("Can't get memory");
        let buf = Uint8Array::new(&mem.buffer());
        let context_vec = self.context.try_to_vec().unwrap();
        let context_size = context_vec.len();
        let context_arr = Uint8Array::new_with_length(context_size as _);
        context_arr.copy_from(&context_vec);
        console_log!("context size: {:?}", context_size);
        let event_vec = event.try_to_vec().unwrap();
        let event_size = event_vec.len();
        let event_arr = Uint8Array::new_with_length(event_size as _);
        console_log!("event size: {:?}", event_size);
        event_arr.copy_from(&event_vec);
        let mut offset = 1u32;
        buf.set(&context_arr, offset);
        offset += context_size as u32;
        buf.set(&event_arr, offset);
        let handle_event = Reflect::get(exports.as_ref(), &"handle_event".into())
            .unwrap()
            .dyn_into::<Function>()
            .expect("Can't get handle_event");
        let new_context_size = handle_event
            .call2(&JsValue::undefined(), &context_size.into(), &event_size.into())
            .unwrap()
            .as_f64()
            .unwrap() as usize;
        console_log!("new context size: {:?}", new_context_size);
        let new_context_vec = Uint8Array::new(&mem.buffer()).to_vec();
        let new_context_slice = &new_context_vec[1..(1 + new_context_size)];
        self.context = GameContext::try_from_slice(&new_context_slice).unwrap();
    }
}

// pub async fn start_event_loop(instance: WebAssembly::Instance) -> Result<(), Error> {
//     // let exports = instance.exports();
//     // postMessage(&format!("exports: {:?}", exports), "http://localhost:8000");
//     Ok(())
// }

// #[wasm_bindgen]
// pub async fn start(addr: &str) {
//     console_log!("Fetch game bundle by address: {:?}", addr);
//     let mut hdlr = WrappedHandler::load_by_addr(addr).await.unwrap();
//     let mut context = GameContext::default();
//     let event = Event::Join {
//         player_addr: "FAKE PLAYER ADDR".into(),
//         timestamp: 0,
//     };
//     console_log!("event: {:?}", event);
//     hdlr.handle_event(&mut context, event).await;
//     console_log!("dispatch: {:?}", context.dispatch);
// }

#[wasm_bindgen(start)]
pub async fn __main() {
    console_log!("WASM module loaded.");
}
