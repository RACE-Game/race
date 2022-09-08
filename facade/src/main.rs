use jsonrpsee::server::{ServerBuilder, ServerHandle};
use jsonrpsee::types::Params;
use jsonrpsee::{core::Error, RpcModule};
use race_core::types::{
    CreateGameAccountParams, GameAccount, GameBundle, GetAccountInfoParams, GetGameBundleParams, JoinParams, Player,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

type Result<T> = std::result::Result<T, Error>;

const HTTP_HOST: &str = "127.0.0.1:12002";

#[derive(Default)]
pub struct Context {
    accounts: HashMap<String, GameAccount>,
    bundles: HashMap<String, GameBundle>,
}

async fn publish_game_bundle(params: Params<'_>, context: Arc<Mutex<Context>>) -> Result<String> {
    let bundle: GameBundle = params.one()?;
    let addr = bundle.addr.clone();
    let mut context = context.lock().await;
    context.bundles.insert(addr.clone(), bundle);
    Ok(addr)
}

async fn get_game_bundle(params: Params<'_>, context: Arc<Mutex<Context>>) -> Result<GameBundle> {
    let GetGameBundleParams { addr } = params.one()?;
    println!("Get game bundle: {:?}", addr);
    let context = context.lock().await;
    println!("Existing bundles: {:?}", context.bundles);
    if let Some(bundle) = context.bundles.get(&addr) {
        Ok(bundle.to_owned())
    } else {
        Err(Error::Custom("Game bundle not found".into()))
    }
}

async fn create_game(params: Params<'_>, context: Arc<Mutex<Context>>) -> Result<String> {
    let CreateGameAccountParams { addr, data } = params.one()?;
    println!("Create game: {:?}", addr);
    let account = GameAccount {
        addr: addr.clone(),
        settle_serial: 0,
        access_serial: 0,
        players: vec![],
        data,
    };
    let mut context = context.lock().await;
    context.accounts.insert(addr.clone(), account);
    Ok(addr)
}

async fn get_account_info(params: Params<'_>, context: Arc<Mutex<Context>>) -> Result<GameAccount> {
    let GetAccountInfoParams { addr } = params.one()?;
    let context = context.lock().await;
    if let Some(account) = context.accounts.get(&addr) {
        Ok(account.to_owned())
    } else {
        Err(Error::Custom("Not found".into()))
    }
}

async fn join(params: Params<'_>, context: Arc<Mutex<Context>>) -> Result<()> {
    let JoinParams {
        player_addr,
        game_addr,
        amount,
    } = params.one()?;
    let context = context.lock().await;
    // if let Some(account) = context.accounts.get(&addr) {
    //     let mut players = &mut account.players;
    //     if players.iter().any(|p| p.is_some_with(|p| p.addr.eq(player_addr))) {

    //     }
    // }
    Ok(())
}

async fn settle(params: Params<'_>, context: Arc<Mutex<Context>>) -> Result<()> {
    Ok(())
}

async fn run_server() -> anyhow::Result<ServerHandle> {
    let http_server = ServerBuilder::default().build(HTTP_HOST.parse::<SocketAddr>()?).await?;
    let context = Mutex::new(Context::default());
    let mut module = RpcModule::new(context);
    module.register_async_method("create_game", create_game)?;
    module.register_async_method("get_account_info", get_account_info)?;
    module.register_async_method("get_game_bundle", get_game_bundle)?;
    module.register_async_method("publish_game_bundle", publish_game_bundle)?;
    module.register_async_method("join", join)?;
    module.register_async_method("settle", settle)?;
    let handle = http_server.start(module)?;
    Ok(handle)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server_handle = run_server().await?;
    server_handle.stopped().await;
    Ok(())
}
