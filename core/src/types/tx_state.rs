//! Used for the broadcaster to indicate various on-chain states
use crate::types::accounts::PlayerJoin;
use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum TxState {
    PlayerConfirming {
        confirm_players: Vec<PlayerJoin>,
        access_version: u64,
    },

    PlayerConfirmingFailed(u64),
}
