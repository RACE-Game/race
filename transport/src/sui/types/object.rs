//! Structs that represent Sui on-chain objects (those with UID or capabilities)
use bcs;
use race_core::{
    error::{Error, Result},
    checkpoint::CheckpointOnChain,
    types::{EntryLock, EntryType, GameAccount, VoteType, RecipientAccount, RecipientSlotType, RecipientSlotOwner},
};
use serde::{Serialize, Deserialize};
use move_core_types::account_address::AccountAddress;
use sui_sdk::types::{
    base_types::{ObjectID, SuiAddress},
    transaction::Argument
};
use sui_json_rpc_types::{SuiMoveStruct, SuiMoveValue};
use std::collections::BTreeMap;

mod game;
mod server;
mod profile;
mod recipient;
mod registry;

pub(crate) use game::*;
pub(crate) use server::*;
pub(crate) use profile::*;
pub(crate) use recipient::*;
pub(crate) use registry::*;
