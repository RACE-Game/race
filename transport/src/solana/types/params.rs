use borsh::{BorshDeserialize, BorshSerialize};
use race_api::types::{BalanceChange, EntryLock};
use race_core::entry_type::EntryType;
use race_core::types::{
    AssignRecipientParams, CreateGameAccountParams, CreatePlayerProfileParams,
    CreateRegistrationParams, JoinParams, PublishGameParams,
    RecipientSlotShareInit, RecipientSlotType,
    RegisterServerParams, ServeParams, Transfer, VoteParams,
    VoteType, Award
};
use super::common::RecipientSlotOwner;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use crate::error::{TransportError, TransportResult};

pub fn parse_pubkey(addr: &str) -> TransportResult<Pubkey> {
    Pubkey::from_str(addr)
        .map_err(|_| TransportError::InvalidConfig(format!("Can't parse public key: {}", addr)))
}

#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct IxRecipientSlotShareInit {
    pub owner: RecipientSlotOwner,
    pub weights: u16,
}

impl TryFrom<RecipientSlotShareInit> for IxRecipientSlotShareInit {
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

#[cfg_attr(test, derive(PartialEq, Clone))]
#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct IxRecipientSlotInit {
    pub id: u8,
    pub slot_type: RecipientSlotType,
    pub token_addr: Pubkey,
    pub stake_addr: Pubkey,
    pub init_shares: Vec<IxRecipientSlotShareInit>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct IxCreateGameAccountParams {
    pub title: String,
    pub max_players: u16,
    pub entry_type: EntryType,
    pub data: Vec<u8>,
}

impl From<CreateGameAccountParams> for IxCreateGameAccountParams {
    fn from(value: CreateGameAccountParams) -> Self {
        Self {
            title: value.title,
            max_players: value.max_players,
            entry_type: value.entry_type,
            data: value.data,
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Clone)]
pub struct IxRegisterServerParams {
    pub endpoint: String,
    pub credentials: Vec<u8>,
}

impl From<RegisterServerParams> for IxRegisterServerParams {
    fn from(value: RegisterServerParams) -> Self {
        Self {
            endpoint: value.endpoint,
            credentials: value.credentials,
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Clone)]
pub struct IxCreateRegistrationParams {
    pub is_private: bool,
    pub size: u16,
}

impl From<CreateRegistrationParams> for IxCreateRegistrationParams {
    fn from(value: CreateRegistrationParams) -> Self {
        let CreateRegistrationParams { is_private, size } = value;
        Self { is_private, size }
    }
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
pub struct IxCreatePlayerProfileParams {
    pub nick: String,
}

impl From<CreatePlayerProfileParams> for IxCreatePlayerProfileParams {
    fn from(value: CreatePlayerProfileParams) -> Self {
        Self { nick: value.nick }
    }
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

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct IxSettle {
    pub access_version: u64,
    pub withdraw: u64,
    pub change: Option<BalanceChange>,
    pub eject: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct IxSettleParams {
    pub settles: Vec<IxSettle>,
    pub transfer: Option<Transfer>,
    pub awards: Vec<Award>,
    pub checkpoint: Vec<u8>,
    pub access_version: u64,
    pub settle_version: u64,
    pub next_settle_version: u64,
    pub entry_lock: Option<EntryLock>,
    pub accept_deposits: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct IxJoinParams {
    pub amount: u64,
    pub access_version: u64,
    pub settle_version: u64,
    pub position: u16,
}

impl From<JoinParams> for IxJoinParams {
    fn from(value: JoinParams) -> Self {
        let JoinParams {
            amount,
            access_version,
            settle_version,
            position,
            ..
        } = value;
        Self {
            amount,
            access_version,
            settle_version,
            position,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct IxServeParams {
}

impl From<ServeParams> for IxServeParams {
    fn from(_value: ServeParams) -> Self {
        Self {}
    }
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct DepositParams {
    pub amount: u64,
    pub settle_version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct IxVoteParams {
    pub vote_type: VoteType,
}

impl From<VoteParams> for IxVoteParams {
    fn from(value: VoteParams) -> Self {
        Self {
            vote_type: value.vote_type,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct IxDepositParams {
    pub amount: u64,
    pub settle_version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct IxPublishParams {
    // Arweave IDX pointing to bundled game data
    pub uri: String,
    pub name: String,
    pub symbol: String,
}

impl From<PublishGameParams> for IxPublishParams {
    fn from(value: PublishGameParams) -> Self {
        Self {
            uri: value.uri,
            name: value.name,
            symbol: value.symbol,
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct IxCreateRecipientParams {
    pub slots: Vec<IxRecipientSlotInit>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct IxAssignRecipientParams {
    pub identifier: String,
}

impl From<AssignRecipientParams> for IxAssignRecipientParams {
    fn from(value: AssignRecipientParams) -> Self {
        Self {
            identifier: value.identifier,
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct IxAttachBonusParams {
    pub identifiers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct IxRejectDepositsParams {
    pub reject_deposits: Vec<u64>,
}
