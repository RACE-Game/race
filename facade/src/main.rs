//! This facade server emulates the behavior of its blockchain counterparts.
//! It is supposed to be used for testing and developing.

mod db;
mod context;

use clap::{arg, Command};
use context::Context;
use db::{Nft, PlayerInfo};
use hyper::Method;
use jsonrpsee::server::{AllowHosts, ServerBuilder, ServerHandle};
use jsonrpsee::types::Params;
use jsonrpsee::{core::Error as RpcError, RpcModule};
use race_core::error::Error;
use race_core::types::RecipientSlotShare;
use race_core::types::{
    DepositParams, EntryType, GameAccount, GameRegistration, PlayerDeposit, PlayerJoin,
    PlayerProfile, RecipientAccount, RecipientSlot, RegistrationAccount, ServerAccount, ServerJoin,
    SettleParams, TokenAccount, Vote, VoteParams, VoteType,
};
use race_core::types::RecipientSlotInit;
use serde::Deserialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use tokio::sync::Mutex;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};

type RpcResult<T> = std::result::Result<T, RpcError>;

const DEFAULT_MAX_SERVERS: usize = 3;
const DEFAULT_VOTES_THRESHOLD: usize = 2;

const DEFAULT_BALANCE: u64 = 10000000;

const HTTP_HOST: &str = "0.0.0.0:12002";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameSpec {
    title: String,
    bundle: String,
    token: String,
    max_players: u16,
    entry_type: EntryType,
    data: Vec<u8>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinInstruction {
    player_addr: String,
    game_addr: String,
    position: u16,
    access_version: u64,
    amount: u64,
    verify_key: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServeInstruction {
    game_addr: String,
    server_addr: String,
    verify_key: String,
}

#[allow(unused)]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRecipientInstruction {
    pub recipient_addr: String,
    pub cap_addr: Option<String>,
    pub slots: Vec<RecipientSlotInit>,
}

#[allow(unused)]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddRecipientSlots {
    addr: String,
    recipient_addr: String,
    slot: RecipientSlot,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterServerInstruction {
    server_addr: String,
    endpoint: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePlayerProfileInstruction {
    player_addr: String,
    nick: String,
    pfp: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGameAccountInstruction {
    wallet_addr: String,
    game_addr: String,
    title: String,
    bundle_addr: String,
    token_addr: String,
    max_players: u16,
    entry_type: EntryType,
    data: Vec<u8>,
}


fn custom_error(e: Error) -> RpcError {
    RpcError::Custom(serde_json::to_string(&e).unwrap())
}

async fn get_game_bundle(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> RpcResult<Option<Vec<u8>>> {
    let addr: String = params.one()?;

    let context = context.lock().await;
    if let Some(bundle) = context.get_game_bundle(&addr)? {
        Ok(borsh::to_vec(&bundle).ok())
    } else {
        println!("? get_game_bundle, addr: {}, not found", addr);
        Ok(None)
    }
}

async fn get_registration_info(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> RpcResult<Option<Vec<u8>>> {
    let addr = params.one()?;
    let context = context.lock().await;
    let games = context
        .list_game_accounts()?
        .into_iter()
        .map(|g| GameRegistration {
            title: g.title,
            addr: g.addr,
            reg_time: 0,
            bundle_addr: g.bundle_addr,
        })
        .collect();
    Ok(Some(
        borsh::to_vec(&RegistrationAccount {
            addr,
            is_private: false,
            size: 100,
            owner: None,
            games,
        })
            .unwrap(),
    ))
}

async fn join(params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<()> {
    let JoinInstruction {
        game_addr,
        amount,
        access_version,
        position,
        player_addr,
        verify_key,
    } = params.one()?;
    let context = context.lock().await;
    if let Some(mut game_account) = context.get_game_account(&game_addr)? {
        if access_version != game_account.access_version {
            return Err(custom_error(Error::TransactionExpired));
        }

        if game_account.players.len() >= game_account.max_players as _ {
            return Err(custom_error(Error::GameIsFull(
                game_account.max_players as _,
            )));
        }

        if game_account
            .players
            .iter()
            .find(|p| p.addr.eq(&player_addr))
            .is_some() {
                return Err(custom_error(Error::PlayerAlreadyJoined(player_addr)));
            }


        // Find available position
        let mut pos_list = vec![position];
        pos_list.extend(0..100);
        let position = pos_list
            .into_iter()
            .find(|p| {
                game_account
                    .players
                    .iter()
                    .find(|player| player.position == *p)
                    .is_none()
            })
            .unwrap();

        match &game_account.entry_type {
            EntryType::Cash {
                min_deposit,
                max_deposit,
            } => {
                if amount < *min_deposit || amount > *max_deposit {
                    return Err(custom_error(Error::InvalidAmount));
                } else {
                    game_account.access_version += 1;

                    let player_join = PlayerJoin {
                        addr: player_addr.clone(),
                        position,
                        access_version: game_account.access_version,
                        verify_key,
                    };
                    let player_deposit = PlayerDeposit {
                        addr: player_addr.clone(),
                        amount,
                        settle_version: game_account.settle_version,
                    };
                    game_account.players.push(player_join);
                    game_account.deposits.push(player_deposit);
                    println!(
                        "! Join game: player: {}, game: {}, amount: {}, access version: {} -> {}",
                        player_addr,
                        game_addr,
                        amount,
                        game_account.access_version - 1,
                        game_account.access_version
                    );
                }
            }
            EntryType::Ticket { amount: ticket_amount } => {
                if *ticket_amount != amount {
                    return Err(custom_error(Error::InvalidAmount));
                } else {
                    game_account.access_version += 1;

                    let player_join = PlayerJoin {
                        addr: player_addr.clone(),
                        position,
                        access_version: game_account.access_version,
                        verify_key,
                    };
                    game_account.players.push(player_join);
                    println!(
                        "! Join game: player: {}, game: {}, amount: {},  access version: {} -> {}",
                        player_addr,
                        game_addr,
                        amount,
                        game_account.access_version - 1,
                        game_account.access_version
                    );
                }
            }
            #[allow(unused)]
            EntryType::Gating { collection } => todo!(),
            #[allow(unused)]
            EntryType::Disabled => todo!(),
        }
        context.update_game_account(&game_account)?;
        Ok(())
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
    println!(
        "! Deposit game: player: {}, game: {}, amount: {}",
        player_addr, game_addr, amount
    );
    let context = context.lock().await;
    let deposit = PlayerDeposit {
        addr: player_addr.clone(),
        amount,
        // Use a larger settle_version to indicate this deposit is not handled.
        settle_version: settle_version + 1,
    };
    if let Some(mut game_account) = context.get_game_account(&game_addr)? {
        if settle_version != game_account.settle_version {
            return Err(custom_error(Error::TransactionExpired));
        }
        if game_account.players.len() >= game_account.max_players as _ {
            return Err(custom_error(Error::GameIsFull(
                game_account.max_players as _,
            )));
        } else {
            game_account.deposits.push(deposit);
            context.update_game_account(&game_account)?;
            Ok(())
        }
    } else {
        return Err(custom_error(Error::GameAccountNotFound));
    }
}

async fn get_server_info(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> RpcResult<Option<Vec<u8>>> {
    let addr: String = params.one()?;
    let context = context.lock().await;
    if let Some(server) = context.get_server_account(&addr)? {
        Ok(Some(borsh::to_vec(&server).unwrap()))
    } else {
        println!("? get_server_info, addr: {}, not found", addr);
        Ok(None)
    }
}

async fn register_server(params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<()> {
    let RegisterServerInstruction {
        server_addr,
        endpoint,
    } = params.one()?;
    let server = ServerAccount {
        addr: server_addr.clone(),
        endpoint,
    };
    let context = context.lock().await;
    if context.get_server_account(&server_addr)?.is_none() {
        context.add_server(&server)?;
    }
    Ok(())
}

async fn create_account(params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<String> {
    let CreateGameAccountInstruction {
        wallet_addr,
        game_addr,
        title,
        bundle_addr,
        token_addr,
        max_players,
        entry_type,
        data,
    } = params.one()?;
    let context = context.lock().await;
    let game_account = GameAccount {
        addr: game_addr.clone(),
        title,
        bundle_addr,
        token_addr,
        owner_addr: wallet_addr,
        entry_type,
        max_players,
        data_len: data.len() as _,
        data,
        ..Default::default()
    };
    context.create_game_account(&game_account)?;
    Ok(game_addr)
}

async fn create_profile(params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<()> {
    let CreatePlayerProfileInstruction {
        player_addr,
        nick,
        pfp,
    } = params.one()?;
    let context = context.lock().await;
    let player_info = PlayerInfo {
        balances: HashMap::from([
            ("FACADE_USDC".to_string(), DEFAULT_BALANCE),
            ("FACADE_USDT".to_string(), DEFAULT_BALANCE),
            ("FACADE_NATIVE".to_string(), DEFAULT_BALANCE),
            ("FACADE_RACE".to_string(), DEFAULT_BALANCE),
        ]),
        nfts: HashMap::from([
            ("FACADE_NFT_1".to_string(), Nft {
                addr: "FACADE_NFT_1".to_string(),
                image: "https://qoyynvvrlnfmvsrie5f7esclpxj7zd2wzwt2neu2gmsdkefq.arweave.net/g7GG1rFbSsrKKCdL8-khLfdP-8j1-bNp6aSmjMkNRCw".to_string(),
                name: "FACADE NFT 01".to_string(),
                symbol: "FACADE NFT".to_string(),
                collection: Some("FACADE COLLECTION".to_string()),
            })
        ]),
        profile: PlayerProfile {
            addr: player_addr.clone(),
            nick,
            pfp,
        },
    };
    context.create_player_info(&player_info)?;

    Ok(())
}

async fn get_profile(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> RpcResult<Option<Vec<u8>>> {
    let addr: String = params.one()?;
    let context = context.lock().await;
    let ret = match context.get_player_info(&addr)? {
        Some(player_info) => Ok(Some(borsh::to_vec(&player_info.profile).unwrap())),
        None => Ok(None),
    };
    println!("? Player profile: {:?}", ret);
    ret
}

async fn vote(params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<()> {
    let VoteParams {
        vote_type,
        voter_addr,
        votee_addr,
        game_addr,
    } = params.one()?;
    println!(
        "! Vote for game {}, voter: {}, votee: {}, type: {:?}",
        game_addr, voter_addr, votee_addr, vote_type
    );
    let context = context.lock().await;
    if let Some(mut game_account) = context.get_game_account(&game_addr)? {
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

        // When there's enough votes, we can cancel the game
        if game_account.votes.len() >= DEFAULT_VOTES_THRESHOLD {
            println!("! Enough votes on game {}!", game_account.addr);
            game_account.transactor_addr = None;
            let unlock_time = std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
                + 60_000;
            game_account.unlock_time = Some(unlock_time as _);
        }
        context.update_game_account(&game_account)?;
    } else {
        return Err(custom_error(Error::GameAccountNotFound));
    }


    Ok(())
}

async fn serve(params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<()> {
    let ServeInstruction {
        game_addr,
        server_addr,
        verify_key,
    } = params.one()?;
    let context = context.lock().await;
    let mut is_transactor = false;

    let Some(server_account) = context.get_server_account(&server_addr)? else {
        return Err(custom_error(Error::ServerAccountNotFound));
    };

    let mut account = context.get_game_account(&game_addr)?
        .ok_or(custom_error(Error::GameAccountNotFound))?;

    let new_access_version = account.access_version + 1;

    if account.transactor_addr.is_none() {
        is_transactor = true;
        account.transactor_addr = Some(server_addr.clone());
    }

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
        if account.servers.len() >= DEFAULT_MAX_SERVERS {
            return Err(custom_error(Error::ServerQueueIsFull(
                DEFAULT_MAX_SERVERS as _,
            )));
        } else {
            account.access_version = new_access_version;
            account.servers.push(ServerJoin::new(
                server_addr.clone(),
                server_account.endpoint.clone(),
                new_access_version,
                verify_key,
            ));
        }
    }
    context.update_game_account(&account)?;

    println!(
        "! Serve game, server: {}, is_transactor: {}, access version: {} -> {}",
        server_addr,
        is_transactor,
        account.access_version - 1,
        account.access_version
    );
    Ok(())
}

async fn get_balance(params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<Vec<u8>> {
    let (player_addr, token_addr) = params.parse::<(String, String)>()?;
    let context = context.lock().await;
    let mut amount = 0u64;
    if let Some(player) = context.get_player_info(&player_addr)? {
        if let Some(balance) = player.balances.get(&token_addr) {
            amount = *balance;
        } else {
            println!("? get_balance, token_addr: {}, not found", token_addr);
        }
    } else {
        println!("? get_balance, player_addr: {}, not found", player_addr);
    }
    Ok(borsh::to_vec(&amount).unwrap())
}

async fn get_account_info(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> RpcResult<Option<Vec<u8>>> {
    let addr: String = params.one()?;
    let context = context.lock().await;
    if let Some(account) = context.get_game_account(&addr)? {
        Ok(Some(borsh::to_vec(&account).unwrap()))
    } else {
        println!("? get_account_info, addr: {}, not found", addr);
        Ok(None)
    }
}

async fn list_tokens(_params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<Vec<u8>> {
    let context = context.lock().await;
    let tokens: Vec<TokenAccount> = context.list_token_accounts()?;
    let bytes = borsh::to_vec(&tokens)?;
    Ok(bytes)
}

async fn get_player_info(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> RpcResult<Option<Vec<u8>>> {
    let addr: String = params.one()?;
    let context = context.lock().await;
    let Some(player) = context.get_player_info(&addr)? else {
        return Ok(None);
    };
    Ok(Some(borsh::to_vec(&player).unwrap()))
}

async fn get_recipient(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> RpcResult<Option<Vec<u8>>> {
    let addr: String = params.one()?;
    let context = context.lock().await;
    let Some(recipient) = context.get_recipient_account(&addr)? else {
        return Ok(None);
    };
    Ok(Some(borsh::to_vec(&recipient).unwrap()))
}

async fn create_recipient(params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<String> {
    let CreateRecipientInstruction {
        recipient_addr, cap_addr, slots
    } = params.one()?;

    let slots = slots.into_iter().map(|slot_init| {
        RecipientSlot {
            id: slot_init.id,
            slot_type: slot_init.slot_type,
            token_addr: slot_init.token_addr,
            shares: slot_init.init_shares.into_iter().map(|share_init| {
                RecipientSlotShare {
                    owner: share_init.owner,
                    weights: share_init.weights,
                    claim_amount: 0
                }
            }).collect(),
            balance: 0,
        }
    }).collect();

    let context = context.lock().await;
    let recipient_account = RecipientAccount { addr: recipient_addr.clone(), cap_addr, slots };
    context.create_recipient_account(&recipient_account)?;

    Ok(recipient_addr)
}

async fn settle(params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<String> {
    let SettleParams {
        addr,
        settles,
        transfers,
        checkpoint,
        settle_version,
        next_settle_version,
        entry_lock,
    } = params.one()?;
    println!(
        "! Handle settlements {}, settles: {:?}, transfers: {:?} ",
        addr, settles, transfers
    );

    // Simulate the finality time
    // tokio::time::sleep(Duration::from_secs(10)).await;
    // ---

    let context = context.lock().await;

    // The manipulation should be atomic.

    let mut game = context.get_game_account(&addr)?
        .ok_or(custom_error(Error::GameAccountNotFound))?;

    // Expire old deposits
    game.deposits
        .retain(|d| d.settle_version < game.settle_version);

    if game.settle_version != settle_version {
        println!("E The settle_versions mismach");
        return Err(custom_error(Error::InvalidSettle(format!(
            "Invalid settle version, current: {}, transaction: {}",
            game.settle_version, settle_version,
        ))));
    }

    // Set entry_lock
    if let Some(entry_lock) = entry_lock {
        game.entry_lock = entry_lock;
    }

    // Increase the `settle_version`
    game.settle_version = next_settle_version;
    println!("! Bump settle version to {}", game.settle_version);
    game.checkpoint_on_chain = Some(checkpoint);

    // Handle settles
    for s in settles.into_iter() {
        if let Some(index) = game.players.iter().position(|p| p.access_version.eq(&s.player_id)) {
            let p = game.players.remove(index);
            let mut player = context.get_player_info(&p.addr)?
                .ok_or(custom_error(Error::InvalidSettle(format!(
                    "Invalid player address: {}",
                    p.addr
                ))))?;
            player
                .balances
                .entry(game.token_addr.to_owned())
                .and_modify(|b| *b += s.amount);
            context.update_player_info(&player)?;
        } else {
            return Err(custom_error(Error::InvalidSettle("Math overflow".into())));
        }
    }

    context.update_game_account(&game)?;
    Ok(format!("facade_settle_{}", settle_version))
}

async fn run_server(context: Context) -> anyhow::Result<ServerHandle> {
    let cors = CorsLayer::new()
        .allow_methods([Method::POST])
        .allow_origin(Any)
        .allow_headers([hyper::header::CONTENT_TYPE]);
    let middleware = ServiceBuilder::new().layer(cors);

    let http_server = ServerBuilder::default()
        .max_response_body_size(64_000_000)
        .set_host_filtering(AllowHosts::Any)
        .set_middleware(middleware)
        .build(HTTP_HOST.parse::<SocketAddr>()?)
        .await?;
    let context = Mutex::new(context);
    let mut module = RpcModule::new(context);
    module.register_async_method("get_account_info", get_account_info)?;
    module.register_async_method("get_server_info", get_server_info)?;
    module.register_async_method("get_game_bundle", get_game_bundle)?;
    module.register_async_method("get_registration_info", get_registration_info)?;
    module.register_async_method("get_balance", get_balance)?;
    module.register_async_method("get_player_info", get_player_info)?;
    module.register_async_method("get_recipient", get_recipient)?;
    module.register_async_method("register_server", register_server)?;
    module.register_async_method("create_profile", create_profile)?;
    module.register_async_method("create_recipient", create_recipient)?;
    module.register_async_method("get_profile", get_profile)?;
    module.register_async_method("create_account", create_account)?;
    module.register_async_method("serve", serve)?;
    module.register_async_method("join", join)?;
    module.register_async_method("deposit", deposit)?;
    module.register_async_method("settle", settle)?;
    module.register_async_method("vote", vote)?;
    module.register_async_method("list_tokens", list_tokens)?;

    let handle = http_server.start(module)?;
    Ok(handle)
}

fn cli() -> Command {
    Command::new("facade")
        .about("A mock server for local development with Race")
        .arg(arg!(-g <game> ... "The path to a game spec json file"))
        .arg(arg!(-b <bundle> ... "The path to a wasm bundle"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Start at {}", HTTP_HOST);
    let matches = cli().get_matches();
    let context = Context::default();
    context.load_default_tokens()?;
    if let Some(game_spec_paths) = matches.get_many::<String>("game") {
        context.load_games(&game_spec_paths.map(String::as_str).collect::<Vec<&str>>())?;
    }
    if let Some(bundle_paths) = matches.get_many::<String>("bundle") {
        context.load_bundles(&bundle_paths.map(String::as_str).collect::<Vec<&str>>())?;
    }
    let server_handle = run_server(context).await?;
    server_handle.stopped().await;
    Ok(())
}
