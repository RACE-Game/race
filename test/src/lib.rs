use std::mem::swap;

use borsh::BorshSerialize;
use race_core::context::GameContext;
use race_core::engine::{general_handle_event, general_init_state, GameHandler};
use race_core::error::Result;
use race_core::event::Event;
use race_core::types::{GameAccount, Player};

/// A wrapped handler for testing
/// This handler includes the general event handling, which is necessary for integration test.
pub struct TestHandler<H>
where
    H: GameHandler,
{
    handler: H,
}

impl<H: GameHandler> TestHandler<H> {
    pub fn init_state(context: &mut GameContext, init_account: GameAccount) -> Result<Self> {
        let mut new_context = context.clone();
        general_init_state(&mut new_context, &init_account)?;
        let handler = H::init_state(&mut new_context, init_account)?;
        swap(context, &mut new_context);
        Ok(Self { handler })
    }

    pub fn handle_event(&mut self, context: &mut GameContext, event: Event) -> Result<()> {
        let mut new_context = context.clone();
        general_handle_event(&mut new_context, &event)?;
        self.handler.handle_event(&mut new_context, event)?;
        swap(context, &mut new_context);
        Ok(())
    }

    pub fn get_state(&self) -> &H {
        &self.handler
    }
}

/// A helper function to create on-chain game account structure.
pub fn create_test_game_account<B: BorshSerialize>(
    players: Vec<Option<Player>>,
    max_players: u8,
    account_data: B,
) -> GameAccount {
    let data = account_data.try_to_vec().unwrap();
    let data_len = data.len();
    GameAccount {
        addr: "FAKE ACCOUNT ADDR".into(),
        bundle_addr: "FAKE BUNDLE ADDR".into(),
        settle_version: 0,
        access_version: 0,
        players,
        transactors: vec![Some("TRANSACTOR 0".into()), Some("TRANSACTOR 1".into())],
        max_players,
        data_len: data_len as _,
        data,
    }
}
