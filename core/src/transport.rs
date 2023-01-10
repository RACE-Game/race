use crate::{
    error::Result,
    types::{
        CloseGameAccountParams, CreateGameAccountParams, CreateRegistrationParams, GameAccount,
        GameBundle, GetRegistrationParams, PlayerProfile, RegisterGameParams,
        RegisterTransactorParams, RegistrationAccount, SettleParams, TransactorAccount,
        UnregisterGameParams, JoinParams,
    },
};
use async_trait::async_trait;
use std::marker::Send;

#[async_trait]
pub trait TransportT: Send + Sync {
    async fn create_game_account(&self, params: CreateGameAccountParams) -> Result<String>;

    async fn close_game_account(&self, params: CloseGameAccountParams) -> Result<()>;

    async fn register_transactor(&self, params: RegisterTransactorParams) -> Result<()>;

    async fn join(&self, params: JoinParams) -> Result<()>;

    async fn get_game_account(&self, addr: &str) -> Option<GameAccount>;

    async fn get_game_bundle(&self, addr: &str) -> Option<GameBundle>;

    async fn get_player_profile(&self, addr: &str) -> Option<PlayerProfile>;

    async fn get_transactor_account(&self, addr: &str) -> Option<TransactorAccount>;

    async fn get_registration(&self, params: GetRegistrationParams) -> Option<RegistrationAccount>;

    async fn publish_game(&self, bundle: GameBundle) -> Result<String>;

    async fn settle_game(&self, params: SettleParams) -> Result<()>;

    async fn create_registration(&self, params: CreateRegistrationParams) -> Result<String>;

    async fn register_game(&self, params: RegisterGameParams) -> Result<()>;

    async fn unregister_game(&self, params: UnregisterGameParams) -> Result<()>;
}
