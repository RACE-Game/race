//! A simple counter, countes how many players sad "Hey!".

use borsh::{BorshDeserialize, BorshSerialize};
use race_core::context::GameContext;
use race_core::context::GameStatus;
use race_core::engine::GameHandler;
use race_core::error::Error;
use race_core::error::Result;
use race_core::event::CustomEvent;
use race_core::event::Event;
use race_core::random::deck_of_cards;
use race_core::random::ShuffledList;
use race_core::types::Settle;
use race_core::types::{GameAccount, RandomId};
use race_proc_macro::game_handler;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum GameEvent {
    Increase(u64),
    RandomPoker,
}

impl CustomEvent for GameEvent {}

#[game_handler]
#[derive(Default, Deserialize, Serialize)]
pub struct Counter {
    value: u64,
    pub poker_random_id: RandomId,
    pub dice_random_id: RandomId,
    pub poker_card: String,
    pub dice_number: String,
    num_of_players: u64,
    num_of_servers: u64,
}

#[derive(Default, BorshSerialize, BorshDeserialize)]
pub struct CounterAccountData {
    pub init_value: u64,
}

impl Counter {
    fn reset(&mut self) {
        self.value = 0;
        self.poker_random_id = 0;
        self.dice_random_id = 0;
        self.poker_card = "??".into();
        self.dice_number = "?".into();
    }

    fn handle_custom_event(
        &mut self,
        context: &mut GameContext,
        sender: String,
        event: GameEvent,
    ) -> Result<()> {
        match event {
            GameEvent::Increase(n) => {
                if self.dice_random_id != 0 {
                    self.value += n;
                    if let Ok(target_value) = self.dice_number.parse::<u64>() {
                        if self.value == target_value {
                            context.add_settle(Settle::eject(sender));
                            self.num_of_players -= 1;
                            self.reset();
                        }
                    }
                }
            }
            GameEvent::RandomPoker => {
                if context.get_status() == GameStatus::Running {
                    // Create some randomness
                    let poker_spec = deck_of_cards();
                    self.poker_random_id = context.init_random_state(&poker_spec)?;
                }
            }
        }
        Ok(())
    }
}

impl GameHandler for Counter {
    fn init_state(context: &mut GameContext, init_account: GameAccount) -> Result<Self> {
        let data = init_account.data;
        let account_data = CounterAccountData::try_from_slice(&data)?;
        Ok(Self {
            value: account_data.init_value,
            poker_random_id: 0,
            dice_random_id: 0,
            poker_card: "??".into(),
            dice_number: "?".into(),
            num_of_players: init_account.players.len() as _,
            num_of_servers: init_account.servers.len() as _,
        })
    }

    fn handle_event(&mut self, context: &mut GameContext, event: Event) -> Result<()> {
        match event {
            Event::Custom { sender, ref raw } => {
                self.handle_custom_event(context, sender, serde_json::from_str(raw).unwrap())
            }
            Event::GameStart { .. } => {
                if context.count_players() == 0 {
                    return Err(Error::NoEnoughPlayers);
                }
                self.value = 0;
                self.dice_random_id = 0;
                self.dice_number = "?".into();
                self.poker_random_id = 0;
                self.poker_card = "??".into();
                let dice_spec = ShuffledList::new(vec!["1", "2", "3", "4", "5", "6"]);
                self.dice_random_id = context.init_random_state(&dice_spec)?;
                Ok(())
            }
            Event::Sync {
                new_players,
                new_servers,
                ..
            } => {
                self.num_of_players += new_players.len() as u64;
                self.num_of_servers += new_servers.len() as u64;
                if self.num_of_players >= 1 {
                    context.start_game();
                }
                Ok(())
            }
            Event::RandomnessReady { random_id } => {
                if self.poker_random_id == random_id {
                    context.reveal(self.poker_random_id, vec![0])?;
                }
                if self.dice_random_id == random_id {
                    context.reveal(self.dice_random_id, vec![0])?;
                }
                Ok(())
            }
            Event::SecretsReady => {
                if context.is_random_ready(self.poker_random_id) {
                    let revealed = context.get_revealed(self.poker_random_id)?;
                    let card = revealed.get(&0).unwrap();
                    self.poker_card = card.to_owned();
                }
                if context.is_random_ready(self.dice_random_id) {
                    let revealed = context.get_revealed(self.dice_random_id)?;
                    let number = revealed.get(&0).unwrap();
                    self.dice_number = number.to_owned();
                }
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
    use race_core::types::PlayerJoin;
    use race_test::{transactor_account_addr, TestGameAccountBuilder};

    use super::*;

    fn init_context() -> GameContext {
        let game_account = TestGameAccountBuilder::default().add_servers(1).build();
        GameContext::try_new(&game_account).unwrap()
    }

    #[test]
    fn test_player_join() {
        let mut ctx = init_context();
        let av = ctx.get_access_version() + 1;
        let evt = Event::Sync {
            new_players: vec![PlayerJoin {
                addr: "Alice".into(),
                balance: 1000,
                position: 0,
                access_version: av,
            }],
            new_servers: vec![],
            transactor_addr: transactor_account_addr(),
            access_version: av,
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
        assert_eq!(0, hdlr.value);
    }

    #[test]
    fn test_random_poker() {
        let mut ctx = init_context();
        let e = Event::custom(
            ctx.get_transactor_addr().to_owned(),
            &GameEvent::RandomPoker,
        );
        let mut h = Counter::default();
        h.handle_event(&mut ctx, e).unwrap();
        assert_eq!(0, h.poker_random_id);
    }
}
