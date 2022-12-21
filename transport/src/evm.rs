use web3::{transports::Http, Web3};

use race_core::{
    transport::TransportT,
    types::{
        CloseGameAccountParams, CreateGameAccountParams, GameAccount, GameBundle, JoinParams, PlayerProfile,
        SettleParams, RegisterTransactorParams, UnregisterTransactorParams,
    },
    error::{Error, Result}
};

use async_trait::async_trait;

pub struct EvmTransport {
    web3: Web3<Http>
}

#[async_trait]
impl TransportT for EvmTransport {
    async fn create_game_account(&self, params: CreateGameAccountParams) -> Result<String> {
        Ok("".into())
    }

    async fn close_game_account(&self, params: CloseGameAccountParams) -> Result<()> {
        todo!()
    }

    async fn join(&self, params: JoinParams) -> Result<()> {
        todo!()
    }

    async fn get_game_account(&self, addr: &str) -> Option<GameAccount> {
        todo!()
    }

    async fn get_game_bundle(&self, addr: &str) -> Option<GameBundle> {
        todo!()
    }

    async fn get_player_profile(&self, addr: &str) -> Option<PlayerProfile> {
        todo!()
    }

    async fn publish_game(&self, bundle: GameBundle) -> Result<String> {
        todo!()
    }

    async fn settle_game(&self, params: SettleParams) -> Result<()> {
        todo!()
    }

    async fn register_transactor(&self, params: RegisterTransactorParams) -> Result<()> {
        Ok(())
    }

    async fn unregister_transactor(&self, params: UnregisterTransactorParams) -> Result<()> {
        Ok(())
    }
}

impl EvmTransport {
    pub fn new(rpc: &str) -> Self {
        let transport = Http::new(rpc).unwrap();
        let web3 = Web3::new(transport);
        Self {
            web3
        }
    }
}
