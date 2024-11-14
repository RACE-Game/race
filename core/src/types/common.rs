use borsh::{BorshDeserialize, BorshSerialize};
use race_api::prelude::InitAccount;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
pub use race_api::types::*;

use crate::context::Node;

pub type SettleTransferCheckpoint = (Vec<Settle>, Vec<Transfer>, Vec<u8>);

#[derive(Debug, PartialEq, Eq, Clone, Copy, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum ClientMode {
    Player,
    Transactor,
    Validator,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum GameMode {
    Main,
    Sub,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Signature {
    pub signer: String,
    pub timestamp: u64,
    pub signature: Vec<u8>,
}

impl std::fmt::Display for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{:?}](signer: {}, timestamp: {})",
            self.signature, self.signer, self.timestamp
        )
    }
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct SubGameSpec {
    pub game_addr: String,
    pub game_id: usize,
    pub bundle_addr: String,
    pub nodes: Vec<Node>,
    pub access_version: u64,
    pub settle_version: u64,
    pub init_account: InitAccount,
}
