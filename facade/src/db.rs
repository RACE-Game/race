//! Database related code for facade.

use std::collections::HashMap;

use borsh::BorshSerialize;
use race_core::types::{
    GameAccount, GameBundle, PlayerProfile, RecipientAccount, RecipientSlot, RegistrationAccount,
    ServerAccount, TokenAccount,
};
use rusqlite::{params, Connection, OptionalExtension, Result};

#[derive(Clone, BorshSerialize)]
pub(crate) struct Nft {
    pub addr: String,
    pub image: String,
    pub name: String,
    pub symbol: String,
    pub collection: Option<String>,
}

#[derive(Clone, BorshSerialize)]
pub(crate) struct PlayerInfo {
    pub balances: HashMap<String, u64>, // token address to balance
    pub nfts: HashMap<String, Nft>,
    pub profile: PlayerProfile,
}

#[derive(Clone, BorshSerialize)]
pub(crate) struct Stake {
    pub addr: String,
    pub amount: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct GuestAccount {
    pub guest_id: String,
    pub player_addr: String,
    pub nick: String,
    pub status: String,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct GuestSession {
    pub session_id: String,
    pub guest_id: String,
    pub session_token_hash: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub revoked_at: Option<u64>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct UserProgress {
    pub guest_id: String,
    pub rank_tier: String,
    pub xp: u64,
    pub level: u32,
    pub updated_at: u64,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct UserRating {
    pub guest_id: String,
    pub rating: i32,
    pub rank_bucket: String,
    pub updated_at: u64,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct UserStats {
    pub guest_id: String,
    pub hands_played: u64,
    pub games_played: u64,
    pub wins: u64,
    pub losses: u64,
    pub last_played_at: Option<u64>,
}

#[derive(Clone, Debug)]
pub(crate) struct ProductEventLogEntry {
    pub event_id: String,
    pub event_type: String,
    pub guest_id: String,
    pub created_at: u64,
}

const DEFAULT_RANK_TIER: &str = "Bronze I";
const DEFAULT_RANK_BUCKET: &str = "Bronze I";
const DEFAULT_LEVEL: u32 = 1;
const DEFAULT_XP: u64 = 0;
const DEFAULT_RATING: i32 = 1000;

// CRUD functions for Stake

pub fn create_stake_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS stake (
            addr TEXT PRIMARY KEY,
            amount INTEGER NOT NULL
        )",
        [],
    )?;
    Ok(())
}

pub fn create_guest_tables(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS guest_account (
            guest_id TEXT PRIMARY KEY,
            player_addr TEXT NOT NULL UNIQUE,
            nick TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS guest_session (
            session_id TEXT PRIMARY KEY,
            guest_id TEXT NOT NULL,
            session_token_hash TEXT NOT NULL UNIQUE,
            created_at INTEGER NOT NULL,
            expires_at INTEGER NOT NULL,
            revoked_at INTEGER,
            FOREIGN KEY (guest_id) REFERENCES guest_account(guest_id)
        )",
        [],
    )?;

    Ok(())
}

pub fn create_product_state_tables(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS user_progress (
            guest_id TEXT PRIMARY KEY,
            rank_tier TEXT NOT NULL,
            xp INTEGER NOT NULL,
            level INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            FOREIGN KEY (guest_id) REFERENCES guest_account(guest_id)
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS user_rating (
            guest_id TEXT PRIMARY KEY,
            rating INTEGER NOT NULL,
            rank_bucket TEXT NOT NULL,
            updated_at INTEGER NOT NULL,
            FOREIGN KEY (guest_id) REFERENCES guest_account(guest_id)
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS user_stats (
            guest_id TEXT PRIMARY KEY,
            hands_played INTEGER NOT NULL DEFAULT 0,
            games_played INTEGER NOT NULL DEFAULT 0,
            wins INTEGER NOT NULL DEFAULT 0,
            losses INTEGER NOT NULL DEFAULT 0,
            last_played_at INTEGER,
            FOREIGN KEY (guest_id) REFERENCES guest_account(guest_id)
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS product_event_log (
            event_id TEXT PRIMARY KEY,
            event_type TEXT NOT NULL,
            guest_id TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            FOREIGN KEY (guest_id) REFERENCES guest_account(guest_id)
        )",
        [],
    )?;

    Ok(())
}

pub fn create_guest_account(conn: &Connection, guest_account: &GuestAccount) -> Result<()> {
    conn.execute(
        "INSERT INTO guest_account (
            guest_id, player_addr, nick, status, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            guest_account.guest_id,
            guest_account.player_addr,
            guest_account.nick,
            guest_account.status,
            guest_account.created_at,
            guest_account.updated_at,
        ],
    )?;
    Ok(())
}

pub fn read_guest_account_by_guest_id(
    conn: &Connection,
    guest_id: &str,
) -> Result<Option<GuestAccount>> {
    conn.query_row(
        "SELECT guest_id, player_addr, nick, status, created_at, updated_at
         FROM guest_account WHERE guest_id = ?1",
        params![guest_id],
        |row| {
            Ok(GuestAccount {
                guest_id: row.get(0)?,
                player_addr: row.get(1)?,
                nick: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        },
    )
    .optional()
}

pub fn read_guest_account_by_player_addr(
    conn: &Connection,
    player_addr: &str,
) -> Result<Option<GuestAccount>> {
    conn.query_row(
        "SELECT guest_id, player_addr, nick, status, created_at, updated_at
         FROM guest_account WHERE player_addr = ?1",
        params![player_addr],
        |row| {
            Ok(GuestAccount {
                guest_id: row.get(0)?,
                player_addr: row.get(1)?,
                nick: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        },
    )
    .optional()
}

pub fn create_guest_session(conn: &Connection, guest_session: &GuestSession) -> Result<()> {
    conn.execute(
        "INSERT INTO guest_session (
            session_id, guest_id, session_token_hash, created_at, expires_at, revoked_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            guest_session.session_id,
            guest_session.guest_id,
            guest_session.session_token_hash,
            guest_session.created_at,
            guest_session.expires_at,
            guest_session.revoked_at,
        ],
    )?;
    Ok(())
}

pub fn read_guest_session_by_token_hash(
    conn: &Connection,
    session_token_hash: &str,
) -> Result<Option<GuestSession>> {
    conn.query_row(
        "SELECT session_id, guest_id, session_token_hash, created_at, expires_at, revoked_at
         FROM guest_session WHERE session_token_hash = ?1",
        params![session_token_hash],
        |row| {
            Ok(GuestSession {
                session_id: row.get(0)?,
                guest_id: row.get(1)?,
                session_token_hash: row.get(2)?,
                created_at: row.get(3)?,
                expires_at: row.get(4)?,
                revoked_at: row.get(5)?,
            })
        },
    )
    .optional()
}

pub fn revoke_guest_session(
    conn: &Connection,
    session_token_hash: &str,
    revoked_at: u64,
) -> Result<()> {
    conn.execute(
        "UPDATE guest_session
         SET revoked_at = ?1
         WHERE session_token_hash = ?2 AND revoked_at IS NULL",
        params![revoked_at, session_token_hash],
    )?;
    Ok(())
}

pub fn initialize_product_state(conn: &Connection, guest_id: &str, now: u64) -> Result<()> {
    conn.execute(
        "INSERT INTO user_progress (guest_id, rank_tier, xp, level, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (guest_id) DO NOTHING",
        params![guest_id, DEFAULT_RANK_TIER, DEFAULT_XP, DEFAULT_LEVEL, now],
    )?;
    conn.execute(
        "INSERT INTO user_rating (guest_id, rating, rank_bucket, updated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (guest_id) DO NOTHING",
        params![guest_id, DEFAULT_RATING, DEFAULT_RANK_BUCKET, now],
    )?;
    conn.execute(
        "INSERT INTO user_stats (guest_id, hands_played, games_played, wins, losses, last_played_at)
         VALUES (?1, 0, 0, 0, 0, NULL)
         ON CONFLICT (guest_id) DO NOTHING",
        params![guest_id],
    )?;
    Ok(())
}

pub fn read_user_progress(conn: &Connection, guest_id: &str) -> Result<Option<UserProgress>> {
    conn.query_row(
        "SELECT guest_id, rank_tier, xp, level, updated_at
         FROM user_progress WHERE guest_id = ?1",
        params![guest_id],
        |row| {
            Ok(UserProgress {
                guest_id: row.get(0)?,
                rank_tier: row.get(1)?,
                xp: row.get(2)?,
                level: row.get(3)?,
                updated_at: row.get(4)?,
            })
        },
    )
    .optional()
}

pub fn read_user_rating(conn: &Connection, guest_id: &str) -> Result<Option<UserRating>> {
    conn.query_row(
        "SELECT guest_id, rating, rank_bucket, updated_at
         FROM user_rating WHERE guest_id = ?1",
        params![guest_id],
        |row| {
            Ok(UserRating {
                guest_id: row.get(0)?,
                rating: row.get(1)?,
                rank_bucket: row.get(2)?,
                updated_at: row.get(3)?,
            })
        },
    )
    .optional()
}

pub fn read_user_stats(conn: &Connection, guest_id: &str) -> Result<Option<UserStats>> {
    conn.query_row(
        "SELECT guest_id, hands_played, games_played, wins, losses, last_played_at
         FROM user_stats WHERE guest_id = ?1",
        params![guest_id],
        |row| {
            Ok(UserStats {
                guest_id: row.get(0)?,
                hands_played: row.get(1)?,
                games_played: row.get(2)?,
                wins: row.get(3)?,
                losses: row.get(4)?,
                last_played_at: row.get(5)?,
            })
        },
    )
    .optional()
}

pub fn record_user_joined_game(conn: &Connection, guest_id: &str, now: u64) -> Result<()> {
    conn.execute(
        "UPDATE user_stats
         SET games_played = games_played + 1,
             last_played_at = ?2
         WHERE guest_id = ?1",
        params![guest_id, now],
    )?;
    Ok(())
}

pub fn insert_product_event_log_entry(
    conn: &Connection,
    entry: &ProductEventLogEntry,
) -> Result<bool> {
    let changed = conn.execute(
        "INSERT INTO product_event_log (
            event_id, event_type, guest_id, created_at
         ) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (event_id) DO NOTHING",
        params![
            entry.event_id,
            entry.event_type,
            entry.guest_id,
            entry.created_at,
        ],
    )?;
    Ok(changed > 0)
}

pub fn update_user_progress(conn: &Connection, progress: &UserProgress) -> Result<()> {
    conn.execute(
        "UPDATE user_progress
         SET rank_tier = ?2,
             xp = ?3,
             level = ?4,
             updated_at = ?5
         WHERE guest_id = ?1",
        params![
            progress.guest_id,
            progress.rank_tier,
            progress.xp,
            progress.level,
            progress.updated_at,
        ],
    )?;
    Ok(())
}

pub fn increment_user_hands_played(conn: &Connection, guest_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE user_stats
         SET hands_played = hands_played + 1
         WHERE guest_id = ?1",
        params![guest_id],
    )?;
    Ok(())
}

pub fn increment_user_wins(conn: &Connection, guest_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE user_stats
         SET wins = wins + 1
         WHERE guest_id = ?1",
        params![guest_id],
    )?;
    Ok(())
}

pub fn increment_user_losses(conn: &Connection, guest_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE user_stats
         SET losses = losses + 1
         WHERE guest_id = ?1",
        params![guest_id],
    )?;
    Ok(())
}

pub fn update_user_rating(conn: &Connection, rating: &UserRating) -> Result<()> {
    conn.execute(
        "UPDATE user_rating
         SET rating = ?2,
             rank_bucket = ?3,
             updated_at = ?4
         WHERE guest_id = ?1",
        params![
            rating.guest_id,
            rating.rating,
            rating.rank_bucket,
            rating.updated_at,
        ],
    )?;
    Ok(())
}

// Function to create a new stake entry
pub fn create_stake(conn: &Connection, stake: &Stake) -> Result<()> {
    conn.execute(
        "INSERT INTO stake (addr, amount) VALUES (?1, ?2)",
        params![stake.addr, stake.amount],
    )?;
    Ok(())
}

// Function to update an existing stake entry
pub fn update_stake(conn: &Connection, stake: &Stake) -> Result<()> {
    conn.execute(
        "UPDATE stake SET amount = ?1 WHERE addr = ?2",
        params![stake.amount, stake.addr],
    )?;
    Ok(())
}

pub fn read_stake(conn: &Connection, addr: &str) -> Result<Option<Stake>> {
    let mut stmt = conn.prepare("SELECT addr, amount FROM stake WHERE addr = ?1")?;
    let mut rows = stmt.query(params![addr])?;

    if let Some(row) = rows.next()? {
        let stake = Stake {
            addr: row.get(0)?,
            amount: row.get(1)?,
        };
        Ok(Some(stake))
    } else {
        Ok(None)
    }
}

// CRUD functions for PlayerInfo

pub fn create_player_info(conn: &Connection, player_info: &PlayerInfo) -> Result<()> {
    let profile = &player_info.profile;
    conn.execute(
        "INSERT INTO player_info (addr, nick, pfp, credentials) VALUES (?1, ?2, ?3, ?4)",
        params![profile.addr, profile.nick, profile.pfp, profile.credentials],
    )?;

    // Assuming a separate table for balances
    for (token_addr, balance) in &player_info.balances {
        conn.execute(
            "INSERT INTO player_balance (player_addr, token_addr, balance) VALUES (?1, ?2, ?3)",
            params![profile.addr, token_addr, balance],
        )?;
    }

    // Assuming a separate table for nfts
    for (nft_addr, nft) in &player_info.nfts {
        conn.execute(
            "INSERT INTO player_nft (player_addr, nft_addr, image, name, symbol, collection) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![profile.addr, nft_addr, nft.image, nft.name, nft.symbol, nft.collection],
        )?;
    }

    Ok(())
}

pub fn read_player_info(conn: &Connection, player_addr: &str) -> Result<Option<PlayerInfo>> {
    let mut stmt = conn.prepare("SELECT addr, nick, pfp, credentials FROM player_info WHERE addr = ?1")?;
    let mut player_iter = stmt.query_map(params![player_addr], |row| {
        Ok(PlayerProfile {
            addr: row.get(0)?,
            nick: row.get(1)?,
            pfp: row.get(2)?,
            credentials: row.get(3)?,
        })
    })?;

    let player_profile = if let Some(player_profile) = player_iter.next() {
        player_profile?
    } else {
        return Ok(None);
    };

    let mut balances = HashMap::new();
    let mut nfts = HashMap::new();

    let mut balance_stmt =
        conn.prepare("SELECT token_addr, balance FROM player_balance WHERE player_addr = ?1")?;
    let balance_iter = balance_stmt.query_map(params![player_addr], |row| {
        let token_addr: String = row.get(0)?;
        let balance: u64 = row.get(1)?;
        balances.insert(token_addr, balance);
        Ok(())
    })?;
    for _ in balance_iter {}

    let mut nft_stmt = conn.prepare(
        "SELECT nft_addr, image, name, symbol, collection FROM player_nft WHERE player_addr = ?1",
    )?;
    let nft_iter = nft_stmt.query_map(params![player_addr], |row| {
        let nft_addr: String = row.get(0)?;
        let nft = Nft {
            addr: nft_addr.clone(),
            image: row.get(1)?,
            name: row.get(2)?,
            symbol: row.get(3)?,
            collection: row.get(4)?,
        };
        nfts.insert(nft_addr, nft);
        Ok(())
    })?;
    for _ in nft_iter {}

    Ok(Some(PlayerInfo {
        balances,
        nfts,
        profile: player_profile,
    }))
}

pub fn update_player_info(conn: &Connection, player_info: &PlayerInfo) -> Result<()> {
    let profile = &player_info.profile;
    conn.execute(
        "UPDATE player_info SET nick = ?1, pfp = ?2, credentials = ?3 WHERE addr = ?4",
        params![profile.nick, profile.pfp, profile.credentials, profile.addr],
    )?;

    conn.execute(
        "DELETE FROM player_balance WHERE player_addr = ?1",
        params![profile.addr],
    )?;
    for (token_addr, balance) in &player_info.balances {
        conn.execute(
            "INSERT INTO player_balance (player_addr, token_addr, balance) VALUES (?1, ?2, ?3)",
            params![profile.addr, token_addr, balance],
        )?;
    }

    conn.execute(
        "DELETE FROM player_nft WHERE player_addr = ?1",
        params![profile.addr],
    )?;
    for (nft_addr, nft) in &player_info.nfts {
        conn.execute(
            "INSERT INTO player_nft (player_addr, nft_addr, image, name, symbol, collection) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![profile.addr, nft_addr, nft.image, nft.name, nft.symbol, nft.collection],
        )?;
    }

    Ok(())
}

#[allow(unused)]
pub fn delete_player_info(conn: &Connection, player_addr: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM player_info WHERE addr = ?1",
        params![player_addr],
    )?;
    conn.execute(
        "DELETE FROM player_balance WHERE player_addr = ?1",
        params![player_addr],
    )?;
    conn.execute(
        "DELETE FROM player_nft WHERE player_addr = ?1",
        params![player_addr],
    )?;
    Ok(())
}

// CRUD functions for Nft

#[allow(unused)]
pub fn create_nft(conn: &Connection, nft: &Nft) -> Result<()> {
    conn.execute(
        "INSERT INTO nft (addr, image, name, symbol, collection) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![nft.addr, nft.image, nft.name, nft.symbol, nft.collection],
    )?;
    Ok(())
}

#[allow(unused)]
pub fn read_nft(conn: &Connection, nft_addr: &str) -> Result<Option<Nft>> {
    let mut stmt =
        conn.prepare("SELECT addr, image, name, symbol, collection FROM nft WHERE addr = ?1")?;

    let mut nft_iter = stmt.query_map(params![nft_addr], |row| {
        Ok(Nft {
            addr: row.get(0)?,
            image: row.get(1)?,
            name: row.get(2)?,
            symbol: row.get(3)?,
            collection: row.get(4)?,
        })
    })?;

    if let Some(nft) = nft_iter.next() {
        nft.map(Some)
    } else {
        Ok(None)
    }
}

#[allow(unused)]
pub fn update_nft(conn: &Connection, nft: &Nft) -> Result<()> {
    conn.execute(
        "UPDATE nft SET image = ?1, name = ?2, symbol = ?3, collection = ?4 WHERE addr = ?5",
        params![nft.image, nft.name, nft.symbol, nft.collection, nft.addr],
    )?;
    Ok(())
}

#[allow(unused)]
pub fn delete_nft(conn: &Connection, nft_addr: &str) -> Result<()> {
    conn.execute("DELETE FROM nft WHERE addr = ?1", params![nft_addr])?;
    Ok(())
}

#[allow(unused)]
pub fn create_nft_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS nft (
            addr TEXT PRIMARY KEY,
            image TEXT NOT NULL,
            name TEXT NOT NULL,
            symbol TEXT NOT NULL,
            collection TEXT NOT NULL
        )",
        [],
    )?;
    Ok(())
}

