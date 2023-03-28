//! This server is the replacement for blockchains in testing and development
//!
//! A list of default accounts will be created during the start:
//! COUNTER_GAME_ADDRESS - A game account of counter example
//! COUNTER_BUNDLE_ADDRESS - The game bundle of counter example
//! DEFAULT_REGISTRATION_ADDRESS - The default registration account which contains all games above
//! DEFAULT_TRANSACTOR_ADDRESS - The address for a transactor
//! DEFAULT_OWNER_ADDRESS - The address of the owner

mod database;

use base64::Engine;
use jsonrpsee::server::{ServerBuilder, ServerHandle};
use jsonrpsee::types::Params;
use jsonrpsee::{core::Error as RpcError, RpcModule};
use race_core::error::Error;
use race_core::types::{
    CreateGameAccountParams, CreatePlayerProfileParams, CreateRegistrationParams, DepositParams,
    GameAccount, GameBundle, GameRegistration, JoinParams, PlayerDeposit, PlayerJoin,
    PlayerProfile, RegisterGameParams, RegisterServerParams, RegistrationAccount, ServeParams,
    ServerAccount, ServerJoin, SettleOp, SettleParams, UnregisterGameParams, Vote, VoteParams,
    VoteType,
};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Instant, UNIX_EPOCH};
use tokio::sync::Mutex;
use tracing::{debug, info};
use uuid::Uuid;

type RpcResult<T> = std::result::Result<T, RpcError>;

// const DEFAULT_MAX_PLAYERS: usize = 10;
const DEFAULT_MAX_SERVERS: usize = 3;
const DEFAULT_VOTES_THRESHOLD: usize = 2;

const DEFAULT_BALANCE: u64 = 10000;

const HTTP_HOST: &str = "0.0.0.0:12002";
const DEFAULT_REGISTRATION_ADDRESS: &str = "DEFAULT_REGISTRATION";
// const COUNTER_GAME_ADDRESS: &str = "COUNTER_GAME_ADDRESS";
// const COUNTER_BUNDLE_ADDRESS: &str = "COUNTER_BUNDLE_ADDRESS";
const SERVER_ADDRESS_1: &str = "SERVER_ADDRESS_1";
const SERVER_ADDRESS_2: &str = "SERVER_ADDRESS_2";
const DEFAULT_OWNER_ADDRESS: &str = "DEFAULT_OWNER";

// Addresses for examples
const CHAT_BUNDLE_ADDRESS: &str = "CHAT_BUNDLE";
const EXAMPLE_CHAT_ADDRESS: &str = "EXAMPLE_CHAT";
const RAFFLE_BUNDLE_ADDRESS: &str = "RAFFLE_BUNDLE";
const EXAMPLE_RAFFLE_ADDRESS: &str = "EXAMPLE_RAFFLE";
const DRAW_CARD_BUNDLE_ADDRESS: &str = "DRAW_CARD_BUNDLE";
const EXAMPLE_DRAW_CARD_ADDRESS: &str = "EXAMPLE_DRAW_CARD";

#[derive(Clone)]
pub struct PlayerInfo {
    profile: PlayerProfile,
    balance: u64,
}

// #[derive(Default)]
pub struct Context {
    players: HashMap<String, PlayerInfo>,
    accounts: HashMap<String, GameAccount>,
    registrations: HashMap<String, RegistrationAccount>,
    transactors: HashMap<String, ServerAccount>,
    bundles: HashMap<String, GameBundle>,
    db_pool: Pool<Sqlite>,
}

impl Context {
    fn new(pool: Pool<Sqlite>) -> Context {
        Context {
            players: HashMap::default(),
            accounts: HashMap::default(),
            registrations: HashMap::default(),
            transactors: HashMap::default(),
            bundles: HashMap::default(),
            db_pool: pool,
        }
    }
}

fn random_addr() -> String {
    Uuid::new_v4().to_string()
}

fn custom_error(e: Error) -> RpcError {
    RpcError::Custom(serde_json::to_string(&e).unwrap())
}

