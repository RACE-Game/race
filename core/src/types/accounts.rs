//! The data structures for on-chain accounts.

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

/// Represent a player call the join instruction in contract.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, BorshSerialize, BorshDeserialize)]
pub struct PlayerJoin {
    pub addr: String,
    pub position: usize,
    pub balance: u64,
    pub access_version: u64,
}

impl PlayerJoin {
    pub fn new<S: Into<String>>(addr: S, position: usize, balance: u64, access_version: u64) -> Self {
        Self {
            addr: addr.into(),
            position,
            balance,
            access_version,
        }
    }
}

/// Represent a player call the deposit instruction in contract.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, BorshSerialize, BorshDeserialize)]
pub struct PlayerDeposit {
    pub addr: String,
    pub amount: u64,
    pub settle_version: u64,
}

impl PlayerDeposit {
    pub fn new<S: Into<String>>(addr: S, balance: u64, settle_version: u64) -> Self {
        Self {
            addr: addr.into(),
            amount: balance,
            settle_version,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, BorshSerialize, BorshDeserialize)]
pub struct ServerJoin {
    pub addr: String,
    pub endpoint: String,
    pub access_version: u64
}

impl ServerJoin {
    pub fn new<S: Into<String>>(addr: S, endpoint: String, access_version: u64) -> Self {
        Self {
            addr: addr.into(),
            endpoint,
            access_version
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
///
/// # Access Version and Settle Version
///
/// Since the blockchain and transactor is not synchronized, and the
/// RPC services usually can't provide a sanitized response, we need
/// two serial number to reflect when the account is updated. We also
/// rely on these versions to filter out latest events.
///
/// * After a player joined, the `access_version` will be increased by 1.
/// * After a settlement processed, the `settle_version` will be increased by 1.
/// * After a server attached, the `access_version` will be increased by 1.
/// * A deposit will use current `settle_version` + 1 to represent an unhandled operation.
///
/// # Players and Servers
///
/// Non-transactor nodes can only add themselves to the `players` list
/// or `servers` list.  Only tranactor node can remove a player with
/// settlement transaction.
///
/// If on-chain account requires a fixed length array to represent these lists:
/// * The max length of `players` is `max_players`.
/// * The max length of `servers` is 10.
///
/// # Deposits
///
/// The `deposits` represents a deposit from a player during the game.
/// The initial join will not produce a deposit record. The timing of
/// deposit is identified by its `settle_version`. A newly generated
/// deposit must have a higher `settle_version` which is the one in
/// game account.  Then, in the settlement, the contract will increase
/// the `settle_version` by 1, then all deposits under the version
/// will be handled as well.
///
/// Expired deposit records can be safely deleted during the
/// settlement.
///
/// # Data and Data Len
///
/// Data is custom-formatted data that depends on the game logic. The
/// data is used to represent the properties of a game, thus they
/// should be immutable. If a mutable state is required, it must
/// always have the same length, which is specified by `data_len`.
///
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
    pub servers: Vec<ServerJoin>,
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
}
