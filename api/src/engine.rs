use borsh::{BorshDeserialize, BorshSerialize};

use crate::{effect::Effect, error::HandleResult, event::Event, init_account::InitAccount};

pub trait GameHandler: Sized + BorshSerialize + BorshDeserialize {
    /// Initialize handler state with on-chain game account data.  The
    /// initial state must be determined by the `init_account`.
    fn init_state(effect: &mut Effect, init_account: InitAccount) -> HandleResult<Self>;

    /// Handle event.
    fn handle_event(&mut self, effect: &mut Effect, event: Event) -> HandleResult<()>;
}
