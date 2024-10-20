use borsh::{BorshDeserialize, BorshSerialize};

use crate::{
    error::HandleError,
    types::{EntryType, GamePlayer},
};

/// A set of arguments for game handler initialization.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct InitAccount {
    pub max_players: u16,
    pub entry_type: EntryType,
    pub players: Vec<GamePlayer>,
    pub data: Vec<u8>,
    pub checkpoint: Option<Vec<u8>>,
}

impl InitAccount {
    pub fn data<S: BorshDeserialize>(&self) -> Result<S, HandleError> {
        S::try_from_slice(&self.data).or(Err(HandleError::MalformedGameAccountData))
    }

    /// Add a new player.  This function is only available in tests.
    /// This function will panic when a duplicated position is
    /// specified.
    pub fn add_player(&mut self, id: u64, position: usize, balance: u64) {
        if self.players.iter().any(|p| p.position as usize == position) {
            panic!("Failed to add player, duplicated position");
        }
        self.players.push(GamePlayer {
            position: position as _,
            balance,
            id,
        })
    }
}

impl Default for InitAccount {
    fn default() -> Self {
        Self {
            max_players: 0,
            entry_type: EntryType::Disabled,
            players: Vec::new(),
            data: Vec::new(),
            checkpoint: None,
        }
    }
}
