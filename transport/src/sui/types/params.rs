//! Parameters for SuiTransport

use bcs;
use race_api::types::EntryLock;
use race_core::types::{
    AssignRecipientParams, CreateGameAccountParams, CreatePlayerProfileParams,
    CreateRegistrationParams, EntryType, JoinParams, PublishGameParams,
    RecipientSlotShareInit, RecipientSlotType,
    RegisterServerParams, ServeParams, Transfer, VoteParams,
    VoteType,
};
use serde::{Serialize, Deserialize};
use std::str::FromStr;
use sui_sdk::types::base_types::SuiAddress;
use super::common::RecipientSlotOwner;
use crate::error::{TransportError, TransportResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McRecipientSlotShareInit {
    pub owner: RecipientSlotOwner,
    pub weights: u16,
}

impl TryFrom<RecipientSlotShareInit> for McRecipientSlotShareInit {
    type Error = TransportError;

    fn try_from(value: RecipientSlotShareInit) -> Result<Self, Self::Error> {
        let RecipientSlotShareInit {
            owner,
            weights,
        } = value;
        Ok(Self {
            owner: owner.try_into()?,
            weights,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McRecipientSlotInit {
    pub id: u8,
    pub slot_type: RecipientSlotType,
    // TODO: rename to coin_addr?
    pub token_addr: SuiAddress,
    pub init_shares: Vec<McRecipientSlotShareInit>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McCreateGameAccountParams {
    pub title: String,
    pub max_players: u16,
    pub entry_type: EntryType,
    pub data: Vec<u8>,
}

impl From<CreateGameAccountParams> for McCreateGameAccountParams {
    fn from(value: CreateGameAccountParams) -> Self {
        Self {
            title: value.title,
            max_players: value.max_players,
            entry_type: value.entry_type,
            data: value.data,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McRegisterServerParams {
    pub endpoint: String,
}

impl From<RegisterServerParams> for McRegisterServerParams {
    fn from(value: RegisterServerParams) -> Self {
        Self {
            endpoint: value.endpoint,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McCreateRegistrationParams {
    pub is_private: bool,
    pub size: u16,
}

impl From<CreateRegistrationParams> for McCreateRegistrationParams {
    fn from(value: CreateRegistrationParams) -> Self {
        let CreateRegistrationParams { is_private, size } = value;
        Self { is_private, size }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterGameParams {
    pub game_addr: String,
    pub reg_addr: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnregisterGameParams {
    pub game_addr: String,
    pub reg_addr: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McCreatePlayerProfileParams {
    pub nick: String,
}

impl From<CreatePlayerProfileParams> for McCreatePlayerProfileParams {
    fn from(value: CreatePlayerProfileParams) -> Self {
        Self { nick: value.nick }
    }
}

/// The player status in settlement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerStatus {
    Normal,
    Left,
    Dropout,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetChange {
    Add,
    Sub,
    NoChange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McSettle {
    pub access_version: u64,
    pub amount: u64,
    pub eject: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McSettleParams {
    pub settles: Vec<McSettle>,
    pub transfers: Vec<Transfer>,
    pub checkpoint: Vec<u8>,
    pub settle_version: u64,
    pub next_settle_version: u64,
    pub entry_lock: Option<EntryLock>,
    pub reset: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McJoinParams {
    pub amount: u64,
    pub access_version: u64,
    pub position: u16,
    pub verify_key: String,
}

impl From<JoinParams> for McJoinParams {
    fn from(value: JoinParams) -> Self {
        let JoinParams {
            amount,
            access_version,
            position,
            verify_key,
            ..
        } = value;
        Self {
            amount,
            access_version,
            position,
            verify_key,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McServeParams {
    pub verify_key: String,
}

impl From<ServeParams> for McServeParams {
    fn from(value: ServeParams) -> Self {
        Self {
            verify_key: value.verify_key,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepositParams {
    pub amount: u64,
    pub settle_version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McVoteParams {
    pub vote_type: VoteType,
}

impl From<VoteParams> for McVoteParams {
    fn from(value: VoteParams) -> Self {
        Self {
            vote_type: value.vote_type,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McPublishParams {
    // Arweave IDX pointing to bundled game data
    pub uri: String,
    pub name: String,
    pub symbol: String,
}

impl From<PublishGameParams> for McPublishParams {
    fn from(value: PublishGameParams) -> Self {
        Self {
            uri: value.uri,
            name: value.name,
            symbol: value.symbol,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McCreateRecipientParams {
    pub slots: Vec<McRecipientSlotInit>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McAssignRecipientParams {
    pub identifier: String,
}

impl From<AssignRecipientParams> for McAssignRecipientParams {
    fn from(value: AssignRecipientParams) -> Self {
        Self {
            identifier: value.identifier,
        }
    }
}
