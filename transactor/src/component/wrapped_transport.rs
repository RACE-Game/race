//! Wrapped transport, which support retry

use jsonrpsee::core::async_trait;
use race_core::error::Result;
use race_core::types::{
    CreatePlayerProfileParams, CreateRegistrationParams, DepositParams, PublishGameParams,
    RegisterGameParams, ServeParams, UnregisterGameParams, VoteParams,
};
use race_core::{
    transport::TransportT,
    types::{
        CloseGameAccountParams, CreateGameAccountParams, GameAccount, GameBundle, JoinParams,
        PlayerProfile, RegisterServerParams, RegistrationAccount, ServerAccount, SettleParams,
    },
};
use race_env::Config;
use race_transport::TransportBuilder;
use std::time::Duration;
use tracing::error;

pub struct WrappedTransport {
    inner: Box<dyn TransportT>,
}

impl WrappedTransport {
    pub async fn try_new(config: &Config) -> Result<Self> {
        let chain: &str = &config
            .transactor
            .as_ref()
            .expect("Missing transactor configuration")
            .chain;
        let transport = TransportBuilder::default()
            .try_with_chain(chain)?
            .try_with_config(config)?
            .build()
            .await?;
        Ok(Self { inner: transport })
    }
}

#[async_trait]
impl TransportT for WrappedTransport {
    async fn create_game_account(&self, params: CreateGameAccountParams) -> Result<String> {
        self.inner.create_game_account(params).await
    }

    async fn create_player_profile(&self, params: CreatePlayerProfileParams) -> Result<()> {
        self.inner.create_player_profile(params).await
    }

    async fn close_game_account(&self, params: CloseGameAccountParams) -> Result<()> {
        self.inner.close_game_account(params).await
    }

    async fn join(&self, params: JoinParams) -> Result<()> {
        self.inner.join(params).await
    }

    async fn serve(&self, params: ServeParams) -> Result<()> {
        self.inner.serve(params).await
    }

    async fn vote(&self, params: VoteParams) -> Result<()> {
        self.inner.vote(params).await
    }

    async fn get_game_account(&self, addr: &str) -> Result<Option<GameAccount>> {
        self.inner.get_game_account(addr).await
    }

    async fn deposit(&self, params: DepositParams) -> Result<()> {
        self.inner.deposit(params).await
    }

    async fn get_game_bundle(&self, addr: &str) -> Result<Option<GameBundle>> {
        self.inner.get_game_bundle(addr).await
    }

    async fn get_server_account(&self, addr: &str) -> Result<Option<ServerAccount>> {
        self.inner.get_server_account(addr).await
    }

    async fn get_player_profile(&self, addr: &str) -> Result<Option<PlayerProfile>> {
        self.inner.get_player_profile(addr).await
    }

    async fn publish_game(&self, params: PublishGameParams) -> Result<String> {
        self.inner.publish_game(params).await
    }

    /// We should keep retrying until success
    async fn settle_game(&self, params: SettleParams) -> Result<()> {
        loop {
            if let Err(e) = self.inner.settle_game(params.clone()).await {
                tokio::time::sleep(Duration::from_secs(10)).await;
                error!("Error in settlement: {:?}", e);
            } else {
                return Ok(());
            }
        }
    }

    async fn register_server(&self, params: RegisterServerParams) -> Result<()> {
        self.inner.register_server(params).await
    }

    async fn get_registration(&self, addr: &str) -> Result<Option<RegistrationAccount>> {
        self.inner.get_registration(addr).await
    }

    async fn create_registration(&self, params: CreateRegistrationParams) -> Result<String> {
        self.inner.create_registration(params).await
    }

    async fn register_game(&self, params: RegisterGameParams) -> Result<()> {
        self.inner.register_game(params).await
    }

    async fn unregister_game(&self, params: UnregisterGameParams) -> Result<()> {
        self.inner.unregister_game(params).await
    }
}
