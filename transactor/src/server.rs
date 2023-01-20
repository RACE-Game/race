use std::{net::SocketAddr, sync::Arc};

use crate::context::ApplicationContext;
use jsonrpsee::core::error::SubscriptionClosed;
use jsonrpsee::core::Error;
use jsonrpsee::types::SubscriptionEmptyError;
use jsonrpsee::SubscriptionSink;
use jsonrpsee::{server::ServerBuilder, types::Params, RpcModule};
use race_core::types::{AttachGameParams, GetStateParams, SubmitEventParams, SubscribeEventParams};
use tokio::sync::Mutex;
use tokio_stream::wrappers::BroadcastStream;
use tracing::info;

type Result<T> = std::result::Result<T, Error>;

async fn attach_game(params: Params<'_>, context: Arc<Mutex<ApplicationContext>>) -> Result<()> {
    let params: AttachGameParams = params.one()?;
    let context = &mut *(context.lock().await);
    context
        .start_game(params)
        .await
        .map_err(|e| Error::Custom(e.to_string()))
}

async fn submit_event(params: Params<'_>, context: Arc<Mutex<ApplicationContext>>) -> Result<()> {
    let params: SubmitEventParams = params.one()?;
    info!("2Receive client event: {:?}", params.event);
    let context = context.lock().await;
    info!("3Receive client event: {:?}", params.event);
    context
        .send_event(&params.addr, params.event)
        .await
        .map_err(|e| Error::Custom(e.to_string()))
}

async fn get_state(params: Params<'_>, context: Arc<Mutex<ApplicationContext>>) -> Result<String> {
    let params: GetStateParams = params.one()?;
    let context = context.lock().await;

    let game_handle = context
        .get_game(&params.addr)
        .ok_or_else(|| Error::Custom(format!("Game: {} not found", params.addr)))?;

    let snapshot = game_handle.broadcaster.get_snapshot().await;
    Ok(snapshot)
}

fn subscribe_event(
    params: Params<'_>,
    mut sink: SubscriptionSink,
    context: Arc<Mutex<ApplicationContext>>,
) -> std::result::Result<(), SubscriptionEmptyError> {
    {
        let params: SubscribeEventParams = params.one()?;
        let addr = params.addr.clone();
        tokio::spawn(async move {
            println!("Subscribe event stream: {:?}", addr);
            let context = context.lock().await;

            let handle = context
                .get_game(&addr)
                .ok_or(SubscriptionEmptyError)
                .expect("Failed to get game");

            let rx = BroadcastStream::new(handle.broadcaster.get_broadcast_rx());

            drop(context);

            match sink.pipe_from_try_stream(rx).await {
                SubscriptionClosed::Success => {
                    sink.close(SubscriptionClosed::Success);
                }
                SubscriptionClosed::RemotePeerAborted => (),
                SubscriptionClosed::Failed(err) => {
                    sink.close(err);
                }
            };
        });
        Ok(())
    }
}

pub async fn run_server(context: Mutex<ApplicationContext>) -> anyhow::Result<()> {
    let host = {
        let context = context.lock().await;
        let port = context.config.port;
        format!("0.0.0.0:{}", port)
    };
    let server = ServerBuilder::default()
        .build(host.parse::<SocketAddr>()?)
        .await?;
    let mut module = RpcModule::new(context);

    module.register_async_method("attach_game", attach_game)?;
    module.register_async_method("submit_event", submit_event)?;
    module.register_async_method("get_state", get_state)?;
    module.register_subscription(
        "subscribe_event",
        "s_event",
        "unsubscribe_event",
        subscribe_event,
    )?;

    let handle = server.start(module)?;
    info!("Server started at {:?}", host);
    handle.stopped().await;
    Ok(())
}
