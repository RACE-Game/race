#![allow(unused)]
use borsh::{BorshDeserialize, BorshSerialize};
use race_core::{
    effect::Effect, engine::GameHandler, error::Result, event::Event, types::GameAccount,
};
use race_proc_macro::game_handler;

#[game_handler]
#[derive(BorshDeserialize, BorshSerialize)]
struct S {}

impl GameHandler for S {
    fn init_state(context: &mut Effect, init_account: GameAccount) -> Result<Self> {
        Ok(Self {})
    }

    fn handle_event(&mut self, context: &mut Effect, event: Event) -> Result<()> {
        Ok(())
    }
}

#[test]
pub fn test() {}
