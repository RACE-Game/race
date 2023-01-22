//! The connection to transactor.

use std::{borrow::Borrow, cell::RefCell};

use async_stream::stream;
use async_trait::async_trait;
use futures::stream::Stream;
use gloo::console::warn;
use jsonrpsee::{
    core::client::{Client, ClientT, SubscriptionClientT},
    rpc_params,
    wasm_client::WasmClientBuilder,
};
use race_core::{
    connection::ConnectionT,
    error::{Error, Result},
    types::{
        AttachGameParams, BroadcastFrame, ExitGameParams, GetStateParams, SubmitEventParams,
        SubscribeEventParams,
    }, encryptor,
};
use race_encryptor::Encryptor;
use serde::{de::DeserializeOwned, Serialize};

pub struct Connection {
    endpoint: String,
    encryptor: Arc<Encryptor>,
    rpc_client: RefCell<Client>,
    max_retries: u32,
}

async fn build_rpc_client(endpoint: &str) -> Result<Client> {
    WasmClientBuilder::default()
        .build(format!("ws://{}", endpoint))
        .await
        .map_err(|e| Error::RpcError(e.to_string()))
}

#[async_trait]
impl ConnectionT for Connection {
    async fn attach_game(&self, game_addr: &str, params: AttachGameParams) -> Result<()> {
        self.request("attach_game", game_addr, params).await
    }

    async fn submit_event(&self, params: SubmitEventParams) -> Result<()> {
        self.request("submit_event", params).await
    }

    async fn exit_game(&self, params: ExitGameParams) -> Result<()> {
        self.request("exit_game", params).await
    }
}

impl Connection {
    pub async fn try_new(endpoint: &str, encryptor: Arc<Encryptor>) -> Result<Self> {
        let rpc_client = RefCell::new(build_rpc_client(endpoint).await?);
        let max_retries = 3;
        Ok(Self {
            endpoint: endpoint.into(),
            encryptor,
            rpc_client,
            max_retries,
        })
    }

    pub async fn request<P, R>(&self, method: &str, game_addr: &str, params: P) -> Result<R>
    where
        P: Serialize + ToString,
        R: DeserializeOwned,
    {
        let mut retried = 0;
        let message = format!("{}{}", game_addr, params.to_string());
        loop {
            let result = self
                .rpc_client
                .borrow()
                .request(method, rpc_params![&params])
                .await;
            use jsonrpsee::core::error::Error::*;
            match result {
                Ok(ret) => return Ok(ret),
                Err(RestartNeeded(_)) => {
                    warn!("Disconnected with transactor, will reconnect.");
                    self.rpc_client
                        .replace(build_rpc_client(&self.endpoint).await?);
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
                    match e.downcast_ref::<Error>() {
                        Some(Error::GameNotLoaded) => {}
                        Some(e) => {
                            return e;
                        }
                        None => {
                            unreachable!();
                        }
                    }
                }
                Err(e) => return Err(Error::RpcError(e.to_string())),
            }
        }
    }

    async fn get_state<R>(&self, params: GetStateParams) -> Result<R>
    where
        R: DeserializeOwned,
    {
        self.request("get_state", params).await
    }

    pub async fn subscribe_events(
        &self,
        params: SubscribeEventParams,
    ) -> Result<impl Stream<Item = BroadcastFrame>> {
        let sub = self
            .rpc_client
            .borrow()
            .subscribe("subscribe_event", rpc_params![params], "unsubscribe_event")
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
