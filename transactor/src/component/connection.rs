//! We use two types of connection in server:
//!
//! - [`LocalConnection`], used to send event to local event bus.
//! - [`RemoteConnection`], used to send event to remote transactor server.

use async_stream::stream;
use async_trait::async_trait;
use futures::Stream;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::warn;

use jsonrpsee::{
    core::{
        client::{ClientT, SubscriptionClientT},
        DeserializeOwned,
    },
    rpc_params,
    ws_client::{WsClient, WsClientBuilder},
};
use race_core::{
    connection::ConnectionT,
    encryptor::EncryptorT,
    types::{AttachGameParams, ExitGameParams, SubmitEventParams},
};
use race_core::{
    error::{Error, Result},
    types::{BroadcastFrame, SubscribeEventParams},
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
    server_addr: String,
    endpoint: String,
    encryptor: Arc<dyn EncryptorT>,
    rpc_client: Mutex<Option<WsClient>>,
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

async fn build_rpc_client(endpoint: &str) -> Result<WsClient> {
    let client = WsClientBuilder::default()
        .build(endpoint)
        .await
        .map_err(|e| Error::InitializeRpcClientError(e.to_string()))?;
    Ok(client)
}

impl RemoteConnection {
    pub async fn try_new(
        server_addr: &str,
        endpoint: &str,
        encryptor: Arc<dyn EncryptorT>,
    ) -> Result<Self> {
        let max_retries = 3;
        Ok(Self {
            server_addr: server_addr.to_owned(),
            endpoint: endpoint.into(),
            encryptor,
            rpc_client: Mutex::new(None),
            max_retries,
        })
    }

    async fn request<P, R>(&self, game_addr: &str, method: &str, params: &P) -> Result<R>
    where
        P: Serialize + ToString,
        R: DeserializeOwned,
    {
        let mut rpc_client = self.rpc_client.lock().await;
        let mut retries = 0;
        let message = format!("{}{}", game_addr, params.to_string());
        let signature = self
            .encryptor
            .sign(message.as_bytes(), self.server_addr.clone())?;

        loop {
            let client = if let Some(rpc_client) = rpc_client.as_ref() {
                rpc_client
            } else {
                *rpc_client = Some(build_rpc_client(&self.endpoint).await?);
                rpc_client.as_ref().unwrap()
            };

            let res = client
                .request(method, rpc_params![game_addr, params, &signature])
                .await;
            use jsonrpsee::core::error::Error::*;
            match res {
                Ok(ret) => return Ok(ret),
                Err(RestartNeeded(e)) => {
                    // For reconnecting
                    warn!("Try reconnect due to error: {:?}", e);
                    *rpc_client = None;
                }
                Err(_) => (),
            }

            if retries < self.max_retries {
                retries += 1;
                continue;
            } else {
                return Err(Error::RpcError("Max retries has been reached".into()));
            }
        }
    }

    pub async fn subscribe_events(
        &self,
        game_addr: &str,
        signer: &str,
        settle_version: u64,
    ) -> Result<impl Stream<Item = BroadcastFrame>> {
        let params = SubscribeEventParams { settle_version };
        let message = format!("{}{}", game_addr, params.to_string());
        let signature = self.encryptor.sign(message.as_bytes(), signer.to_owned())?;

        let mut rpc_client = self.rpc_client.lock().await;
        let client = if let Some(client) = rpc_client.as_ref() {
            client
        } else {
            *rpc_client = Some(build_rpc_client(&self.endpoint).await?);
            rpc_client.as_ref().unwrap()
        };

        let sub = client
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
