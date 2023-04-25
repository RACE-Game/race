#![allow(unused_variables)]
use async_trait::async_trait;
use borsh::BorshDeserialize;
use gloo::utils::format::JsValueSerdeExt;
use js_sys::Uint8Array;
use race_core::types::{
    CloseGameAccountParams, CreateGameAccountParams, CreatePlayerProfileParams,
    CreateRegistrationParams, DepositParams, GameAccount, GameBundle, JoinParams, PlayerProfile,
    PublishGameParams, RegisterGameParams, RegistrationAccount, ServerAccount,
    UnregisterGameParams, VoteParams,
};
use race_transport::{
    error::{TransportError, TransportResult},
    TransportLocalT,
};
use wasm_bindgen::{JsCast, JsValue};

use crate::utils::{get_function, resolve_promise};

pub struct Transport {
    inner: JsValue,
}

impl Transport {
    pub fn new(inner: JsValue) -> Self {
        Self { inner }
    }
}

fn parse_js_value<T: BorshDeserialize>(value: &JsValue) -> T {
    let f = get_function(value, "serialize");


    let r = match f.call0(value) {
        Ok(r) => r,
        Err(e) => {
            gloo::console::error!("Failed to serialize object:", value, e);
            panic!("Parse response failed");
        }
    };

    let r = match r.dyn_into::<Uint8Array>() {
        Ok(r) => r,
        Err(e) => {
            gloo::console::error!("Failed to parse object to Uint8Array:", e);
            panic!("Parse response failed");
        }
    };

    match T::try_from_slice(&r.to_vec()){
        Ok(r) => r,
        Err(e) => {
            gloo::console::error!("Failed to deserialize:", r);
            gloo::console::error!(format!("Error: {:?}", e));
            panic!("Parse response failed");
        }
    }
}

#[async_trait(?Send)]
impl TransportLocalT for Transport {
    async fn create_game_account(
        &self,
        wallet: &JsValue,
        params: CreateGameAccountParams,
    ) -> TransportResult<String> {
        let f = get_function(&self.inner, "createGameAccount");
        let params = JsValue::from_serde(&params).unwrap();
        let p = f.call2(&self.inner, &wallet, &params).unwrap();
        resolve_promise(p)
            .await
            .unwrap()
            .as_string()
            .ok_or(TransportError::TransactionNotConfirmed)
    }

    async fn close_game_account(
        &self,
        wallet: &JsValue,
        params: CloseGameAccountParams,
    ) -> TransportResult<()> {
        let f = get_function(&self.inner, "closeGameAccount");
        let params = JsValue::from_serde(&params).unwrap();
        let p = f.call2(&self.inner, &wallet, &params).unwrap();
        resolve_promise(p).await.unwrap();
        Ok(())
    }

    async fn create_player_profile(
        &self,
        wallet: &JsValue,
        params: CreatePlayerProfileParams,
    ) -> TransportResult<String> {
        let f = get_function(&self.inner, "createPlayerProfile");
        let params = JsValue::from_serde(&params).unwrap();
        let p = f.call2(&self.inner, &wallet, &params).unwrap();
        resolve_promise(p)
            .await
            .unwrap()
            .as_string()
            .ok_or(TransportError::TransactionNotConfirmed)
    }

    async fn join(&self, wallet: &JsValue, params: JoinParams) -> TransportResult<()> {
        let f = get_function(&self.inner, "join");
        let params = JsValue::from_serde(&params).unwrap();
        gloo::console::info!(wallet);
        let p = f.call2(&self.inner, &wallet, &params).unwrap();
        resolve_promise(p).await.unwrap();
        Ok(())
    }

    async fn deposit(&self, wallet: &JsValue, params: DepositParams) -> TransportResult<()> {
        let f = get_function(&self.inner, "deposit");
        let params = JsValue::from_serde(&params).unwrap();
        let p = f.call2(&self.inner, &wallet, &params).unwrap();
        resolve_promise(p).await.unwrap();
        Ok(())
    }

    async fn vote(&self, wallet: &JsValue, params: VoteParams) -> TransportResult<()> {
        let f = get_function(&self.inner, "vote");
        let params = JsValue::from_serde(&params).unwrap();
        let p = f.call2(&self.inner, &wallet, &params).unwrap();
        resolve_promise(p).await.unwrap();
        Ok(())
    }

    async fn publish_game(
        &self,
        wallet: &JsValue,
        params: PublishGameParams,
    ) -> TransportResult<String> {
        let f = get_function(&self.inner, "publishGame");
        let params = JsValue::from_serde(&params).unwrap();
        let p = f.call2(&self.inner, &wallet, &params).unwrap();
        resolve_promise(p)
            .await
            .unwrap()
            .as_string()
            .ok_or(TransportError::TransactionNotConfirmed)
    }

    async fn create_registration(
        &self,
        wallet: &JsValue,
        params: CreateRegistrationParams,
    ) -> TransportResult<String> {
        let f = get_function(&self.inner, "createRegistration");
        let params = JsValue::from_serde(&params).unwrap();
        let p = f.call2(&self.inner, &wallet, &params).unwrap();
        resolve_promise(p)
            .await
            .unwrap()
            .as_string()
            .ok_or(TransportError::TransactionNotConfirmed)
    }

    async fn register_game(
        &self,
        wallet: &JsValue,
        params: RegisterGameParams,
    ) -> TransportResult<()> {
        Ok(())
    }

    async fn unregister_game(
        &self,
        wallet: &JsValue,
        _params: UnregisterGameParams,
    ) -> TransportResult<()> {
        Ok(())
    }

    async fn get_player_profile(&self, addr: &str) -> Option<PlayerProfile> {
        let f = get_function(&self.inner, "getPlayerProfile");
        let p = f.call1(&self.inner, &addr.into()).unwrap();
        let value = resolve_promise(p).await.unwrap();
        Some(parse_js_value(&value))
    }

    async fn get_game_account(&self, addr: &str) -> Option<GameAccount> {
        let f = get_function(&self.inner, "getGameAccount");
        let p = f.call1(&self.inner, &addr.into()).unwrap();
        let value = resolve_promise(p).await.unwrap();
        Some(parse_js_value(&value))
    }

    async fn get_game_bundle(&self, addr: &str) -> Option<GameBundle> {
        let f = get_function(&self.inner, "getGameBundle");
        let p = f.call1(&self.inner, &addr.into()).unwrap();
        let value = resolve_promise(p).await.unwrap();
        Some(parse_js_value(&value))
    }

    async fn get_server_account(&self, addr: &str) -> Option<ServerAccount> {
        let f = get_function(&self.inner, "getServerAccount");
        let p = f.call1(&self.inner, &addr.into()).unwrap();
        let value = resolve_promise(p).await.unwrap();
        Some(parse_js_value(&value))
    }

    async fn get_registration(&self, addr: &str) -> Option<RegistrationAccount> {
        let f = get_function(&self.inner, "getRegistration");
        let p = f.call1(&self.inner, &addr.into()).unwrap();
        let value = resolve_promise(p).await.unwrap();
        Some(parse_js_value(&value))
    }
}
