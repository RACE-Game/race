//! A minimal game to demonstrate how the protocol works.

use std::collections::HashMap;

use borsh::{BorshDeserialize, BorshSerialize};
use race_core::{
    context::{GameContext, GameStatus},
    engine::GameHandler,
    error::{Error, Result},
    event::{CustomEvent, Event},
    random::deck_of_cards,
    types::GameAccount,
};
use race_proc_macro::game_handler;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum GameEvent {
    Bet(u64),
    Call,
    Fold,
}

impl CustomEvent for GameEvent {}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct OneCardGameAccountData {}

#[game_handler]
#[derive(Default, Serialize, Deserialize)]
pub struct OneCard {
    pub deck_random_id: usize,

    // Current dealer position
    pub dealer: usize,

    // Real-time chips
    pub chips: HashMap<String, u64>,

    // Bet amounts by player addresses
    pub bets: HashMap<String, u64>,
}

impl OneCard {
    fn custom_handle_event(
        &mut self,
        context: &GameContext,
        sender: String,
        event: GameEvent,
    ) -> Result<()> {
        match event {
            GameEvent::Bet(amount) => {
                let player = context.get_player_by_address(&sender).unwrap();
                if player.balance < amount {
                    return Err(Error::InvalidAmount);
                }
                *self.bets.entry(sender.clone()).or_insert(0) += amount;
                *self.chips.get_mut(&sender).unwrap() -= amount;
            }
            GameEvent::Call => {}
            GameEvent::Fold => {}
        }

        Ok(())
    }
}

impl GameHandler for OneCard {
    fn init_state(context: &mut GameContext, init_account: GameAccount) -> Result<Self> {
        Ok(Self {
            deck_random_id: 0,
            dealer: 0,
            chips: context.players().iter().map(|p| (p.addr.to_owned(), p.balance)).collect(),
            bets: HashMap::new(),
        })
    }

    fn handle_event(&mut self, context: &mut GameContext, event: Event) -> Result<()> {
        match event {
            // Custom events are the events we defined for this game epecifically
            // See [[GameEvent]].
            Event::Custom { sender, raw } => {
                let event = serde_json::from_str(&raw).unwrap();
                self.custom_handle_event(context, sender, event)?;
            }

            // Reset current game state.  Set up randomness
            Event::GameStart => {
                let rnd_spec = deck_of_cards();
                self.deck_random_id = context.init_random_state(&rnd_spec);
            }

            // Wait player join to start.  We don't need to handle
            // this event in this game.  The start will be triggered
            // by PlayerJoined event.
            Event::WaitTimeout => {}

            // Player send ready.
            Event::Ready { sender } => {}

            Event::ShareSecrets {
                sender,
                secret_ident,
                secret_data,
            } => {}

            Event::Randomize {
                sender,
                random_id,
                ciphertexts,
            } => {}

            Event::Lock {
                sender,
                random_id,
                ciphertexts_and_tests,
            } => {}

            // Deal player cards, each player will get one card.
            Event::RandomnessReady => {
                let addr0 = context.get_player_by_index(0).unwrap().addr.clone();
                let addr1 = context.get_player_by_index(1).unwrap().addr.clone();
                context.assign(self.deck_random_id, addr0, vec![0])?;
                context.assign(self.deck_random_id, addr1, vec![1])?;
            }

            // Start game when there are two players.
            Event::Join { player_addr, balance } => {

                if context.players().len() == 2 {
                    context.set_game_status(GameStatus::Initializing);
                }
                self.chips.insert(player_addr.to_owned(), balance);
            }

            Event::Leave { player_addr } => {}
            Event::DrawRandomItems {
                sender,
                random_id,
                indexes,
            } => {}
            Event::DrawTimeout => {}
            Event::ActionTimeout { player_addr } => {}
            Event::SecretsReady => {}
        }

        Ok(())
    }
}
