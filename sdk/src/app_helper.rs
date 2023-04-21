//! A common client to use in dapp(native version).

use gloo::console::info;
use gloo::{console::warn, utils::format::JsValueSerdeExt};
use js_sys::Array;
use js_sys::{Object, Uint8Array};
use race_core::types::CreatePlayerProfileParams;
use race_transport::TransportLocalT;
use wasm_bindgen::prelude::*;

use crate::error::Result;
use crate::js::{rget, rget_string, rget_u64, rget_u8, rget_u16};
use crate::transport::Transport;
use race_core::{
    error::Error,
    types::{CreateGameAccountParams, RegisterGameParams},
};

#[wasm_bindgen]
pub struct AppHelper {
    transport: Transport,
}

#[wasm_bindgen]
impl AppHelper {
    /// Try initialize an app helper which provides out game functionalities.
    ///
    /// # Arguments
    /// * `chain`, The name of blockchain, currently only `facade` is supported.
    /// * `rpc`, The endpoint of blockchain RPC.
    #[wasm_bindgen]
    pub async fn try_init(transport: JsValue) -> Result<AppHelper> {
        let transport = Transport::new(transport);
        Ok(AppHelper { transport })
    }

    #[wasm_bindgen]
    pub async fn get_game_account(&self, game_addr: &str) -> Result<JsValue> {
        let game_account = self.transport.get_game_account(game_addr).await;
        Ok(JsValue::from_serde(&game_account).or(Err(Error::JsonParseError))?)
    }

    #[wasm_bindgen]
    pub async fn create_game_account(&self, wallet: &JsValue, opts: &Object) -> Result<String> {
        let title = rget_string(opts, "title")?;
        let token_addr = rget_string(opts, "token_addr")?;
        let bundle_addr = rget_string(opts, "bundle_addr")?;
        let max_players = rget_u16(opts, "max_players")?;
        let data: Uint8Array = rget(opts, "data")?;
        let min_deposit = rget_u64(opts, "min_deposit")?;
        let max_deposit = rget_u64(opts, "max_deposit")?;
        let addr = self
            .transport
            .create_game_account(
                wallet,
                CreateGameAccountParams {
                    title,
                    bundle_addr,
                    token_addr,
                    max_players,
                    data: data.to_vec(),
                    max_deposit,
                    min_deposit,
                },
            )
            .await?;
        Ok(addr)
    }

    #[wasm_bindgen]
    pub async fn register_game(
        &self,
        wallet: &JsValue,
        game_addr: &str,
        reg_addr: &str,
    ) -> Result<()> {
        self.transport
            .register_game(
                wallet,
                RegisterGameParams {
                    game_addr: game_addr.to_owned(),
                    reg_addr: reg_addr.to_owned(),
                },
            )
            .await?;
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn create_profile(&self, wallet: JsValue, nick: &str, pfp: &str) -> Result<String> {
        let pfp = if pfp.eq("") {
            None
        } else {
            Some(pfp.to_owned())
        };
        info!(format!("Create profile, nick: {}, pfp: {:?}", nick, pfp));
        let addr = self
            .transport
            .create_player_profile(
                &wallet,
                CreatePlayerProfileParams {
                    nick: nick.to_owned(),
                    pfp,
                },
            )
            .await?;
        info!(format!("Profile created at address: {}", addr));
        Ok(addr)
    }

    #[wasm_bindgen]
    pub async fn get_profile(&self, addr: &str) -> Option<JsValue> {
        if let Some(p) = self.transport.get_player_profile(addr).await {
            Some(JsValue::from_serde(&p).unwrap())
        } else {
            None
        }
    }

    #[wasm_bindgen]
    pub async fn list_games(&self, registration_addrs: Box<[JsValue]>) -> Array {
        let games = Array::new();
        for reg_addr in registration_addrs.into_iter() {
            if let Some(reg_addr) = JsValue::as_string(reg_addr) {
                info!(format!("Reg addr: {:?}", reg_addr));
                if let Some(reg) = self.transport.get_registration(&reg_addr).await {
                    info!(format!("Games: {:?}", reg));
                    for game in reg.games {
                        games.push(&JsValue::from_serde(&game).unwrap());
                    }
                } else {
                    warn!(format!("Registration account {} not found!", reg_addr));
                }
            }
        }
        games
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
