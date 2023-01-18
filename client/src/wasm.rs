#![cfg(target_arch = "wasm32")]

use std::sync::Arc;

use crate::app_client::AppClient;
use js_sys::Uint8Array;
use race_core::{transport::TransportT, types::CreateGameAccountParams};
use race_transport::{ChainType, TransportBuilder};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn info(s: &str);
    #[wasm_bindgen(js_namespace = console)]
    fn error(s: &str);
    #[wasm_bindgen(js_namespace = JSON, js_name = parse)]
    fn json_parse(s: &str) -> JsValue;
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

    fn get_client_unchecked(&self) -> &AppClient {
        match self.app_client {
            None => panic!("Client is not initialized"),
            Some(ref client) => client,
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

    #[wasm_bindgen]
    pub async fn attach_game(&self) {
        self.get_client_unchecked().attach_game().await;
    }

    #[wasm_bindgen]
    pub async fn get_state(&self) -> JsValue {
        let state = self.get_client_unchecked().get_state().await;
        console_info!("State: {:?}", state);
        json_parse(&state)
    }
}

#[wasm_bindgen]
pub struct AppHelper {
    chain: String,
    rpc: String,
    transport: Option<Box<dyn TransportT>>,
}

#[wasm_bindgen]
impl AppHelper {
    #[wasm_bindgen(constructor)]
    pub fn new(chain: &str, rpc: &str) -> Self {
        Self {
            chain: chain.into(),
            rpc: rpc.into(),
            transport: None,
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

        self.transport = Some(transport);
    }

    fn get_transport_unchecked(&self) -> &Box<dyn TransportT> {
        match self.transport {
            None => panic!("Not initialized"),
            Some(ref t) => t,
        }
    }

    #[wasm_bindgen]
    pub async fn get_game_account(&self, game_addr: &str) {
        let game_account = self
            .get_transport_unchecked()
            .get_game_account(game_addr)
            .await;
        console_info!("Game account: {:?}", game_account);
    }

    #[wasm_bindgen]
    pub async fn create_game_account(
        &self,
        bundle_addr: String,
        max_players: u8,
        data: Uint8Array,
    ) -> String {
        let addr = self
            .get_transport_unchecked()
            .create_game_account(CreateGameAccountParams {
                bundle_addr,
                max_players,
                data: data.to_vec(),
            })
            .await
            .expect("Failed to create account");
        console_info!("Game account created at {:?}", addr);
        addr
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use serde_json::json;
    use wasm_bindgen_test::*;
    use web_sys::console::{log, log_1};

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_helper_get_game() {
        let mut app_helper = AppHelper::new("facade", "ws://localhost:12002");
        app_helper.initialize().await;
        app_helper.get_game_account("COUNTER_GAME_ADDRESS").await;
    }

    #[wasm_bindgen_test]
    async fn test_helper_create_game() {
        let mut app_helper = AppHelper::new("facade", "ws://localhost:12002");
        app_helper.initialize().await;
        let data = Uint8Array::new_with_length(8);
        data.copy_from(&[1u8; 8]);
        let addr = app_helper
            .create_game_account("COUNTER_BUNDLE_ADDRESS".into(), 10, data)
            .await;
        console_log!("test_helper_create_game: address {:?}", addr);
    }

    #[wasm_bindgen_test]
    async fn test_init_client() {
        let mut client =
            WasmAppClient::new("facade", "ws://localhost:12002", "COUNTER_GAME_ADDRESS");
        client.initialize().await;
        client.attach_game().await;
        let state = client.get_state().await;
        log_1(&state);
    }
}
