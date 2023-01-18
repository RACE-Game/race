//! This server is the replacement for blockchains in testing and development
//!
//! A list of default accounts will be created during the start:
//! COUNTER_GAME_ADDRESS - A game account of counter example
//! COUNTER_BUNDLE_ADDRESS - The game bundle of counter example
//! DEFAULT_REGISTRATION_ADDRESS - The default registration account which contains all games above
//! DEFAULT_TRANSACTOR_ADDRESS - The address for a transactor
//! DEFAULT_OWNER_ADDRESS - The address of the owner

use jsonrpsee::server::{ServerBuilder, ServerHandle};
use jsonrpsee::types::Params;
use jsonrpsee::{core::Error, RpcModule};
use race_core::types::{
    CreateGameAccountParams, CreateRegistrationParams, GameAccount, GameBundle, GameRegistration,
    GetAccountInfoParams, GetGameBundleParams, GetRegistrationParams, GetTransactorInfoParams,
    JoinParams, PlayerDeposit, PlayerJoin, RegisterGameParams, RegisterServerParams,
    RegistrationAccount, ServeParams, ServerAccount, UnregisterGameParams,
};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use uuid::Uuid;

type Result<T> = std::result::Result<T, Error>;

const HTTP_HOST: &str = "127.0.0.1:12002";
const DEFAULT_REGISTRATION_ADDRESS: &str = "DEFAULT_REGISTRATION_ADDRESS";
const COUNTER_GAME_ADDRESS: &str = "COUNTER_GAME_ADDRESS";
const COUNTER_BUNDLE_ADDRESS: &str = "COUNTER_BUNDLE_ADDRESS";
const SERVER_ADDRESS_1: &str = "SERVER_ADDRESS_1";
const SERVER_ADDRESS_2: &str = "SERVER_ADDRESS_2";
const DEFAULT_OWNER_ADDRESS: &str = "DEFAULT_OWNER_ADDRESS";

#[derive(Default)]
pub struct Context {
    accounts: HashMap<String, GameAccount>,
    registrations: HashMap<String, RegistrationAccount>,
    transactors: HashMap<String, ServerAccount>,
    bundles: HashMap<String, GameBundle>,
}

fn random_addr() -> String {
    Uuid::new_v4().to_string()
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
    if let Some(bundle) = context.bundles.get(&addr) {
        Ok(bundle.to_owned())
    } else {
        Err(Error::Custom("Game bundle not found".into()))
    }
}

