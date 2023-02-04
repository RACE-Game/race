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
    CreateGameAccountParams, CreatePlayerProfileParams, CreateRegistrationParams, DepositParams,
    GameAccount, GameBundle, GameRegistration, JoinParams, PlayerDeposit, PlayerJoin,
    PlayerProfile, RegisterGameParams, RegisterServerParams, RegistrationAccount, ServeParams,
    ServerAccount, ServerJoin, SettleOp, SettleParams, UnregisterGameParams,
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

const DEFAULT_BALANCE: u64 = 10000;
const HTTP_HOST: &str = "127.0.0.1:12002";
const DEFAULT_REGISTRATION_ADDRESS: &str = "DEFAULT_REGISTRATION_ADDRESS";
const COUNTER_GAME_ADDRESS: &str = "COUNTER_GAME_ADDRESS";
const COUNTER_BUNDLE_ADDRESS: &str = "COUNTER_BUNDLE_ADDRESS";
const SERVER_ADDRESS_1: &str = "SERVER_ADDRESS_1";
const SERVER_ADDRESS_2: &str = "SERVER_ADDRESS_2";
const DEFAULT_OWNER_ADDRESS: &str = "DEFAULT_OWNER_ADDRESS";

// Addresses for examples
const CHAT_BUNDLE_ADDRESS: &str = "CHAT_BUNDLE_ADDRESS";
const EXAMPLE_CHAT_ADDRESS: &str = "EXAMPLE_CHAT_ADDRESS";
const RAFFLE_BUNDLE_ADDRESS: &str = "RAFFLE_BUNDLE_ADDRESS";
const EXAMPLE_RAFFLE_ADDRESS: &str = "EXAMPLE_RAFFLE_ADDRESS";

#[derive(Clone)]
pub struct PlayerInfo {
    profile: PlayerProfile,
    balance: u64,
}

#[derive(Default)]
pub struct Context {
    players: HashMap<String, PlayerInfo>,
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
    let addr: String = params.one()?;
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
    let addr: String = params.one()?;
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
        title,
        max_players,
        bundle_addr,
        data,
    } = params.one()?;

    if !context.bundles.contains_key(&bundle_addr) {
        return Err(Error::Custom("Game bundle not exist!".into()));
    }

    let account = GameAccount {
        addr: addr.clone(),
        title,
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
    if let Some(game_account) = context.accounts.get_mut(&game_addr) {
        if access_version != game_account.access_version {
            return Err(Error::Custom("Expired transaction".into()));
        }
        if game_account.players.len() >= game_account.max_players as _ {
            Err(Error::Custom("Game is full".into()))
        } else if game_account
            .players
            .iter()
            .find(|p| p.addr.eq(&player_addr))
            .is_some()
        {
            Err(Error::Custom("Player already joined".into()))
        } else {
            let access_version = game_account.access_version + 1;
            let player_join = PlayerJoin {
                addr: player_addr.clone(),
                position,
                balance: amount,
                access_version,
            };
            game_account.players.push(player_join);
            game_account.access_version = access_version;
            println!("Player joined!");
            Ok(())
        }
    } else {
        Err(Error::Custom("Game not found".into()))
    }
}

async fn deposit(params: Params<'_>, context: Arc<Mutex<Context>>) -> Result<()> {
    let DepositParams {
        player_addr,
        game_addr,
        amount,
        settle_version,
    } = params.one()?;
    println!(
        "Join game: player: {:?}, game: {:?}, amount: {:?}",
        player_addr, game_addr, amount
    );
    let mut context = context.lock().await;
    let deposit = PlayerDeposit {
        addr: player_addr.clone(),
        amount,
        // Use a larger settle_version to indicate this deposit is not handled.
        settle_version: settle_version + 1,
    };
    if let Some(game_account) = context.accounts.get_mut(&game_addr) {
        if settle_version != game_account.settle_version {
            return Err(Error::Custom("Expired transaction".into()));
        }
        if game_account.players.len() >= game_account.max_players as _ {
            Err(Error::Custom("Game is full".into()))
        } else {
            game_account.deposits.push(deposit);
            Ok(())
        }
    } else {
        Err(Error::Custom("Game not found".into()))
    }
}