// Function to create player related tables
#[allow(unused)]
pub fn create_player_tables(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS player_info (
            addr TEXT PRIMARY KEY,
            nick TEXT NOT NULL,
            pfp TEXT,
            credentials BLOB
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS player_balance (
            player_addr TEXT,
            token_addr TEXT,
            balance INTEGER,
            PRIMARY KEY (player_addr, token_addr),
            FOREIGN KEY (player_addr) REFERENCES player_info(addr)
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS player_nft (
            player_addr TEXT,
            nft_addr TEXT,
            image TEXT NOT NULL,
            name TEXT NOT NULL,
            symbol TEXT NOT NULL,
            collection TEXT,
            PRIMARY KEY (player_addr, nft_addr),
            FOREIGN KEY (player_addr) REFERENCES player_info(addr)
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS nft (
            addr TEXT PRIMARY KEY,
            image TEXT NOT NULL,
            name TEXT NOT NULL,
            symbol TEXT NOT NULL,
            collection TEXT
        )",
        [],
    )?;

    Ok(())
}

// Create a new GameAccount in the database
pub fn create_game_account(conn: &Connection, game_account: &GameAccount) -> Result<usize> {
    conn.execute(
        "INSERT INTO game_account (
            addr, title, bundle_addr, token_addr, owner_addr, settle_version, access_version,
            transactor_addr, unlock_time, max_players, data_len, data, entry_type, recipient_addr,
            players, deposits, servers, votes, checkpoint_on_chain, entry_lock, bonuses, balances
        ) VALUES (
            ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
        )",
        params![
            game_account.addr,
            game_account.title,
            game_account.bundle_addr,
            game_account.token_addr,
            game_account.owner_addr,
            game_account.settle_version,
            game_account.access_version,
            game_account.transactor_addr,
            game_account.unlock_time,
            game_account.max_players,
            game_account.data_len,
            game_account.data,
            serde_json::to_string(&game_account.entry_type).unwrap(),
            game_account.recipient_addr,
            serde_json::to_string(&game_account.players).unwrap(),
            serde_json::to_string(&game_account.deposits).unwrap(),
            serde_json::to_string(&game_account.servers).unwrap(),
            serde_json::to_string(&game_account.votes).unwrap(),
            serde_json::to_string(&game_account.checkpoint_on_chain).unwrap(),
            serde_json::to_string(&game_account.entry_lock).unwrap(),
            serde_json::to_string(&game_account.bonuses).unwrap(),
            serde_json::to_string(&game_account.balances).unwrap(),
        ],
    )
}

// Read a GameAccount by address
pub fn read_game_account(conn: &Connection, addr: &str) -> Result<Option<GameAccount>> {
    let mut stmt = conn.prepare(
        "SELECT addr, title, bundle_addr, token_addr, owner_addr, settle_version, access_version,
        transactor_addr, unlock_time, max_players, data_len, data, entry_type, recipient_addr,
        players, deposits, servers, votes, checkpoint_on_chain, entry_lock, bonuses, balances
        FROM game_account WHERE addr = ?",
    )?;

    let game_account = stmt
        .query_row(params![addr], |row| {
            Ok(GameAccount {
                addr: row.get(0)?,
                title: row.get(1)?,
                bundle_addr: row.get(2)?,
                token_addr: row.get(3)?,
                owner_addr: row.get(4)?,
                settle_version: row.get(5)?,
                access_version: row.get(6)?,
                transactor_addr: row.get(7)?,
                unlock_time: row.get(8)?,
                max_players: row.get(9)?,
                data_len: row.get(10)?,
                data: row.get(11)?,
                entry_type: serde_json::from_str(row.get::<_, String>(12)?.as_str()).unwrap(),
                recipient_addr: row.get(13)?,
                players: serde_json::from_str(row.get::<_, String>(14)?.as_str()).unwrap(),
                deposits: serde_json::from_str(row.get::<_, String>(15)?.as_str()).unwrap(),
                servers: serde_json::from_str(row.get::<_, String>(16)?.as_str()).unwrap(),
                votes: serde_json::from_str(row.get::<_, String>(17)?.as_str()).unwrap(),
                checkpoint_on_chain: serde_json::from_str(row.get::<_, String>(18)?.as_str())
                    .unwrap(),
                entry_lock: serde_json::from_str(row.get::<_, String>(19)?.as_str()).unwrap(),
                bonuses: serde_json::from_str(row.get::<_, String>(20)?.as_str()).unwrap(),
                balances: serde_json::from_str(row.get::<_, String>(21)?.as_str()).unwrap(),
            })
        })
        .optional()?;

    Ok(game_account)
}

pub fn list_game_accounts(conn: &Connection) -> Result<Vec<GameAccount>> {
    let mut stmt = conn.prepare(
        "SELECT addr, title, bundle_addr, token_addr, owner_addr, settle_version, access_version,
        transactor_addr, unlock_time, max_players, data_len, data, entry_type, recipient_addr,
        players, deposits, servers, votes, checkpoint_on_chain, entry_lock, bonuses, balances
        FROM game_account",
    )?;

    let game_account_iter = stmt.query_map([], |row| {
        Ok(GameAccount {
            addr: row.get(0)?,
            title: row.get(1)?,
            bundle_addr: row.get(2)?,
            token_addr: row.get(3)?,
            owner_addr: row.get(4)?,
            settle_version: row.get(5)?,
            access_version: row.get(6)?,
            transactor_addr: row.get(7)?,
            unlock_time: row.get(8)?,
            max_players: row.get(9)?,
            data_len: row.get(10)?,
            data: row.get(11)?,
            entry_type: serde_json::from_str(row.get::<_, String>(12)?.as_str()).unwrap(),
            recipient_addr: row.get(13)?,
            players: serde_json::from_str(row.get::<_, String>(14)?.as_str()).unwrap(),
            deposits: serde_json::from_str(row.get::<_, String>(15)?.as_str()).unwrap(),
            servers: serde_json::from_str(row.get::<_, String>(16)?.as_str()).unwrap(),
            votes: serde_json::from_str(row.get::<_, String>(17)?.as_str()).unwrap(),
            checkpoint_on_chain: serde_json::from_str(row.get::<_, String>(18)?.as_str()).unwrap(),
            entry_lock: serde_json::from_str(row.get::<_, String>(19)?.as_str()).unwrap(),
            bonuses: serde_json::from_str(row.get::<_, String>(20)?.as_str()).unwrap(),
            balances: serde_json::from_str(row.get::<_, String>(21)?.as_str()).unwrap(),
        })
    })?;

    let mut game_accounts = Vec::new();
    for game_account in game_account_iter {
        game_accounts.push(game_account?);
    }

    Ok(game_accounts)
}

// Update a GameAccount in the database
pub fn update_game_account(conn: &Connection, game_account: &GameAccount) -> Result<usize> {
    conn.execute(
        "UPDATE game_account SET
            title = ?, bundle_addr = ?, token_addr = ?, owner_addr = ?, settle_version = ?,
            access_version = ?, transactor_addr = ?, unlock_time = ?, max_players = ?, data_len = ?,
            data = ?, entry_type = ?, recipient_addr = ?, players = ?, deposits = ?, servers = ?,
            votes = ?, checkpoint_on_chain = ?, entry_lock = ?, balances = ?
        WHERE addr = ?",
        params![
            game_account.title,
            game_account.bundle_addr,
            game_account.token_addr,
            game_account.owner_addr,
            game_account.settle_version,
            game_account.access_version,
            game_account.transactor_addr,
            game_account.unlock_time,
            game_account.max_players,
            game_account.data_len,
            game_account.data,
            serde_json::to_string(&game_account.entry_type).unwrap(),
            game_account.recipient_addr,
            serde_json::to_string(&game_account.players).unwrap(),
            serde_json::to_string(&game_account.deposits).unwrap(),
            serde_json::to_string(&game_account.servers).unwrap(),
            serde_json::to_string(&game_account.votes).unwrap(),
            serde_json::to_string(&game_account.checkpoint_on_chain).unwrap(),
            serde_json::to_string(&game_account.entry_lock).unwrap(),
            serde_json::to_string(&game_account.balances).unwrap(),
            game_account.addr,
        ],
    )
}

// Delete a GameAccount by address
#[allow(unused)]
pub fn delete_game_account(conn: &Connection, addr: &str) -> Result<usize> {
    conn.execute("DELETE FROM game_account WHERE addr = ?", params![addr])
}

#[allow(unused)]
pub fn create_game_account_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS game_account (
            addr TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            bundle_addr TEXT NOT NULL,
            token_addr TEXT NOT NULL,
            owner_addr TEXT NOT NULL,
            settle_version INTEGER NOT NULL,
            access_version INTEGER NOT NULL,
            transactor_addr TEXT,
            unlock_time INTEGER,
            max_players INTEGER NOT NULL,
            data_len INTEGER NOT NULL,
            data BLOB,
            entry_type TEXT NOT NULL,          -- JSON serialized
            recipient_addr TEXT NOT NULL,
            players TEXT NOT NULL,             -- JSON serialized
            deposits TEXT NOT NULL,            -- JSON serialized
            servers TEXT NOT NULL,             -- JSON serialized
            votes TEXT NOT NULL,               -- JSON serialized
            checkpoint_on_chain TEXT,          -- JSON serialized
            entry_lock INTEGER NOT NULL,
            bonuses TEXT NOT NULL,             -- JSON serialized
            balances TEXT NOT NULL             -- JSON serialized
        )",
        [],
    )?;
    Ok(())
}

