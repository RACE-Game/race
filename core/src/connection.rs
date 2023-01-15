//! The connection to transactor, used by player and validator.

use crate::{
    error::{Error, Result},
    types::{
        AttachGameParams, BroadcastFrame, GetStateParams, SubmitEventParams, SubscribeEventParams,
    },
};
use async_trait::async_trait;
use jsonrpsee::{
    core::{
        client::{ClientT, Subscription, SubscriptionClientT},
        traits::ToRpcParams,
        DeserializeOwned,
    },
    rpc_params,
};

#[async_trait]
pub trait ConnectionT
{
    type Transport: ClientT + SubscriptionClientT + Sync + Send;

    fn transport(&self) -> &Self::Transport;

    async fn request<R, Params>(&self, method: &str, params: Params) -> Result<R>
    where
        R: DeserializeOwned,
        Params: ToRpcParams + Send,
    {
        self.transport()
            .request(method, params)
            .await
            .map_err(|e| Error::RpcError(e.to_string()))
    }

    async fn attach_game(&self, params: AttachGameParams) -> Result<()> {
        self.request("attach_game", rpc_params![params]).await
    }

    async fn submit_event(&self, params: SubmitEventParams) -> Result<()> {
        self.request("submit_event", rpc_params![params]).await
    }

    async fn get_state<R>(&self, params: GetStateParams) -> Result<R>
    where
        R: DeserializeOwned,
    {
        self.request("get_state", rpc_params![params]).await
    }

    async fn subscribe(
        &self,
        params: SubscribeEventParams,
    ) -> Result<Subscription<BroadcastFrame>> {
        let sub: Subscription<BroadcastFrame> = self
            .transport()
            .subscribe("subscribe_event", rpc_params![params], "unsubscribe_event")
            .await
            .map_err(|e| Error::RpcError(e.to_string()))?;

        Ok(sub)
    }
}
