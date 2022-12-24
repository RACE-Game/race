use core::slice;
use std::ptr::copy;
use std::str;

use borsh::{BorshDeserialize, BorshSerialize};
use race_core::context::GameContext;
use race_core::engine::GameHandler;
use race_core::error::{Error, Result};
use race_core::event::CustomEvent;
use race_core::event::Event;
use race_core::types::GameAccount;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum GameEvent {
    Increase(u64),
    Dispatch,
}

impl CustomEvent for GameEvent {}

#[derive(Default, Serialize, Deserialize)]
pub struct Minimal {
    counter_value: u64,
    counter_players: u64,
}

#[derive(Default, BorshSerialize, BorshDeserialize)]
pub struct MinimalAccountData {
    pub counter_value_default: u64,
}

impl Minimal {
    fn handle_custom_event(&mut self, context: &mut GameContext, event: GameEvent) -> Result<()> {
        match event {
            GameEvent::Increase(n) => {
                self.counter_value += n;
            }
            GameEvent::Dispatch => {
                context.dispatch(Event::system_custom(&GameEvent::Increase(1)), 0);
            }
        }
        Ok(())
    }
}

impl GameHandler for Minimal {
    fn init_state(_context: &mut GameContext, init_account: GameAccount) -> Result<Self> {
        let data = init_account.data;
        let account_data = MinimalAccountData::try_from_slice(&data).or(Err(Error::DeserializeError))?;
        Ok(Self {
            counter_value: account_data.counter_value_default,
            counter_players: init_account.players.len() as _,
        })
    }

    fn handle_event(&mut self, context: &mut GameContext, event: Event) -> Result<()> {
        match event {
            Event::SystemCustom{ raw } => {
                let event: GameEvent = serde_json::from_str(&raw)?;
                self.handle_custom_event(context, event)
            }
            Event::Join {
                player_addr: _,
                balance: _,
            } => {
                self.counter_players += 1;
                Ok(())
            }
            Event::Leave {
                player_addr: _,
            } => {
                self.counter_players -= 1;
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use race_core::context::DispatchEvent;

    use super::*;

    #[test]
    fn test_player_join() {
        let mut ctx = GameContext::default();
        let evt = Event::Join {
            player_addr: "Alice".into(),
        };
        let mut hdlr = Minimal::default();
        hdlr.handle_event(&mut ctx, evt).unwrap();
        assert_eq!(1, hdlr.counter_players);
    }

    #[test]
    fn test_dispatch() {
        let mut ctx = GameContext::default();
        let evt = Event::system_custom(&GameEvent::Dispatch);
        let mut hdlr = Minimal::default();
        hdlr.handle_event(&mut ctx, evt).unwrap();
        assert_eq!(
            Some(DispatchEvent::new(Event::system_custom(&GameEvent::Increase(1)), 0)),
            ctx.dispatch
        );
    }

    #[test]
    fn test_increase() {
        let mut ctx = GameContext::default();
        let evt = Event::system_custom(&GameEvent::Increase(1));
        let mut hdlr = Minimal::default();
        hdlr.handle_event(&mut ctx, evt).unwrap();
        assert_eq!(1, hdlr.counter_value);
    }
}

// to be generated

pub fn read_ptr<T: BorshDeserialize>(ptr: &mut *mut u8, size: u32) -> T {
    let slice = unsafe { slice::from_raw_parts_mut(*ptr, size as _) };
    let parsed = T::try_from_slice(&slice).expect("Borsh deserialize error");
    *ptr = unsafe { ptr.add(size as _) };
    parsed
}

pub fn write_ptr<T: BorshSerialize>(ptr: &mut *mut u8, data: T) -> u32 {
    let vec = data.try_to_vec().expect("Borsh serialize error");
    unsafe { copy(vec.as_ptr(), *ptr, vec.len()) }
    *ptr = unsafe { ptr.add(vec.len() as _) };
    vec.len() as _
}

#[no_mangle]
pub extern "C" fn handle_event(context_size: u32, event_size: u32) -> u32 {
    let mut ptr = 1 as *mut u8;
    let mut context: GameContext = read_ptr(&mut ptr, context_size);
    let event: Event = read_ptr(&mut ptr, event_size);
    let mut handler: Minimal = serde_json::from_str(&context.state_json).unwrap();
    handler.handle_event(&mut context, event).unwrap();
    context.state_json = serde_json::to_string(&handler).unwrap();
    let mut ptr = 1 as *mut u8;
    write_ptr(&mut ptr, context)
}

#[no_mangle]
pub extern "C" fn init_state(context_size: u32, init_account_size: u32) -> u32 {
    let mut ptr = 1 as *mut u8;
    let mut context: GameContext = read_ptr(&mut ptr, context_size);
    let init_account: GameAccount = read_ptr(&mut ptr, init_account_size);
    let handler = Minimal::init_state(&mut context, init_account).unwrap();
    context.state_json = serde_json::to_string(&handler).unwrap();
    let mut ptr = 1 as *mut u8;
    write_ptr(&mut ptr, context)
}
