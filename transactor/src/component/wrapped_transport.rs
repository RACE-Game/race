//! Wrapped transport, which support retry

use jsonrpsee::core::async_trait;
use race_core::error::Result;
use race_core::types::{CreateRegistrationParams, RegisterGameParams, UnregisterGameParams, ServeParams};
use race_core::{
    transport::TransportT,
    types::{
        CloseGameAccountParams, CreateGameAccountParams, GameAccount, GameBundle,
        JoinParams, PlayerProfile, RegisterServerParams,
        RegistrationAccount, SettleParams, ServerAccount,
    },
};
use race_env::Config;
use race_transport::TransportBuilder;

pub struct WrappedTransport {
    internal: Box<dyn TransportT>,
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
        Ok(Self {
            internal: transport,
        })
    }
}

#[async_trait]
impl TransportT for WrappedTransport {
    async fn create_game_account(&self, params: CreateGameAccountParams) -> Result<String> {
        self.internal.create_game_account(params).await
    }

    async fn close_game_account(&self, params: CloseGameAccountParams) -> Result<()> {
        self.internal.close_game_account(params).await
    }

    async fn join(&self, params: JoinParams) -> Result<()> {
        self.internal.join(params).await
    }

    async fn serve(&self, params: ServeParams) -> Result<()> {
        self.internal.serve(params).await
    }

    async fn get_game_account(&self, addr: &str) -> Option<GameAccount> {
        self.internal.get_game_account(addr).await
    }

    async fn get_game_bundle(&self, addr: &str) -> Option<GameBundle> {
        self.internal.get_game_bundle(addr).await
    }

    async fn get_server_account(&self, addr: &str) -> Option<ServerAccount> {
        self.internal.get_server_account(addr).await
    }

    async fn get_player_profile(&self, addr: &str) -> Option<PlayerProfile> {
        self.internal.get_player_profile(addr).await
    }

    async fn publish_game(&self, bundle: GameBundle) -> Result<String> {
        self.internal.publish_game(bundle).await
    }

    async fn settle_game(&self, params: SettleParams) -> Result<()> {
        self.internal.settle_game(params).await
    }

    async fn register_server(&self, params: RegisterServerParams) -> Result<String> {
        self.internal.register_server(params).await
    }

    async fn get_registration(&self, addr: &str) -> Option<RegistrationAccount> {
        self.internal.get_registration(addr).await
    }

    async fn create_registration(&self, params: CreateRegistrationParams) -> Result<String> {
        self.internal.create_registration(params).await
    }

    async fn register_game(&self, params: RegisterGameParams) -> Result<()> {
        self.internal.register_game(params).await
    }

    async fn unregister_game(&self, params: UnregisterGameParams) -> Result<()> {
        self.internal.unregister_game(params).await
    }
}
