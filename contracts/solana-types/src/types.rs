//! Parameters for sonala contracts

use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "program")]
use solana_program::pubkey::Pubkey;
#[cfg(feature = "sdk")]
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct TokenInfo {
    name: String,
    addr: String,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct CreateGameAccountParams {
    pub title: String,
    // pub token: TokenInfo,
    pub max_players: u8,
    pub data: Vec<u8>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Clone)]
pub struct RegisterServerParams {
    pub endpoint: String,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct UnregisterTransactorParams {
    pub addr: String,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct CreateRegistrationParams {
    pub is_private: bool,
    pub size: u16,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct RegisterGameParams {
    pub game_addr: String,
    pub reg_addr: String,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct UnregisterGameParams {
    pub game_addr: String,
    pub reg_addr: String,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct GetTransactorInfoParams {
    pub addr: String,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct CreatePlayerProfileParams {
    pub nick: String,
}

/// The player status in settlement.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum PlayerStatus {
    Normal,
    Left,
    Dropout,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum AssetChange {
    Add,
    Sub,
    NoChange,
}

/// The data represents how a player's asset & status changed.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum SettleOp {
    Eject,
    Add(u64),
    Sub(u64),
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Settle {
    pub addr: Pubkey,
    pub op: SettleOp,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct SettleParams {
    pub settles: Vec<Settle>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinParams {
    pub player_addr: String,
    pub game_addr: String,
    pub amount: u64,
    pub access_version: u64,
    pub position: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepositParams {
    pub player_addr: String,
    pub game_addr: String,
    pub amount: u64,
    pub settle_version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum VoteType {
    ServerVoteTransactorDropOff,
    ClientVoteTransactorDropOff,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct VoteParams {
    pub vote_type: VoteType,
}
