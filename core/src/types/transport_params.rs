//! Parameterrs for calling transports

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct CreateGameAccountParams {
    pub title: String,
    pub bundle_addr: String,
    pub max_players: u8,
    pub data: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct ServeParams {
    pub game_addr: String,
    pub server_addr: String,
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone)]
pub struct RegisterServerParams {
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
pub struct GetTransactorInfoParams {
    pub addr: String,
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct CloseGameAccountParams {
    pub addr: String,
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct CreatePlayerProfileParams {
    pub addr: String,
    pub nick: String,
    pub pfp: Option<String>,
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

/// The data represents how a player's asset & status changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum SettleOp {
    Eject,
    Add(u64),
    Sub(u64),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct Settle {
    pub addr: String,
    pub op: SettleOp,
}

impl Settle {
    pub fn add<S: Into<String>>(addr: S, amount: u64) -> Self {
        Self {
            addr: addr.into(),
            op: SettleOp::Add(amount),
        }
    }
    pub fn sub<S: Into<String>>(addr: S, amount: u64) -> Self {
        Self {
            addr: addr.into(),
            op: SettleOp::Sub(amount),
        }
    }
    pub fn eject<S: Into<String>>(addr: S) -> Self {
        Self {
            addr: addr.into(),
            op: SettleOp::Eject,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct SettleParams {
    pub addr: String,
    pub settles: Vec<Settle>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinParams {
    pub player_addr: String,
    pub game_addr: String,
    pub amount: u64,
    pub access_version: u64,
    pub position: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepositParams {
    pub player_addr: String,
    pub game_addr: String,
    pub amount: u64,
    pub settle_version: u64,
}
