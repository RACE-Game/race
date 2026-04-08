//! This facade server emulates the behavior of its blockchain counterparts.
//! It is supposed to be used for testing and developing.

mod context;
mod db;
mod product_event_service;
mod product_rules;
mod product_store;

use clap::{arg, Command};
use context::Context;
use db::{GuestAccount, GuestSession, Nft, PlayerInfo, UserProgress, UserRating, UserStats};
use hyper::Method;
use jsonrpsee::server::{AllowHosts, ServerBuilder, ServerHandle};
use jsonrpsee::types::Params;
use jsonrpsee::{core::Error as RpcError, RpcModule};
use race_api::types::{BalanceChange, PlayerBalance};
use race_core::error::Error;
use race_core::types::RecipientSlotShare;
use race_core::entry_type::EntryType;
use race_core::types::{
    DepositParams, GameAccount, GameRegistration, PlayerDeposit, PlayerJoin,
    PlayerProfile, RecipientAccount, RecipientSlot, RegistrationAccount, ServerAccount, ServerJoin,
    SettleParams, TokenAccount, Vote, VoteParams, VoteType,
};
use race_core::types::{DepositStatus, RecipientSlotInit, RejectDepositsParams};
use product_event_service::{ProductEvent, ProductEventService};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

type RpcResult<T> = std::result::Result<T, RpcError>;

const DEFAULT_MAX_SERVERS: usize = 3;
const DEFAULT_VOTES_THRESHOLD: usize = 2;

const DEFAULT_BALANCE: u64 = 1000000000;
const GUEST_INITIAL_BALANCE: u64 = 1000000;
const GUEST_SESSION_TTL_MS: u64 = 7 * 24 * 60 * 60 * 1000;
const GUEST_TOKEN_ADDR: &str = "FACADE_GUEST_CHIPS";