pub fn create_game_bundle(conn: &Connection, game_bundle: &GameBundle) -> Result<usize> {
    conn.execute(
        "INSERT OR IGNORE INTO game_bundle (addr, uri, name, data) VALUES (?, ?, ?, ?)",
        params![
            game_bundle.addr,
            game_bundle.uri,
            game_bundle.name,
            game_bundle.data
        ],
    )
}

// Read a GameBundle by uri
pub fn read_game_bundle(conn: &Connection, addr: &str) -> Result<Option<GameBundle>> {
    let mut stmt = conn.prepare("SELECT addr, uri, name, data FROM game_bundle WHERE addr = ?")?;

    let game_bundle = stmt
        .query_row(params![addr], |row| {
            Ok(GameBundle {
                addr: row.get(0)?,
                uri: row.get(1)?,
                name: row.get(2)?,
                data: row.get(3)?,
            })
        })
        .optional()?;

    Ok(game_bundle)
}

#[allow(unused)]
pub fn create_game_bundle_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS game_bundle (
            addr TEXT NOT NULL PRIMARY KEY,
            uri TEXT NOT NULL,
            name TEXT NOT NULL,
            data BLOB
        )",
        params![],
    )?;
    Ok(())
}

// CRUD functions for RegistrationAccount

#[allow(unused)]
pub fn create_registration_account(conn: &Connection, account: &RegistrationAccount) -> Result<()> {
    conn.execute(
        "INSERT INTO registration_account (addr, is_private, size, owner, games) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            account.addr,
            account.is_private,
            account.size,
            account.owner,
            serde_json::to_string(&account.games).unwrap()
        ],
    )?;
    Ok(())
}

