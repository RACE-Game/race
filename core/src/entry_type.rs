use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum EntryType {
    /// A player can join the game by sending assets to game account directly
    #[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
    Cash { min_deposit: u64, max_deposit: u64 },
    /// A player can join the game by pay a ticket.
    #[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
    Ticket { amount: u64 },
    /// A player can join the game by showing a gate NFT
    #[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
    Gating { collection: String },
    #[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
    Disabled,
}

impl Default for EntryType {
    fn default() -> Self {
        EntryType::Cash {
            min_deposit: 0,
            max_deposit: 1000000,
        }
    }
}
