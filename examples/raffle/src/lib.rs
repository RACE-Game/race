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
    pub addr: String,
    pub balance: u64,
}

impl From<GamePlayer> for Player {
    fn from(value: GamePlayer) -> Self {
        Self {
            addr: value.addr,
            balance: value.balance,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
#[game_handler]
struct Raffle {
    last_winner: Option<String>,
    players: Vec<Player>,
    random_id: RandomId,
    draw_time: u64,
}

impl Raffle {
    fn cleanup(&mut self) {
        self.players.clear();
        self.random_id = 0;
        self.draw_time = 0;
    }
}

impl GameHandler for Raffle {

    type Checkpoint = ();

    /// Initialize handler state with on-chain game account data.
    fn init_state(_effect: &mut Effect, init_account: InitAccount) -> HandleResult<Self> {
        let players = init_account.players.into_iter().map(Into::into).collect();
        let draw_time = 0;
        Ok(Self {
            last_winner: None,
            players,
            random_id: 0,
            draw_time,
        })
    }

    /// Handle event.
    fn handle_event(&mut self, effect: &mut Effect, event: Event) -> HandleResult<()> {
        match event {
            Event::GameStart { .. } => {
                // We need at least one player to start, otherwise we will skip this draw.
                if self.players.len() >= 1 {
                    let options = self.players.iter().map(|p| p.addr.to_owned()).collect();
                    let rnd_spec = RandomSpec::shuffled_list(options);
                    self.random_id = effect.init_random_state(rnd_spec);
                }
            }

            Event::Sync { new_players, .. } => {
                let players = new_players.into_iter().map(Into::into);
                self.players.extend(players);
                if self.players.len() >= 1 && self.draw_time == 0 {
                    self.draw_time = effect.timestamp() + DRAW_TIMEOUT;
                    effect.wait_timeout(DRAW_TIMEOUT);
                }
            }

            // Reveal the first address when randomness is ready.
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
                    .to_owned();

                for p in self.players.iter() {
                    if p.addr.ne(&winner) {
                        effect.settle(Settle::add(&winner, p.balance));
                        effect.settle(Settle::sub(&p.addr, p.balance));
                    }
                    effect.settle(Settle::eject(&p.addr));
                }
                self.last_winner = Some(winner);
                self.cleanup();
            }
            _ => (),
        }
        Ok(())
    }

    fn into_checkpoint(self) -> HandleResult<()> {
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
        let event = Event::Sync {
            new_players: vec![PlayerJoin {
                addr: "alice".into(),
                position: 0,
                balance: 100,
                access_version: 0,
                verify_key: "".into(),
            }],
            new_servers: vec![ServerJoin {
                addr: "foo".into(),
                endpoint: "foo.endpoint".into(),
                access_version: 0,
                verify_key: "".into(),
            }],
            transactor_addr: "".into(),
            access_version: 0,
        };

        state.handle_event(&mut effect, event).unwrap();
        assert_eq!(state.players.len(), 1);
        assert_eq!(effect.wait_timeout, Some(DRAW_TIMEOUT));
    }
}
