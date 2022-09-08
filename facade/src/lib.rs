use async_trait::async_trait;
use futures::TryFutureExt;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee::rpc_params;
use race_core::error::{Error, Result};
use race_core::transport::TransportT;
use race_core::types::{
    CreateGameAccountParams, GameAccount, GameBundle, GetAccountInfoParams, GetGameBundleParams, SettleParams,
};

pub struct FacadeTransport {
    client: HttpClient,
}

impl Default for FacadeTransport {
    fn default() -> Self {
        Self {
            client: HttpClientBuilder::default().build("http://localhost:12002").unwrap(),
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

    async fn get_game_account(&self, addr: &str) -> Option<GameAccount> {
        let params = GetAccountInfoParams { addr: addr.into() };
        self.client.request("get_account_info", rpc_params![params]).await.ok()
    }

    async fn get_game_bundle(&self, addr: &str) -> Option<GameBundle> {
        let params = GetGameBundleParams { addr: addr.into() };
        self.client.request("get_game_bundle", rpc_params![params]).await.ok()
    }

    async fn publish_game(&self, bundle: GameBundle) -> Result<String> {
        self.client.request("publish_game_bundle", rpc_params![bundle])
        .await
        .map_err(|e| Error::RpcError(e.to_string()))
    }

    async fn settle_game(&self, params: SettleParams) -> Result<()> {
        self.client
            .request("settle", rpc_params![params])
            .await
            .map_err(|e| Error::RpcError(e.to_string()))
    }
}
