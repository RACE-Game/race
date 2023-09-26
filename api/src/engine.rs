use borsh::{BorshDeserialize, BorshSerialize};

use crate::{
    error::HandleError,
    effect::Effect,
    event::Event,
    prelude::ServerJoin,
    types::{PlayerJoin},
};

/// A subset of on-chain account, used for game handler
/// initialization.  The `access_version` may refer to an old state
/// when the game is started by transactor.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct InitAccount {
    pub addr: String,
    pub players: Vec<PlayerJoin>,
    pub servers: Vec<ServerJoin>,
    pub data: Vec<u8>,
    pub access_version: u64,
    pub settle_version: u64,
    pub max_players: u16,
}

impl InitAccount {

    pub fn data<S: BorshDeserialize>(&self) -> Result<S, HandleError> {
        S::try_from_slice(&self.data).or(Err(HandleError::MalformedGameAccountData))
    }

    /// Add a new player.  This function is only available in tests.
    /// This function will panic when a duplicated position is
    /// specified.
    pub fn add_player<S: Into<String>>(
        &mut self,
        addr: S,
        position: usize,
        balance: u64,
        verify_key: String,
    ) {
        self.access_version += 1;
        let access_version = self.access_version;
        if self.players.iter().any(|p| p.position as usize == position) {
            panic!("Failed to add player, duplicated position");
        }
        self.players.push(PlayerJoin {
            position: position as _,
            balance,
            addr: addr.into(),
            access_version,
            verify_key,
        })
    }
}

impl Default for InitAccount {
    fn default() -> Self {
        Self {
            addr: "".into(),
            players: Vec::new(),
            servers: Vec::new(),
            data: Vec::new(),
            access_version: 0,
            settle_version: 0,
            max_players: 10,
        }
    }
}

pub trait GameHandler: Sized + BorshSerialize + BorshDeserialize {
    /// Initialize handler state with on-chain game account data.
    fn init_state(effect: &mut Effect, init_account: InitAccount) -> Result<Self, HandleError>;

    /// Handle event.
    fn handle_event(&mut self, effect: &mut Effect, event: Event) -> Result<(), HandleError>;
}
