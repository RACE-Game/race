use std::{net::SocketAddr, sync::Arc};

use crate::context::ApplicationContext;
use jsonrpsee::core::error::SubscriptionClosed;
use jsonrpsee::core::Error;
use jsonrpsee::types::error::CallError;
use jsonrpsee::types::SubscriptionEmptyError;
use jsonrpsee::SubscriptionSink;
use jsonrpsee::{server::ServerBuilder, types::Params, RpcModule};
use race_core::types::{
    AttachGameParams, BroadcastFrame, ExitGameParams, GetStateParams, Signature, SubmitEventParams,
    SubscribeEventParams,
};
use tokio::sync::Mutex;
use tokio_stream::wrappers::BroadcastStream;
use tracing::info;

type Result<T> = std::result::Result<T, Error>;

/// Ask transactor to load game and provide client's public key for further encryption.
async fn attach_game(params: Params<'_>, context: Arc<Mutex<ApplicationContext>>) -> Result<()> {
    info!("Attach to game");

    let (game_addr, AttachGameParams { key }, Signature { signer, .. }) =
        params.parse::<(String, AttachGameParams, Signature)>()?;
    // TODO: check signature
    let context = &mut *(context.lock().await);
    context
        .start_game(game_addr)
        .await
        .map_err(|e| Error::Call(CallError::InvalidParams(e.into())))?;
    info!("Register the key provided by client {}", signer);
    context
        .register_key(signer, key)
        .await
        .map_err(|e| Error::Call(CallError::InvalidParams(e.into())))
}

async fn submit_event(params: Params<'_>, context: Arc<Mutex<ApplicationContext>>) -> Result<()> {
    let (game_addr, arg, sig) = params.parse::<(String, SubmitEventParams, Signature)>()?;
    info!("Submit event: {:?}", arg);

    let context = context.lock().await;
    context
        .verify(&game_addr, &arg, &sig)
        .await
        .map_err(|e| Error::Call(CallError::Failed(e.into())))?;
    context
        .send_event(&game_addr, arg.event)
        .await
        .map_err(|e| Error::Call(CallError::Failed(e.into())))
}

// async fn retrieve_events(params: Params<'_>, context: Arc<Mutex<ApplicationContext>>) -> Result<Vec<Event>> {
//     let (game_addr, arg, sig) = params.parse::<(String, RetrieveEventsParams, Signature)>()?;

//     let context = context.lock().await;
//     context
//         .verify(&game_addr, &arg, &sig)
//         .await
//         .map_err(|e| Error::Call(CallError::Failed(e.into())))?;
//     let game_handle = context.get_game(&game_addr).ok_or_else(|| {
//         Error::Call(CallError::Failed(
//             race_core::error::Error::GameNotLoaded.into(),
//         ))
//     })?;

//     Ok(game_handle.broadcaster.retrieve_events(arg.settle_version).await)
// }

async fn get_state(params: Params<'_>, context: Arc<Mutex<ApplicationContext>>) -> Result<String> {
    let (game_addr, arg, sig) = params.parse::<(String, GetStateParams, Signature)>()?;

    let context = context.lock().await;
    context
        .verify(&game_addr, &arg, &sig)
        .await
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
    let (game_addr, arg, sig) = params.parse::<(String, ExitGameParams, Signature)>()?;

    let context = context.lock().await;
    context
        .verify(&game_addr, &arg, &sig)
        .await
        .map_err(|e| Error::Call(CallError::Failed(e.into())))?;
    context
        .eject_player(&game_addr, &sig.signer)
        .await
        .map_err(|e| Error::Call(CallError::Failed(e.into())))
}

fn subscribe_event(
    params: Params<'_>,
    mut sink: SubscriptionSink,
    context: Arc<Mutex<ApplicationContext>>,
) -> std::result::Result<(), SubscriptionEmptyError> {
    {
        let (game_addr, arg, sig) = params.parse::<(String, SubscribeEventParams, Signature)>()?;
        info!("Subscribe event stream: {:?}", game_addr);

        tokio::spawn(async move {
            let context = context.lock().await;
            if let Err(e) = context.verify(&game_addr, &arg, &sig).await {
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

            let events = handle.broadcaster.retrieve_events(arg.settle_version).await;

            events.into_iter().for_each(|e| {
                sink.send(&BroadcastFrame {
                    game_addr: game_addr.clone(),
                    event: e,
                })
                .unwrap();
            });

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
        .max_request_body_size(100_1000)
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
