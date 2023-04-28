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
use serde::Serialize;
use wasm_bindgen::{JsCast, JsValue};

use crate::{
    error::SdkError,
    utils::{get_function, resolve_promise},
};

pub struct Transport {
    inner: JsValue,
}

impl Transport {
    pub fn new(inner: JsValue) -> Self {
        Self { inner }
    }
}

fn parse_params<T: Serialize>(params: &T) -> TransportResult<JsValue> {
    JsValue::from_serde(params).map_err(|e| {
        TransportError::InvalidParameter(
            "Failed to serialize parameters for transport invocation".into(),
        )
    })
}

fn parse_js_value<T: BorshDeserialize>(value: &JsValue) -> TransportResult<Option<T>> {
    if value.is_undefined() {
        return Ok(None);
    }
    let f = get_function(value, "serialize")?;

    let r = match f.call0(value) {
        Ok(r) => r,
        Err(e) => {
            gloo::console::error!("Failed to serialize object:", value, e);
            return Err(TransportError::InteropError);
        }
    };

    let r = match r.dyn_into::<Uint8Array>() {
        Ok(r) => r,
        Err(e) => {
            gloo::console::error!("Failed to parse object to Uint8Array:", e);
            return Err(TransportError::InteropError);
        }
    };

    match T::try_from_slice(&r.to_vec()) {
        Ok(r) => Ok(Some(r)),
        Err(e) => {
            gloo::console::error!("Failed to deserialize:", r);
            gloo::console::error!(format!("Error: {:?}", e));
            return Err(TransportError::InteropError);
        }
    }
}

impl Transport {
    async fn api_fetch<T: BorshDeserialize>(
        &self,
        api: &str,
        addr: &str,
    ) -> TransportResult<Option<T>> {
        let f = get_function(&self.inner, api)?;
        let p = f.call1(&self.inner, &addr.into()).map_err(|e| {
            SdkError::InteropError(format!("An error occurred in API fetch: {}", api))
        })?;
        let value = resolve_promise(p).await?;
        Ok(parse_js_value(&value)?)
    }

    async fn api_call<T: Serialize>(
        &self,
        api: &str,
        wallet: &JsValue,
        params: T,
    ) -> TransportResult<JsValue> {
        let f = get_function(&self.inner, api)?;
        let params = parse_params(&params)?;
        let p: JsValue = f.call2(&self.inner, &wallet, &params).map_err(|e| {
            SdkError::InteropError(format!("An error occurred in API call: {}", api))
        })?;
        Ok(resolve_promise(p).await?)
    }
}

#[async_trait(?Send)]
impl TransportLocalT for Transport {
    async fn create_game_account(
        &self,
        wallet: &JsValue,
        params: CreateGameAccountParams,
    ) -> TransportResult<String> {
        self.api_call("createGameAccount", wallet, &params)
            .await?
            .as_string()
            .ok_or(TransportError::ParseAddressError)
    }

    async fn close_game_account(
        &self,
        wallet: &JsValue,
        params: CloseGameAccountParams,
    ) -> TransportResult<()> {
        self.api_call("closeGameAccount", wallet, &params).await?;
        Ok(())
    }

    async fn create_player_profile(
        &self,
        wallet: &JsValue,
        params: CreatePlayerProfileParams,
    ) -> TransportResult<()> {
        self.api_call("createPlayerProfile", wallet, &params)
            .await?;
        Ok(())
    }

    async fn join(&self, wallet: &JsValue, params: JoinParams) -> TransportResult<()> {
        self.api_call("join", wallet, &params).await?;
        Ok(())
    }

    async fn deposit(&self, wallet: &JsValue, params: DepositParams) -> TransportResult<()> {
        self.api_call("deposit", wallet, &params).await?;
        Ok(())
    }

    async fn vote(&self, wallet: &JsValue, params: VoteParams) -> TransportResult<()> {
        self.api_call("vote", wallet, &params).await?;
        Ok(())
    }

    async fn publish_game(
        &self,
        wallet: &JsValue,
        params: PublishGameParams,
    ) -> TransportResult<String> {
        self.api_call("publishGame", wallet, &params)
            .await?
            .as_string()
            .ok_or(TransportError::ParseAddressError)
    }

    async fn create_registration(
        &self,
        wallet: &JsValue,
        params: CreateRegistrationParams,
    ) -> TransportResult<String> {
        self.api_call("createRegistration", wallet, &params)
            .await?
            .as_string()
            .ok_or(TransportError::ParseAddressError)
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

    async fn get_player_profile(&self, addr: &str) -> TransportResult<Option<PlayerProfile>> {
        self.api_fetch("getPlayerProfile", addr).await
    }

    async fn get_game_account(&self, addr: &str) -> TransportResult<Option<GameAccount>> {
        self.api_fetch("getGameAccount", addr).await
    }

    async fn get_game_bundle(&self, addr: &str) -> TransportResult<Option<GameBundle>> {
        self.api_fetch("getGameBundle", addr).await
    }

    async fn get_server_account(&self, addr: &str) -> TransportResult<Option<ServerAccount>> {
        self.api_fetch("getServerAccount", addr).await
    }

    async fn get_registration(&self, addr: &str) -> TransportResult<Option<RegistrationAccount>> {
        self.api_fetch("getRegistration", addr).await
    }
}
