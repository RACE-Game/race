//! The connection to transactor.

use async_stream::stream;
use futures::stream::Stream;
use jsonrpsee::{
    core::client::{Client, ClientT, SubscriptionClientT},
    rpc_params,
    wasm_client::WasmClientBuilder,
};
use race_core::{
    error::{Error, Result},
    types::{
        AttachGameParams, BroadcastFrame, GetStateParams, SubmitEventParams, SubscribeEventParams,
    },
};
use serde::de::DeserializeOwned;

pub struct Connection {
    rpc_client: Client,
}

impl Connection {
    pub async fn try_new(endpoint: &str) -> Result<Self> {
        let rpc_client = WasmClientBuilder::default()
            .build(format!("ws://{}", endpoint))
            .await
            .map_err(|e| Error::RpcError(e.to_string()))?;
        Ok(Self { rpc_client })
    }

    pub async fn attach_game(&self, params: AttachGameParams) -> Result<()> {
        self.rpc_client
            .request("attach_game", rpc_params![params])
            .await
            .map_err(|e| Error::RpcError(e.to_string()))
    }

    pub async fn submit_event(&self, params: SubmitEventParams) -> Result<()> {
        self.rpc_client
            .request("submit_event", rpc_params![params])
            .await
            .map_err(|e| Error::RpcError(e.to_string()))
    }

    pub async fn get_state<R>(&self, params: GetStateParams) -> Result<R>
    where
        R: DeserializeOwned,
    {
        self.rpc_client
            .request("get_state", rpc_params![params])
            .await
            .map_err(|e| Error::RpcError(e.to_string()))
    }

    pub async fn subscribe_events(
        &self,
        params: SubscribeEventParams,
    ) -> Result<impl Stream<Item = BroadcastFrame>> {
        let sub = self
            .rpc_client
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
