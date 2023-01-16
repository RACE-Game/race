#![cfg(target_arch = "wasm32")]

use std::sync::Arc;

use crate::app_client::AppClient;
use race_transport::{ChainType, TransportBuilder};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn info(s: &str);
    #[wasm_bindgen(js_namespace = console)]
    fn error(s: &str);
}

macro_rules! console_info {
    ($($t:tt)*) => (info(&format_args!($($t)*).to_string()))
}
macro_rules! console_error {
    ($($t:tt)*) => (error(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
pub struct WasmAppClient {
    chain: String,
    rpc: String,
    game_addr: String,
    app_client: Option<AppClient>,
}

#[wasm_bindgen]
impl WasmAppClient {
    #[wasm_bindgen(constructor)]
    pub fn new(chain: &str, rpc: &str, game_addr: &str) -> Self {
        Self {
            app_client: None,
            chain: chain.into(),
            rpc: rpc.into(),
            game_addr: game_addr.into(),
        }
    }

    #[wasm_bindgen]
    pub async fn initialize(&mut self) {
        let chain_type: ChainType = match self.chain.as_str().try_into() {
            Ok(ct) => ct,
            Err(e) => {
                console_error!("{:?}", e);
                return;
            }
        };
        let transport = match TransportBuilder::default()
            .with_chain(chain_type)
            .with_rpc(&self.rpc)
            .build()
            .await
        {
            Ok(x) => x,
            Err(e) => {
                console_error!("{:?}", e);
                return;
            }
        };
        let app_client = AppClient::try_new(Arc::from(transport), &self.game_addr)
            .await
            .unwrap();
        info("App client initialized");
        self.app_client = Some(app_client);
    }
}
