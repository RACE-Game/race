use std::sync::Arc;
use tokio_stream::StreamExt;
use tower::ServiceBuilder;
use tower_http::cors::Any;
use tower_http::cors::CorsLayer;
use hyper::Method;
use std::net::SocketAddr;
use crate::error::ReplayerError;
use crate::context::ReplayerContext;
use tracing::{info, warn, error};
use serde::{Serialize, Deserialize};
use jsonrpsee::core::error::Error as RpcError;
use jsonrpsee::core::StringError;
use jsonrpsee::server::{ServerHandle, AllowHosts};
use jsonrpsee::types::error::CallError;
use jsonrpsee::types::ErrorObjectOwned;
use jsonrpsee::{server::ServerBuilder, types::Params, RpcModule};
use jsonrpsee::{PendingSubscriptionSink, SubscriptionMessage, TrySendError};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct SubscribeEventOptions {
    game_addr: String,
    player_addr: String,
    start_settle_version: Option<u64>, // the starting point, None stands for starting from very beginning.
}

async fn ping(_: Params<'_>, _: Arc<ReplayerContext>) -> Result<String, RpcError> {
    Ok("pong".to_string())
}

async fn subscribe_event(
    params: Params<'_>,
    pending: PendingSubscriptionSink,
    context: Arc<ReplayerContext>,
) -> Result<(), StringError> {

    let opts = params.parse::<SubscribeEventOptions>()?;

    Ok(())
}

pub async fn run_server(
    context: ReplayerContext,
) -> Result<ServerHandle, ReplayerError> {
    let port = 11222;
    let cors = CorsLayer::new()
        .allow_methods([Method::POST, Method::OPTIONS])
        .allow_origin(Any)
        .allow_headers([hyper::header::CONTENT_TYPE]);

    let middleware = ServiceBuilder::new().layer(cors);

    let host = format!("0.0.0.0:{}", port);

    let server = ServerBuilder::default()
        .max_connections(500)
        .set_host_filtering(AllowHosts::Any)
        .set_middleware(middleware)
        .max_request_body_size(100_1000)
        .build(host.parse::<SocketAddr>()?)
        .await?;

    let mut module = RpcModule::new(context);

    module.register_async_method("attach_replay", ping)?;
    module.register_async_method("replay_control", ping)?;
    module.register_subscription(
        "subscribe_event",
        "s_event",
        "unsubscribe_event",
        subscribe_event,
    )?;

    let handle = server.start(module)?;
    Ok(handle)
}
