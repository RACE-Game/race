//! A simple counter, countes how many players sad "Hey!".

use borsh::{BorshDeserialize, BorshSerialize};
use race_core::context::GameContext;
use race_core::engine::GameHandler;
use race_core::error::Result;
use race_core::event::CustomEvent;
use race_core::event::Event;
use race_core::random::deck_of_cards;
use race_core::types::{GameAccount, RandomId};
use race_proc_macro::game_handler;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum GameEvent {
    Increase(u64),
    RandomPoker,
    RandomDice,
}

impl CustomEvent for GameEvent {}

#[game_handler]
#[derive(Default, Deserialize, Serialize)]
pub struct Counter {
    value: u64,
    poker_random_id: RandomId,
    poker_card: String,
    num_of_players: u64,
    num_of_servers: u64,
}

#[derive(Default, BorshSerialize, BorshDeserialize)]
pub struct CounterAccountData {
    pub init_value: u64,
}

impl Counter {
    fn handle_custom_event(&mut self, _context: &mut GameContext, event: GameEvent) -> Result<()> {
        match event {
            GameEvent::Increase(n) => {
                self.value += n;
            }
            GameEvent::RandomPoker => {

            }
            GameEvent::RandomDice => {

            }
        }
        Ok(())
    }
}

impl GameHandler for Counter {
    fn init_state(context: &mut GameContext, init_account: GameAccount) -> Result<Self> {
        let data = init_account.data;
        let account_data = CounterAccountData::try_from_slice(&data)?;
        context.set_allow_exit(true);
        Ok(Self {
            value: account_data.init_value,
            poker_random_id: 0,
            poker_card: "??".into(),
            num_of_players: init_account.players.len() as _,
            num_of_servers: init_account.servers.len() as _,
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
                self.num_of_players += 1;
                // Create some randomness
                let poker_spec = deck_of_cards();
                self.poker_random_id = context.init_random_state(&poker_spec)?;
                Ok(())
            }
            Event::RandomnessReady => {
                context.reveal(self.poker_random_id, vec![0])?;
                Ok(())
            }
            Event::SecretsReady => {
                let revealed = context.get_revealed(self.poker_random_id)?;
                println!("Revealed: {:?}", revealed);
                let card = revealed.get(&0).unwrap();
                self.poker_card = card.to_owned();
                Ok(())
            }
            Event::Leave { player_addr: _ } => {
                self.num_of_players -= 1;
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use race_test::TestGameAccountBuilder;

    use super::*;

    fn init_context() -> GameContext {
        let game_account = TestGameAccountBuilder::default().add_servers(1).build();
        GameContext::try_new(&game_account).unwrap()
    }

    #[test]
    fn test_player_join() {
        let mut ctx = init_context();
        let evt = Event::Join {
            player_addr: "Alice".into(),
            balance: 1000,
            position: 0,
        };
        let mut hdlr = Counter::default();
        hdlr.handle_event(&mut ctx, evt)
            .expect("handle event error");
        assert_eq!(1, hdlr.num_of_players);
    }

    #[test]
    fn test_increase() {
        let mut ctx = init_context();
        let evt = Event::custom(
            ctx.get_transactor_addr().to_owned(),
            &GameEvent::Increase(1),
        );
        let mut hdlr = Counter::default();
        hdlr.handle_event(&mut ctx, evt).unwrap();
        assert_eq!(1, hdlr.value);
    }
}
