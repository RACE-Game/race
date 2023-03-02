//! A raffle example
//!
//! To start a raffle, at least two players are required.  For each
//! round, there's 5 seconds waiting to allow players join.  One of
//! the player will be picked as winner, and receive all the tokens.

use borsh::{BorshDeserialize, BorshSerialize};
use race_core::effect::Effect;
use race_core::error::Result;
use race_core::random::ShuffledList;
use race_core::types::{PlayerJoin, Settle};
use race_core::{
    engine::GameHandler,
    event::Event,
    types::{GameAccount, RandomId},
};
use race_proc_macro::game_handler;

#[derive(BorshDeserialize, BorshSerialize)]
struct Player {
    pub addr: String,
    pub balance: u64,
}

impl From<PlayerJoin> for Player {
    fn from(value: PlayerJoin) -> Self {
        Self {
            addr: value.addr,
            balance: value.balance,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
#[game_handler]
struct Raffle {
    players: Vec<Player>,
    random_id: RandomId,
    draw_time: u64,
}

impl Raffle {
    fn cleanup(&mut self, winner: Option<String>) {
        self.players.clear();
        self.random_id = 0;
    }
}

impl GameHandler for Raffle {
    /// Initialize handler state with on-chain game account data.
    fn init_state(context: &mut Effect, init_account: GameAccount) -> Result<Self> {
        let players = init_account.players.into_iter().map(Into::into).collect();
        let draw_time = context.timestamp() + 30_000;
        Ok(Self {
            players,
            random_id: 0,
            draw_time,
        })
    }

    /// Handle event.
    fn handle_event(&mut self, context: &mut Effect, event: Event) -> Result<()> {
        match event {
            Event::GameStart { .. } => {
                // We need at least two players to start
                // Otherwise we will schedule next round
                if context.count_players() >= 1 {
                    self.options = players.iter().map(|p| p.addr.to_owned()).collect();
                    let rnd_spec = ShuffledList::new(self.options.clone());
                    self.random_id = context.init_random_state(&rnd_spec)?;
                } else {
                    self.next_draw = context.get_timestamp() + 10_000;
                    context.wait_timeout(10_000);
                }
            }

            Event::Sync { new_players, .. } => {
                let players = new_players.into_iter().map(Into::into).collect();
                self.players.extend(players);
                context.start_game();
            }

            // Reveal the first address when randomness is ready.
            Event::RandomnessReady { .. } => {
                context.reveal(self.random_id, vec![0]);
            }

            // Start game when we have enough players.
            Event::WaitingTimeout => {
                context.start_game();
            }

            // Eject all players when encryption failed.
            Event::OperationTimeout { .. } => {
                context.wait_timeout(60_000);
                self.cleanup(None);
            }

            Event::SecretsReady => {
                let winner = context
                    .get_revealed(self.random_id)?
                    .get(&0)
                    .unwrap()
                    .to_owned();
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
                self.cleanup(Some(winner));
            }
            _ => (),
        }
        Ok(())
    }
}