#[allow(unused)]
pub fn read_registration_account(
    conn: &Connection,
    addr: &str,
) -> Result<Option<RegistrationAccount>> {
    let mut stmt = conn.prepare(
        "SELECT addr, is_private, size, owner, games FROM registration_account WHERE addr = ?1",
    )?;

    stmt.query_row(params![addr], |row| {
        Ok(RegistrationAccount {
            addr: row.get(0)?,
            is_private: row.get(1)?,
            size: row.get(2)?,
            owner: row.get(3)?,
            games: serde_json::from_str(row.get::<_, String>(4)?.as_str()).unwrap(),
        })
    })
        .optional()
}

#[allow(unused)]
pub fn update_registration_account(conn: &Connection, account: &RegistrationAccount) -> Result<()> {
    conn.execute(
        "UPDATE registration_account SET is_private = ?1, size = ?2, owner = ?3, games = ?4 WHERE addr = ?5",
        params![
            account.is_private,
            account.size,
            account.owner,
            serde_json::to_string(&account.games).unwrap(),
            account.addr
        ],
    )?;
    Ok(())
}

#[allow(unused)]
pub fn delete_registration_account(conn: &Connection, addr: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM registration_account WHERE addr = ?1",
        params![addr],
    )?;
    Ok(())
}

// Function to create the registration_account table

