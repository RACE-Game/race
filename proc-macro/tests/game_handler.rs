#![allow(unused)]
use race_core::prelude::*;
use race_proc_macro::game_handler;

#[game_handler]
#[derive(BorshDeserialize, BorshSerialize)]
struct S {}

impl GameHandler for S {
    fn init_state(context: &mut Effect, init_account: InitAccount) -> Result<Self> {
        Ok(Self {})
    }

    fn handle_event(&mut self, context: &mut Effect, event: Event) -> Result<()> {
        Ok(())
    }
}

#[test]
pub fn test() {}
