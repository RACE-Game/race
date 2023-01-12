//! The connection to transactor, used by player and validator.

use crate::{
    error::{Error, Result},
    types::{
        AttachGameParams, BroadcastFrame, GetStateParams, SubmitEventParams,
        SubscribeEventParams,
    },
};
use jsonrpsee::{
    core::{
        client::{ClientT, Subscription, SubscriptionClientT},
        traits::ToRpcParams,
        DeserializeOwned,
    },
    rpc_params,
};

pub struct Connection<T>
where
    T: ClientT + SubscriptionClientT,
{
    transport: T,
    pub endpoint: String,
}

impl<T> Connection<T>
where
    T: ClientT + SubscriptionClientT,
{
    pub fn new(endpoint: String, transport: T) -> Self {
        Self {
            endpoint,
            transport,
        }
    }

    pub async fn request<R, Params>(&self, method: &str, params: Params) -> Result<R>
    where
        R: DeserializeOwned,
        Params: ToRpcParams + Send,
    {
        self.transport
            .request(method, params)
            .await
            .map_err(|e| Error::RpcError(e.to_string()))
    }

    pub async fn attach_game(&self, params: AttachGameParams) -> Result<()> {
        self.request("attach_game", rpc_params![params]).await
    }

    pub async fn submit_event(&self, params: SubmitEventParams) -> Result<()> {
        self.request("submit_event", rpc_params![params]).await
    }

    pub async fn get_state<R>(&self, params: GetStateParams) -> Result<R>
    where
        R: DeserializeOwned,
    {
        self.request("get_state", rpc_params![params]).await
    }

    pub async fn subscribe(
        &self,
        params: SubscribeEventParams,
    ) -> Result<Subscription<BroadcastFrame>> {
        let sub: Subscription<BroadcastFrame> = self
            .transport
            .subscribe("subscribe_event", rpc_params![params], "unsubscribe_event")
            .await
            .map_err(|e| Error::RpcError(e.to_string()))?;

        Ok(sub)
    }
}