const HTTP_HOST: &str = "0.0.0.0:12002";
const DEFAULT_DB_PATH: &str = "data/facade.sqlite3";
const DEFAULT_PRODUCT_DB_URL: &str = "postgresql://postgres@localhost/race_poker_product";

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

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinInstruction {
    player_addr: String,
    game_addr: String,
    position: u16,
    access_version: u64,
    amount: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServeInstruction {
    game_addr: String,
    server_addr: String,
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
    credentials: Vec<u8>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePlayerProfileInstruction {
    player_addr: String,
    nick: String,
    pfp: Option<String>,
    credentials: Vec<u8>,
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

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GuestRegisterRequest {
    nick: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GuestSessionRequest {
    session_token: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GuestAccountSummary {
    guest_id: String,
    player_addr: String,
    nick: String,
    status: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GuestIdentityResponse {
    guest: GuestAccountSummary,
    profile: PlayerProfile,
    balances: HashMap<String, u64>,
    progress: GuestProgressSummary,
    rating: GuestRatingSummary,
    stats: GuestStatsSummary,
    session_expires_at: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GuestRegisterResponse {
    guest: GuestAccountSummary,
    profile: PlayerProfile,
    balances: HashMap<String, u64>,
    progress: GuestProgressSummary,
    rating: GuestRatingSummary,
    stats: GuestStatsSummary,
    session_token: String,
    expires_at: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GuestProgressSummary {
    rank_tier: String,
    xp: u64,
    level: u32,
    updated_at: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GuestRatingSummary {
    rating: i32,
    rank_bucket: String,
    updated_at: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GuestStatsSummary {
    hands_played: u64,
    games_played: u64,
    wins: u64,
    losses: u64,
    last_played_at: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GuestLogoutResponse {
    ok: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct InternalGuestHandFinishedRequest {
    event_id: String,
    guest_id: String,
    player_addr: String,
    hand_id: String,
    did_participate: bool,
    did_win_hand: bool,
    timestamp: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct InternalGuestSessionFinishedRequest {
    event_id: String,
    guest_id: String,
    player_addr: String,
    session_id: String,
    hands_played_in_session: u64,
    session_duration_seconds: u64,
    timestamp: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct InternalGuestTableResultRecordedRequest {
    event_id: String,
    guest_id: String,
    player_addr: String,
    game_id: String,
    result_id: String,
    entry_value: u64,
    ending_value: u64,
    opponent_count: u32,
    hands_played_in_session: u64,
    session_duration_seconds: u64,
    timestamp: Option<u64>,
}

fn custom_error(e: Error) -> RpcError {
    RpcError::Custom(serde_json::to_string(&e).unwrap())
}

fn session_error(msg: &str) -> RpcError {
    RpcError::Custom(msg.to_string())
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn hash_session_token(session_token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(session_token.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn make_guest_account_summary(guest: &GuestAccount) -> GuestAccountSummary {
    GuestAccountSummary {
        guest_id: guest.guest_id.clone(),
        player_addr: guest.player_addr.clone(),
        nick: guest.nick.clone(),
        status: guest.status.clone(),
    }
}

fn make_guest_progress_summary(progress: &UserProgress) -> GuestProgressSummary {
    GuestProgressSummary {
        rank_tier: progress.rank_tier.clone(),
        xp: progress.xp,
        level: progress.level,
        updated_at: progress.updated_at,
    }
}

fn make_guest_rating_summary(rating: &UserRating) -> GuestRatingSummary {
    GuestRatingSummary {
        rating: rating.rating,
        rank_bucket: rating.rank_bucket.clone(),
        updated_at: rating.updated_at,
    }
}

fn make_guest_stats_summary(stats: &UserStats) -> GuestStatsSummary {
    GuestStatsSummary {
        hands_played: stats.hands_played,
        games_played: stats.games_played,
        wins: stats.wins,
        losses: stats.losses,
        last_played_at: stats.last_played_at,
    }
}

fn load_guest_identity(
    context: &mut Context,
    session_token: &str,
) -> anyhow::Result<(GuestSession, GuestAccount, PlayerInfo, UserProgress, UserRating, UserStats)> {
    let session_token_hash = hash_session_token(session_token);
    let now = now_millis();

    let guest_session = context
        .get_guest_session_by_token_hash(&session_token_hash)?
        .ok_or_else(|| anyhow::anyhow!("invalid-session"))?;

    if guest_session.revoked_at.is_some() {
        return Err(anyhow::anyhow!("session-revoked"));
    }

    if guest_session.expires_at < now {
        return Err(anyhow::anyhow!("session-expired"));
    }

    let guest_account = context
        .get_guest_account_by_guest_id(&guest_session.guest_id)?
        .ok_or_else(|| anyhow::anyhow!("guest-not-found"))?;

    let player_info = context
        .get_player_info(&guest_account.player_addr)?
        .ok_or_else(|| anyhow::anyhow!("player-profile-not-found"))?;

    let user_progress = context
        .get_user_progress(&guest_account.guest_id)?
        .ok_or_else(|| anyhow::anyhow!("guest-progress-not-found"))?;

    let user_rating = context
        .get_user_rating(&guest_account.guest_id)?
        .ok_or_else(|| anyhow::anyhow!("guest-rating-not-found"))?;

    let user_stats = context
        .get_user_stats(&guest_account.guest_id)?
        .ok_or_else(|| anyhow::anyhow!("guest-stats-not-found"))?;

    Ok((
        guest_session,
        guest_account,
        player_info,
        user_progress,
        user_rating,
        user_stats,
    ))
}

fn make_guest_identity_response(
    guest_session: &GuestSession,
    guest_account: &GuestAccount,
    player_info: &PlayerInfo,
    user_progress: &UserProgress,
    user_rating: &UserRating,
    user_stats: &UserStats,
) -> GuestIdentityResponse {
    GuestIdentityResponse {
        guest: make_guest_account_summary(guest_account),
        profile: player_info.profile.clone(),
        balances: player_info.balances.clone(),
        progress: make_guest_progress_summary(user_progress),
        rating: make_guest_rating_summary(user_rating),
        stats: make_guest_stats_summary(user_stats),
        session_expires_at: guest_session.expires_at,
    }
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
    let games: Vec<GameRegistration> = context
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
    } = params.one()?;
    let mut context = context.lock().await;

    // Check if the player profile exists?
    if context.get_player_info(&player_addr)?.is_none() {
        println!("E Can't join game, profile not found");
        return Err(custom_error(Error::PlayerProfileNotFound));
    }


    if let Some(mut game_account) = context.get_game_account(&game_addr)? {
        let mut stake = context.get_stake(&game_addr)?;

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
            .is_some()
        {
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
                    stake.amount += amount;

                    let player_join = PlayerJoin {
                        addr: player_addr.clone(),
                        position,
                        access_version: game_account.access_version,
                    };
                    let player_deposit = PlayerDeposit {
                        addr: player_addr.clone(),
                        amount,
                        access_version: game_account.access_version,
                        settle_version: game_account.settle_version,
                        status: DepositStatus::Pending,
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
            EntryType::Ticket {
                amount: ticket_amount,
            } => {
                if *ticket_amount != amount {
                    return Err(custom_error(Error::InvalidAmount));
                } else {
                    game_account.access_version += 1;
                    stake.amount += amount;

                    let player_join = PlayerJoin {
                        addr: player_addr.clone(),
                        position,
                        access_version: game_account.access_version,
                    };
                    println!(
                        "! Join game: player: {}, game: {}, amount: {},  access version: {} -> {}",
                        player_addr,
                        game_addr,
                        amount,
                        game_account.access_version - 1,
                        game_account.access_version
                    );
                    let player_deposit = PlayerDeposit {
                        addr: player_addr.clone(),
                        amount,
                        access_version: game_account.access_version,
                        settle_version: game_account.settle_version,
                        status: DepositStatus::Accepted,
                    };
                    game_account.players.push(player_join);
                    game_account.deposits.push(player_deposit);
                }
            }
            #[allow(unused)]
            EntryType::Gating { collection } => todo!(),
            #[allow(unused)]
            EntryType::Disabled => todo!(),
        }
        context.update_game_account(&game_account)?;
        context.update_stake(&stake)?;
        if let Some(guest_account) = context.get_guest_account_by_player_addr(&player_addr)? {
            let event_timestamp = now_millis();
            let join_event = ProductEvent::GuestTableJoined {
                event_id: format!(
                    "table_joined:{}:{}:{}",
                    guest_account.guest_id, game_addr, player_addr
                ),
                guest_id: guest_account.guest_id,
                player_addr: player_addr.clone(),
                game_id: game_addr.clone(),
                timestamp: event_timestamp,
            };
            ProductEventService::apply(&mut context, join_event)
                .map_err(|err| session_error(&err.to_string()))?;
        }
        Ok(())
    } else {
        return Err(custom_error(Error::GameAccountNotFound));
    }
}

async fn internal_guest_hand_finished(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> RpcResult<()> {
    let request: InternalGuestHandFinishedRequest = params.one()?;
    let mut context = context.lock().await;
    let event = ProductEvent::GuestHandFinished {
        event_id: request.event_id,
        guest_id: request.guest_id,
        player_addr: request.player_addr,
        hand_id: request.hand_id,
        did_participate: request.did_participate,
        did_win_hand: request.did_win_hand,
        timestamp: request.timestamp.unwrap_or_else(now_millis),
    };
    ProductEventService::apply(&mut context, event).map_err(|err| session_error(&err.to_string()))?;
    Ok(())
}

async fn internal_guest_session_finished(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> RpcResult<()> {
    let request: InternalGuestSessionFinishedRequest = params.one()?;
    let mut context = context.lock().await;
    let event = ProductEvent::GuestSessionFinished {
        event_id: request.event_id,
        guest_id: request.guest_id,
        player_addr: request.player_addr,
        session_id: request.session_id,
        hands_played_in_session: request.hands_played_in_session,
        session_duration_seconds: request.session_duration_seconds,
        timestamp: request.timestamp.unwrap_or_else(now_millis),
    };
    ProductEventService::apply(&mut context, event).map_err(|err| session_error(&err.to_string()))?;
    Ok(())
}

async fn internal_guest_table_result_recorded(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> RpcResult<()> {
    let request: InternalGuestTableResultRecordedRequest = params.one()?;
    let mut context = context.lock().await;
    let event = ProductEvent::GuestTableResultRecorded {
        event_id: request.event_id,
        guest_id: request.guest_id,
        player_addr: request.player_addr,
        game_id: request.game_id,
        result_id: request.result_id,
        entry_value: request.entry_value,
        ending_value: request.ending_value,
        opponent_count: request.opponent_count,
        hands_played_in_session: request.hands_played_in_session,
        session_duration_seconds: request.session_duration_seconds,
        timestamp: request.timestamp.unwrap_or_else(now_millis),
    };
    ProductEventService::apply(&mut context, event).map_err(|err| session_error(&err.to_string()))?;
    Ok(())
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
    if let Some(mut game_account) = context.get_game_account(&game_addr)? {
        let mut stake = context.get_stake(&game_addr)?;

        if settle_version != game_account.settle_version {
            return Err(custom_error(Error::TransactionExpired));
        }
        if game_account.players.len() >= game_account.max_players as _ {
            return Err(custom_error(Error::GameIsFull(
                game_account.max_players as _,
            )));
        } else {
            game_account.access_version += 1;
            stake.amount += amount;
            let deposit = PlayerDeposit {
                addr: player_addr.clone(),
                amount,
                access_version: game_account.access_version,
                // Use a larger settle_version to indicate this deposit is not handled.
                settle_version: settle_version + 1,
                status: DepositStatus::Pending,
            };
            game_account.deposits.push(deposit);
            context.update_game_account(&game_account)?;
            context.update_stake(&stake)?;
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
        credentials,
    } = params.one()?;
    let server = ServerAccount {
        addr: server_addr.clone(),
        endpoint,
        credentials,
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
        credentials,
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
            credentials,
        },
    };
    context.create_player_info(&player_info)?;

    Ok(())
}

async fn guest_register(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> RpcResult<GuestRegisterResponse> {
    let GuestRegisterRequest { nick } = params.one()?;
    let nick = nick.trim().to_string();
    if nick.is_empty() {
        return Err(session_error("invalid-nick"));
    }

    let now = now_millis();
    let guest_id = format!("guest_{}", Uuid::new_v4().simple());
    let player_addr = format!("guest_player_{}", Uuid::new_v4().simple());
    let session_id = format!("guest_session_{}", Uuid::new_v4().simple());
    let session_token = Uuid::new_v4().to_string();
    let expires_at = now + GUEST_SESSION_TTL_MS;

    let guest_account = GuestAccount {
        guest_id: guest_id.clone(),
        player_addr: player_addr.clone(),
        nick: nick.clone(),
        status: "active".into(),
        created_at: now,
        updated_at: now,
    };

    let profile = PlayerProfile {
        addr: player_addr.clone(),
        nick,
        pfp: None,
        credentials: Uuid::new_v4().as_bytes().to_vec(),
    };

    let player_info = PlayerInfo {
        balances: HashMap::from([(GUEST_TOKEN_ADDR.to_string(), GUEST_INITIAL_BALANCE)]),
        nfts: HashMap::new(),
        profile: profile.clone(),
    };

    let guest_session = GuestSession {
        session_id,
        guest_id,
        session_token_hash: hash_session_token(&session_token),
        created_at: now,
        expires_at,
        revoked_at: None,
    };

    let mut context = context.lock().await;
    context.create_guest_account(&guest_account)?;
    context.create_player_info(&player_info)?;
    context.create_guest_session(&guest_session)?;
    let user_progress = context
        .get_user_progress(&guest_account.guest_id)?
        .ok_or_else(|| session_error("guest-progress-not-found"))?;
    let user_rating = context
        .get_user_rating(&guest_account.guest_id)?
        .ok_or_else(|| session_error("guest-rating-not-found"))?;
    let user_stats = context
        .get_user_stats(&guest_account.guest_id)?
        .ok_or_else(|| session_error("guest-stats-not-found"))?;

    Ok(GuestRegisterResponse {
        guest: make_guest_account_summary(&guest_account),
        profile,
        balances: player_info.balances,
        progress: make_guest_progress_summary(&user_progress),
        rating: make_guest_rating_summary(&user_rating),
        stats: make_guest_stats_summary(&user_stats),
        session_token,
        expires_at,
    })
}

async fn guest_resume_session(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> RpcResult<GuestIdentityResponse> {
    let GuestSessionRequest { session_token } = params.one()?;
    let mut context = context.lock().await;
    let (guest_session, guest_account, player_info, user_progress, user_rating, user_stats) =
        load_guest_identity(&mut context, &session_token)
            .map_err(|e| session_error(&e.to_string()))?;
    Ok(make_guest_identity_response(
        &guest_session,
        &guest_account,
        &player_info,
        &user_progress,
        &user_rating,
        &user_stats,
    ))
}

async fn guest_get_me(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> RpcResult<GuestIdentityResponse> {
    let GuestSessionRequest { session_token } = params.one()?;
    let mut context = context.lock().await;
    let (guest_session, guest_account, player_info, user_progress, user_rating, user_stats) =
        load_guest_identity(&mut context, &session_token)
            .map_err(|e| session_error(&e.to_string()))?;
    Ok(make_guest_identity_response(
        &guest_session,
        &guest_account,
        &player_info,
        &user_progress,
        &user_rating,
        &user_stats,
    ))
}

async fn guest_logout(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> RpcResult<GuestLogoutResponse> {
    let GuestSessionRequest { session_token } = params.one()?;
    let now = now_millis();
    let mut context = context.lock().await;
    let _ = load_guest_identity(&mut context, &session_token)
        .map_err(|e| session_error(&e.to_string()))?;
    context.revoke_guest_session(&hash_session_token(&session_token), now)?;
    Ok(GuestLogoutResponse { ok: true })
}

async fn get_profile(
    params: Params<'_>,
    context: Arc<Mutex<Context>>,
) -> RpcResult<Option<Vec<u8>>> {
    let addr: String = params.one()?;
    let context = context.lock().await;
    let ret = match context.get_player_info(&addr)? {
        Some(player_info) => {
            println!("? Player profile: {:?}", addr);
            Ok(Some(borsh::to_vec(&player_info.profile).unwrap()))
        },
        None => {
            println!("E Player profile {:?} not found", addr);
            Ok(None)
        },
    };
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
    } = params.one()?;
    let context = context.lock().await;
    let mut is_transactor = false;

    let Some(server_account) = context.get_server_account(&server_addr)? else {
        return Err(custom_error(Error::ServerAccountNotFound));
    };

    let mut account = context
        .get_game_account(&game_addr)?
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
    let mut amount = 999999999u64;
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
        recipient_addr,
        cap_addr,
        slots,
    } = params.one()?;

    let slots = slots
        .into_iter()
        .map(|slot_init| RecipientSlot {
            id: slot_init.id,
            slot_type: slot_init.slot_type,
            token_addr: slot_init.token_addr,
            shares: slot_init
                .init_shares
                .into_iter()
                .map(|share_init| RecipientSlotShare {
                    owner: share_init.owner,
                    weights: share_init.weights,
                    claim_amount: 0,
                })
                .collect(),
            balance: 0,
        })
        .collect();

    let context = context.lock().await;
    let recipient_account = RecipientAccount {
        addr: recipient_addr.clone(),
        cap_addr,
        slots,
    };
    context.create_recipient_account(&recipient_account)?;

    Ok(recipient_addr)
}

async fn reject_deposits(params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<String> {
    let RejectDepositsParams {
        addr,
        reject_deposits,
    } = params.one()?;

    let context = context.lock().await;

    let mut game = context
        .get_game_account(&addr)?
        .ok_or(custom_error(Error::GameAccountNotFound))?;

    let mut stake = context.get_stake(&addr)?;

    println!("! Reject deposits {:?}", reject_deposits);

    game.deposits.iter_mut().for_each(|d| {
        d.status = DepositStatus::Refunded;
        stake.amount -= d.amount;
    });

    for reject_deposit in reject_deposits {
        // TODO, do refund
        game.players.retain(|p| p.access_version != reject_deposit);
    }

    let settle_version = game.settle_version;
    context.update_game_account(&game)?;
    context.update_stake(&stake)?;
    Ok(format!("facade_reject_deposit_{}", settle_version))
}

async fn settle(params: Params<'_>, context: Arc<Mutex<Context>>) -> RpcResult<String> {
    let SettleParams {
        addr,
        settles,
        transfer,
        awards,
        checkpoint,
        access_version,
        settle_version,
        next_settle_version,
        entry_lock,
        accept_deposits,
    } = params.one()?;
    println!(
        "! Handle settlements {}, settles: {:?}, transfer: {:?}",
        addr, settles, transfer
    );

    // Simulate the finality time
    // tokio::time::sleep(Duration::from_secs(10)).await;
    // ---

    let context = context.lock().await;

    // The manipulation should be atomic.

    let mut game = context
        .get_game_account(&addr)?
        .ok_or(custom_error(Error::GameAccountNotFound))?;

    let mut stake = context.get_stake(&addr)?;

    for d in game.deposits.iter_mut() {
        if accept_deposits.contains(&d.access_version) {
            println!("! Mark deposit accepted: {}", d.access_version);
            d.status = DepositStatus::Accepted;
        }
    }

    // Expire old deposits
    game.deposits
        .retain(|d| d.access_version <= access_version && d.status != DepositStatus::Pending);

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

    if let Some(transfer) = transfer {
        stake.amount -= transfer.amount;
    }

    // Handle settles
    for s in settles.into_iter() {
        // Handle balance changes
        if let Some(player_balance) = game
            .balances
            .iter_mut()
            .find(|pb| pb.player_id == s.player_id)
        {
            match s.change {
                Some(BalanceChange::Add(amount)) => {
                    player_balance.balance += amount;
                }
                Some(BalanceChange::Sub(amount)) => {
                    player_balance.balance =
                        player_balance
                            .balance
                            .checked_sub(amount)
                            .ok_or(custom_error(Error::InvalidSettle(
                                "Cannot sub balance".into(),
                            )))?;
                }
                None => (),
            }
        } else {
            match s.change {
                Some(BalanceChange::Add(amount)) => game.balances.push(PlayerBalance {
                    player_id: s.player_id,
                    balance: amount,
                }),
                Some(BalanceChange::Sub(amount)) => {
                    println!("E Cannot initiate balance with Sub({})", amount);
                    return Err(custom_error(Error::InvalidSettle(
                        "Cannot sub balance".into(),
                    )));
                }
                None => (),
            }
        }

        if s.player_id != 0 {
            if let Some(index) = game
                .players
                .iter()
                .position(|p| p.access_version.eq(&s.player_id))
            {
                let p = &game.players[index];
                let mut player =
                    context
                        .get_player_info(&p.addr)?
                        .ok_or(custom_error(Error::InvalidSettle(format!(
                            "Invalid player address: {}",
                            p.addr
                        ))))?;
                player
                    .balances
                    .entry(game.token_addr.to_owned())
                    .and_modify(|b| *b += s.withdraw);
                stake.amount -= s.withdraw;
                context.update_player_info(&player)?;
                if s.eject {
                    println!("! Eject player {} from game", s.player_id);
                    game.players.remove(index);
                }
            } else {
                return Err(custom_error(Error::InvalidSettle("Math overflow".into())));
            }
        }
    }

    for award in awards {
        println!(
            "! Awarad {} to player {}",
            award.bonus_identifier, award.player_id
        );
    }

    game.balances.retain(|pb| pb.balance != 0);

    // Validate stake and balances

    let player_balance_sum = game.balances.iter().map(|p| p.balance).sum::<u64>();
    let unhandled_deposits_sum = game
        .deposits
        .iter()
        .filter_map(|d| {
            if d.status == DepositStatus::Pending {
                Some(d.amount)
            } else {
                None
            }
        })
        .sum::<u64>();

    if stake.amount == player_balance_sum + unhandled_deposits_sum {
        println!("! Balance validation passed!");
    } else {
        println!(
            "E Balance validation failed: stake ({}) != balance ({}) + pending_deposit ({})",
            stake.amount, player_balance_sum, unhandled_deposits_sum
        );
    }

    context.update_game_account(&game)?;
    context.update_stake(&stake)?;

    Ok(format!("facade_settle_{}", settle_version))
}

async fn run_server(context: Context) -> anyhow::Result<ServerHandle> {
    run_server_at(context, HTTP_HOST.parse::<SocketAddr>()?).await
}

async fn run_server_at(context: Context, bind_addr: SocketAddr) -> anyhow::Result<ServerHandle> {
    let cors = CorsLayer::new()
        .allow_methods([Method::POST])
        .allow_origin(Any)
        .allow_headers([hyper::header::CONTENT_TYPE]);
    let middleware = ServiceBuilder::new().layer(cors);

    let http_server = ServerBuilder::default()
        .max_response_body_size(1_000_000_000)
        .set_host_filtering(AllowHosts::Any)
        .set_middleware(middleware)
        .build(bind_addr)
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
    module.register_async_method("internal_guest_hand_finished", internal_guest_hand_finished)?;
    module.register_async_method(
        "internal_guest_session_finished",
        internal_guest_session_finished,
    )?;
    module.register_async_method(
        "internal_guest_table_result_recorded",
        internal_guest_table_result_recorded,
    )?;
    module.register_async_method("deposit", deposit)?;
    module.register_async_method("settle", settle)?;
    module.register_async_method("vote", vote)?;
    module.register_async_method("list_tokens", list_tokens)?;
    module.register_async_method("reject_deposits", reject_deposits)?;
    module.register_async_method("guest_register", guest_register)?;
    module.register_async_method("guest_resume_session", guest_resume_session)?;
    module.register_async_method("guest_get_me", guest_get_me)?;
    module.register_async_method("guest_logout", guest_logout)?;

    let handle = http_server.start(module)?;
    Ok(handle)
}

fn cli() -> Command {
    Command::new("facade")
        .about("A mock server for local development with Race")
        .arg(arg!(-g <game> ... "The path to a game spec json file"))
        .arg(arg!(-b <bundle> ... "The path to a wasm bundle"))
        .arg(
            arg!(--db <db_path> "Path to the facade sqlite database file")
                .required(false)
                .default_value(DEFAULT_DB_PATH),
        )
        .arg(
            arg!(--"product-db-url" <product_db_url> "Optional Postgres URL for product-layer guest/session/progression data")
                .required(false),
        )
}

#[derive(Clone, Debug)]
enum ProductDbMode {
    Explicit(String),
    Default(String),
    Disabled,
}

fn main() -> anyhow::Result<()> {
    println!("Start at {}", HTTP_HOST);
    let matches = cli().get_matches();
    let db_path = matches
        .get_one::<String>("db")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DB_PATH));
    println!("Using sqlite db at {}", db_path.display());
    let product_db_mode = if let Some(product_db_url) =
        matches.get_one::<String>("product-db-url").cloned()
    {
        ProductDbMode::Explicit(product_db_url)
    } else if let Ok(product_db_url) = std::env::var("RACE_FACADE_PRODUCT_DB_URL") {
        ProductDbMode::Explicit(product_db_url)
    } else if std::env::var("RACE_FACADE_DISABLE_DEFAULT_PRODUCT_DB")
        .ok()
        .as_deref()
        == Some("1")
    {
        ProductDbMode::Disabled
    } else {
        ProductDbMode::Default(DEFAULT_PRODUCT_DB_URL.to_string())
    };

    let context = match &product_db_mode {
        ProductDbMode::Explicit(product_db_url) => {
            println!("Using Postgres product db: explicit configuration");
            Context::open_sqlite_with_product_store(&db_path, Some(product_db_url.as_str()))?
        }
        ProductDbMode::Default(product_db_url) => {
            match Context::open_sqlite_with_product_store(&db_path, Some(product_db_url.as_str()))
            {
                Ok(context) => {
                    println!(
                        "Using Postgres product db: dev default {}",
                        DEFAULT_PRODUCT_DB_URL
                    );
                    context
                }
                Err(err) => {
                    println!(
                        "Default Postgres product db unavailable, falling back to sqlite-only mode: {err}"
                    );
                    Context::open_sqlite_with_product_store(&db_path, None)?
                }
            }
        }
        ProductDbMode::Disabled => {
            println!("Default Postgres product db disabled; using sqlite-only mode");
            Context::open_sqlite_with_product_store(&db_path, None)?
        }
    };
    context.load_default_tokens()?;
    if let Some(game_spec_paths) = matches.get_many::<String>("game") {
        context.load_games(&game_spec_paths.map(String::as_str).collect::<Vec<&str>>())?;
    }
    if let Some(bundle_paths) = matches.get_many::<String>("bundle") {
        context.load_bundles(&bundle_paths.map(String::as_str).collect::<Vec<&str>>())?;
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async move {
        let server_handle = run_server(context).await?;
        server_handle.stopped().await;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonrpsee::{
        core::client::ClientT,
        http_client::HttpClientBuilder,
        rpc_params,
    };
    use std::fs;
    use tokio::net::TcpListener;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_guest_rpc_flow() {
        let context = Context::in_memory();
        context.load_default_tokens().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let bind_addr = listener.local_addr().unwrap();
        drop(listener);
        let server_handle = run_server_at(context, bind_addr).await.unwrap();

        let client = HttpClientBuilder::default()
            .build(format!("http://{bind_addr}"))
            .unwrap();

        let register_response: GuestRegisterResponse = client
            .request(
                "guest_register",
                rpc_params![GuestRegisterRequest {
                    nick: "SmokeGuest".into(),
                }],
            )
            .await
            .unwrap();

        assert_eq!(register_response.guest.nick, "SmokeGuest");
        assert_eq!(
            register_response
                .balances
                .get(GUEST_TOKEN_ADDR)
                .copied(),
            Some(GUEST_INITIAL_BALANCE)
        );
        assert_eq!(register_response.profile.addr, register_response.guest.player_addr);
        assert_eq!(register_response.progress.rank_tier, "Bronze I");
        assert_eq!(register_response.rating.rating, 1000);
        assert_eq!(register_response.stats.games_played, 0);

        let resume_response: GuestIdentityResponse = client
            .request(
                "guest_resume_session",
                rpc_params![GuestSessionRequest {
                    session_token: register_response.session_token.clone(),
                }],
            )
            .await
            .unwrap();

        assert_eq!(resume_response.guest.guest_id, register_response.guest.guest_id);
        assert_eq!(resume_response.profile.addr, register_response.profile.addr);
        assert_eq!(resume_response.progress.level, 1);
        assert_eq!(resume_response.rating.rank_bucket, "Bronze I");
        assert_eq!(resume_response.stats.games_played, 0);
        assert_eq!(
            resume_response.balances.get(GUEST_TOKEN_ADDR).copied(),
            Some(GUEST_INITIAL_BALANCE)
        );

        let me_response: GuestIdentityResponse = client
            .request(
                "guest_get_me",
                rpc_params![GuestSessionRequest {
                    session_token: register_response.session_token.clone(),
                }],
            )
            .await
            .unwrap();

        assert_eq!(me_response.guest.player_addr, register_response.guest.player_addr);
        assert_eq!(me_response.profile.nick, "SmokeGuest");
        assert_eq!(me_response.progress.xp, 0);
        assert_eq!(me_response.stats.hands_played, 0);

        let logout_response: GuestLogoutResponse = client
            .request(
                "guest_logout",
                rpc_params![GuestSessionRequest {
                    session_token: register_response.session_token.clone(),
                }],
            )
            .await
            .unwrap();

        assert!(logout_response.ok);

        let resume_after_logout = client
            .request::<GuestIdentityResponse, _>(
                "guest_resume_session",
                rpc_params![GuestSessionRequest {
                    session_token: register_response.session_token,
                }],
            )
            .await;

        assert!(resume_after_logout.is_err());

        server_handle.stop().unwrap();
        server_handle.stopped().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_join_updates_progress_and_stats_via_real_join_path() {
        let context = Context::in_memory();
        context.load_default_tokens().unwrap();

        let game_account = GameAccount {
            addr: "game_join_test".into(),
            title: "Join Test".into(),
            bundle_addr: "bundle_join_test".into(),
            owner_addr: "".into(),
            settle_version: 0,
            access_version: 0,
            players: vec![],
            data_len: 0,
            data: vec![],
            transactor_addr: None,
            servers: vec![],
            votes: vec![],
            unlock_time: None,
            max_players: 6,
            deposits: vec![],
            recipient_addr: "".into(),
            entry_type: EntryType::Cash {
                min_deposit: 100,
                max_deposit: 1_000,
            },
            token_addr: GUEST_TOKEN_ADDR.into(),
            checkpoint_on_chain: None,
            entry_lock: Default::default(),
            bonuses: vec![],
            balances: vec![],
        };
        context.create_game_account(&game_account).unwrap();
        context
            .create_stake(&db::Stake {
                addr: "game_join_test".into(),
                amount: 0,
            })
            .unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let bind_addr = listener.local_addr().unwrap();
        drop(listener);
        let server_handle = run_server_at(context, bind_addr).await.unwrap();

        let client = HttpClientBuilder::default()
            .build(format!("http://{bind_addr}"))
            .unwrap();

        let register_response: GuestRegisterResponse = client
            .request(
                "guest_register",
                rpc_params![GuestRegisterRequest {
                    nick: "JoinGuest".into(),
                }],
            )
            .await
            .unwrap();

        client
            .request::<(), _>(
                "join",
                rpc_params![JoinInstruction {
                    player_addr: register_response.guest.player_addr.clone(),
                    game_addr: "game_join_test".into(),
                    position: 0,
                    access_version: 0,
                    amount: 100,
                }],
            )
            .await
            .unwrap();

        let me_response: GuestIdentityResponse = client
            .request(
                "guest_get_me",
                rpc_params![GuestSessionRequest {
                    session_token: register_response.session_token,
                }],
            )
            .await
            .unwrap();

        assert_eq!(me_response.stats.games_played, 1);
        assert!(me_response.stats.last_played_at.is_some());
        assert_eq!(me_response.progress.xp, product_rules::JOIN_XP_BONUS);
        assert_eq!(me_response.progress.level, 1);
        assert_eq!(me_response.progress.rank_tier, "Bronze I");

        server_handle.stop().unwrap();
        server_handle.stopped().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_internal_ingestion_paths_for_hand_and_session_events() {
        let context = Context::in_memory();
        context.load_default_tokens().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let bind_addr = listener.local_addr().unwrap();
        drop(listener);
        let server_handle = run_server_at(context, bind_addr).await.unwrap();

        let client = HttpClientBuilder::default()
            .build(format!("http://{bind_addr}"))
            .unwrap();

        let register_response: GuestRegisterResponse = client
            .request(
                "guest_register",
                rpc_params![GuestRegisterRequest {
                    nick: "EventGuest".into(),
                }],
            )
            .await
            .unwrap();

        client
            .request::<(), _>(
                "internal_guest_hand_finished",
                rpc_params![InternalGuestHandFinishedRequest {
                    event_id: "hand:event:1".into(),
                    guest_id: register_response.guest.guest_id.clone(),
                    player_addr: register_response.guest.player_addr.clone(),
                    hand_id: "hand-1".into(),
                    did_participate: true,
                    did_win_hand: true,
                    timestamp: Some(200),
                }],
            )
            .await
            .unwrap();
        client
            .request::<(), _>(
                "internal_guest_hand_finished",
                rpc_params![InternalGuestHandFinishedRequest {
                    event_id: "hand:event:1".into(),
                    guest_id: register_response.guest.guest_id.clone(),
                    player_addr: register_response.guest.player_addr.clone(),
                    hand_id: "hand-1".into(),
                    did_participate: true,
                    did_win_hand: true,
                    timestamp: Some(200),
                }],
            )
            .await
            .unwrap();
        client
            .request::<(), _>(
                "internal_guest_session_finished",
                rpc_params![InternalGuestSessionFinishedRequest {
                    event_id: "session:event:1".into(),
                    guest_id: register_response.guest.guest_id.clone(),
                    player_addr: register_response.guest.player_addr.clone(),
                    session_id: "session-1".into(),
                    hands_played_in_session: 3,
                    session_duration_seconds: 300,
                    timestamp: Some(300),
                }],
            )
            .await
            .unwrap();
        client
            .request::<(), _>(
                "internal_guest_session_finished",
                rpc_params![InternalGuestSessionFinishedRequest {
                    event_id: "session:event:1".into(),
                    guest_id: register_response.guest.guest_id.clone(),
                    player_addr: register_response.guest.player_addr.clone(),
                    session_id: "session-1".into(),
                    hands_played_in_session: 3,
                    session_duration_seconds: 300,
                    timestamp: Some(300),
                }],
            )
            .await
            .unwrap();

        let me_response: GuestIdentityResponse = client
            .request(
                "guest_get_me",
                rpc_params![GuestSessionRequest {
                    session_token: register_response.session_token,
                }],
            )
            .await
            .unwrap();

        assert_eq!(me_response.stats.hands_played, 1);
        assert_eq!(
            me_response.progress.xp,
            product_rules::HAND_PARTICIPATION_XP
                + product_rules::HAND_WIN_BONUS
                + product_rules::SESSION_COMPLETION_XP
        );

        server_handle.stop().unwrap();
        server_handle.stopped().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_internal_table_result_recorded_updates_stats_and_rating() {
        let context = Context::in_memory();
        context.load_default_tokens().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let bind_addr = listener.local_addr().unwrap();
        drop(listener);
        let server_handle = run_server_at(context, bind_addr).await.unwrap();

        let client = HttpClientBuilder::default()
            .build(format!("http://{bind_addr}"))
            .unwrap();

        let register_response: GuestRegisterResponse = client
            .request(
                "guest_register",
                rpc_params![GuestRegisterRequest {
                    nick: "ResultGuest".into(),
                }],
            )
            .await
            .unwrap();

        client
            .request::<(), _>(
                "internal_guest_table_result_recorded",
                rpc_params![InternalGuestTableResultRecordedRequest {
                    event_id: "result:event:1".into(),
                    guest_id: register_response.guest.guest_id.clone(),
                    player_addr: register_response.guest.player_addr.clone(),
                    game_id: "table-1".into(),
                    result_id: "table-result-1".into(),
                    entry_value: 1_000,
                    ending_value: 1_300,
                    opponent_count: 3,
                    hands_played_in_session: 6,
                    session_duration_seconds: 420,
                    timestamp: Some(500),
                }],
            )
            .await
            .unwrap();
        client
            .request::<(), _>(
                "internal_guest_table_result_recorded",
                rpc_params![InternalGuestTableResultRecordedRequest {
                    event_id: "result:event:1".into(),
                    guest_id: register_response.guest.guest_id.clone(),
                    player_addr: register_response.guest.player_addr.clone(),
                    game_id: "table-1".into(),
                    result_id: "table-result-1".into(),
                    entry_value: 1_000,
                    ending_value: 1_300,
                    opponent_count: 3,
                    hands_played_in_session: 6,
                    session_duration_seconds: 420,
                    timestamp: Some(500),
                }],
            )
            .await
            .unwrap();

        let me_response: GuestIdentityResponse = client
            .request(
                "guest_get_me",
                rpc_params![GuestSessionRequest {
                    session_token: register_response.session_token,
                }],
            )
            .await
            .unwrap();

        assert_eq!(me_response.stats.wins, 1);
        assert_eq!(me_response.stats.losses, 0);
        assert_eq!(
            me_response.rating.rating,
            1000 + product_rules::STRONG_POSITIVE_RATING_DELTA
        );
        assert_eq!(me_response.rating.rank_bucket, "Silver");

        server_handle.stop().unwrap();
        server_handle.stopped().await;
    }

    #[test]
    fn test_guest_persistence_across_context_restart() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let db_path = std::env::temp_dir().join(format!("race_facade_context_restart_{unique}.sqlite"));
        let _ = fs::remove_file(&db_path);

        let session_token = "persist-session-token";
        let session_hash = hash_session_token(session_token);

        {
            let mut context = Context::open_sqlite(&db_path).unwrap();
            context.load_default_tokens().unwrap();

            let player_info = PlayerInfo {
                balances: HashMap::from([(GUEST_TOKEN_ADDR.to_string(), GUEST_INITIAL_BALANCE)]),
                nfts: HashMap::new(),
                profile: PlayerProfile {
                    addr: "guest_player_restart".into(),
                    nick: "Restart Guest".into(),
                    pfp: None,
                    credentials: vec![1, 2, 3],
                },
            };
            let guest_account = GuestAccount {
                guest_id: "guest-restart".into(),
                player_addr: "guest_player_restart".into(),
                nick: "Restart Guest".into(),
                status: "active".into(),
                created_at: 100,
                updated_at: 100,
            };
            let guest_session = GuestSession {
                session_id: "guest-session-restart".into(),
                guest_id: "guest-restart".into(),
                session_token_hash: session_hash.clone(),
                created_at: 100,
                expires_at: now_millis() + 60_000,
                revoked_at: None,
            };

            context.create_guest_account(&guest_account).unwrap();
            context.create_player_info(&player_info).unwrap();
            context.create_guest_session(&guest_session).unwrap();
        }

        {
            let mut context = Context::open_sqlite(&db_path).unwrap();
            context.load_default_tokens().unwrap();

            let (guest_session, guest_account, player_info, user_progress, user_rating, user_stats) =
                load_guest_identity(&mut context, session_token).unwrap();
            assert_eq!(guest_account.guest_id, "guest-restart");
            assert_eq!(player_info.profile.addr, "guest_player_restart");
            assert_eq!(user_progress.rank_tier, "Bronze I");
            assert_eq!(user_rating.rating, 1000);
            assert_eq!(user_stats.games_played, 0);
            assert_eq!(
                player_info.balances.get(GUEST_TOKEN_ADDR).copied(),
                Some(GUEST_INITIAL_BALANCE)
            );

            context
                .revoke_guest_session(&session_hash, guest_session.created_at + 1)
                .unwrap();
        }

        {
            let mut context = Context::open_sqlite(&db_path).unwrap();
            let result = load_guest_identity(&mut context, session_token);
            assert!(result.is_err());
            let err = result.err().unwrap();
            assert_eq!(err.to_string(), "session-revoked");
        }

        let _ = fs::remove_file(&db_path);
    }
}
