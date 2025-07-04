//! A raffle example
//!
//! To start a raffle, at least two players are required.  For each
//! round, there's 5 seconds waiting to allow players join.  One of
//! the player will be picked as winner, and receive all the tokens.

use race_api::prelude::*;
use race_proc_macro::game_handler;

const DRAW_TIMEOUT: u64 = 30_000;

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(BorshSerialize, BorshDeserialize)]
struct Player {
    pub id: u64,
    pub balance: u64,
}

impl From<GamePlayer> for Player {
    fn from(value: GamePlayer) -> Self {
        Player { id: value.id(), balance: 0 }
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
#[game_handler]
struct Raffle {
    last_winner: Option<u64>,
    players: Vec<Player>,
    random_id: RandomId,
    draw_time: u64,
    prize_pool: u64,
}

impl Raffle {
    fn cleanup(&mut self) {
        self.players.clear();
        self.random_id = 0;
        self.draw_time = 0;
    }
}

impl GameHandler for Raffle {

    /// Initialize handler state with on-chain game account data.
    fn init_state(_effect: &mut Effect, _init_account: InitAccount) -> HandleResult<Self> {
        let draw_time = 0;
        Ok(Self {
            last_winner: None,
            players: vec![],
            random_id: 0,
            draw_time,
            prize_pool: 0,
        })
    }

    fn balances(&self) -> Vec<PlayerBalance> {
        self.players.iter().map(|p| PlayerBalance::new(p.id, p.balance)).collect()
    }

    /// Handle event.
    fn handle_event(&mut self, effect: &mut Effect, event: Event) -> HandleResult<()> {
        match event {
            Event::GameStart { .. } => {
                // We need at least one player to start, otherwise we will skip this draw.
                if self.players.len() >= 1 {
                    let options = self.players.iter().map(|p| p.id.to_string()).collect();
                    let rnd_spec = RandomSpec::shuffled_list(options);
                    self.random_id = effect.init_random_state(rnd_spec);
                }
            }

            Event::Join { players } => {
                let players = players.into_iter().map(Into::into);
                self.players.extend(players);
                if self.players.len() >= 1 && self.draw_time == 0 {
                    self.draw_time = effect.timestamp() + DRAW_TIMEOUT;
                    effect.wait_timeout(DRAW_TIMEOUT);
                }
            }

            Event::Deposit { deposits } => {
                for d in deposits {
                    self.prize_pool += d.balance();
                    let Some(player) = self.players.iter_mut().find(|p| p.id == d.id()) else {
                        return Err(HandleError::InvalidPlayer)
                    };
                    player.balance = d.balance();
                }
            }

            // Reveal the first idess when randomness is ready.
            Event::RandomnessReady { .. } => {
                effect.reveal(self.random_id, vec![0]);
            }

            // Start game when we have enough players.
            Event::WaitingTimeout => {
                if self.players.len() >= 1 {
                    effect.start_game();
                }
            }

            // Eject all players when encryption failed.
            Event::OperationTimeout { .. } => {
                self.cleanup();
            }

            Event::SecretsReady { .. } => {
                let winner = effect
                    .get_revealed(self.random_id)?
                    .get(&0)
                    .unwrap()
                    .parse::<u64>()
                    .unwrap();

                for p in self.players.iter() {
                    if p.id != winner {
                        effect.eject(p.id);
                    }
                    effect.withdraw(p.id, self.prize_pool);
                    effect.eject(p.id);
                }
                effect.checkpoint();
                self.last_winner = Some(winner);
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
        assert_eq!(state.draw_time, 0);
    }

    #[test]
    fn test_sync() {
        let mut effect = Effect::default();
        let mut state = Raffle {
            draw_time: 0,
            last_winner: None,
            players: vec![],
            random_id: 0,
        };
        let event = Event::Join {
            players: vec![GamePlayer::new(0, 100, 0), GamePlayer::new(1, 100, 1)],
        };

        state.handle_event(&mut effect, event).unwrap();
        assert_eq!(state.players.len(), 2);
        assert_eq!(effect.wait_timeout, Some(DRAW_TIMEOUT));
    }
}
