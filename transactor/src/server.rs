use std::{net::SocketAddr, sync::Arc};

use crate::context::ApplicationContext;
use jsonrpsee::core::error::SubscriptionClosed;
use jsonrpsee::core::Error;
use jsonrpsee::types::error::CallError;
use jsonrpsee::types::SubscriptionEmptyError;
use jsonrpsee::SubscriptionSink;
use jsonrpsee::{server::ServerBuilder, types::Params, RpcModule};
use race_core::types::{
    AttachGameParams, ExitGameParams, Signature, SubmitEventParams, SubscribeEventParams,
};
use tokio_stream::wrappers::BroadcastStream;
use tracing::{error, info, warn};

type Result<T> = std::result::Result<T, Error>;

/// Ask transactor to load game and provide client's public key for further encryption.
async fn attach_game(params: Params<'_>, context: Arc<ApplicationContext>) -> Result<()> {
    info!("Attach to game");
    let (_game_addr, AttachGameParams { key }, Signature { signer, .. }) =
        params.parse::<(String, AttachGameParams, Signature)>()?;
    // TODO: check signature
    info!("Register the key provided by client {}", signer);
    context
        .register_key(signer, key)
        .await
        .map_err(|e| Error::Call(CallError::InvalidParams(e.into())))
}

async fn submit_event(params: Params<'_>, context: Arc<ApplicationContext>) -> Result<()> {
    let (game_addr, arg, sig) = params.parse::<(String, SubmitEventParams, Signature)>()?;
    context.verify(&game_addr, &arg, &sig).await.map_err(|e| {
        warn!("Reject event due to verification failed: {}", e);
        Error::Call(CallError::InvalidParams(e.into()))
    })?;
    context
        .send_event(&game_addr, arg.event)
        .await
        .map_err(|e| Error::Call(CallError::Failed(e.into())))
}

async fn exit_game(params: Params<'_>, context: Arc<ApplicationContext>) -> Result<()> {
    let (game_addr, arg, sig) = params.parse::<(String, ExitGameParams, Signature)>()?;

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
    context: Arc<ApplicationContext>,
) -> std::result::Result<(), SubscriptionEmptyError> {
    {
        let (game_addr, arg, _sig) = params.parse::<(String, SubscribeEventParams, Signature)>()?;

        tokio::spawn(async move {
            // We don't need verification.
            // if let Err(e) = context.verify(&game_addr, &arg, &sig).await {
            //     error!("Subscription verification failed: {:?}", e);
            //     sink.close(SubscriptionClosed::Failed(
            //         CallError::Failed(e.into()).into(),
            //     ));
            //     return;
            // }

            let (receiver, histories) =
                match context.get_broadcast(&game_addr, arg.settle_version).await {
                    Ok(x) => x,
                    Err(e) => {
                        sink.close(SubscriptionClosed::Failed(
                            CallError::Failed(e.into()).into(),
                        ));
                        return;
                    }
                };

            info!("Subscribe event stream: {:?}", game_addr);
            let rx = BroadcastStream::new(receiver);

            histories.into_iter().for_each(|x| {
                sink.send(&x)
                    .map_err(|e| {
                        error!("Error occurred when broadcasting event histories: {:?}", e);
                        e
                    })
                    .unwrap();
            });

            drop(context);

            match sink.pipe_from_try_stream(rx).await {
                SubscriptionClosed::Success => {
                    info!("Subscription closed successfully");
                    sink.close(SubscriptionClosed::Success);
                }
                SubscriptionClosed::RemotePeerAborted => {
                    warn!("Remote peer aborted");
                }
                SubscriptionClosed::Failed(err) => {
                    warn!("Subscription error: {:?}", err);
                    sink.close(err);
                }
            };
        });
        Ok(())
    }
}

pub async fn run_server(context: ApplicationContext) -> anyhow::Result<()> {
    let host = {
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
