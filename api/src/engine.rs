use borsh::{BorshDeserialize, BorshSerialize};

use crate::{
    effect::Effect,
    error::{HandleError, HandleResult},
    event::Event,
    prelude::GamePlayer,
    types::EntryType,
};

/// A set of arguments for game handler initialization.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct InitAccount {
    pub max_players: u16,
    pub entry_type: EntryType,
    pub players: Vec<GamePlayer>,
    pub data: Vec<u8>,
    pub checkpoint: Vec<u8>,
}

impl InitAccount {
    pub fn data<S: BorshDeserialize>(&self) -> Result<S, HandleError> {
        S::try_from_slice(&self.data).or(Err(HandleError::MalformedGameAccountData))
    }

    pub fn checkpoint<S: BorshDeserialize>(&self) -> Result<Option<S>, HandleError> {
        if self.checkpoint.is_empty() {
            Ok(None)
        } else {
            S::try_from_slice(&self.checkpoint)
                .or(Err(HandleError::MalformedCheckpointData))
                .map(Some)
        }
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
            checkpoint: Vec::new(),
        }
    }
}

pub trait GameHandler: Sized + BorshSerialize + BorshDeserialize {
    /// Initialize handler state with on-chain game account data.  The
    /// initial state must be determined by the `init_account`.
    fn init_state(effect: &mut Effect, init_account: InitAccount) -> HandleResult<Self>;

    /// Handle event.
    fn handle_event(&mut self, effect: &mut Effect, event: Event) -> HandleResult<()>;
}