#[allow(unused)]
pub fn create_registration_account_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS registration_account (
            addr TEXT PRIMARY KEY,
            is_private BOOLEAN NOT NULL,
            size INTEGER NOT NULL,
            owner TEXT,
            games TEXT NOT NULL -- JSON serialized
        )",
        [],
    )?;
    Ok(())
}

// CRUD functions for TokenAccount

pub fn create_token_account(conn: &Connection, account: &TokenAccount) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO token_account (name, symbol, icon, addr, decimals) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            account.name,
            account.symbol,
            account.icon,
            account.addr,
            account.decimals
        ],
    )?;
    Ok(())
}

#[allow(unused)]
pub fn read_token_account(conn: &Connection, addr: &str) -> Result<Option<TokenAccount>> {
    let mut stmt = conn
        .prepare("SELECT name, symbol, icon, addr, decimals FROM token_account WHERE addr = ?1")?;

    stmt.query_row(params![addr], |row| {
        Ok(TokenAccount {
            name: row.get(0)?,
            symbol: row.get(1)?,
            icon: row.get(2)?,
            addr: row.get(3)?,
            decimals: row.get(4)?,
        })
    })
        .optional()
}

pub fn list_token_accounts(conn: &Connection) -> Result<Vec<TokenAccount>> {
    let mut stmt = conn.prepare("SELECT name, symbol, icon, addr, decimals FROM token_account")?;
    let token_account_iter = stmt.query_map([], |row| {
        Ok(TokenAccount {
            name: row.get(0)?,
            symbol: row.get(1)?,
            icon: row.get(2)?,
            addr: row.get(3)?,
            decimals: row.get(4)?,
        })
    })?;

    let mut token_accounts = Vec::new();
    for token_account in token_account_iter {
        token_accounts.push(token_account?);
    }

    Ok(token_accounts)
}

