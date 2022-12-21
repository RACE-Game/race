use async_trait::async_trait;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::rpc_params;

use jsonrpsee::http_client::{HttpClient as Client, HttpClientBuilder as ClientBuilder};

use race_core::error::{Error, Result};
use race_core::transport::TransportT;
use race_core::types::{
    CloseGameAccountParams, CreateGameAccountParams, GameAccount, GameBundle, GetAccountInfoParams,
    GetGameBundleParams, JoinParams, SettleParams, PlayerProfile, UnregisterTransactorParams, RegisterTransactorParams,
};

pub struct FacadeTransport {
    client: Client,
}

impl FacadeTransport {
    pub fn new(url: &str) -> Self {
        Self {
            client: ClientBuilder::default().build("http://localhost:12002").unwrap(),
        }
    }
}

#[async_trait]
impl TransportT for FacadeTransport {
    async fn create_game_account(&self, params: CreateGameAccountParams) -> Result<String> {
        self.client
            .request("create_game", rpc_params![params])
            .await
            .map_err(|e| Error::RpcError(e.to_string()))
    }

    async fn close_game_account(&self, params: CloseGameAccountParams) -> Result<()> {
        self.client
            .request("close_game", rpc_params![params])
            .await
            .map_err(|e| Error::RpcError(e.to_string()))
    }

    async fn get_game_account(&self, addr: &str) -> Option<GameAccount> {
        let params = GetAccountInfoParams { addr: addr.into() };
        self.client.request("get_account_info", rpc_params![params]).await.ok()
    }

    async fn get_game_bundle(&self, addr: &str) -> Option<GameBundle> {
        let params = GetGameBundleParams { addr: addr.into() };
        self.client.request("get_game_bundle", rpc_params![params]).await.ok()
    }

    async fn get_player_profile(&self, addr: &str) -> Option<PlayerProfile> {
        None
    }

    async fn join(&self, params: JoinParams) -> Result<()> {
        self.client
            .request("join", rpc_params![params])
            .await
            .map_err(|e| Error::RpcError(e.to_string()))
    }

    async fn publish_game(&self, bundle: GameBundle) -> Result<String> {
        self.client
            .request("publish_game_bundle", rpc_params![bundle])
            .await
            .map_err(|e| Error::RpcError(e.to_string()))
    }

    async fn settle_game(&self, params: SettleParams) -> Result<()> {
        self.client
            .request("settle", rpc_params![params])
            .await
            .map_err(|e| Error::RpcError(e.to_string()))
    }

    async fn register_transactor(&self, params: RegisterTransactorParams) -> Result<()> {
        Ok(())
    }

    async fn unregister_transactor(&self, params: UnregisterTransactorParams) -> Result<()> {
        Ok(())
    }
}
