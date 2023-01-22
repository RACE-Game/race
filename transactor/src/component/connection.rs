//! We use two types of connection in server:
//!
//! - [`LocalConnection`], used to send event to local event bus.
//! - [`RemoteConnection`], used to send event to remote transactor server.

use async_trait::async_trait;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::warn;

use jsonrpsee::{
    core::{client::ClientT, DeserializeOwned},
    http_client::{HttpClient, HttpClientBuilder},
    rpc_params,
};
use race_core::error::{Error, Result};
use race_core::{
    connection::ConnectionT,
    encryptor::EncryptorT,
    types::{AttachGameParams, ExitGameParams, SubmitEventParams},
};

use crate::component::traits::Attachable;
use crate::frame::EventFrame;

/// A connection to local event bus, for transactor loopback.
#[allow(dead_code)]
pub struct LocalConnection {
    encryptor: Arc<dyn EncryptorT>,
    output_tx: mpsc::Sender<EventFrame>,
    output_rx: Option<mpsc::Receiver<EventFrame>>,
}

#[async_trait]
impl ConnectionT for LocalConnection {
    async fn attach_game(&self, _game_addr: &str, _params: AttachGameParams) -> Result<()> {
        Ok(())
    }

    async fn submit_event(&self, _game_addr: &str, params: SubmitEventParams) -> Result<()> {
        self.output_tx
            .send(EventFrame::SendEvent {
                event: params.event,
            })
            .await
            .map_err(|e| Error::InternalError(e.to_string()))
    }

    async fn exit_game(&self, _game_addr: &str, _params: ExitGameParams) -> Result<()> {
        Ok(())
    }
}

impl LocalConnection {
    pub fn new(encryptor: Arc<dyn EncryptorT>) -> Self {
        let (output_tx, output_rx) = mpsc::channel(3);
        Self {
            encryptor,
            output_tx,
            output_rx: Some(output_rx),
        }
    }
}

impl Attachable for LocalConnection {
    fn input(&self) -> Option<mpsc::Sender<EventFrame>> {
        None
    }

    fn output(&mut self) -> Option<mpsc::Receiver<EventFrame>> {
        let mut ret = None;
        std::mem::swap(&mut self.output_rx, &mut ret);
        ret
    }
}

pub struct RemoteConnection {
    endpoint: String,
    encryptor: Arc<dyn EncryptorT>,
    rpc_client: Mutex<HttpClient>,
    max_retries: u32,
}

#[async_trait]
impl ConnectionT for RemoteConnection {
    async fn attach_game(&self, game_addr: &str, params: AttachGameParams) -> Result<()> {
        self.request(game_addr, "attach_game", &params).await
    }

    async fn submit_event(&self, game_addr: &str, params: SubmitEventParams) -> Result<()> {
        self.request(game_addr, "submit_event", &params).await
    }

    async fn exit_game(&self, game_addr: &str, params: ExitGameParams) -> Result<()> {
        self.request(game_addr, "exit_game", &params).await
    }
}

fn build_rpc_client(endpoint: &str) -> Result<HttpClient> {
    HttpClientBuilder::default()
        .build(format!("ws://{}", endpoint))
        .map_err(|e| Error::RpcError(e.to_string()))
}

impl RemoteConnection {
    pub fn try_new(endpoint: &str, encryptor: Arc<dyn EncryptorT>) -> Result<Self> {
        let max_retries = 3;
        Ok(Self {
            endpoint: endpoint.into(),
            encryptor,
            rpc_client: Mutex::new(build_rpc_client(endpoint)?),
            max_retries,
        })
    }

    async fn request<P, R>(&self, game_addr: &str, method: &str, params: &P) -> Result<R>
    where
        P: Serialize + ToString,
        R: DeserializeOwned,
    {
        let retries = 3;
        loop {
            let message = format!("{}{}", game_addr, params.to_string());
            let signature = self
                .encryptor
                .sign(message.as_bytes(), self.encryptor.export_public_key(None)?)?;
            let mut rpc_client = self.rpc_client.lock().await;
            let res = rpc_client
                .request(method, rpc_params![game_addr, params, signature])
                .await;
            use jsonrpsee::core::error::Error::*;
            match res {
                Ok(ret) => return Ok(ret),
                Err(RestartNeeded(e)) => {
                    if retries >= self.max_retries {
                        return Err(Error::RpcError(e));
                    } else {
                        warn!(
                            "Restart RPC client for the connection to transactor, due to error: {}",
                            e
                        );
                        let old = std::mem::replace(&mut *rpc_client, build_rpc_client(&self.endpoint)?);
                        drop(old);
                        continue;
                    }
                }
                Err(e) => return Err(Error::RpcError(e.to_string())),
            }
        }
    }
}
