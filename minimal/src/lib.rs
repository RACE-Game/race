use core::slice;
use std::alloc::dealloc;
use std::alloc::Layout;
use std::ffi::c_void;
use std::mem;
use std::ptr::copy;
use std::str;

use borsh::{BorshDeserialize, BorshSerialize};
use race_core::context::GameContext;
use race_core::engine::{GameHandler, Result};
use race_core::event::CustomEvent;
use race_core::event::Event;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum GameEvent {
    Increase(u64),
}

impl CustomEvent for GameEvent {}

#[derive(Default, Serialize, Deserialize)]
pub struct Minimal {
    pub counter: u64,
}

impl Minimal {
    fn handle_custom_event(&mut self, context: &mut GameContext, event: GameEvent) -> Result<()> {
        match event {
            GameEvent::Increase(n) => {
                self.counter += n;
            }
        }
        Ok(())
    }
}

impl GameHandler for Minimal {
    fn handle_event(&mut self, context: &mut GameContext, event: Event) -> Result<()> {
        match event {
            Event::Custom(s) => {
                let event: GameEvent = serde_json::from_str(&s)?;
                self.handle_custom_event(context, event)
            }
            _ => {
                context.dispatch(Event::custom(&GameEvent::Increase(1)), 0);
                Ok(())
            }
        }
    }
}

// to be generated

#[no_mangle]
pub extern "C" fn handle_event(context_size: u32, event_size: u32) -> u32 {
    let context_ptr = 1 as *mut u8;
    let context_slice = unsafe { slice::from_raw_parts_mut(context_ptr, context_size as _) };
    let mut context = GameContext::try_from_slice(&context_slice).unwrap();
    let event_ptr = unsafe { context_ptr.add(context_size as usize) };
    let event_slice = unsafe { slice::from_raw_parts(event_ptr, event_size as _) };
    let event = Event::try_from_slice(&event_slice).unwrap();
    let mut handler = if let Some(ref state_json) = context.state_json {
        serde_json::from_str(&state_json).unwrap()
    } else {
        Minimal::default()
    };
    handler.handle_event(&mut context, event).unwrap();
    context.state_json = Some(serde_json::to_string(&handler).unwrap());
    let context_vec = context.try_to_vec().unwrap();
    unsafe { copy(context_vec.as_ptr(), context_ptr, context_vec.len()) }
    context_vec.len().try_into().unwrap()
}
