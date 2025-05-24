use std::pin::Pin;

use async_stream::stream;
use async_trait::async_trait;
use borsh::BorshDeserialize;
use futures::Stream;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::transport::HttpBackend;
use jsonrpsee::http_client::{HttpClient as Client, HttpClientBuilder as ClientBuilder};
use jsonrpsee::rpc_params;
use std::time::Duration;
use tokio::time;

use race_core::error::{Error, Result};
use race_core::transport::TransportT;

use race_core::types::{
    AddRecipientSlotParams, AssignRecipientParams, CloseGameAccountParams, CreateGameAccountParams, CreatePlayerProfileParams, CreateRecipientParams, CreateRegistrationParams, DepositParams, GameAccount, GameBundle, JoinParams, PlayerProfile, PublishGameParams, RecipientAccount, RecipientClaimParams, RegisterGameParams, RegisterServerParams, RegistrationAccount, RejectDepositsParams, RejectDepositsResult, ServeParams, ServerAccount, SettleParams, SettleResult, UnregisterGameParams, VoteParams
};
use serde::Serialize;

use crate::error::{TransportError, TransportResult};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServeInstruction {
    game_addr: String,
    server_addr: String,
    verify_key: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterServerInstruction {
    server_addr: String,
    endpoint: String,
}

pub struct FacadeTransport {
    addr: Option<String>,
    client: Client<HttpBackend>,
}

impl FacadeTransport {
    pub async fn try_new(addr: Option<String>, url: &str) -> TransportResult<Self> {
        let client = ClientBuilder::default()
            .max_request_size(64_000_000)
            .build(url)
            .map_err(|e| TransportError::InitializationFailed(e.to_string()))?;

        Ok(Self { addr, client })
    }

    pub async fn fetch<T: BorshDeserialize>(&self, method: &str, addr: &str) -> Result<Option<T>> {
        let data: Option<Vec<u8>> = self
            .client
            .request(method, rpc_params![addr])
            .await
            .map_err(|e| TransportError::NetworkError(e.to_string()))?;
        if let Some(data) = data {
            Ok(Some(T::try_from_slice(&data).unwrap()))
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
#[allow(unused_variables)]
impl TransportT for FacadeTransport {
    async fn create_game_account(&self, params: CreateGameAccountParams) -> Result<String> {
        unimplemented!()
    }

    async fn close_game_account(&self, params: CloseGameAccountParams) -> Result<()> {
        unimplemented!()
    }

    async fn recipient_claim(&self, params: RecipientClaimParams) -> Result<()> {
        unimplemented!()
    }

    async fn register_server(&self, params: RegisterServerParams) -> Result<()> {
        self.client
            .request(
                "register_server",
                rpc_params![RegisterServerInstruction {
                    server_addr: self.addr(),
                    endpoint: params.endpoint
                }],
            )
            .await
            .map_err(|e| Error::RpcError(e.to_string()))
    }

    async fn join(&self, params: JoinParams) -> Result<()> {
        unimplemented!()
    }

    async fn serve(&self, params: ServeParams) -> Result<()> {
        self.client
            .request(
                "serve",
                rpc_params![ServeInstruction {
                    game_addr: params.game_addr,
                    server_addr: self.addr(),
                    verify_key: params.verify_key,
                }],
            )
            .await
            .map_err(|e| Error::RpcError(e.to_string()))
    }

    async fn vote(&self, params: VoteParams) -> Result<()> {
        if let Some(game_account) = self.get_game_account(&params.game_addr).await? {
            if game_account
                .votes
                .iter()
                .any(|v| v.voter.eq(&params.voter_addr))
            {
                Err(Error::DuplicatedVote)
            } else {
                self.client
                    .request("vote", rpc_params![params])
                    .await
                    .map_err(|e| Error::RpcError(e.to_string()))
            }
        } else {
            Err(Error::GameAccountNotFound)
        }
    }

    async fn subscribe_game_account<'a>(
        &'a self,
        addr: &'a str,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<GameAccount>> + Send + 'a>>> {
        Ok(Box::pin(stream! {
            let mut access_version = 0;
            loop {
                match self.fetch::<GameAccount>("get_account_info", addr).await {
                    Ok(game_account_opt) => {
                        if let Some(game_account) = game_account_opt {
                            if game_account.access_version > access_version {
                                access_version = game_account.access_version;
                                yield Ok(game_account);
                            }
                        }
                    }
                    Err(e) => yield Err(Error::TransportError(e.to_string())),
                }
                time::sleep(Duration::from_secs(1)).await;
            }
        }))
    }

    async fn get_game_account(&self, addr: &str) -> Result<Option<GameAccount>> {
        self.fetch("get_account_info", addr).await
    }

    async fn get_game_bundle(&self, addr: &str) -> Result<Option<GameBundle>> {
        self.fetch("get_game_bundle", addr).await
    }

    async fn create_player_profile(&self, params: CreatePlayerProfileParams) -> Result<()> {
        unimplemented!()
    }

    async fn get_player_profile(&self, addr: &str) -> Result<Option<PlayerProfile>> {
        self.fetch("get_profile", addr).await
    }

    async fn get_server_account(&self, addr: &str) -> Result<Option<ServerAccount>> {
        self.fetch("get_server_info", addr).await
    }

    async fn get_registration(&self, addr: &str) -> Result<Option<RegistrationAccount>> {
        self.fetch("get_registration_info", addr).await
    }

    async fn get_recipient(&self, addr: &str) -> Result<Option<RecipientAccount>> {
        Ok(None)
    }

    async fn publish_game(&self, params: PublishGameParams) -> Result<String> {
        unimplemented!()
    }

    async fn settle_game(&self, params: SettleParams) -> Result<SettleResult> {
        let game_addr = params.addr.clone();
        let signature = self
            .client
            .request("settle", rpc_params![params])
            .await
            .map_err(|e| Error::RpcError(e.to_string()))?;

        let game_account = self.get_game_account(&game_addr).await.unwrap().unwrap();

        return Ok(SettleResult {
            signature,
            game_account,
        });
    }

    async fn create_recipient(&self, params: CreateRecipientParams) -> Result<String> {
        unimplemented!()
    }

    async fn add_recipient_slot(&self, params: AddRecipientSlotParams) -> Result<String> {
        unimplemented!()
    }

    async fn assign_recipient(&self, params: AssignRecipientParams) -> Result<()> {
        unimplemented!()
    }

    async fn deposit(&self, params: DepositParams) -> Result<()> {
        unimplemented!()
    }

    async fn create_registration(&self, params: CreateRegistrationParams) -> Result<String> {
        unimplemented!()
    }

    async fn register_game(&self, params: RegisterGameParams) -> Result<()> {
        unimplemented!()
    }

    async fn unregister_game(&self, params: UnregisterGameParams) -> Result<()> {
        unimplemented!()
    }

    async fn reject_deposits(&self, params: RejectDepositsParams) -> Result<RejectDepositsResult> {
        let game_addr = params.addr.clone();
        let signature = self
            .client
            .request("reject_deposits", rpc_params![params])
            .await
            .map_err(|e| Error::RpcError(e.to_string()))?;

        return Ok(RejectDepositsResult { signature });
    }
}

impl FacadeTransport {
    fn addr(&self) -> String {
        self.addr.clone().expect("Address not specified, facade transport need an address to mock its identity. It can be either in config file or command line arguments.")
    }
}
