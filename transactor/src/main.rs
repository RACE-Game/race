use std::{net::SocketAddr, sync::Arc};

use context::Context;
use jsonrpsee::core::Error;
use jsonrpsee::{server::ServerBuilder, types::Params, RpcModule};
use race_core::types::{AttachGameParams, SendEventParams};
use tokio::sync::Mutex;

mod component;
mod context;
mod runtime;

type Result<T> = std::result::Result<T, Error>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run_server().await?;
    Ok(())
}

const HTTP_HOST: &str = "127.0.0.1:12000";

async fn attach_game(params: Params<'_>, context: Arc<Mutex<Context>>) -> Result<()> {
    let params: AttachGameParams = params.one()?;
    println!("Attach game: {:?}", params);
    let mut context = context.lock().await;
    context
        .game_manager
        .start_game(params)
        .await
        .map_err(|e| Error::Custom(e.to_string()))
}

async fn send_event(params: Params<'_>, context: Arc<Mutex<Context>>) -> Result<()> {
    let params: SendEventParams = params.one()?;
    let context = context.lock().await;
    context
        .game_manager
        .send_event(&params.addr, params.event)
        .await
        .map_err(|e| Error::Custom(e.to_string()))
}

async fn run_server() -> anyhow::Result<()> {
    let server = ServerBuilder::default().build(HTTP_HOST.parse::<SocketAddr>()?).await?;
    let context = Mutex::new(Context::default());
    let mut module = RpcModule::new(context);

    module.register_async_method("attach_game", attach_game)?;
    module.register_async_method("send_event", send_event)?;

    let handle = server.start(module)?;
    println!("Server started");
    handle.stopped().await;
    Ok(())
}
