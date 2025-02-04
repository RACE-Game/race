//! Database related code for facade.
//! A memory-based instance is preferred.

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

// CRUD functions for PlayerInfo

pub fn create_player_info(conn: &Connection, player_info: &PlayerInfo) -> Result<()> {
    let profile = &player_info.profile;
    conn.execute(
        "INSERT INTO player_info (addr, nick, pfp) VALUES (?1, ?2, ?3)",
        params![profile.addr, profile.nick, profile.pfp],
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
    let mut stmt = conn.prepare("SELECT addr, nick, pfp FROM player_info WHERE addr = ?1")?;
    let mut player_iter = stmt.query_map(params![player_addr], |row| {
        Ok(PlayerProfile {
            addr: row.get(0)?,
            nick: row.get(1)?,
            pfp: row.get(2)?,
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
        "UPDATE player_info SET nick = ?1, pfp = ?2 WHERE addr = ?3",
        params![profile.nick, profile.pfp, profile.addr],
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
            pfp TEXT
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
        "INSERT INTO game_bundle (addr, uri, name, data) VALUES (?, ?, ?, ?)",
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
        "INSERT INTO token_account (name, symbol, icon, addr, decimals) VALUES (?1, ?2, ?3, ?4, ?5)",
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
        "INSERT INTO server_account (addr, endpoint) VALUES (?1, ?2)",
        params![account.addr, account.endpoint],
    )?;
    Ok(())
}

pub fn read_server_account(conn: &Connection, addr: &str) -> Result<Option<ServerAccount>> {
    let mut stmt = conn.prepare("SELECT addr, endpoint FROM server_account WHERE addr = ?1")?;
    stmt.query_row(params![addr], |row| {
        Ok(ServerAccount {
            addr: row.get(0)?,
            endpoint: row.get(1)?,
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
            endpoint TEXT NOT NULL
        )",
        [],
    )?;
    Ok(())
}


pub fn prepare_all_tables(conn: &Connection) -> Result<()> {
    create_player_tables(conn)?;
    create_nft_table(conn)?;
    create_game_account_table(conn)?;
    create_game_bundle_table(conn)?;
    create_registration_account_table(conn)?;
    create_token_account_table(conn)?;
    create_recipient_account_table(conn)?;
    create_server_account_table(conn)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
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
}
