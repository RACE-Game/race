use std::sync::Arc;
use tracing::{warn, info};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tower::ServiceBuilder;
use tower_http::cors::Any;
use tower_http::cors::CorsLayer;
use hyper::Method;
use std::net::SocketAddr;
use crate::error::ReplayerError;
use crate::context::ReplayerContext;
use jsonrpsee::core::error::Error as RpcError;
use jsonrpsee::core::StringError;
use jsonrpsee::server::{ServerHandle, AllowHosts};
use jsonrpsee::types::error::CallError;
use jsonrpsee::types::ErrorObjectOwned;
use jsonrpsee::{server::ServerBuilder, types::Params, RpcModule};
use jsonrpsee::{PendingSubscriptionSink, SubscriptionMessage, TrySendError};

fn ping(_: Params<'_>, _: &Arc<ReplayerContext>) -> Result<String, RpcError> {
    Ok("pong".to_string())
}

// async fn subscribe_event(
//     params: Params<'_>,
//     pending: PendingSubscriptionSink,
//     context: Arc<ReplayerContext>,
// ) -> Result<(), StringError> {

//     let (game_addr, SubscribeEventParams { settle_version }) = match parse_params_no_sig(params) {
//         Ok(p) => p,
//         Err(e) => {
//             let _ = pending.reject(ErrorObjectOwned::from(e)).await;
//             return Ok(());
//         }
//     };

//     let (receiver, backlogs_frame) =
//         match context.get_broadcast_and_backlogs(&game_addr, settle_version).await {
//             Ok(x) => x,
//             Err(e) => {
//                 warn!("Game not found: {}", game_addr);
//                 let _ = pending.reject(CallError::Failed(e.into())).await;
//                 return Ok(());
//             }
//         };

//     drop(context);
//     info!(
//         "Subscribe event stream, game: {:?}, settle version: {}",
//         game_addr, settle_version,
//     );

//     let mut sink = pending.accept().await?;

//     let v = borsh::to_vec(&backlogs_frame).unwrap();
//     let s = utils::base64_encode(&v);
//     sink.send(SubscriptionMessage::from(&s))
//         .await
//         .map_err(|e| {
//             error!("Error occurred when broadcasting historical frame: {:?}", e);
//             e
//         })
//         .unwrap();

//     let rx = BroadcastStream::new(receiver);
//     let mut serialized_rx = rx.map(|f| match f {
//         Ok(x) => {
//             let v = borsh::to_vec(&x).unwrap();
//             let s = utils::base64_encode(&v);
//             Ok(s)
//         }
//         Err(e) => Err(e),
//     });

//     loop {
//         tokio::select! {
//             _ = sink.closed() => break Err(anyhow::anyhow!("Subscription was closed")),
//             maybe_item = serialized_rx.next() => {
//                 let item = match maybe_item {
//                     Some(Ok(item)) => item,
//                     _ => break Err(anyhow::anyhow!("Event stream ended")),
//                 };
//                 let msg = SubscriptionMessage::from(&item);
//                 match sink.try_send(msg) {
//                     Ok(_) => (),
//                     Err(TrySendError::Closed(_)) => break Err(anyhow::anyhow!("Client disconnected, subscription closed")),
//                     Err(TrySendError::Full(_)) => {
//                         warn!("TrySendError::Full");
//                     }
//                 }
//             },
//         }
//     }?;
//     Ok(())
// }

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

    // module.register_subscription(
    //     "subscribe_event",
    //     "s_event",
    //     "unsubscribe_event",
    //     subscribe_event,
    // )?;
    module.register_method("ping", ping)?;
    let handle = server.start(module)?;
    Ok(handle)
}
