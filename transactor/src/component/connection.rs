//! We use two types of connection in server:
//!
//! - [`LocalConnection`], used to send event to local event bus.
//! - [`RemoteConnection`], used to send event to remote transactor server.

use async_stream::stream;
use async_trait::async_trait;
use borsh::{BorshDeserialize, BorshSerialize};
use futures::Stream;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::warn;

use jsonrpsee::{
    core::{
        client::{ClientT, Subscription, SubscriptionClientT},
        params::ArrayParams,
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
use race_api::error::{Error, Result};
use race_core::{
    types::{BroadcastFrame, SubscribeEventParams},
};

use crate::frame::EventFrame;
use crate::utils::base64_decode;
use crate::{component::common::Attachable, utils::base64_encode};

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
    fn id(&self) -> &str {
        "LocalConnection"
    }

    fn input(&mut self) -> Option<mpsc::Sender<EventFrame>> {
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
        let req = self.make_request_no_sig(game_addr, &params)?;
        self.request("attach_game", req).await
    }

    async fn submit_event(&self, game_addr: &str, params: SubmitEventParams) -> Result<()> {
        let req = self.make_request(game_addr, &params)?;
        self.request("submit_event", req).await
    }

    async fn exit_game(&self, game_addr: &str, params: ExitGameParams) -> Result<()> {
        let req = self.make_request(game_addr, &params)?;
        self.request("exit-game", req).await
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

    // async fn request(&self, game_addr: &str, method: &str) {
    //     let mut rpc_client = self.rpc_client.lock().await;
    //     let mut retries = 0;

    // }

    fn make_request<P>(&self, game_addr: &str, params: &P) -> Result<ArrayParams>
    where
        P: BorshSerialize,
    {
        let params_bytes = params.try_to_vec()?;
        let sig = self
            .encryptor
            .sign(&params_bytes, self.server_addr.clone())?;
        let sig_bytes = sig.try_to_vec()?;
        let p = base64_encode(&params_bytes);
        let s = base64_encode(&sig_bytes);
        Ok(rpc_params![game_addr, p, s])
    }

    fn make_request_no_sig<P>(&self, game_addr: &str, params: &P) -> Result<ArrayParams>
    where
        P: BorshSerialize,
    {
        let params_bytes = params.try_to_vec()?;
        let p = base64_encode(&params_bytes);
        Ok(rpc_params![game_addr, p])
    }

    async fn request<R>(&self, method: &str, params: ArrayParams) -> Result<R>
    where
        R: DeserializeOwned,
    {
        let mut rpc_client = self.rpc_client.lock().await;
        let mut retries = 0;

        loop {
            let client = if let Some(rpc_client) = rpc_client.as_ref() {
                rpc_client
            } else {
                *rpc_client = Some(build_rpc_client(&self.endpoint).await?);
                rpc_client.as_ref().unwrap()
            };

            let res = client.request(method, params.clone()).await;
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
        settle_version: u64,
    ) -> Result<impl Stream<Item = BroadcastFrame>> {
        let params = SubscribeEventParams { settle_version };

        let mut rpc_client = self.rpc_client.lock().await;
        let client = if let Some(client) = rpc_client.as_ref() {
            client
        } else {
            *rpc_client = Some(build_rpc_client(&self.endpoint).await?);
            rpc_client.as_ref().unwrap()
        };
        let req = self.make_request_no_sig(game_addr, &params)?;

        let sub: Subscription<String> = client
            .subscribe("subscribe_event", req, "unsubscribe_event")
            .await
            .map_err(|e| Error::RpcError(e.to_string()))?;

        Ok(stream! {
            for await s in sub {
                if let Ok(s) = s {
                    if let Ok(v) = base64_decode(&s) {
                        if let Ok(frame) = BroadcastFrame::try_from_slice(&v) {
                            yield frame;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        })
    }
}
