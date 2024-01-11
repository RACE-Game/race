//! This facade server emulates the behavior of its blockchain counterparts.
//! It is supposed to be used for testing and developing.

use borsh::BorshSerialize;
use clap::{Command, arg};
use hyper::Method;
use jsonrpsee::server::{AllowHosts, ServerBuilder, ServerHandle};
use jsonrpsee::types::Params;
use jsonrpsee::{core::Error as RpcError, RpcModule};
use race_api::error::Error;
use race_core::types::{
    DepositParams, EntryType, GameAccount, GameBundle, GameRegistration, PlayerDeposit, PlayerJoin,
    PlayerProfile, RecipientSlot, RegistrationAccount, ServerAccount, ServerJoin, SettleOp,
    SettleParams, TokenAccount, Vote, VoteParams, VoteType,
};
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};
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
    addr: String,
    creator_addr: String,
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

#[derive(Default)]
pub struct Context {
    tokens: HashMap<String, TokenAccount>,
    players: HashMap<String, PlayerInfo>,
    servers: HashMap<String, ServerAccount>,
    games: HashMap<String, GameAccount>,
    bundles: HashMap<String, GameBundle>,
}

#[derive(Clone, BorshSerialize)]
pub struct Nft {
    addr: String,
    image: String,
    name: String,
    symbol: String,
    collection: Option<String>,
}

#[derive(Clone, BorshSerialize)]
pub struct PlayerInfo {
    balances: HashMap<String, u64>, // token address to balance
    nfts: HashMap<String, Nft>,
    profile: PlayerProfile,
}

impl Context {
    pub fn load_games(&mut self, spec_paths: &[&str]) {
        for spec_path in spec_paths.into_iter() {
            self.add_game(spec_path);
        }
    }

    pub fn load_bundles(&mut self, bundle_paths: &[&str]) {
        for bundle_path in bundle_paths.into_iter() {
            self.add_bundle(bundle_path)
        }
    }

    fn add_token(&mut self, token_account: TokenAccount) {
        self.tokens
            .insert(token_account.addr.clone(), token_account);
    }

    fn load_default_tokens(&mut self) {
        self.add_token(TokenAccount {
            name: "USD Coin".into(),
            symbol: "USDC".into(),
            decimals: 6,
            icon: "https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v/logo.png".into(),
            addr: "FACADE_USDC".into(),
        });
        self.add_token(TokenAccount {
            name: "Tether USD".into(),
            symbol: "USDT".into(),
            decimals: 6,
            icon: "https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB/logo.svg".into(),
            addr: "FACADE_USDT".into(),
        });
        self.add_token(TokenAccount {
            name: "Native Token".into(),
            symbol: "NATIVE".into(),
            decimals: 9,
            icon: "https://arweave.net/SH106hrChudKjQ_c6e6yd0tsGUbFIScv2LL6Dp-LDiI".into(),
            addr: "FACADE_NATIVE".into(),
        });
        self.add_token(TokenAccount {
            name: "Race Protocol".into(),
            symbol: "RACE".into(),
            decimals: 9,
            icon: "https://raw.githubusercontent.com/NutsPokerTeam/token-list/main/assets/mainnet/RACE5fnTKB9obGtCusArTQ6hhdNXAtf3HarvJM17rxJ/logo.svg".into(),
            addr: "FACADE_RACE".into(),
        });
    }

    fn add_bundle(&mut self, bundle_path: &str) {
        let re = Regex::new(r"[^a-zA-Z0-9]").unwrap();
        let bundle_addr = re.replace_all(&bundle_path, "").into_owned();
        let mut f = File::open(bundle_path).expect(&format!("Bundle {} not found", &bundle_path));
        let mut data = vec![];
        f.read_to_end(&mut data).unwrap();
        let bundle = GameBundle {
            name: bundle_addr.clone(),
            uri: "".into(),
            data,
        };
        self.bundles.insert(bundle_addr.clone(), bundle);
        println!("+ Bundle: {}", bundle_addr);
    }

