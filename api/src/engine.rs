use borsh::{BorshDeserialize, BorshSerialize};

use crate::{effect::Effect, error::HandleResult, event::Event, init_account::InitAccount, types::PlayerBalance};

pub trait GameHandler: Sized + BorshSerialize + BorshDeserialize {
    /// Initialize handler state with on-chain game account data.
    fn init_state(effect: &mut Effect, init_account: InitAccount) -> HandleResult<Self>;

    /// Handle event.
    fn handle_event(&mut self, effect: &mut Effect, event: Event) -> HandleResult<()>;

    /// Report the balances of players.
    /// The return must contain all players and zero balance is allowed.
    fn balances(&self) -> Vec<PlayerBalance>;
}
