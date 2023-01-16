#![cfg(target_arch = "wasm32")]

use std::sync::Arc;

use crate::app_client::AppClient;
use race_core::{types::CreateGameAccountParams, transport::TransportT};
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
            Some(ref t) => t
        }
    }

    #[wasm_bindgen]
    pub async fn get_game_account(&self, game_addr: &str) {
        let game_account = self.get_transport_unchecked()
            .get_game_account(game_addr)
            .await;
        console_info!("Game account: {:?}", game_account);
    }
}

#[cfg(test)]
mod tests {

    use wasm_bindgen_test::*;
    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_helper() {
        let mut app_helper = AppHelper::new("facade", "ws://localhost:12002");
        app_helper.initialize().await;
        app_helper.get_game_account("COUNTER_GAME_ADDRESS").await;
    }
    #[wasm_bindgen_test]
    async fn test_init_client() {
        let mut client = WasmAppClient::new("facade", "ws://localhost:12002", "COUNTER_GAME_ADDRESS");
        client.initialize().await;
    }
}