#[allow(unused)]
pub fn update_token_account(conn: &Connection, account: &TokenAccount) -> Result<()> {
    conn.execute(
        "UPDATE token_account SET name = ?1, symbol = ?2, icon = ?3, decimals = ?4 WHERE addr = ?5",
        params![
            account.name,
            account.symbol,
            account.icon,
            account.decimals,
            account.addr
        ],
    )?;
    Ok(())
}

#[allow(unused)]
pub fn delete_token_account(conn: &Connection, addr: &str) -> Result<()> {
    conn.execute("DELETE FROM token_account WHERE addr = ?1", params![addr])?;
    Ok(())
}

// Function to create the token_account table

#[allow(unused)]
pub fn create_token_account_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS token_account (
            name TEXT NOT NULL,
            symbol TEXT NOT NULL,
            icon TEXT NOT NULL,
            addr TEXT PRIMARY KEY,
            decimals INTEGER NOT NULL
        )",
        [],
    )?;
    Ok(())
}

// CRUD functions for RecipientAccount and related structures

// Create a new RecipientAccount
pub fn create_recipient_account(conn: &Connection, account: &RecipientAccount) -> Result<()> {
    conn.execute(
        "INSERT INTO recipient_account (addr, cap_addr) VALUES (?1, ?2)",
        params![account.addr, account.cap_addr],
    )?;

    for slot in &account.slots {
        create_recipient_slot(conn, account.addr.as_str(), slot)?;
    }

    Ok(())
}

// Read a RecipientAccount by address
pub fn read_recipient_account(conn: &Connection, addr: &str) -> Result<Option<RecipientAccount>> {
    let mut stmt = conn.prepare("SELECT addr, cap_addr FROM recipient_account WHERE addr = ?1")?;
    let account = stmt
        .query_row(params![addr], |row| {
            Ok(RecipientAccount {
                addr: row.get(0)?,
                cap_addr: row.get(1)?,
                slots: Vec::new(), // Slots will be loaded separately
            })
        })
        .optional()?;

    if let Some(mut account) = account {
        account.slots = read_recipient_slots(conn, addr)?;
        return Ok(Some(account));
    }

    Ok(None)
}

// Update a RecipientAccount
#[allow(unused)]
pub fn update_recipient_account(conn: &Connection, account: &RecipientAccount) -> Result<()> {
    conn.execute(
        "UPDATE recipient_account SET cap_addr = ?1 WHERE addr = ?2",
        params![account.cap_addr, account.addr],
    )?;

    // Delete existing slots and create new ones
    conn.execute(
        "DELETE FROM recipient_slot WHERE recipient_addr = ?1",
        params![account.addr],
    )?;
    for slot in &account.slots {
        create_recipient_slot(conn, account.addr.as_str(), slot)?;
    }

    Ok(())
}

