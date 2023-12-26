use borsh::{BorshDeserialize, BorshSerialize};

use crate::{
    effect::Effect,
    error::{HandleError, HandleResult},
    event::Event,
    prelude::GamePlayer, types::EntryType,
};

/// A subset of on-chain account, used for game handler
/// initialization.  The `access_version` may refer to an old state
/// when the game is started by transactor.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct InitAccount {
    pub addr: String,
    pub players: Vec<GamePlayer>,
    pub data: Vec<u8>,
    pub access_version: u64,
    pub settle_version: u64,
    pub max_players: u16,
    pub checkpoint: Vec<u8>,
    pub entry_type: EntryType,
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
        self.access_version += 1;
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
            addr: "".into(),
            players: Vec::new(),
            data: Vec::new(),
            access_version: 0,
            settle_version: 0,
            max_players: 10,
            checkpoint: Vec::new(),
            entry_type: EntryType::Cash {
                min_deposit: 100,
                max_deposit: 200,
            }
        }
    }
}

pub trait GameHandler: Sized + BorshSerialize + BorshDeserialize {
    type Checkpoint: BorshSerialize + BorshDeserialize;

    /// Initialize handler state with on-chain game account data.
    fn init_state(effect: &mut Effect, init_account: InitAccount) -> HandleResult<Self>;

    /// Handle event.
    fn handle_event(&mut self, effect: &mut Effect, event: Event) -> HandleResult<()>;

    /// Create checkpoint from current state.
    fn into_checkpoint(self) -> HandleResult<Self::Checkpoint>;
}
