use std::{net::SocketAddr, sync::Arc};

use crate::context::ApplicationContext;
use crate::utils;
use borsh::BorshDeserialize;
use hyper::Method;
use jsonrpsee::core::error::Error as RpcError;
use jsonrpsee::core::StringError;
use jsonrpsee::server::AllowHosts;
use jsonrpsee::types::error::CallError;
use jsonrpsee::types::ErrorObjectOwned;
use jsonrpsee::{server::ServerBuilder, types::Params, RpcModule};
use jsonrpsee::{PendingSubscriptionSink, SubscriptionMessage, TrySendError};
use race_api::event::Message;
use race_core::checkpoint::CheckpointOffChain;
use race_core::types::SubmitMessageParams;
use race_core::types::{
    AttachGameParams, CheckpointParams, ExitGameParams, Signature, SubmitEventParams,
    SubscribeEventParams,
};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tower::ServiceBuilder;
use tower_http::cors::Any;
use tower_http::cors::CorsLayer;
use tracing::{error, info, warn};

fn base64_decode(data: &str) -> Result<Vec<u8>, RpcError> {
    utils::base64_decode(data).map_err(|e| RpcError::Call(CallError::InvalidParams(e.into())))
}

fn parse_params_no_sig<T: BorshDeserialize>(params: Params<'_>) -> Result<(String, T), RpcError> {
    let (game_addr, arg_base64) = params.parse::<(String, String)>()?;

    let arg_vec = base64_decode(&arg_base64)?;

    let arg = T::try_from_slice(&arg_vec)
        .map_err(|e| RpcError::Call(CallError::InvalidParams(e.into())))?;
    Ok((game_addr, arg))
}

fn parse_params<T: BorshDeserialize>(
    params: Params<'_>,
    context: &ApplicationContext,
) -> Result<(String, T, Signature), RpcError> {
    let (game_addr, arg_base64, sig_base64) = params.parse::<(String, String, String)>()?;
    let arg_vec = base64_decode(&arg_base64)?;
    let sig_vec = base64_decode(&sig_base64)?;

    let signature = Signature::try_from_slice(&sig_vec)
        .map_err(|e| RpcError::Call(CallError::InvalidParams(e.into())))?;

    context
        .verify(&arg_vec, &signature)
        .map_err(|e| RpcError::Call(CallError::InvalidParams(e.into())))?;

    let arg = T::try_from_slice(&arg_vec)
        .map_err(|e| RpcError::Call(CallError::InvalidParams(e.into())))?;

    Ok((game_addr, arg, signature))
}

/// Ask transactor to load game and provide client's public key for further encryption.
async fn attach_game(params: Params<'_>, context: Arc<ApplicationContext>) -> Result<(), RpcError> {
    let (game_addr, AttachGameParams { signer, key }) = parse_params_no_sig(params)?;

    info!("Attach to game, signer: {}", signer);

    if !context.game_manager.is_game_loaded(&game_addr).await {
        return Err(RpcError::Custom("Game not loaded".to_string()));
    }

    context
        .register_key(signer, key)
        .await
        .map_err(|e| RpcError::Call(CallError::Failed(e.into())))?;

    Ok(())
}

fn ping(_: Params<'_>, _: &ApplicationContext) -> Result<String, RpcError> {
    Ok("pong".to_string())
}

async fn submit_message(
    params: Params<'_>,
    context: Arc<ApplicationContext>,
) -> Result<(), RpcError> {
    let (game_addr, SubmitMessageParams { content }, sig) = parse_params(params, &context)?;

    let sender = sig.signer;
    info!("Player message, {}: {}", sender, content);
    let message = Message { content, sender };

    context
        .send_message(&game_addr, message)
        .await
        .map_err(|e| RpcError::Call(CallError::Failed(e.into())))
}

async fn submit_event(
    params: Params<'_>,
    context: Arc<ApplicationContext>,
) -> Result<(), RpcError> {
    let (game_addr, SubmitEventParams { event }, _sig) = parse_params(params, &context)?;

    info!("Submit event, game_addr: {}, event: {}", game_addr, event);

    context
        .send_event(&game_addr, event)
        .await
        .map_err(|e| RpcError::Call(CallError::Failed(e.into())))
}

async fn get_checkpoint(
    params: Params<'_>,
    context: Arc<ApplicationContext>,
) -> Result<Option<Vec<u8>>, RpcError> {
    let (game_addr, CheckpointParams { settle_version }) = parse_params_no_sig(params)?;

    info!("Get checkpoint, game_addr: {}", game_addr);

    let checkpoint: Option<CheckpointOffChain> = context
        .game_manager
        .get_checkpoint(&game_addr, settle_version)
        .await
        .map_err(|e| RpcError::Call(CallError::Failed(e.into())))?;

    let bs = checkpoint
        .map(|c| borsh::to_vec(&c).map_err(|e| RpcError::Call(CallError::Failed(e.into()))))
        .transpose()?;

    Ok(bs)
}

async fn get_latest_checkpoints(params: Params<'_>, context: Arc<ApplicationContext>) -> Result<Vec<u8>, RpcError> {
    let game_addrs = params.parse::<Vec<String>>()?;
    let mut result = Vec::with_capacity(game_addrs.len());

    for addr in game_addrs {
        let checkpoint: Option<CheckpointOffChain> = context
            .game_manager
            .get_latest_checkpoint(&addr)
            .await
            .ok()
            .flatten();
        result.push(checkpoint);
    }
    let bs = borsh::to_vec(&result).map_err(|e| RpcError::Call(CallError::Failed(e.into())))?;
    Ok(bs)
}

