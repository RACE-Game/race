use std::{net::SocketAddr, sync::Arc};

use crate::context::ApplicationContext;
use jsonrpsee::core::error::SubscriptionClosed;
use jsonrpsee::core::Error;
use jsonrpsee::types::error::CallError;
use jsonrpsee::types::SubscriptionEmptyError;
use jsonrpsee::SubscriptionSink;
use jsonrpsee::{server::ServerBuilder, types::Params, RpcModule};
use race_core::types::{
    AttachGameParams, ExitGameParams, GetStateParams, Signature, SubmitEventParams,
    SubscribeEventParams,
};
use race_transport::signer;
use tokio::sync::Mutex;
use tokio_stream::wrappers::BroadcastStream;
use tracing::info;

type Result<T> = std::result::Result<T, Error>;

/// Ask transactor to load game and provide client's public key for further encryption.
async fn attach_game(params: Params<'_>, context: Arc<Mutex<ApplicationContext>>) -> Result<()> {
    let game_addr: String = params.one()?;
    let AttachGameParams { key } = params.one()?;
    let Signature { signer, .. } = params.one()?;
    // TODO: check signature
    let context = &mut *(context.lock().await);
    context
        .start_game(game_addr)
        .await
        .map_err(|e| Error::Call(CallError::InvalidParams(e.into())))?;
    context
        .register_key(signer, key)
        .map_err(|e| Error::Call(CallError::InvalidParams(e.into())))
}

async fn submit_event(params: Params<'_>, context: Arc<Mutex<ApplicationContext>>) -> Result<()> {
    let game_addr: String = params.one()?;
    let arg: SubmitEventParams = params.one()?;
    let sig: Signature = params.one()?;
    let context = context.lock().await;
    context
        .verify(&game_addr, &arg, sig)
        .map_err(|e| Error::Call(CallError::Failed(e.into())))?;
    context
        .send_event(&game_addr, arg.event)
        .await
        .map_err(|e| Error::Call(CallError::Failed(e.into())))
}

async fn get_state(params: Params<'_>, context: Arc<Mutex<ApplicationContext>>) -> Result<String> {
    let game_addr: String = params.one()?;
    let arg: GetStateParams = params.one()?;
    let sig: Signature = params.one()?;
    let context = context.lock().await;
    context
        .verify(&game_addr, &arg, sig)
        .map_err(|e| Error::Call(CallError::Failed(e.into())))?;
    let game_handle = context.get_game(&game_addr).ok_or_else(|| {
        Error::Call(CallError::Failed(
            race_core::error::Error::GameNotLoaded.into(),
        ))
    })?;

    let snapshot = game_handle.broadcaster.get_snapshot().await;
    Ok(snapshot)
}

async fn exit_game(params: Params<'_>, context: Arc<Mutex<ApplicationContext>>) -> Result<()> {
    let game_addr: String = params.one()?;
    let arg: ExitGameParams = params.one()?;
    let sig: Signature = params.one()?;
    let context = context.lock().await;
    context
        .verify(&game_addr, &arg, sig)
        .map_err(|e| Error::Call(CallError::Failed(e.into())))?;
    context
        .eject_player(&game_addr, "")
        .await
        .map_err(|e| Error::Call(CallError::Failed(e.into())))
}

fn subscribe_event(
    params: Params<'_>,
    mut sink: SubscriptionSink,
    context: Arc<Mutex<ApplicationContext>>,
) -> std::result::Result<(), SubscriptionEmptyError> {
    {
        let game_addr: String = params.one()?;
        let arg: SubscribeEventParams = params.one()?;
        let sig: Signature = params.one()?;

        tokio::spawn(async move {
            println!("Subscribe event stream: {:?}", game_addr);
            let context = context.lock().await;
            if let Err(e) = context.verify(&game_addr, &arg, sig) {
                sink.close(SubscriptionClosed::Failed(
                    CallError::Failed(e.into()).into(),
                ));
                return;
            }

            let handle = context
                .get_game(&game_addr)
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
    module.register_async_method("exit_game", exit_game)?;
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
