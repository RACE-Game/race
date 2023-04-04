//! The connection to transactor.

use std::sync::Arc;

use async_stream::stream;
use async_trait::async_trait;
use futures::{lock::Mutex, Stream};
use gloo::console::{info, warn};
use jsonrpsee::{
    core::client::{Client, ClientT, SubscriptionClientT},
    rpc_params,
    wasm_client::WasmClientBuilder,
};
use race_core::{
    connection::ConnectionT,
    encryptor::EncryptorT,
    error::{Error, Result},
    types::{
        AttachGameParams, BroadcastFrame, ExitGameParams, GetStateParams, SubmitEventParams,
        SubscribeEventParams,
    },
};
use race_encryptor::Encryptor;
use serde::{de::DeserializeOwned, Serialize};

pub struct Connection {
    player_addr: String,
    endpoint: String,
    encryptor: Arc<Encryptor>,
    rpc_client: Arc<Mutex<Client>>,
    max_retries: u32,
}

async fn build_rpc_client(endpoint: &str) -> Result<Client> {
    WasmClientBuilder::default()
        .build(endpoint)
        .await
        .map_err(|e| Error::RpcError(e.to_string()))
}

#[async_trait(?Send)]
impl ConnectionT for Connection {
    async fn attach_game(&self, game_addr: &str, params: AttachGameParams) -> Result<()> {
        self.request("attach_game", game_addr, params).await
    }

    async fn submit_event(&self, game_addr: &str, params: SubmitEventParams) -> Result<()> {
        self.request("submit_event", game_addr, params).await
    }

    async fn exit_game(&self, game_addr: &str, params: ExitGameParams) -> Result<()> {
        self.request("exit_game", game_addr, params).await
    }
}

impl Connection {
    pub async fn try_new(
        player_addr: &str,
        endpoint: &str,
        encryptor: Arc<Encryptor>,
    ) -> Result<Self> {
        info!(format!(
            "Establish connection to transactor at {}",
            endpoint
        ));
        let rpc_client = Arc::new(Mutex::new(build_rpc_client(endpoint).await?));
        let max_retries = 3;
        Ok(Self {
            player_addr: player_addr.into(),
            endpoint: endpoint.into(),
            encryptor,
            rpc_client,
            max_retries,
        })
    }

    async fn request<P, R>(&self, method: &str, game_addr: &str, params: P) -> Result<R>
    where
        P: Serialize + ToString,
        R: DeserializeOwned,
    {
        let mut retried = 0;
        let message = format!("{}{}", game_addr, params.to_string());
        // info!("Message to encrypt: {}", &message);
        let mut rpc_client = self.rpc_client.lock().await;
        loop {
            let signature = self
                .encryptor
                .sign(message.as_bytes(), self.player_addr.clone())?;
            let result = rpc_client
                .request(method, rpc_params![game_addr, &params, signature])
                .await;
            use jsonrpsee::core::error::Error::*;
            match result {
                Ok(ret) => return Ok(ret),
                Err(RestartNeeded(_)) => {
                    warn!("Disconnected with transactor, will reconnect.");
                    *rpc_client = build_rpc_client(&self.endpoint).await?;
                    continue;
                }
                Err(RequestTimeout) => {
                    if retried < self.max_retries {
                        retried += 1;
                        continue;
                    } else {
                        return Err(Error::RpcError(RequestTimeout.to_string()));
                    }
                }
                Err(Call(jsonrpsee::types::error::CallError::Failed(e))) => {
                    warn!("RPC CallError due to", e.to_string());
                    match e.downcast_ref::<Error>() {
                        Some(Error::GameNotLoaded) => {}
                        Some(e) => {
                            return Err(e.to_owned());
                        }
                        None => {
                            unreachable!();
                        }
                    }
                }
                Err(e) => {
                    warn!("RPC Call failed due to", e.to_string());
                    return Err(Error::RpcError(e.to_string()));
                }
            }
        }
    }

    pub async fn get_state<R>(&self, game_addr: &str, params: GetStateParams) -> Result<R>
    where
        R: DeserializeOwned,
    {
        self.request("get_state", game_addr, params).await
    }

    pub async fn subscribe_events(
        &self,
        game_addr: &str,
        settle_version: u64,
    ) -> Result<impl Stream<Item = BroadcastFrame>> {
        let params = SubscribeEventParams { settle_version };
        let message = format!("{}{}", game_addr, params.to_string());
        let signature = self
            .encryptor
            .sign(message.as_bytes(), self.player_addr.clone())?;

        let sub = self
            .rpc_client
            .lock()
            .await
            .subscribe(
                "subscribe_event",
                rpc_params![game_addr, params, signature],
                "unsubscribe_event",
            )
            .await
            .map_err(|e| Error::RpcError(e.to_string()))?;

        Ok(stream! {
            for await frame in sub {
                if let Ok(frame) = frame {
                    yield frame;
                } else {
                    break;
                }
            }
        })
    }
}
