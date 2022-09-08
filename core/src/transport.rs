use crate::{
    error::{Error, Result},
    types::{GameAccount, GameBundle, Settle, SettleParams, CreateGameAccountParams},
};
use std::marker::Send;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait TransportT: Send + Sync {
    async fn create_game_account(&self, params: CreateGameAccountParams) -> Result<String>;

    async fn get_game_account(&self, addr: &str) -> Option<GameAccount>;

    async fn get_game_bundle(&self, addr: &str) -> Option<GameBundle>;

    async fn publish_game(&self, bundle: GameBundle) -> Result<String>;

    async fn settle_game(&self, params: SettleParams) -> Result<()>;
}
