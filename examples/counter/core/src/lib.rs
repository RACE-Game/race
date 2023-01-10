use borsh::{BorshDeserialize, BorshSerialize};
use race_core::context::GameContext;
use race_core::engine::GameHandler;
use race_core::error::{Error, Result};
use race_core::event::CustomEvent;
use race_core::event::Event;
use race_core::types::GameAccount;
use race_proc_macro::game_handler;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum GameEvent {
    Increase(u64),
    Dispatch,
}

impl CustomEvent for GameEvent {}

#[game_handler]
#[derive(Default, Deserialize, Serialize)]
pub struct Counter {
    counter_value: u64,
    counter_players: u64,
}

#[derive(Default, BorshSerialize, BorshDeserialize)]
pub struct CounterAccountData {
    pub counter_value_default: u64,
}

impl Counter {
    fn handle_custom_event(&mut self, context: &mut GameContext, event: GameEvent) -> Result<()> {
        match event {
            GameEvent::Increase(n) => {
                self.counter_value += n;
            }
            GameEvent::Dispatch => {
                context.dispatch_custom(&GameEvent::Increase(1), 0);
            }
        }
        Ok(())
    }
}

impl GameHandler for Counter {
    fn init_state(_context: &mut GameContext, init_account: GameAccount) -> Result<Self> {
        let data = init_account.data;
        let account_data =
            CounterAccountData::try_from_slice(&data).or(Err(Error::DeserializeError))?;
        Ok(Self {
            counter_value: account_data.counter_value_default,
            counter_players: init_account.players.len() as _,
        })
    }

    fn handle_event(&mut self, context: &mut GameContext, event: Event) -> Result<()> {
        match event {
            Event::Custom { sender: _, ref raw } => {
                self.handle_custom_event(context, serde_json::from_str(raw).unwrap())
            }
            Event::Join {
                player_addr: _,
                balance: _,
                position: _,
            } => {
                self.counter_players += 1;
                Ok(())
            }
            Event::Leave { player_addr: _ } => {
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
            balance: 1000,
            position: 0,
        };
        let mut hdlr = Counter::default();
        hdlr.handle_event(&mut ctx, evt).unwrap();
        assert_eq!(1, hdlr.counter_players);
    }

    #[test]
    fn test_dispatch() {
        let mut ctx = GameContext::default();
        let evt = Event::custom(ctx.get_transactor_addr().to_owned(), &GameEvent::Dispatch);
        let mut hdlr = Counter::default();
        hdlr.handle_event(&mut ctx, evt).unwrap();
        assert_eq!(
            Some(DispatchEvent::new(
                Event::custom(ctx.get_transactor_addr().to_owned(), &GameEvent::Increase(1)),
                0
            )),
            *ctx.get_dispatch()
        );
    }

    #[test]
    fn test_increase() {
        let mut ctx = GameContext::default();
        let evt = Event::custom(ctx.get_transactor_addr().to_owned(), &GameEvent::Increase(1));
        let mut hdlr = Counter::default();
        hdlr.handle_event(&mut ctx, evt).unwrap();
        assert_eq!(1, hdlr.counter_value);
    }
}
