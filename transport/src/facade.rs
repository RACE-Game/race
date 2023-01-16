use async_trait::async_trait;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::rpc_params;

#[cfg(target_arch = "wasm32")]
use jsonrpsee::core::client::Client;
#[cfg(not(target_arch = "wasm32"))]
use jsonrpsee::http_client::{HttpClient as Client, HttpClientBuilder as ClientBuilder};
#[cfg(target_arch = "wasm32")]
use jsonrpsee::wasm_client::WasmClientBuilder as ClientBuilder;

use race_core::error::{Result, Error};
use race_core::transport::TransportT;
use race_core::types::{
    CloseGameAccountParams, CreateGameAccountParams, CreateRegistrationParams, GameAccount,
    GameBundle, GetAccountInfoParams, GetGameBundleParams, GetRegistrationParams, JoinParams,
    PlayerProfile, RegisterGameParams, RegisterTransactorParams, RegistrationAccount, SettleParams,
    TransactorAccount, UnregisterGameParams,
};

use crate::error::{TransportResult, TransportError};

pub struct FacadeTransport {
    client: Client,
}

impl FacadeTransport {
    pub async fn try_new(url: &str) -> TransportResult<Self> {
        #[cfg(not(target_arch = "wasm32"))]
        let client = ClientBuilder::default()
            .build(url)
            .map_err(|e| TransportError::InitializationFailed(e.to_string()))?;
        #[cfg(target_arch = "wasm32")]
        let client = ClientBuilder::default()
            .build(url)
            .await
            .map_err(|e| TransportError::InitializationFailed(e.to_string()))?;
        Ok(Self { client })
    }
}

#[async_trait]
#[allow(unused_variables)]
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
        let rs = self.client
            .request("get_account_info", rpc_params![params])
            .await;
        if let Ok(rs) = rs {
            Some(rs)
        } else {
            None
        }
    }

    async fn get_game_bundle(&self, addr: &str) -> Option<GameBundle> {
        let params = GetGameBundleParams { addr: addr.into() };
        self.client
            .request("get_game_bundle", rpc_params![params])
            .await
            .ok()
    }

    async fn get_transactor_account(&self, addr: &str) -> Option<TransactorAccount> {
        todo!()
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
        self.client
            .request("register_transactor", rpc_params![params])
            .await
            .map_err(|e| Error::RpcError(e.to_string()))
    }

    async fn create_registration(&self, params: CreateRegistrationParams) -> Result<String> {
        self.client
            .request("create_registration", rpc_params![params])
            .await
            .map_err(|e| Error::RpcError(e.to_string()))
    }

    async fn register_game(&self, params: RegisterGameParams) -> Result<()> {
        self.client
            .request("register_game", rpc_params![params])
            .await
            .map_err(|e| Error::RpcError(e.to_string()))
    }

    async fn unregister_game(&self, params: UnregisterGameParams) -> Result<()> {
        self.client
            .request("unregister_game", rpc_params![params])
            .await
            .map_err(|e| Error::RpcError(e.to_string()))
    }

    async fn get_registration(&self, params: GetRegistrationParams) -> Option<RegistrationAccount> {
        println!("Get registration: {:?}", params);
        self.client
            .request("get_registration_info", rpc_params![params])
            .await
            .map_err(|e| {
                println!("error: {:?}", e);
                e
            })
            .ok()
    }
}
