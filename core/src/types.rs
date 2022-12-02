use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use crate::event::Event;

/// The data represent on-chain in-game player information.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Player {
    pub addr: String,
    pub balance: u64,
}

/// The data represent the state of on-chain game account.
/// A larger `access_serial` means the account has been updated by players.
/// A larger `settle_serial` means the account has been updated by transactors.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameAccount {
    pub addr: String,
    pub version: u64,
    pub settle_serial: u64,
    pub access_serial: u64,
    pub players: Vec<Option<Player>>,
    pub data: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameBundle {
    pub addr: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct CreateGameAccountParams {
    pub addr: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct GetAccountInfoParams {
    pub addr: String,
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct GetGameBundleParams {
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
pub enum AssetChangeType {
    Add,
    Sub,
    NoChange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct AssetChange {
    pub token_addr: String,
    pub change_type: AssetChangeType,
    pub amount: u64,
}

/// The data represents how a player's asset changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct Settle {
    pub player_addr: String,
    pub player_status: PlayerStatus,
    pub asset_change: AssetChange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct SettleParams {
    pub game_addr: String,
    pub settles: Vec<Settle>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinParams {
    pub player_addr: String,
    pub game_addr: String,
    pub amount: u64,
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
        players: Vec<Option<Player>>
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
        params: SettleParams
    }
}
