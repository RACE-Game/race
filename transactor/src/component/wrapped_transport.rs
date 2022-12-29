//! Wrapped transport, which support retry

use jsonrpsee::core::async_trait;
use race_core::{transport::TransportT, types::{GameAccount, CreateGameAccountParams, CloseGameAccountParams, GameBundle, JoinParams, PlayerProfile, SettleParams, UnregisterTransactorParams, RegisterTransactorParams, TransactorAccount}};
use race_core::error::Result;
use race_env::Config;
use race_transport::create_transport;

pub struct WrappedTransport {
    internal: Box<dyn TransportT>
}

impl WrappedTransport {
    pub fn new(config: &Config) -> Self {
        let chain: &str = &config.transactor.as_ref().expect("Missing transactor configuration").chain;
        let transport = create_transport(config, chain).expect("Failed to create transport");
        Self {
            internal: transport
        }
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

    async fn get_game_account(&self, addr: &str) -> Option<GameAccount> {
        self.internal.get_game_account(addr).await
    }

    async fn get_game_bundle(&self, addr: &str) -> Option<GameBundle> {
        self.internal.get_game_bundle(addr).await
    }

    async fn get_transactor_account(&self, addr: &str) -> Option<TransactorAccount> {
        self.internal.get_transactor_account(addr).await
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

    async fn register_transactor(&self, params: RegisterTransactorParams) -> Result<()> {
        self.internal.register_transactor(params).await
    }

    async fn unregister_transactor(&self, params: UnregisterTransactorParams) -> Result<()> {
        self.internal.unregister_transactor(params).await
    }
}
