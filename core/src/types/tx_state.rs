use crate::types::PlayerJoin;
use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ConfirmingPlayer {
    id: u64,
    addr: String,
    position: u16,
    balance: u64,
}

impl From<PlayerJoin> for ConfirmingPlayer {
    fn from(value: PlayerJoin) -> Self {
        Self {
            id: value.access_version,
            addr: value.addr,
            position: value.position,
            balance: value.balance,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum TxState {
    PlayerConfirming {
        confirm_players: Vec<ConfirmingPlayer>,
        access_version: u64,
    },

    PlayerConfirmingFailed(u64),

    SettleSucceed {
        settle_version: u64,
        signature: Option<String>,
    },
}
