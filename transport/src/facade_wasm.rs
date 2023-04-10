#![cfg(target_arch = "wasm32")]

use async_trait::async_trait;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::rpc_params;
use tracing::{debug, error};

use jsonrpsee::core::client::Client;
use jsonrpsee::wasm_client::WasmClientBuilder as ClientBuilder;

use race_core::types::{
    CloseGameAccountParams, CreateGameAccountParams, CreatePlayerProfileParams,
    CreateRegistrationParams, DepositParams, GameAccount, GameBundle, JoinParams, PlayerProfile,
    RegisterGameParams, RegistrationAccount, ServerAccount, UnregisterGameParams, VoteParams,
};
use wasm_bindgen::JsValue;

use crate::error::{TransportError, TransportResult};
use crate::wasm_trait::TransportLocalT;

pub struct FacadeTransport {
    client: Client,
}

impl FacadeTransport {
    pub async fn try_new(url: &str) -> TransportResult<Self> {
        let client = ClientBuilder::default()
            .build(url)
            .await
            .map_err(|e| TransportError::InitializationFailed(e.to_string()))?;
        Ok(Self { client })
    }
}

#[async_trait(?Send)]
#[allow(unused_variables)]
impl TransportLocalT for FacadeTransport {
    async fn create_game_account(
        &self,
        _wallet: &JsValue,
        params: CreateGameAccountParams,
    ) -> TransportResult<String> {
        self.client
            .request("create_game", rpc_params![params])
            .await
            .map_err(|e| TransportError::NetworkError(e.to_string()))
    }

    async fn close_game_account(
        &self,
        _wallet: &JsValue,
        params: CloseGameAccountParams,
    ) -> TransportResult<()> {
        self.client
            .request("close_game", rpc_params![params])
            .await
            .map_err(|e| TransportError::NetworkError(e.to_string()))
    }

    async fn join(&self, _wallet: &JsValue, params: JoinParams) -> TransportResult<()> {
        self.client
            .request("join", rpc_params![params])
            .await
            .map_err(|e| TransportError::NetworkError(e.to_string()))
    }

    async fn vote(&self, _wallet: &JsValue, params: VoteParams) -> TransportResult<()> {
        if let Some(game_account) = self.get_game_account(&params.game_addr).await {
            if game_account
                .votes
                .iter()
                .find(|v| v.voter.eq(&params.voter_addr))
                .is_some()
            {
                Err(TransportError::DuplicatedVote)
            } else {
                self.client
                    .request("vote", rpc_params![params])
                    .await
                    .map_err(|e| TransportError::NetworkError(e.to_string()))
            }
        } else {
            Err(TransportError::GameAccountNotFound)
        }
    }

    async fn get_game_account(&self, addr: &str) -> Option<GameAccount> {
        debug!("Fetch game account: {:?}", addr);
        let rs = self
            .client
            .request("get_account_info", rpc_params![addr])
            .await;
        if let Ok(rs) = rs {
            Some(rs)
        } else {
            None
        }
    }

    async fn get_game_bundle(&self, addr: &str) -> Option<GameBundle> {
        self.client
            .request("get_game_bundle", rpc_params![addr])
            .await
            .ok()
    }

    async fn create_player_profile(
        &self,
        _wallet: &JsValue,
        params: CreatePlayerProfileParams,
    ) -> TransportResult<String> {
        self.client
            .request("create_profile", rpc_params![params])
            .await
            .map_err(|e| TransportError::NetworkError(e.to_string()))
    }

    async fn get_player_profile(&self, addr: &str) -> Option<PlayerProfile> {
        self.client
            .request("get_profile", rpc_params![addr])
            .await
            .ok()
    }

    async fn get_server_account(&self, addr: &str) -> Option<ServerAccount> {
        debug!("Fetch server account: {:?}", addr);
        let resp = self
            .client
            .request("get_server_info", rpc_params![addr])
            .await;
        match resp {
            Ok(server_account) => Some(server_account),
            Err(e) => {
                error!("Failed to get server account due to {:?}", e);
                None
            }
        }
    }

    async fn get_registration(&self, addr: &str) -> Option<RegistrationAccount> {
        debug!("Fetch registration account: {:?}", addr);
        self.client
            .request("get_registration_info", rpc_params![addr])
            .await
            .ok()
    }

    async fn publish_game(&self, _wallet: &JsValue, bundle: GameBundle) -> TransportResult<String> {
        self.client
            .request("publish_game_bundle", rpc_params![bundle])
            .await
            .map_err(|e| TransportError::NetworkError(e.to_string()))
    }

    async fn deposit(&self, _wallet: &JsValue, params: DepositParams) -> TransportResult<()> {
        self.client
            .request("deposit", rpc_params![params])
            .await
            .map_err(|e| TransportError::NetworkError(e.to_string()))
    }

    async fn create_registration(
        &self,
        _wallet: &JsValue,
        params: CreateRegistrationParams,
    ) -> TransportResult<String> {
        self.client
            .request("create_registration", rpc_params![params])
            .await
            .map_err(|e| TransportError::NetworkError(e.to_string()))
    }

    async fn register_game(
        &self,
        _wallet: &JsValue,
        params: RegisterGameParams,
    ) -> TransportResult<()> {
        self.client
            .request("register_game", rpc_params![params])
            .await
            .map_err(|e| TransportError::NetworkError(e.to_string()))
    }

    async fn unregister_game(
        &self,
        _wallet: &JsValue,
        params: UnregisterGameParams,
    ) -> TransportResult<()> {
        self.client
            .request("unregister_game", rpc_params![params])
            .await
            .map_err(|e| TransportError::NetworkError(e.to_string()))
    }
}
