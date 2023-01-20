//! A common client to use in dapp(native version).

use gloo::utils::format::JsValueSerdeExt;
use js_sys::Uint8Array;
use race_transport::TransportBuilder;
use wasm_bindgen::prelude::*;

use crate::error::Result;
use race_core::{
    error::Error,
    transport::TransportT,
    types::{CreateGameAccountParams, RegisterGameParams},
};

#[wasm_bindgen]
pub struct AppHelper {
    transport: Box<dyn TransportT>,
}

#[wasm_bindgen]
impl AppHelper {
    #[wasm_bindgen]
    pub async fn try_init(chain: &str, rpc: &str) -> Result<AppHelper> {
        let transport = TransportBuilder::default()
            .try_with_chain(chain)?
            .with_rpc(rpc)
            .build()
            .await?;
        Ok(AppHelper { transport })
    }

    #[wasm_bindgen]
    pub async fn get_game_account(&self, game_addr: &str) -> Result<JsValue> {
        let game_account = self.transport.get_game_account(game_addr).await;
        Ok(JsValue::from_serde(&game_account).or(Err(Error::JsonParseError))?)
    }

    #[wasm_bindgen]
    pub async fn create_game_account(
        &self,
        bundle_addr: &str,
        max_players: u8,
        data: Uint8Array,
    ) -> Result<String> {
        let addr = self
            .transport
            .create_game_account(CreateGameAccountParams {
                bundle_addr: bundle_addr.to_owned(),
                max_players,
                data: data.to_vec(),
            })
            .await?;
        Ok(addr)
    }

    #[wasm_bindgen]
    pub async fn register_game(&self, game_addr: &str, reg_addr: &str) -> Result<()> {
        self.transport
            .register_game(RegisterGameParams {
                game_addr: game_addr.to_owned(),
                reg_addr: reg_addr.to_owned(),
            })
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use serde_json::json;
    use wasm_bindgen_test::*;
    use web_sys::console::log_1;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_helper_get_game() {
        let app_helper = AppHelper::try_init("facade", "ws://localhost:12002")
            .await
            .map_err(JsValue::from)
            .unwrap();
        let game_account = app_helper
            .get_game_account("COUNTER_GAME_ADDRESS")
            .await
            .map_err(JsValue::from)
            .unwrap();
        log_1(&game_account);
    }

    #[wasm_bindgen_test]
    async fn test_helper_create_game() {
        let app_helper = AppHelper::try_init("facade", "ws://localhost:12002")
            .await
            .map_err(JsValue::from)
            .unwrap();
        let data = Uint8Array::new_with_length(8);
        data.copy_from(&[1u8; 8]);

        let addr = app_helper
            .create_game_account("COUNTER_BUNDLE_ADDRESS".into(), 10, data)
            .await
            .map_err(JsValue::from)
            .unwrap();
        log_1(&JsValue::from_str(&addr));

        let game_account = app_helper
            .get_game_account(&addr)
            .await
            .map_err(JsValue::from)
            .unwrap();
        log_1(&game_account);

        app_helper
            .register_game(&addr, "DEFAULT_REGISTRATION_ADDRESS")
            .await
            .map_err(JsValue::from)
            .unwrap();
    }
}