// Delete a RecipientAccount
#[allow(unused)]
pub fn delete_recipient_account(conn: &Connection, addr: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM recipient_account WHERE addr = ?1",
        params![addr],
    )?;
    conn.execute(
        "DELETE FROM recipient_slot WHERE recipient_addr = ?1",
        params![addr],
    )?;
    Ok(())
}

// Function to create recipient_account and recipient_slot tables
#[allow(unused)]
pub fn create_recipient_account_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS recipient_account (
            addr TEXT PRIMARY KEY,
            cap_addr TEXT
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS recipient_slot (
            recipient_addr TEXT,
            id INTEGER,
            slot_type TEXT NOT NULL, -- Serialized RecipientSlotType
            token_addr TEXT,
            balance INTEGER NOT NULL,
            shares TEXT NOT NULL, -- Serialized Vec<RecipientSlotShare>
            PRIMARY KEY (recipient_addr, id),
            FOREIGN KEY (recipient_addr) REFERENCES recipient_account(addr)
        )",
        [],
    )?;
    Ok(())
}

// Helper Method: Create recipient_slot
pub fn create_recipient_slot(
    conn: &Connection,
    recipient_addr: &str,
    slot: &RecipientSlot,
) -> Result<()> {
    conn.execute(
        "INSERT INTO recipient_slot (recipient_addr, id, slot_type, token_addr, balance, shares)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            recipient_addr,
            slot.id,
            serde_json::to_string(&slot.slot_type).unwrap(),
            slot.token_addr,
            slot.balance,
            serde_json::to_string(&slot.shares).unwrap()
        ],
    )?;
    Ok(())
}

// Helper Method: Read recipient_slots
pub fn read_recipient_slots(conn: &Connection, recipient_addr: &str) -> Result<Vec<RecipientSlot>> {
    let mut stmt = conn.prepare("SELECT id, slot_type, token_addr, balance, shares FROM recipient_slot WHERE recipient_addr = ?1")?;
    let slot_iter = stmt.query_map(params![recipient_addr], |row| {
        Ok(RecipientSlot {
            id: row.get(0)?,
            slot_type: serde_json::from_str(row.get::<_, String>(1)?.as_str()).unwrap(),
            token_addr: row.get(2)?,
            balance: row.get(3)?,
            shares: serde_json::from_str(row.get::<_, String>(4)?.as_str()).unwrap(),
        })
    })?;

    let mut slots = Vec::new();
    for slot in slot_iter {
        slots.push(slot?);
    }

    Ok(slots)
}

// CRUD functions for ServerAccount

pub fn create_server_account(conn: &Connection, account: &ServerAccount) -> Result<()> {
    conn.execute(
        "INSERT INTO server_account (addr, endpoint, credentials) VALUES (?1, ?2, ?3)",
        params![account.addr, account.endpoint, account.credentials],
    )?;
    Ok(())
}

pub fn read_server_account(conn: &Connection, addr: &str) -> Result<Option<ServerAccount>> {
    let mut stmt = conn.prepare("SELECT addr, endpoint, credentials FROM server_account WHERE addr = ?1")?;
    stmt.query_row(params![addr], |row| {
        Ok(ServerAccount {
            addr: row.get(0)?,
            endpoint: row.get(1)?,
            credentials: row.get(2)?,
        })
    })
        .optional()
}

#[allow(unused)]
pub fn update_server_account(conn: &Connection, account: &ServerAccount) -> Result<()> {
    conn.execute(
        "UPDATE server_account SET endpoint = ?1 WHERE addr = ?2",
        params![account.endpoint, account.addr],
    )?;
    Ok(())
}

#[allow(unused)]
pub fn delete_server_account(conn: &Connection, addr: &str) -> Result<()> {
    conn.execute("DELETE FROM server_account WHERE addr = ?1", params![addr])?;
    Ok(())
}

// Function to create the server_account table

#[allow(unused)]
pub fn create_server_account_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS server_account (
            addr TEXT PRIMARY KEY,
            endpoint TEXT NOT NULL,
            credentials BLOB
        )",
        [],
    )?;
    Ok(())
}


