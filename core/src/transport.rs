use crate::{
    error::Result,
    types::{CreateGameAccountParams, GameAccount, GameBundle, JoinParams, SettleParams, CloseGameAccountParams, PlayerProfile},
};
use async_trait::async_trait;
use std::marker::Send;

#[async_trait]
pub trait TransportT: Send + Sync {

    async fn create_game_account(&self, params: CreateGameAccountParams) -> Result<String>;

    async fn close_game_account(&self, params: CloseGameAccountParams) -> Result<()>;

    async fn join(&self, params: JoinParams) -> Result<()>;

    async fn get_game_account(&self, addr: &str) -> Option<GameAccount>;

    async fn get_game_bundle(&self, addr: &str) -> Option<GameBundle>;

    async fn get_player_profile(&self, addr: &str) -> Option<PlayerProfile>;

    async fn publish_game(&self, bundle: GameBundle) -> Result<String>;

    async fn settle_game(&self, params: SettleParams) -> Result<()>;
}