async fn exit_game(params: Params<'_>, context: Arc<ApplicationContext>) -> Result<(), RpcError> {
    let (game_addr, ExitGameParams {}, sig) = parse_params(params, &context)?;
    info!("Exit game");

    context
        .eject_player(&game_addr, &sig.signer)
        .await
        .map_err(|e| RpcError::Call(CallError::Failed(e.into())))
}

async fn subscribe_event(
    params: Params<'_>,
    pending: PendingSubscriptionSink,
    context: Arc<ApplicationContext>,
) -> Result<(), StringError> {
    {
        let (game_addr, SubscribeEventParams { settle_version }) = match parse_params_no_sig(params)
        {
            Ok(p) => p,
            Err(e) => {
                let _ = pending.reject(ErrorObjectOwned::from(e)).await;
                return Ok(());
            }
        };

        let (receiver, backlogs_frame) =
            match context.get_broadcast(&game_addr, settle_version).await {
                Ok(x) => x,
                Err(e) => {
                    warn!("Game not found: {}", game_addr);
                    let _ = pending.reject(CallError::Failed(e.into())).await;
                    return Ok(());
                }
            };

        drop(context);
        info!(
            "Subscribe event stream, game: {:?}, settle version: {}",
            game_addr, settle_version,
        );

        let mut sink = pending.accept().await?;

        let v = borsh::to_vec(&backlogs_frame).unwrap();
        let s = utils::base64_encode(&v);
        sink.send(SubscriptionMessage::from(&s))
            .await
            .map_err(|e| {
                error!("Error occurred when broadcasting historical frame: {:?}", e);
                e
            })
            .unwrap();

        let rx = BroadcastStream::new(receiver);
        let mut serialized_rx = rx.map(|f| match f {
            Ok(x) => {
                let v = borsh::to_vec(&x).unwrap();
                let s = utils::base64_encode(&v);
                Ok(s)
            }
            Err(e) => Err(e),
        });

        loop {
            tokio::select! {
                _ = sink.closed() => break Err(anyhow::anyhow!("Subscription was closed")),
                maybe_item = serialized_rx.next() => {
                    let item = match maybe_item {
                        Some(Ok(item)) => item,
                        _ => break Err(anyhow::anyhow!("Event stream ended")),
                    };
                    let msg = SubscriptionMessage::from(&item);
                    match sink.try_send(msg) {
                        Ok(_) => (),
                        Err(TrySendError::Closed(_)) => break Err(anyhow::anyhow!("Client disconnected, subscription closed")),
                        Err(TrySendError::Full(_)) => {
                            warn!("TrySendError::Full");
                        }
                    }
                },
            }
        }?;
        Ok(())
    }
}

pub async fn run_server(
    context: ApplicationContext,
) -> anyhow::Result<tokio::task::JoinHandle<()>> {
    let cors = CorsLayer::new()
        .allow_methods([Method::POST, Method::OPTIONS])
        .allow_origin(Any)
        .allow_headers([hyper::header::CONTENT_TYPE]);

    let middleware = ServiceBuilder::new().layer(cors);

    let host = {
        let port = context.config.port;
        format!("0.0.0.0:{}", port)
    };

    let server = ServerBuilder::default()
        .max_connections(500)
        .set_host_filtering(AllowHosts::Any)
        .set_middleware(middleware)
        .max_request_body_size(100_1000)
        .build(host.parse::<SocketAddr>()?)
        .await?;

    let mut shutdown_rx = context.get_shutdown_receiver();

    let mut module = RpcModule::new(context);

    module.register_method("ping", ping)?;
    module.register_async_method("get_checkpoint", get_checkpoint)?;
    module.register_async_method("get_latest_checkpoints", get_latest_checkpoints)?;
    module.register_async_method("attach_game", attach_game)?;
    module.register_async_method("submit_event", submit_event)?;
    module.register_async_method("submit_message", submit_message)?;
    module.register_async_method("exit_game", exit_game)?;
    module.register_subscription(
        "subscribe_event",
        "s_event",
        "unsubscribe_event",
        subscribe_event,
    )?;
    let handle = server.start(module)?;
    info!("Server started at {:?}", host);

    Ok(tokio::spawn(async move {
        shutdown_rx
            .changed()
            .await
            .expect("Server listens to shutdown signal");
        info!("Stop jsonrpc server");
        handle.stop().expect("Stop jsonrpc server");
        handle.stopped().await;
        info!("Jsonrpc server stopped");
    }))
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test() {
        let data = "ABUAAABIVWlXWm1zRm00SXhWLVpndmVCR1EJAAAAAGQAAAAAAAAA";
        let v = base64_decode(data).unwrap();
        let p = SubmitEventParams::try_from_slice(&v);
        println!("Params: {:?}", p);
        let sig = "FQAAAEhVaVdabXNGbTRJeFYtWmd2ZUJHUTutKUmIAQAAQAAAALUql7fxjNhbQtNq2M5xKe9SnAz5ZEchVxTcxfAEDpg9Dx4RlFTr7tx+M5BhUw3fddmVsmiWzJXmi/4mr5SgJss=";
        let v = base64_decode(sig).unwrap();
        let s = Signature::try_from_slice(&v);
        println!("Signature: {:?}", s);
    }
}
