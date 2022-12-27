use borsh::{BorshDeserialize, BorshSerialize};
use race_core::{
    context::{GameContext, GameContextUpdates},
    engine::GameHandler,
    error::Result,
    event::Event,
    types::GameAccount,
};
use race_proc_macro::game_handler;
use serde::{Deserialize, Serialize};

#[game_handler]
#[derive(Serialize, Deserialize)]
struct MyGameHandler {}

impl GameHandler for MyGameHandler {
    fn init_state(context: &GameContext, init_account: GameAccount) -> Result<Self> {
        Ok(Self {})
    }

    fn handle_event(&mut self, context: &GameContext, event: Event) -> Result<GameContextUpdates> {
        let updates = GameContextUpdates::default();
        Ok(updates)
    }
}

#[test]
fn test_macro() {}