    fn add_game(&mut self, spec_path: &str) {
        let f = File::open(spec_path).expect("Spec file not found");
        let GameSpec {
            title,
            bundle,
            token,
            max_players,
            entry_type,
            data: spec_data,
        } = serde_json::from_reader(f).expect(&format!("Invalid spec file: {}", spec_path));

        let re = Regex::new(r"[^a-zA-Z0-9]").unwrap();
        let bundle_addr = re.replace_all(&bundle, "").into_owned();
        let game_addr = re.replace_all(&spec_path, "").into_owned();
        let mut f = File::open(&bundle).expect(&format!("Bundle {} not found", &bundle));
        let mut data = vec![];
        f.read_to_end(&mut data).unwrap();
        let bundle = GameBundle {
            name: bundle_addr.clone(),
            uri: "".into(),
            data,
        };
        let game = GameAccount {
            addr: game_addr.clone(),
            title,
            token_addr: token.to_owned(),
            bundle_addr: bundle_addr.clone(),
            data_len: spec_data.len() as u32,
            data: spec_data,
            max_players,
            entry_type,
            ..Default::default()
        };
        self.bundles.insert(bundle_addr.clone(), bundle);
        self.games.insert(game_addr.clone(), game);
        println!("! Load game from `{}`", spec_path);
        println!("+ Game: {}", game_addr);
        println!("+ Bundle: {}", bundle_addr);
    }
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
    if let Some(bundle) = context.bundles.get(&addr) {
        Ok(Some(bundle.to_owned().try_to_vec().unwrap()))
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
        .games
        .iter()
        .map(|(addr, g)| GameRegistration {
            title: g.title.clone(),
            addr: addr.clone(),
            reg_time: 0,
            bundle_addr: g.bundle_addr.clone(),
        })
        .collect();
    Ok(Some(
        RegistrationAccount {
            addr,
            is_private: false,
            size: 100,
            owner: None,
            games,
        }
        .try_to_vec()
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
    let mut context = context.lock().await;
    if let Some(game_account) = context.games.get_mut(&game_addr) {
        if access_version != game_account.access_version {
            return Err(custom_error(Error::TransactionExpired));
        }
        match &game_account.entry_type {
            EntryType::Cash {
                min_deposit,
                max_deposit,
            } => {
                if amount < *min_deposit || amount > *max_deposit {
                    return Err(custom_error(Error::InvalidAmount));
                } else if game_account.players.len() >= game_account.max_players as _ {
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
                    game_account.access_version += 1;
                    let player_join = PlayerJoin {
                        addr: player_addr.clone(),
                        position,
                        balance: amount,
                        access_version: game_account.access_version,
                        verify_key,
                    };
                    game_account.players.push(player_join);
                    println!(
                        "! Join game: player: {}, game: {}, amount: {}, access version: {} -> {}",
                        player_addr,
                        game_addr,
                        amount,
                        game_account.access_version - 1,
                        game_account.access_version
                    );
                    Ok(())
                }
            }
            #[allow(unused)]
            EntryType::Ticket { slot_id, amount } => todo!(),
            #[allow(unused)]
            EntryType::Gating { collection } => todo!(),
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
    println!(
        "! Deposit game: player: {}, game: {}, amount: {}",
        player_addr, game_addr, amount
    );
    let mut context = context.lock().await;
    let deposit = PlayerDeposit {
        addr: player_addr.clone(),
        amount,
        // Use a larger settle_version to indicate this deposit is not handled.
        settle_version: settle_version + 1,
    };
    if let Some(game_account) = context.games.get_mut(&game_addr) {
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
) -> RpcResult<Option<Vec<u8>>> {
    let addr: String = params.one()?;
    let context = context.lock().await;
    if let Some(server) = context.servers.get(&addr) {
        Ok(Some(server.to_owned().try_to_vec().unwrap()))
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
    let transactor = ServerAccount {
        addr: server_addr.clone(),
        endpoint,
    };
    let mut context = context.lock().await;
    context.servers.insert(server_addr.clone(), transactor);
    println!("+ Server: {}", server_addr);
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
    let mut context = context.lock().await;
    context.games.insert(
        game_addr.clone(),
        GameAccount {
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
        },
    );
    Ok(game_addr)
}

async fn create_profile(params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<()> {
    let CreatePlayerProfileInstruction {
        player_addr,
        nick,
        pfp,
    } = params.one()?;
    let mut context = context.lock().await;

    context
        .players
        .entry(player_addr.clone())
        .and_modify(|pi| {
            pi.profile.nick = nick.clone();
            pi.profile.pfp = pfp.clone();
        })
        .or_insert(PlayerInfo {
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
        });
    println!("+ Player profile: {}", player_addr);

    Ok(())
}

async fn get_profile(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> RpcResult<Option<Vec<u8>>> {
    let addr: String = params.one()?;
    let context = context.lock().await;
    match context.players.get(&addr) {
        Some(player_info) => Ok(Some(player_info.profile.clone().try_to_vec().unwrap())),
        None => Ok(None),
    }
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
    let mut context = context.lock().await;
    let Context {
        ref mut games,
        ref mut players,
        ..
    } = &mut *context;
    if let Some(game_account) = games.get_mut(&game_addr) {
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
                player
                    .balances
                    .entry(game_account.token_addr.to_owned())
                    .and_modify(|b| *b += p.balance);
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
    let ServeInstruction {
        game_addr,
        server_addr,
        verify_key,
    } = params.one()?;
    let mut context = context.lock().await;
    let mut is_transactor = false;

    if !context.servers.contains_key(&server_addr) {
        return Err(custom_error(Error::ServerAccountNotFound));
    }

    let Context {
        servers,
        ref mut games,
        ..
    } = &mut *context;

    let account = games
        .get_mut(&game_addr)
        .ok_or(custom_error(Error::GameAccountNotFound))?;

    if account.transactor_addr.is_none() {
        is_transactor = true;
        account.transactor_addr = Some(server_addr.clone());
    }

    let server_account = servers
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
        if account.servers.len() >= DEFAULT_MAX_SERVERS {
            return Err(custom_error(Error::ServerQueueIsFull(
                DEFAULT_MAX_SERVERS as _,
            )));
        } else {
            account.access_version += 1;
            account.servers.push(ServerJoin::new(
                server_addr.clone(),
                server_account.endpoint.clone(),
                account.access_version,
                verify_key,
            ));
        }
    }
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
    if let Some(player) = context.players.get(&player_addr) {
        if let Some(balance) = player.balances.get(&token_addr) {
            amount = *balance;
        } else {
            println!("? get_balance, token_addr: {}, not found", token_addr);
        }
    } else {
        println!("? get_balance, player_addr: {}, not found", player_addr);
    }
    Ok(amount.try_to_vec().unwrap())
}

async fn get_account_info(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> RpcResult<Option<Vec<u8>>> {
    let addr: String = params.one()?;
    let context = context.lock().await;
    if let Some(account) = context.games.get(&addr) {
        Ok(Some(account.to_owned().try_to_vec().unwrap()))
    } else {
        println!("? get_account_info, addr: {}, not found", addr);
        Ok(None)
    }
}

async fn list_tokens(_params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<Vec<u8>> {
    let context = context.lock().await;
    let tokens: Vec<&TokenAccount> = context.tokens.values().collect();
    let bytes = tokens.try_to_vec()?;
    Ok(bytes)
}

async fn get_player_info(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> RpcResult<Option<Vec<u8>>> {
    let addr: String = params.one()?;
    let context = context.lock().await;
    let Some(player) = context.players.get(&addr) else {
        return Ok(None);
    };
    Ok(Some(player.try_to_vec().unwrap()))
}

async fn settle(params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<String> {
    let SettleParams {
        addr,
        settles,
        transfers,
        checkpoint,
        settle_version,
        next_settle_version,
    } = params.one()?;
    println!(
        "! Handle settlements {}, settles: {:?}, transfers: {:?} ",
        addr, settles, transfers
    );

    // Simulate the finality time
    tokio::time::sleep(Duration::from_secs(1)).await;
    // ---

    let mut context = context.lock().await;
    let Context {
        ref mut games,
        ref mut players,
        ..
    } = &mut *context;

    // The manipulation should be atomic.
    let mut games = games.clone();
    let mut players = players.clone();

    let game = games
        .get_mut(&addr)
        .ok_or(custom_error(Error::GameAccountNotFound))?;

    // Expire old deposits
    game.deposits
        .retain(|d| d.settle_version < game.settle_version);

    if game.settle_version != settle_version {
        return Err(custom_error(Error::InvalidSettle(format!(
            "Invalid settle version, current: {}, transaction: {}",
            game.settle_version, settle_version,
        ))));
    }

    // Increase the `settle_version`
    game.settle_version = next_settle_version;
    println!("! Bump settle version to {}", game.settle_version);
    game.checkpoint = checkpoint;
    game.checkpoint_access_version = game.access_version;

    // Handle settles
    for s in settles.into_iter() {
        match s.op {
            SettleOp::Eject => {
                // Remove player
                if let Some(index) = game.players.iter().position(|p| p.addr.eq(&s.addr)) {
                    let p = game.players.remove(index);
                    let player =
                        players
                            .get_mut(&p.addr)
                            .ok_or(custom_error(Error::InvalidSettle(format!(
                                "Invalid player address: {}",
                                p.addr
                            ))))?;
                    player
                        .balances
                        .entry(game.token_addr.to_owned())
                        .and_modify(|b| *b += p.balance);
                } else {
                    return Err(custom_error(Error::InvalidSettle("Math overflow".into())));
                }
            }
            SettleOp::Add(amount) => {
                let p =
                    game.players
                        .iter_mut()
                        .find(|p| p.addr.eq(&s.addr))
                        .ok_or(custom_error(Error::InvalidSettle(
                            "Invalid player address".into(),
                        )))?;
                p.balance = p
                    .balance
                    .checked_add(amount)
                    .ok_or(custom_error(Error::InvalidSettle("Math overflow".into())))?;
            }
            SettleOp::Sub(amount) => {
                let p =
                    game.players
                        .iter_mut()
                        .find(|p| p.addr.eq(&s.addr))
                        .ok_or(custom_error(Error::InvalidSettle(
                            "Invalid player address".into(),
                        )))?;
                p.balance = p
                    .balance
                    .checked_sub(amount)
                    .ok_or(custom_error(Error::InvalidSettle("Math overflow".into())))?;
            }
            SettleOp::AssignSlot(_) => {}
        }
    }

    context.players = players;
    context.games = games;
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
    module.register_async_method("register_server", register_server)?;
    module.register_async_method("create_profile", create_profile)?;
    module.register_async_method("get_profile", get_profile)?;
    module.register_async_method("create_account", create_account)?;
    module.register_async_method("serve", serve)?;
    module.register_async_method("join", join)?;
    module.register_async_method("deposit", deposit)?;
    module.register_async_method("settle", settle)?;
    module.register_async_method("vote", vote)?;
    module.register_async_method("list_tokens", list_tokens)?;
    module.register_async_method("get_player_info", get_player_info)?;

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
    let matches = cli().get_matches();
    let mut context = Context::default();
    context.load_default_tokens();
    if let Some(game_spec_paths) = matches.get_many::<String>("game") {
        context.load_games(&game_spec_paths.map(String::as_str).collect::<Vec<&str>>());
    }
    if let Some(bundle_paths) = matches.get_many::<String>("bundle") {
        context.load_bundles(&bundle_paths.map(String::as_str).collect::<Vec<&str>>());
    }
    let server_handle = run_server(context).await?;
    server_handle.stopped().await;
    Ok(())
}