pub fn prepare_all_tables(conn: &Connection) -> Result<()> {
    create_player_tables(conn)?;
    create_nft_table(conn)?;
    create_guest_tables(conn)?;
    create_product_state_tables(conn)?;
    create_game_account_table(conn)?;
    create_game_bundle_table(conn)?;
    create_registration_account_table(conn)?;
    create_token_account_table(conn)?;
    create_recipient_account_table(conn)?;
    create_server_account_table(conn)?;
    create_stake_table(conn)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, fs, time::{SystemTime, UNIX_EPOCH}};
    use rusqlite::{Connection, Result};
    use race_core::types::{GameAccount, PlayerProfile, RecipientAccount, RecipientSlot, TokenAccount};

    #[test]
    // Test creating and reading a player_info
    fn test_player_info_crud() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        super::prepare_all_tables(&conn)?;

        // Create PlayerInfo
        let player_profile = PlayerProfile {
            addr: "player1".to_string(),
            nick: "Player One".to_string(),
            pfp: Some("pfp1".to_string()),
            credentials: vec![1, 2, 3],
        };
        let balances = HashMap::from([("token1".to_string(), 100u64)]);
        let nft = super::Nft {
            addr: "nft1".to_string(),
            image: "image1".to_string(),
            name: "NFT One".to_string(),
            symbol: "N1".to_string(),
            collection: Some("Collection One".to_string()),
        };
        let nfts = HashMap::from([("nft1".to_string(), nft)]);
        let player_info = super::PlayerInfo {
            balances,
            nfts,
            profile: player_profile.clone(),
        };

        // Test Create
        super::create_player_info(&conn, &player_info)?;

        // Test Read
        let retrieved_player_info = super::read_player_info(&conn, "player1")?;
        assert_eq!(retrieved_player_info.unwrap().profile.nick, "Player One");

        Ok(())
    }

    #[test]
    // Test creating and reading a game_account
    fn test_game_account_crud() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        super::prepare_all_tables(&conn)?;

        // Create GameAccount
        let game_account = GameAccount {
            addr: "game1".to_string(),
            title: "Game One".to_string(),
            // Remaining fields omitted for brevity...
            ..Default::default()
        };

        // Test Create
        super::create_game_account(&conn, &game_account)?;

        // Test Read
        let retrieved_game_account = super::read_game_account(&conn, "game1")?;
        assert_eq!(retrieved_game_account.unwrap().title, "Game One");

        Ok(())
    }

    #[test]
    // Test creating and reading a token_account
    fn test_token_account_crud() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        super::prepare_all_tables(&conn)?;

        // Create TokenAccount
        let token_account = TokenAccount {
            name: "Token One".to_string(),
            symbol: "T1".to_string(),
            icon: "icon1".to_string(),
            addr: "token1".to_string(),
            decimals: 8,
        };

        // Test Create
        super::create_token_account(&conn, &token_account)?;

        // Test Read
        let retrieved_token_account = super::read_token_account(&conn, "token1")?;
        assert_eq!(retrieved_token_account.unwrap().symbol, "T1");

        Ok(())
    }

    #[test]
    // Test creating and reading a recipient_account
    fn test_recipient_account_crud() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        super::prepare_all_tables(&conn)?;

        // Create RecipientAccount and RecipientSlot
        let slot = RecipientSlot {
            id: 1,
            shares: vec![],
            token_addr: "token".to_string(),
            slot_type: race_core::types::RecipientSlotType::Token,
            balance: 0,
        };
        let recipient_account = RecipientAccount {
            addr: "recipient1".to_string(),
            cap_addr: Some("cap1".to_string()),
            slots: vec![slot],
        };

        // Test Create
        super::create_recipient_account(&conn, &recipient_account)?;

        // Test Read
        let retrieved_recipient_account = super::read_recipient_account(&conn, "recipient1")?;
        assert_eq!(retrieved_recipient_account.unwrap().cap_addr.unwrap(), "cap1");

        Ok(())
    }

    #[test]
    fn test_guest_account_and_session_crud() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        super::prepare_all_tables(&conn)?;

        let guest_account = super::GuestAccount {
            guest_id: "guest-1".into(),
            player_addr: "guest_player_1".into(),
            nick: "SmokeGuest".into(),
            status: "active".into(),
            created_at: 100,
            updated_at: 100,
        };
        let guest_session = super::GuestSession {
            session_id: "session-1".into(),
            guest_id: "guest-1".into(),
            session_token_hash: "token-hash".into(),
            created_at: 100,
            expires_at: 200,
            revoked_at: None,
        };

        super::create_guest_account(&conn, &guest_account)?;
        super::create_guest_session(&conn, &guest_session)?;

        let stored_account = super::read_guest_account_by_guest_id(&conn, "guest-1")?.unwrap();
        assert_eq!(stored_account.player_addr, "guest_player_1");

        let stored_session =
            super::read_guest_session_by_token_hash(&conn, "token-hash")?.unwrap();
        assert_eq!(stored_session.guest_id, "guest-1");

        super::revoke_guest_session(&conn, "token-hash", 150)?;
        let revoked_session =
            super::read_guest_session_by_token_hash(&conn, "token-hash")?.unwrap();
        assert_eq!(revoked_session.revoked_at, Some(150));

        Ok(())
    }

    #[test]
    fn test_guest_data_persists_across_sqlite_reopen() -> Result<()> {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let db_path = std::env::temp_dir().join(format!("race_facade_guest_persist_{unique}.sqlite"));
        let _ = fs::remove_file(&db_path);

        {
            let conn = Connection::open(&db_path)?;
            super::prepare_all_tables(&conn)?;

            let player_info = super::PlayerInfo {
                balances: HashMap::from([("FACADE_GUEST_CHIPS".to_string(), 1_000_000u64)]),
                nfts: HashMap::new(),
                profile: PlayerProfile {
                    addr: "guest_player_persist".to_string(),
                    nick: "Persisted Guest".to_string(),
                    pfp: None,
                    credentials: vec![7, 8, 9],
                },
            };
            let guest_account = super::GuestAccount {
                guest_id: "guest-persist".into(),
                player_addr: "guest_player_persist".into(),
                nick: "Persisted Guest".into(),
                status: "active".into(),
                created_at: 100,
                updated_at: 100,
            };
            let guest_session = super::GuestSession {
                session_id: "session-persist".into(),
                guest_id: "guest-persist".into(),
                session_token_hash: "token-hash-persist".into(),
                created_at: 100,
                expires_at: 200,
                revoked_at: None,
            };

            super::create_player_info(&conn, &player_info)?;
            super::create_guest_account(&conn, &guest_account)?;
            super::create_guest_session(&conn, &guest_session)?;
        }

        {
            let conn = Connection::open(&db_path)?;
            super::prepare_all_tables(&conn)?;

            let stored_account =
                super::read_guest_account_by_guest_id(&conn, "guest-persist")?.unwrap();
            assert_eq!(stored_account.player_addr, "guest_player_persist");

            let stored_session =
                super::read_guest_session_by_token_hash(&conn, "token-hash-persist")?.unwrap();
            assert_eq!(stored_session.guest_id, "guest-persist");

            let stored_player =
                super::read_player_info(&conn, "guest_player_persist")?.unwrap();
            assert_eq!(stored_player.profile.nick, "Persisted Guest");
            assert_eq!(
                stored_player.balances.get("FACADE_GUEST_CHIPS").copied(),
                Some(1_000_000)
            );

            super::revoke_guest_session(&conn, "token-hash-persist", 150)?;
        }

        {
            let conn = Connection::open(&db_path)?;
            super::prepare_all_tables(&conn)?;
            let revoked_session =
                super::read_guest_session_by_token_hash(&conn, "token-hash-persist")?.unwrap();
            assert_eq!(revoked_session.revoked_at, Some(150));
        }

        let _ = fs::remove_file(&db_path);
        Ok(())
    }
}