async fn publish_game_bundle(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> RpcResult<String> {
    let bundle: GameBundle = params.one()?;
    let addr = bundle.addr.clone();
    let mut context = context.lock().await;
    context.bundles.insert(addr.clone(), bundle.clone());
    database::context::create_game_bundle(&context.db_pool, bundle.clone()).await?;
    Ok(addr)
}

async fn get_game_bundle(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> RpcResult<GameBundle> {
    let addr: String = params.one()?;
    debug!("Get game bundle: {:?}", addr);
    let context = context.lock().await;
    if let Some(bundle) = database::context::get_game_bundle_by_addr(&context.db_pool, &addr).await {
        Ok(bundle.to_owned())
    } else {
        Err(custom_error(Error::GameBundleNotFound))
    }

    // if let Some(bundle) = context.bundles.get(&addr) {
    //     Ok(bundle.to_owned())
    // } else {
    //     Err(custom_error(Error::GameBundleNotFound))
    // }
}

async fn get_registration_info(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> RpcResult<RegistrationAccount> {
    let addr: String = params.one()?;
    debug!("Get registration account: {:?}", addr);
    let context = context.lock().await;
    if let Some(registration) = context.registrations.get(&addr) {
        Ok(registration.to_owned())
    } else {
        Err(custom_error(Error::RegistrationNotFound))
    }
}

async fn create_game(params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<String> {
    let addr: String = random_addr();
    let mut context = context.lock().await;
    let CreateGameAccountParams {
        title,
        max_players,
        bundle_addr,
        data,
    } = params.one()?;

    if !context.bundles.contains_key(&bundle_addr) {
        return Err(custom_error(Error::GameBundleNotFound));
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

async fn create_registration(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> RpcResult<String> {
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

async fn join(params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<()> {
    let JoinParams {
        player_addr,
        game_addr,
        position,
        amount,
        access_version,
    } = params.one()?;
    info!(
        "Join game: player: {}, game: {}, amount: {}",
        player_addr, game_addr, amount
    );
    let mut context = context.lock().await;
    if let Some(game_account) = context.accounts.get_mut(&game_addr) {
        if access_version != game_account.access_version {
            return Err(custom_error(Error::TransactionExpired));
        }
        if game_account.players.len() >= game_account.max_players as _ {
            return Err(custom_error(Error::GameIsFull(
                game_account.max_players as _,
            )));
        } else if game_account
            .players
            .iter()
            .find(|p| p.addr.eq(&player_addr))
            .is_some()
        {
            return Err(custom_error(Error::PlayerAlreadyJoined(player_addr)));
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
            Ok(())
        }
    } else {
        return Err(custom_error(Error::GameAccountNotFound));
    }
}

async fn deposit(params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<()> {
    let DepositParams {
        player_addr,
        game_addr,
        amount,
        settle_version,
    } = params.one()?;
    info!(
        "Deposit game: player: {}, game: {}, amount: {}",
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
            return Err(custom_error(Error::TransactionExpired));
        }
        if game_account.players.len() >= game_account.max_players as _ {
            return Err(custom_error(Error::GameIsFull(
                game_account.max_players as _,
            )));
        } else {
            game_account.deposits.push(deposit);
            Ok(())
        }
    } else {
        return Err(custom_error(Error::GameAccountNotFound));
    }
}

async fn get_server_info(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> RpcResult<ServerAccount> {
    let addr: String = params.one()?;
    let context = context.lock().await;
    if let Some(transactor) = context.transactors.get(&addr) {
        Ok(transactor.to_owned())
    } else {
        Err(custom_error(Error::ServerAccountNotFound))
    }
}

async fn register_server(params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<String> {
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

async fn create_profile(params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<()> {
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

async fn get_profile(params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<PlayerProfile> {
    let addr: String = params.one()?;
    let context = context.lock().await;
    match context.players.get(&addr) {
        Some(player_info) => Ok(player_info.profile.clone()),
        None => Err(custom_error(Error::PlayerProfileNotFound)),
    }
}

async fn vote(params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<()> {
    let VoteParams {
        vote_type,
        voter_addr,
        votee_addr,
        game_addr,
    } = params.one()?;
    info!(
        "Vote for game {}, voter: {}, votee: {}, type: {:?}",
        game_addr, voter_addr, votee_addr, vote_type
    );
    let mut context = context.lock().await;
    let Context {
        ref mut accounts,
        ref mut players,
        ..
    } = &mut *context;
    if let Some(game_account) = accounts.get_mut(&game_addr) {
        // Check if game is served
        if let Some(ref transactor_addr) = game_account.transactor_addr {
            if transactor_addr.ne(&votee_addr) {
                return Err(custom_error(Error::InvalidVotee(votee_addr)));
            }
        } else {
            return Err(custom_error(Error::GameNotServed));
        }

        // Check voter
        match vote_type {
            VoteType::ServerVoteTransactorDropOff => {
                // Check if server is in game
                if game_account
                    .servers
                    .iter()
                    .skip(1)
                    .find(|s| s.addr.eq(&voter_addr))
                    .is_none()
                {
                    return Err(custom_error(Error::InvalidVoter(voter_addr)));
                }
            }
            VoteType::ClientVoteTransactorDropOff => {
                // Check if client is in game
                if game_account
                    .players
                    .iter()
                    .find(|p| p.addr.eq(&voter_addr))
                    .is_none()
                {
                    return Err(custom_error(Error::InvalidVoter(voter_addr)));
                }
            }
        }

        // Check if vote is duplicated
        if game_account
            .votes
            .iter()
            .find(|v| v.voter.eq(&voter_addr))
            .is_some()
        {
            return Err(custom_error(Error::DuplicatedVote));
        }

        game_account.votes.push(Vote {
            voter: voter_addr.clone(),
            votee: votee_addr.clone(),
            vote_type,
        });

        // When there's enough votes, we can cancel the game, and eject all players and servers.
        // The server account should be slashed.
        if game_account.votes.len() >= DEFAULT_VOTES_THRESHOLD {
            for p in game_account.players.iter() {
                let player = players.get_mut(&p.addr).unwrap();
                player.balance += p.balance;
            }
            game_account.players.clear();
            game_account.servers.clear();
            game_account.transactor_addr = None;
            let unlock_time = std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
                + 60_000;
            game_account.unlock_time = Some(unlock_time as _);
        }
    } else {
        return Err(custom_error(Error::GameAccountNotFound));
    }

    Ok(())
}

async fn serve(params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<()> {
    let ServeParams {
        game_addr,
        server_addr,
    } = params.one()?;
    let mut context = context.lock().await;

    if !context.transactors.contains_key(&server_addr) {
        return Err(custom_error(Error::ServerAccountNotFound));
    }

    let Context {
        transactors,
        ref mut accounts,
        ..
    } = &mut *context;

    let account = accounts
        .get_mut(&game_addr)
        .ok_or(custom_error(Error::GameAccountNotFound))?;

    if account.transactor_addr.is_none() {
        info!(
            "Set game transactor, game: {}, transactor: {}",
            game_addr, server_addr
        );
        account.transactor_addr = Some(server_addr.clone());
    }

    let server_account = transactors
        .get(&server_addr)
        .ok_or(custom_error(Error::ServerAccountNotFound))?;

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
        if account.servers.len() >= DEFAULT_MAX_SERVERS {
            return Err(custom_error(Error::ServerQueueIsFull(
                DEFAULT_MAX_SERVERS as _,
            )));
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

async fn register_game(params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<()> {
    let RegisterGameParams {
        game_addr,
        reg_addr,
    } = params.one()?;
    let mut context = context.lock().await;

    let game_acc = context
        .accounts
        .get(&game_addr)
        .ok_or(custom_error(Error::GameAccountNotFound))?;
    let bundle_addr = game_acc.bundle_addr.clone();
    let title = game_acc.title.clone();

    let reg_acc = context
        .registrations
        .get_mut(&reg_addr)
        .ok_or(custom_error(Error::RegistrationNotFound))?;

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

async fn unregister_game(params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<()> {
    let UnregisterGameParams {
        game_addr,
        reg_addr,
    } = params.one()?;
    let mut context = context.lock().await;

    let reg_acc = context
        .registrations
        .get_mut(&reg_addr)
        .ok_or(custom_error(Error::RegistrationNotFound))?;

    reg_acc.games.retain(|gr| gr.addr.ne(&game_addr));
    Ok(())
}

async fn get_account_info(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> RpcResult<GameAccount> {
    let addr: String = params.one()?;
    let context = context.lock().await;
    if let Some(account) = context.accounts.get(&addr) {
        Ok(account.to_owned())
    } else {
        Err(custom_error(Error::GameAccountNotFound))
    }
}

async fn settle(params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<()> {
    let SettleParams { addr, settles } = params.one()?;
    info!("Handle settlements {}, with {:?} ", addr, settles);
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
        .ok_or(custom_error(Error::GameAccountNotFound))?;

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
                        .ok_or(custom_error(Error::InvalidSettle))?;
                    player.balance += p.balance;
                } else {
                    return Err(custom_error(Error::InvalidSettle));
                }
            }
            SettleOp::Add(amount) => {
                let p = game
                    .players
                    .iter_mut()
                    .find(|p| p.addr.eq(&s.addr))
                    .ok_or(custom_error(Error::InvalidSettle))?;
                p.balance = p
                    .balance
                    .checked_add(amount)
                    .ok_or(custom_error(Error::InvalidSettle))?;
            }
            SettleOp::Sub(amount) => {
                let p = game
                    .players
                    .iter_mut()
                    .find(|p| p.addr.eq(&s.addr))
                    .ok_or(custom_error(Error::InvalidSettle))?;
                p.balance = p
                    .balance
                    .checked_sub(amount)
                    .ok_or(custom_error(Error::InvalidSettle))?;
            }
        }
    }

    context.players = players;
    context.accounts = accounts;
    Ok(())
}

async fn run_server() -> anyhow::Result<ServerHandle> {
    tracing_subscriber::fmt::init();
    let http_server = ServerBuilder::default()
        .build(HTTP_HOST.parse::<SocketAddr>()?)
        .await?;

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite::memory:")
        .await?;

    let mut context = Context::new(pool);
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
    module.register_async_method("vote", vote)?;
    let handle = http_server.start(module)?;
    Ok(handle)
}

fn add_bundle(ctx: &mut Context, path: &str, bundle_addr: &str) {
    let mut f = File::open(path).expect("race_example_chat.wasm not found");
    let mut data = vec![];
    f.read_to_end(&mut data).unwrap();
    let base64 = base64::prelude::BASE64_STANDARD;
    let data = base64.encode(data);
    let bundle = GameBundle {
        addr: bundle_addr.into(),
        data,
    };
    ctx.bundles.insert(bundle_addr.into(), bundle);
    info!("Added the bundle account at {}", bundle_addr);
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
        votes: vec![],
        unlock_time: None,
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
    info!("Added the game account at {}", game_addr);
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
    info!("Start facade server at: {:?}", HTTP_HOST);
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
    info!(
        "Default registration created at {:?}",
        DEFAULT_REGISTRATION_ADDRESS
    );
    ctx.registrations = HashMap::from([(DEFAULT_REGISTRATION_ADDRESS.into(), def_reg)]);

    // add_bundle_and_game(
    //     ctx,
    //     "./target/race_example_chat.wasm",
    //     CHAT_BUNDLE_ADDRESS,
    //     EXAMPLE_CHAT_ADDRESS,
    //     "Chat Room",
    //     vec![],
    // );
    // add_bundle_and_game(
    //     ctx,
    //     "./target/race_example_raffle.wasm",
    //     RAFFLE_BUNDLE_ADDRESS,
    //     EXAMPLE_RAFFLE_ADDRESS,
    //     "Raffle",
    //     vec![],
    // );
    add_bundle_and_game(
        ctx,
        "./target/race_example_draw_card.wasm",
        DRAW_CARD_BUNDLE_ADDRESS,
        EXAMPLE_DRAW_CARD_ADDRESS,
        "Draw Card",
        vec![
            100, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 232, 3, 0, 0, 0, 0, 0, 0,
        ],
    );

    let server1 = ServerAccount {
        addr: SERVER_ADDRESS_1.into(),
        owner_addr: DEFAULT_OWNER_ADDRESS.into(),
        endpoint: "ws://localhost:12003".into(),
    };
    info!("Transactor account created at {:?}", SERVER_ADDRESS_1);
    let server2 = ServerAccount {
        addr: SERVER_ADDRESS_2.into(),
        owner_addr: DEFAULT_OWNER_ADDRESS.into(),
        endpoint: "ws://localhost:12004".into(),
    };
    info!("Transactor account created at {:?}", SERVER_ADDRESS_2);

    ctx.transactors = HashMap::from([
        (SERVER_ADDRESS_1.into(), server1),
        (SERVER_ADDRESS_2.into(), server2),
    ]);
}
