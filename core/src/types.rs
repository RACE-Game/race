use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use crate::event::Event;

/// The data represent on-chain in-game player information.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, BorshSerialize, BorshDeserialize)]
pub struct Player {
    pub addr: String,
    pub balance: u64,
}

impl Player {
    pub fn new<S: Into<String>>(addr: S, balance: u64) -> Self {
        Self {
            addr: addr.into(),
            balance,
        }
    }
}

/// The data represent the state of on-chain transactor registration.
#[derive(Debug, Default, Serialize, Deserialize, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct TransactorAccount {
    pub addr: String,
    // The public key of transactor owner
    pub owner_addr: String,
    // The endpoint for transactor server
    pub endpoint: String,
}

/// The data represent the state of on-chain game account.
/// A larger `access_serial` means the account has been updated by players.
/// The length of `players` is `max_players`.
#[derive(Debug, Default, Serialize, Deserialize, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct GameAccount {
    pub addr: String,
    pub bundle_addr: String,
    pub settle_version: u64,
    pub access_version: u64,
    pub players: Vec<Player>,
    pub server_addrs: Vec<String>,
    pub transactor_addr: Option<String>,
    pub max_players: u8,
    pub data_len: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct GameRegistration {
    pub addr: String,
    pub reg_time: u64,
    pub bundle_addr: String,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct RegistrationAccount {
    pub addr: String,
    pub is_private: bool,
    pub size: u16,
    pub owner: Option<String>, // No owner for public registration
    pub games: Vec<GameRegistration>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameBundle {
    pub addr: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlayerProfile {
    pub addr: String,
    pub pfp: String,
    pub data: Vec<u8>,
}

// ---------------------------------------------
// RPC Parameters
// ---------------------------------------------

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct CreateGameAccountParams {
    pub bundle_addr: String,
    pub max_players: u8,
    pub data: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct ServeParams {
    pub account_addr: String,
    pub transactor_addr: String,
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone)]
pub struct RegisterTransactorParams {
    pub owner_addr: String,
    pub endpoint: String,
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct UnregisterTransactorParams {
    pub addr: String,
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct CreateRegistrationParams {
    pub is_private: bool,
    pub size: u16,
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct RegisterGameParams {
    pub game_addr: String,
    pub reg_addr: String,
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct UnregisterGameParams {
    pub game_addr: String,
    pub reg_addr: String,
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct GetAccountInfoParams {
    pub addr: String,
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct GetTransactorInfoParams {
    pub addr: String,
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct CloseGameAccountParams {
    pub addr: String,
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct GetGameBundleParams {
    pub addr: String,
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct GetRegistrationParams {
    pub addr: String,
}

/// The player status in settlement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum PlayerStatus {
    Normal,
    Left,
    Dropout,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum AssetChange {
    Add,
    Sub,
    NoChange,
}

/// The data represents how a player's asset changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct Settle {
    pub addr: String,
    pub status: PlayerStatus,
    pub change: AssetChange,
    pub amount: u64,
}

impl Settle {
    pub fn new<S: Into<String>>(addr: S, status: PlayerStatus, change: AssetChange, amount: u64) -> Self {
        Self {
            addr: addr.into(),
            status,
            change,
            amount,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct SettleParams {
    pub addr: String,
    pub settles: Vec<Settle>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachGameParams {
    pub addr: String,
    pub chain: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetStateParams {
    pub addr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetContextParams {
    pub addr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscribeEventParams {
    pub addr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinParams {
    pub player_addr: String,
    pub game_addr: String,
    pub amount: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendEventParams {
    pub addr: String,
    pub event: Event,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BroadcastFrame {
    pub addr: String,
    pub event: Event,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventFrame {
    Empty,
    PlayerJoined {
        addr: String,
        players: Vec<Player>,
    },
    SendEvent {
        addr: String,
        event: Event,
    },
    Broadcast {
        addr: String,
        state_json: String,
        event: Event,
    },
    Settle {
        addr: String,
        params: SettleParams,
    },
    Shutdown,
}
