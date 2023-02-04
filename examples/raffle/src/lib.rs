//! A raffle example
//!
//! To start a raffle, at least two players are required.  For each
//! round, there's 5 seconds waiting to allow players join.  One of
//! the player will be picked as winner, and receive all the tokens.

use borsh::{BorshDeserialize, BorshSerialize};
use race_core::error::Result;
use race_core::random::ShuffledList;
use race_core::types::Settle;
use race_core::{
    context::GameContext,
    engine::GameHandler,
    event::Event,
    types::{GameAccount, RandomId},
};
use race_proc_macro::game_handler;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[game_handler]
struct Raffle {
    previous_winner: Option<String>,
    random_id: RandomId,
    next_draw: u64,
    options: Vec<String>,
}

impl Raffle {
    fn cleanup(&mut self, winner: String) {
        self.options.clear();
        self.previous_winner = Some(winner);
        self.random_id = 0;
    }
}

impl GameHandler for Raffle {
    /// Initialize handler state with on-chain game account data.
    fn init_state(_context: &mut GameContext, _init_account: GameAccount) -> Result<Self> {
        Ok(Self {
            previous_winner: None,
            random_id: 0,
            options: Vec::default(),
            next_draw: 0,
        })
    }

    /// Handle event.
    fn handle_event(&mut self, context: &mut GameContext, event: Event) -> Result<()> {
        match event {
            Event::GameStart { .. } => {
                // We need at least two players to start
                // Otherwise we will schedule next round
                let players = context.get_players();
                if players.len() >= 1 {
                    self.options = players.iter().map(|p| p.addr.to_owned()).collect();
                    let rnd_spec = ShuffledList::new(self.options.clone());
                    self.random_id = context.init_random_state(&rnd_spec)?;
                } else {
                    self.next_draw = context.get_timestamp() + 5_000;
                    context.wait_timeout(5_000);
                }
            }
            Event::RandomnessReady { .. } => {
                context.reveal(self.random_id, vec![0])?;
            }
            Event::WaitTimeout => {
                context.start_game();
            }
            Event::SecretsReady => {
                let winner = context.get_revealed(self.random_id)?.get(&0).unwrap().to_owned();
                let mut settles = vec![];

                for p in context.get_players().iter() {
                    if p.addr.ne(&winner) {
                        settles.push(Settle::add(&winner, p.balance));
                        settles.push(Settle::sub(&p.addr, p.balance));
                    }
                }
                for p in context.get_players().iter() {
                    settles.push(Settle::eject(&p.addr));
                }
                context.settle(settles);
                context.wait_timeout(5_000);
                self.cleanup(winner);
            }
            _ => (),
        }
        Ok(())
    }
}
