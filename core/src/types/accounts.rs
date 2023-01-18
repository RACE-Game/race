//! The data structures for on-chain accounts.

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

/// Represent a player call the join instruction in contract.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, BorshSerialize, BorshDeserialize)]
pub struct PlayerJoin {
    pub addr: String,
    pub position: usize,
    pub access_version: u64,
}

impl PlayerJoin {
    pub fn new<S: Into<String>>(addr: S, position: usize, access_version: u64) -> Self {
        Self {
            addr: addr.into(),
            position,
            access_version,
        }
    }
}

/// Represent a player call the deposit instruction in contract.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, BorshSerialize, BorshDeserialize)]
pub struct PlayerDeposit {
    pub addr: String,
    pub amount: u64,
    pub access_version: u64,
}

impl PlayerDeposit {
    pub fn new<S: Into<String>>(addr: S, balance: u64, access_version: u64) -> Self {
        Self {
            addr: addr.into(),
            amount: balance,
            access_version,
        }
    }
}

/// The data represent the state of on-chain transactor registration.
#[derive(
    Debug, Default, Serialize, Deserialize, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq,
)]
pub struct ServerAccount {
    pub addr: String,
    // The public key of transactor owner
    pub owner_addr: String,
    // The endpoint for transactor server
    pub endpoint: String,
}

/// The data represent the state of on-chain game account.
/// A larger `access_serial` means the account has been updated by players.
/// The length of `players` is `max_players`.
#[derive(
    Debug, Default, Serialize, Deserialize, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq,
)]
pub struct GameAccount {
    pub addr: String,
    pub bundle_addr: String,
    pub settle_version: u64,
    pub access_version: u64,
    pub players: Vec<PlayerJoin>,
    pub deposits: Vec<PlayerDeposit>,
    pub server_addrs: Vec<String>,
    pub transactor_addr: Option<String>,
    pub max_players: u8,
    pub data_len: u32,
    pub data: Vec<u8>,
}

#[derive(
    Debug, Default, Serialize, Deserialize, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq,
)]
pub struct GameRegistration {
    pub addr: String,
    pub reg_time: u64,
    pub bundle_addr: String,
}

#[derive(
    Debug, Default, Serialize, Deserialize, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq,
)]
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