async fn get_server_info(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> Result<ServerAccount> {
    let addr: String = params.one()?;
    let context = context.lock().await;
    if let Some(transactor) = context.transactors.get(&addr) {
        Ok(transactor.to_owned())
    } else {
        println!("Fetch server failed");
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

async fn create_profile(params: Params<'_>, context: Arc<Mutex<Context>>) -> Result<()> {
    let CreatePlayerProfileParams { addr, nick, pfp } = params.one()?;
    let mut context = context.lock().await;
    context.players.insert(
        addr.clone(),
        PlayerInfo {
            balance: DEFAULT_BALANCE,
            profile: PlayerProfile { addr, nick, pfp },
        },
    );
    Ok(())
}

async fn get_profile(params: Params<'_>, context: Arc<Mutex<Context>>) -> Result<PlayerProfile> {
    let addr: String = params.one()?;
    let context = context.lock().await;
    match context.players.get(&addr) {
        Some(player_info) => Ok(player_info.profile.clone()),
        None => Err(Error::Custom("Player profile not found".into())),
    }
}

async fn serve(params: Params<'_>, context: Arc<Mutex<Context>>) -> Result<()> {
    let ServeParams {
        game_addr,
        server_addr,
    } = params.one()?;
    let mut context = context.lock().await;
    context
        .transactors
        .get(&server_addr)
        .ok_or(Error::Custom("Server not found".into()))?;
    let Context {
        transactors,
        ref mut accounts,
        ..
    } = &mut *context;

    let account = accounts
        .get_mut(&game_addr)
        .ok_or(Error::Custom("Account not found".into()))?;

    if account.transactor_addr.is_none() {
        println!(
            "Set game transactor, game: {:?}, transactor: {:?}",
            game_addr, server_addr
        );
        account.transactor_addr = Some(server_addr.clone());
    }

    let server_account = transactors
        .get(&server_addr)
        .ok_or(Error::Custom("Server not found".into()))?;

    if account
        .servers
        .iter()
        .find(|s| s.addr.eq(&server_addr))
        .is_some()
    {
        // Game is already served.
        // We just ignore
        // However, this transaction should be avoid.
    } else {
        // Should be larger in real case
        //
        if account.servers.len() >= 3 {
            return Err(Error::Custom("Server queue is full".into()));
        } else {
            account.access_version += 1;
            account.servers.push(ServerJoin::new(
                server_addr,
                server_account.endpoint.clone(),
                account.access_version,
            ));
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
    let title = game_acc.title.clone();

    let reg_acc = context
        .registrations
        .get_mut(&reg_addr)
        .ok_or(Error::Custom("Registration not found".into()))?;

    let game_reg = GameRegistration {
        addr: game_addr.clone(),
        title,
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
    let addr: String = params.one()?;
    let context = context.lock().await;
    if let Some(account) = context.accounts.get(&addr) {
        Ok(account.to_owned())
    } else {
        Err(Error::Custom("Not found".into()))
    }
}

async fn settle(params: Params<'_>, context: Arc<Mutex<Context>>) -> Result<()> {
    let SettleParams { addr, settles } = params.one()?;
    println!("Handle settlements {}, with {:?} ", addr, settles);
    let mut context = context.lock().await;
    let Context {
        ref mut accounts,
        ref mut players,
        ..
    } = &mut *context;

    // The manipulation should be atomic.
    let mut accounts = accounts.clone();
    let mut players = players.clone();

    let game = accounts
        .get_mut(&addr)
        .ok_or(Error::Custom("Game not found".into()))?;

    // Expire old deposits
    game.deposits
        .retain(|d| d.settle_version < game.settle_version);

    // Increase the `settle_version`
    game.settle_version += 1;

    // Handle settles
    for s in settles.into_iter() {
        match s.op {
            SettleOp::Eject => {
                // Remove player
                if let Some(index) = game.players.iter().position(|p| p.addr.eq(&s.addr)) {
                    let p = game.players.remove(index);
                    let player = players
                        .get_mut(&p.addr)
                        .ok_or(Error::Custom("Invalid settle".into()))?;
                    player.balance += p.balance;
                } else {
                    return Err(Error::Custom("Invalid settle".into()));
                }
            }
            SettleOp::Add(amount) => {
                let p = game
                    .players
                    .iter_mut()
                    .find(|p| p.addr.eq(&s.addr))
                    .ok_or(Error::Custom("Invalid settle".into()))?;
                p.balance = p
                    .balance
                    .checked_add(amount)
                    .ok_or(Error::Custom("Invalid settle".into()))?;
            }
            SettleOp::Sub(amount) => {
                let p = game
                    .players
                    .iter_mut()
                    .find(|p| p.addr.eq(&s.addr))
                    .ok_or(Error::Custom("Invalid settle".into()))?;
                p.balance = p
                    .balance
                    .checked_sub(amount)
                    .ok_or(Error::Custom("Invalid settle".into()))?;
            }
        }
    }

    context.players = players;
    context.accounts = accounts;
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
    module.register_async_method("get_server_info", get_server_info)?;
    module.register_async_method("get_game_bundle", get_game_bundle)?;
    module.register_async_method("get_registration_info", get_registration_info)?;
    module.register_async_method("publish_game_bundle", publish_game_bundle)?;
    module.register_async_method("register_server", register_server)?;
    module.register_async_method("create_registration", create_registration)?;
    module.register_async_method("register_game", register_game)?;
    module.register_async_method("unregister_game", unregister_game)?;
    module.register_async_method("create_profile", create_profile)?;
    module.register_async_method("get_profile", get_profile)?;
    module.register_async_method("serve", serve)?;
    module.register_async_method("join", join)?;
    module.register_async_method("deposit", deposit)?;
    module.register_async_method("settle", settle)?;
    let handle = http_server.start(module)?;
    Ok(handle)
}

fn add_bundle(ctx: &mut Context, path: &str, bundle_addr: &str) {
    let mut f = File::open(path).expect("race_example_chat.wasm not found");
    let mut data = vec![];
    f.read_to_end(&mut data).unwrap();
    let bundle = GameBundle {
        addr: bundle_addr.into(),
        data,
    };
    ctx.bundles.insert(bundle_addr.into(), bundle);
    println!("Added the bundle account at {}", bundle_addr);
}

fn add_game(ctx: &mut Context, title: &str, game_addr: &str, bundle_addr: &str, data: Vec<u8>) {
    let account = GameAccount {
        addr: game_addr.into(),
        title: title.into(),
        bundle_addr: bundle_addr.into(),
        settle_version: 0,
        access_version: 0,
        players: vec![],
        deposits: vec![],
        servers: vec![],
        transactor_addr: None,
        max_players: 20,
        data_len: data.len() as _,
        data,
    };
    ctx.accounts.insert(game_addr.into(), account);
    ctx.registrations
        .get_mut(DEFAULT_REGISTRATION_ADDRESS)
        .unwrap()
        .games
        .push(GameRegistration {
            title: title.into(),
            addr: game_addr.into(),
            reg_time: 0,
            bundle_addr: bundle_addr.into(),
        });
    println!("Add the game account at {}", game_addr);
}

fn add_bundle_and_game(
    ctx: &mut Context,
    path: &str,
    bundle_addr: &str,
    game_addr: &str,
    title: &str,
    data: Vec<u8>,
) {
    add_bundle(ctx, path, bundle_addr);
    add_game(ctx, title, game_addr, bundle_addr, data);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Start facade server at: {:?}", HTTP_HOST);
    let server_handle = run_server().await?;
    server_handle.stopped().await;
    Ok(())
}

pub fn setup(ctx: &mut Context) {
    let def_reg = RegistrationAccount {
        addr: DEFAULT_REGISTRATION_ADDRESS.into(),
        is_private: false,
        size: 10,
        owner: None,
        games: Vec::default(),
    };
    println!(
        "Default registration created at {:?}",
        DEFAULT_REGISTRATION_ADDRESS
    );
    ctx.registrations = HashMap::from([(DEFAULT_REGISTRATION_ADDRESS.into(), def_reg)]);

    add_bundle_and_game(
        ctx,
        "../target/wasm32-unknown-unknown/release/race_example_chat.wasm",
        CHAT_BUNDLE_ADDRESS,
        EXAMPLE_CHAT_ADDRESS,
        "Chat Room",
        vec![],
    );
    add_bundle_and_game(
        ctx,
        "../target/wasm32-unknown-unknown/release/race_example_raffle.wasm",
        RAFFLE_BUNDLE_ADDRESS,
        EXAMPLE_RAFFLE_ADDRESS,
        "Raffle",
        vec![],
    );

    let server1 = ServerAccount {
        addr: SERVER_ADDRESS_1.into(),
        owner_addr: DEFAULT_OWNER_ADDRESS.into(),
        endpoint: "ws://localhost:12003".into(),
    };
    println!("Transactor account created at {:?}", SERVER_ADDRESS_1);
    let server2 = ServerAccount {
        addr: SERVER_ADDRESS_2.into(),
        owner_addr: DEFAULT_OWNER_ADDRESS.into(),
        endpoint: "ws://localhost:12004".into(),
    };
    println!("Transactor account created at {:?}", SERVER_ADDRESS_2);

    ctx.transactors = HashMap::from([
        (SERVER_ADDRESS_1.into(), server1),
        (SERVER_ADDRESS_2.into(), server2),
    ]);
}
