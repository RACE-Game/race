use std::sync::Mutex;

use race_core::{
    context::GameContext,
    engine::{GameHandler, Result},
    event::{CustomEvent, Event},
};
use serde::{Deserialize, Serialize};
use once_cell::sync::Lazy;

#[derive(Serialize, Deserialize)]
pub enum GameEvent {
    Fold,
    Check,
    Call,
    Bet(u64),
    Raise(u64),
}

impl CustomEvent for GameEvent {}

#[derive(Default, Serialize, Deserialize)]
pub struct Pot {
    pub owners: Vec<String>,
    pub amount: u64,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Holdem {
    pub pots: Vec<Pot>,
}

impl Holdem {
    fn handle_game_event(&mut self, context: &mut GameContext, event: GameEvent) -> Result<()> {
        match event {
            GameEvent::Fold => todo!(),
            GameEvent::Check => todo!(),
            GameEvent::Call => todo!(),
            GameEvent::Bet(_) => todo!(),
            GameEvent::Raise(_) => todo!(),
        }
    }
}

impl GameHandler for Holdem {
    fn handle_event(&mut self, context: &mut GameContext, event: Event) -> Result<()> {
        match event {
            Event::Custom(s) => {
                let event = serde_json::from_str(&s)?;
                self.handle_game_event(context, event)
            }
            _ => Ok(()),
        }
    }
}
