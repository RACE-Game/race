//! Parameters for SuiTransport
use bcs;
use race_api::types::EntryLock;
use race_core::types::{
    AssignRecipientParams, CreateGameAccountParams, CreatePlayerProfileParams,
    CreateRegistrationParams, EntryType, JoinParams, PublishGameParams,
    RegisterServerParams, ServeParams, Transfer, VoteParams,
    VoteType,
};
use serde::{Serialize, Deserialize};
use std::str::FromStr;
use sui_sdk::{
    rpc_types::SuiObjectDataFilter,
    types::base_types::{ObjectID, SuiAddress},
};
use crate::error::{TransportError, TransportResult};

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
pub struct DepositParams {
    pub amount: u64,
    pub settle_version: u64,
}


#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Debug, Clone)]
pub enum BonusType {
    // coin type used for bonus, not necessarily the same as that of game
    Coin(String),
    // id of the object as the bonus
    Object(ObjectID)
}

#[derive(Debug, Clone)]
pub struct AttachBonusParams {
    pub game_id: ObjectID,
    pub token_addr: String,         // coin type (token) used for game
    pub identifier: String,
    pub amount: u64,
    pub bonus_type: BonusType,
    pub filter: Option<SuiObjectDataFilter> // None when `bonus_type` is Coin

}
