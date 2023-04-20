#![cfg(target_arch = "wasm32")]
use crate::error::TransportResult;
use async_trait::async_trait;
use race_core::types::PublishGameParams;
#[allow(unused_imports)]
use race_core::types::{
    CloseGameAccountParams, CreateGameAccountParams, CreatePlayerProfileParams,
    CreateRegistrationParams, DepositParams, GameAccount, GameBundle, JoinParams, PlayerProfile,
    RegisterGameParams, RegisterServerParams, RegistrationAccount, ServeParams, ServerAccount,
    SettleParams, UnregisterGameParams, VoteParams,
};
use wasm_bindgen::JsValue;

/// The Transport API for client side.
///
/// This trait should be used in WASM context, in SDK.
/// Check [`TransportT`] for more information.
///
/// All transaction methods require a wallet instance.
#[async_trait(?Send)]
pub trait TransportLocalT {
    async fn create_game_account(
        &self,
        wallet: &JsValue,
        params: CreateGameAccountParams,
    ) -> TransportResult<String>;

    async fn close_game_account(
        &self,
        wallet: &JsValue,
        params: CloseGameAccountParams,
    ) -> TransportResult<()>;

    async fn join(&self, wallet: &JsValue, params: JoinParams) -> TransportResult<()>;

    async fn deposit(&self, wallet: &JsValue, params: DepositParams) -> TransportResult<()>;

    async fn vote(&self, wallet: &JsValue, params: VoteParams) -> TransportResult<()>;

    async fn create_player_profile(
        &self,
        wallet: &JsValue,
        params: CreatePlayerProfileParams,
    ) -> TransportResult<String>;

    async fn publish_game(
        &self,
        wallet: &JsValue,
        params: PublishGameParams,
    ) -> TransportResult<String>;

    async fn create_registration(
        &self,
        wallet: &JsValue,
        params: CreateRegistrationParams,
    ) -> TransportResult<String>;

    async fn register_game(
        &self,
        wallet: &JsValue,
        params: RegisterGameParams,
    ) -> TransportResult<()>;

    async fn unregister_game(
        &self,
        wallet: &JsValue,
        params: UnregisterGameParams,
    ) -> TransportResult<()>;

    async fn get_game_account(&self, addr: &str) -> Option<GameAccount>;

    async fn get_game_bundle(&self, addr: &str) -> Option<GameBundle>;

    async fn get_player_profile(&self, addr: &str) -> Option<PlayerProfile>;

    async fn get_server_account(&self, addr: &str) -> Option<ServerAccount>;

    async fn get_registration(&self, addr: &str) -> Option<RegistrationAccount>;
}
