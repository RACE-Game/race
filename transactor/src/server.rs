use std::{net::SocketAddr, sync::Arc};

use crate::context::ApplicationContext;
use crate::utils;
use borsh::BorshDeserialize;
use borsh::BorshSerialize;
use hyper::Method;
use jsonrpsee::core::error::Error as RpcError;
use jsonrpsee::core::error::SubscriptionClosed;
use jsonrpsee::server::AllowHosts;
use jsonrpsee::types::error::CallError;
use jsonrpsee::types::SubscriptionEmptyError;
use jsonrpsee::SubscriptionSink;
use jsonrpsee::{server::ServerBuilder, types::Params, RpcModule};
use race_api::event::Message;
use race_core::types::SubmitMessageParams;
use race_core::types::{
    AttachGameParams, ExitGameParams, Signature, SubmitEventParams, SubscribeEventParams,
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
    let (_game_addr, AttachGameParams { signer, key }) = parse_params_no_sig(params)?;

    info!("Attach to game, signer: {}", signer);

    context
        .register_key(signer, key)
        .await
        .map_err(|e| RpcError::Call(CallError::Failed(e.into())))
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

async fn exit_game(params: Params<'_>, context: Arc<ApplicationContext>) -> Result<(), RpcError> {
    let (game_addr, ExitGameParams {}, sig) = parse_params(params, &context)?;
    info!("Exit game");

    context
        .eject_player(&game_addr, &sig.signer)
        .await
        .map_err(|e| RpcError::Call(CallError::Failed(e.into())))
}

fn subscribe_event(
    params: Params<'_>,
    mut sink: SubscriptionSink,
    context: Arc<ApplicationContext>,
) -> Result<(), SubscriptionEmptyError> {
    {
        let (game_addr, SubscribeEventParams { settle_version }) =
            parse_params_no_sig(params).or(Err(SubscriptionEmptyError))?;

        tokio::spawn(async move {
            let (receiver, histories) =
                match context.get_broadcast(&game_addr, settle_version).await {
                    Ok(x) => x,
                    Err(e) => {
                        sink.close(SubscriptionClosed::Failed(
                            CallError::Failed(e.into()).into(),
                        ));
                        return;
                    }
                };

            drop(context);

            info!(
                "Subscribe event stream, game: {:?}, settle version: {}, number of histories: {}",
                game_addr,
                settle_version,
                histories.len()
            );
            histories.into_iter().for_each(|x| {
                info!("Push history event: {}", x);
                let v = x.try_to_vec().unwrap();
                let s = utils::base64_encode(&v);
                // info!("Push event history: {}", s);
                sink.send(&s)
                    .map_err(|e| {
                        error!("Error occurred when broadcasting event histories: {:?}", e);
                        e
                    })
                    .unwrap();
            });

            let rx = BroadcastStream::new(receiver);
            let serialized_rx = rx.map(|f| match f {
                Ok(x) => {
                    let v = x.try_to_vec().unwrap();
                    let s = utils::base64_encode(&v);
                    info!("Push new event: {}", x);
                    Ok(s)
                }
                Err(e) => Err(e),
            });

            match sink.pipe_from_try_stream(serialized_rx).await {
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
    let cors = CorsLayer::new()
        .allow_methods([Method::POST])
        .allow_origin(Any)
        .allow_headers([hyper::header::CONTENT_TYPE]);

    let middleware = ServiceBuilder::new().layer(cors);

    let host = {
        let port = context.config.port;
        format!("0.0.0.0:{}", port)
    };

    let server = ServerBuilder::default()
        .set_host_filtering(AllowHosts::Any)
        .set_middleware(middleware)
        .max_request_body_size(100_1000)
        .build(host.parse::<SocketAddr>()?)
        .await?;
    let mut module = RpcModule::new(context);

    module.register_method("ping", ping)?;
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
    handle.stopped().await;
    Ok(())
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
