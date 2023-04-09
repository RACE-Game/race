#![cfg(target_arch = "wasm32")]
use crate::error::TransportResult;
#[allow(unused_imports)]
use race_core::types::{
    CloseGameAccountParams, CreateGameAccountParams, CreatePlayerProfileParams,
    CreateRegistrationParams, DepositParams, GameAccount, GameBundle, JoinParams, PlayerProfile,
    RegisterGameParams, RegisterServerParams, RegistrationAccount, ServeParams, ServerAccount,
    SettleParams, UnregisterGameParams, VoteParams,
};
use async_trait::async_trait;

/// The Transport API for client side.
///
/// This trait should be used in WASM context, in SDK.
/// Check [`TransportT`] for more information.
#[async_trait(?Send)]
pub trait TransportLocalT {
    async fn create_game_account(&self, params: CreateGameAccountParams)
        -> TransportResult<String>;

    async fn close_game_account(&self, params: CloseGameAccountParams) -> TransportResult<()>;

    async fn join(&self, params: JoinParams) -> TransportResult<()>;

    async fn deposit(&self, params: DepositParams) -> TransportResult<()>;

    async fn vote(&self, params: VoteParams) -> TransportResult<()>;

    async fn create_player_profile(
        &self,
        params: CreatePlayerProfileParams,
    ) -> TransportResult<String>;

    async fn publish_game(&self, bundle: GameBundle) -> TransportResult<String>;

    async fn create_registration(
        &self,
        params: CreateRegistrationParams,
    ) -> TransportResult<String>;

    async fn register_game(&self, params: RegisterGameParams) -> TransportResult<()>;

    async fn unregister_game(&self, params: UnregisterGameParams) -> TransportResult<()>;

    async fn get_game_account(&self, addr: &str) -> Option<GameAccount>;

    async fn get_game_bundle(&self, addr: &str) -> Option<GameBundle>;

    async fn get_player_profile(&self, addr: &str) -> Option<PlayerProfile>;

    async fn get_server_account(&self, addr: &str) -> Option<ServerAccount>;

    async fn get_registration(&self, addr: &str) -> Option<RegistrationAccount>;
}
