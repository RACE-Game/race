//! Structs that represent Sui on-chain objects (those with UID or capabilities)
use bcs;
use borsh::{BorshDeserialize, BorshSerialize};
use race_core::{
    error::{Error, Result},
    checkpoint::CheckpointOnChain,
    entry_type::EntryType,
    types::{PlayerProfile, EntryLock, GameAccount, GameRegistration, VoteType, RecipientAccount, RecipientSlotType, RecipientSlot, RegistrationAccount, ServerAccount},
};
use serde::{Serialize, Deserialize};
use move_core_types::account_address::AccountAddress;
use sui_sdk::types::{
    base_types::{ObjectID, SuiAddress},
    transaction::Argument
};
use sui_json_rpc_types::{Coin, SuiMoveStruct, SuiMoveValue};
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