async fn get_registration_info(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> Result<RegistrationAccount> {
    let GetRegistrationParams { addr } = params.one()?;
    println!("Get registration account: {:?}", addr);
    let context = context.lock().await;
    if let Some(registration) = context.registrations.get(&addr) {
        Ok(registration.to_owned())
    } else {
        Err(Error::Custom("Registration not found".into()))
    }
}

async fn create_game(params: Params<'_>, context: Arc<Mutex<Context>>) -> Result<String> {
    let addr: String = random_addr();
    let mut context = context.lock().await;
    let CreateGameAccountParams {
        max_players,
        bundle_addr,
        data,
    } = params.one()?;

    if !context.bundles.contains_key(&bundle_addr) {
        return Err(Error::Custom("Game bundle not exist!".into()));
    }

    let account = GameAccount {
        addr: addr.clone(),
        bundle_addr,
        data_len: data.len() as u32,
        data,
        max_players,
        ..Default::default()
    };
    context.accounts.insert(addr.clone(), account);
    Ok(addr)
}

async fn create_registration(params: Params<'_>, context: Arc<Mutex<Context>>) -> Result<String> {
    let addr = random_addr();
    let mut context = context.lock().await;
    let CreateRegistrationParams { is_private, size } = params.one()?;
    let reg = RegistrationAccount {
        addr: addr.clone(),
        is_private,
        size,
        owner: None,
        games: vec![],
    };
    context.registrations.insert(addr.clone(), reg);
    Ok(addr)
}

async fn join(params: Params<'_>, context: Arc<Mutex<Context>>) -> Result<()> {
    let JoinParams {
        player_addr,
        game_addr,
        position,
        amount,
        access_version,
    } = params.one()?;
    println!(
        "Join game: player: {:?}, game: {:?}, amount: {:?}",
        player_addr, game_addr, amount
    );
    let mut context = context.lock().await;
    let player_join = PlayerJoin {
        addr: player_addr.clone(),
        position,
        access_version,
    };
    let player_deposit = PlayerDeposit {
        addr: player_addr,
        amount,
        access_version,
    };
    if let Some(game_account) = context.accounts.get_mut(&game_addr) {
        if game_account.players.len() >= game_account.max_players as _ {
            Err(Error::Custom("Game is full".into()))
        } else {
            game_account.players.push(player_join);
            game_account.deposits.push(player_deposit);
            Ok(())
        }
    } else {
        Err(Error::Custom("Game not found".into()))
    }
}

async fn get_transactor_info(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> Result<ServerAccount> {
    let GetTransactorInfoParams { addr } = params.one()?;
    let context = context.lock().await;
    if let Some(transactor) = context.transactors.get(&addr) {
        Ok(transactor.to_owned())
    } else {
        Err(Error::Custom("Not found".into()))
    }
}

async fn register_server(params: Params<'_>, context: Arc<Mutex<Context>>) -> Result<String> {
    let RegisterServerParams { endpoint } = params.one()?;
    let addr = random_addr();
    let transactor = ServerAccount {
        addr: addr.clone(),
        owner_addr: DEFAULT_OWNER_ADDRESS.into(),
        endpoint,
    };
    let mut context = context.lock().await;
    context.transactors.insert(addr.clone(), transactor);
    Ok(addr)
}

async fn serve(params: Params<'_>, context: Arc<Mutex<Context>>) -> Result<()> {
    let ServeParams {
        account_addr,
        transactor_addr,
    } = params.one()?;
    let mut context = context.lock().await;
    context
        .transactors
        .get(&transactor_addr)
        .ok_or(Error::Custom("Transactor not found".into()))?;
    let account = context
        .accounts
        .get_mut(&account_addr)
        .ok_or(Error::Custom("Account not found".into()))?;
    if account.server_addrs.contains(&transactor_addr) {
        return Err(Error::Custom(
            "Game is already served by this transactor".into(),
        ));
    } else {
        if account.server_addrs.len() >= 3 {
            return Err(Error::Custom("Transactor queue is full".into()));
        } else {
            account.server_addrs.push(transactor_addr);
        }
    }
    Ok(())
}

async fn register_game(params: Params<'_>, context: Arc<Mutex<Context>>) -> Result<()> {
    let RegisterGameParams {
        game_addr,
        reg_addr,
    } = params.one()?;
    let mut context = context.lock().await;

    let game_acc = context
        .accounts
        .get(&game_addr)
        .ok_or(Error::Custom("Game not found".into()))?;
    let bundle_addr = game_acc.bundle_addr.clone();

    let reg_acc = context
        .registrations
        .get_mut(&reg_addr)
        .ok_or(Error::Custom("Registration not found".into()))?;

    let game_reg = GameRegistration {
        addr: game_addr.clone(),
        reg_time: Instant::now().elapsed().as_secs(),
        bundle_addr,
    };

    if reg_acc.games.len() < reg_acc.size as _ {
        reg_acc.games.push(game_reg);
    }
    Ok(())
}

async fn unregister_game(params: Params<'_>, context: Arc<Mutex<Context>>) -> Result<()> {
    let UnregisterGameParams {
        game_addr,
        reg_addr,
    } = params.one()?;
    let mut context = context.lock().await;

    let reg_acc = context
        .registrations
        .get_mut(&reg_addr)
        .ok_or(Error::Custom("Registration not found".into()))?;

    reg_acc.games.retain(|gr| gr.addr.ne(&game_addr));
    Ok(())
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

#[allow(unused_variables)]
async fn settle(params: Params<'_>, context: Arc<Mutex<Context>>) -> Result<()> {
    Ok(())
}

async fn run_server() -> anyhow::Result<ServerHandle> {
    let http_server = ServerBuilder::default()
        .build(HTTP_HOST.parse::<SocketAddr>()?)
        .await?;
    let mut context = Context::default();
    setup(&mut context);
    let context = Mutex::new(context);
    let mut module = RpcModule::new(context);
    module.register_async_method("create_game", create_game)?;
    module.register_async_method("get_account_info", get_account_info)?;
    module.register_async_method("get_transactor_info", get_transactor_info)?;
    module.register_async_method("get_game_bundle", get_game_bundle)?;
    module.register_async_method("get_registration_info", get_registration_info)?;
    module.register_async_method("publish_game_bundle", publish_game_bundle)?;
    module.register_async_method("register_server", register_server)?;
    module.register_async_method("create_registration", create_registration)?;
    module.register_async_method("register_game", register_game)?;
    module.register_async_method("unregister_game", unregister_game)?;
    module.register_async_method("serve", serve)?;
    module.register_async_method("join", join)?;
    module.register_async_method("settle", settle)?;
    let handle = http_server.start(module)?;
    Ok(handle)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Start facade server at: {:?}", HTTP_HOST);
    let server_handle = run_server().await?;
    server_handle.stopped().await;
    Ok(())
}

pub fn setup(context: &mut Context) {
    let def_reg = RegistrationAccount {
        addr: DEFAULT_REGISTRATION_ADDRESS.into(),
        is_private: false,
        size: 10,
        owner: None,
        games: Vec::with_capacity(10),
    };
    println!(
        "Default registration created at {:?}",
        DEFAULT_REGISTRATION_ADDRESS
    );

    let mut f = File::open("../target/wasm32-unknown-unknown/release/race_example_counter.wasm")
        .expect("race_example_counter.wasm not found");
    let mut data = vec![];
    f.read_to_end(&mut data).unwrap();
    let counter_bundle = GameBundle {
        addr: COUNTER_BUNDLE_ADDRESS.into(),
        data,
    };
    println!("Counter bundle created at {:?}", COUNTER_BUNDLE_ADDRESS);

    let server1 = ServerAccount {
        addr: SERVER_ADDRESS_1.into(),
        owner_addr: DEFAULT_OWNER_ADDRESS.into(),
        endpoint: "localhost:12003".into(),
    };
    println!("Transactor account created at {:?}", SERVER_ADDRESS_1);
    let server2 = ServerAccount {
        addr: SERVER_ADDRESS_2.into(),
        owner_addr: DEFAULT_OWNER_ADDRESS.into(),
        endpoint: "localhost:12004".into(),
    };
    println!("Transactor account created at {:?}", SERVER_ADDRESS_2);

    let counter_game = GameAccount {
        addr: COUNTER_GAME_ADDRESS.into(),
        bundle_addr: COUNTER_BUNDLE_ADDRESS.into(),
        settle_version: 0,
        access_version: 0,
        players: vec![],
        deposits: vec![],
        server_addrs: vec![],
        transactor_addr: None,
        max_players: 10,
        data_len: 8,
        data: vec![1u8; 8],
    };
    println!("Counter game created at {:?}", COUNTER_GAME_ADDRESS);

    context.registrations = HashMap::from([(DEFAULT_REGISTRATION_ADDRESS.into(), def_reg)]);
    context.accounts = HashMap::from([(COUNTER_GAME_ADDRESS.into(), counter_game)]);
    context.bundles = HashMap::from([(COUNTER_BUNDLE_ADDRESS.into(), counter_bundle)]);
    context.transactors = HashMap::from([
        (SERVER_ADDRESS_1.into(), server1),
        (SERVER_ADDRESS_2.into(), server2),
    ]);
}
