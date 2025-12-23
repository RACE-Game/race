use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use crate::entry_type::EntryType;

/// The static information of a game.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GameSpec {
    pub game_addr: String,
    pub game_id: usize,
    pub bundle_addr: String,
    pub max_players: u16,
    pub entry_type: EntryType,
}
