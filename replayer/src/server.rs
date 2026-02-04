use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::Any;
use tower_http::cors::CorsLayer;
use hyper::Method;
use std::net::SocketAddr;
use crate::error::ReplayerError;
use crate::context::ReplayerContext;
use jsonrpsee::core::error::Error as RpcError;
use jsonrpsee::server::{ServerHandle, AllowHosts};
use jsonrpsee::{server::ServerBuilder, types::Params, RpcModule};

fn ping(_: Params<'_>, _: &Arc<ReplayerContext>) -> Result<String, RpcError> {
    Ok("pong".to_string())
}

pub async fn run_server(
    context: Arc<ReplayerContext>,
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

    module.register_method("ping", ping)?;
//    module.register_method("")?;
    let handle = server.start(module)?;
    Ok(handle)
}
