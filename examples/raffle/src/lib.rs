//! A raffle example
//!
//! To start a raffle, at least two players are required.  For each
//! round, there's 5 seconds waiting to allow players join.  One of
//! the player will be picked as winner, and receive all the tokens.

use race_core::prelude::*;

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
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
    fn cleanup(&mut self) {
        self.players.clear();
        self.random_id = 0;
    }
}

impl GameHandler for Raffle {
    /// Initialize handler state with on-chain game account data.
    fn init_state(context: &mut Effect, init_account: InitAccount) -> Result<Self> {
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
                // We need at least one player to start, otherwise we will skip this draw.
                if context.count_players() >= 1 {
                    let options = self.players.iter().map(|p| p.addr.to_owned()).collect();
                    let rnd_spec = RandomSpec::shuffled_list(options);
                    self.random_id = context.init_random_state(rnd_spec);
                } else {
                    self.draw_time = context.timestamp() + 30_000;
                    context.wait_timeout(30_000);
                }
            }

            Event::Sync { new_players, .. } => {
                let players = new_players.into_iter().map(Into::into);
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
                self.cleanup();
            }

            Event::SecretsReady => {
                let winner = context
                    .get_revealed(self.random_id)?
                    .get(&0)
                    .unwrap()
                    .to_owned();

                for p in self.players.iter() {
                    if p.addr.ne(&winner) {
                        context.settle(Settle::add(&winner, p.balance));
                        context.settle(Settle::sub(&p.addr, p.balance));
                    }
                    context.settle(Settle::eject(&p.addr));
                }
                context.wait_timeout(5_000);
                self.cleanup();
            }
            _ => (),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_state() {
        let mut effect = Effect::default();
        let init_account = InitAccount::default();
        let state = Raffle::init_state(&mut effect, init_account).expect("Failed to init state");
        assert_eq!(state.random_id, 0);
        assert_eq!(state.players, Vec::new());
        assert_eq!(state.draw_time, 30_000);
    }
}
