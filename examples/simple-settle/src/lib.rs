//! A simple game handler to test settlement
//! Whenever there are more than 1 players, a settle will be sent
//! and all players will be ejected.
//! In the settlement, all tokens are moved to the first player.

use race_api::prelude::*;
use race_proc_macro::game_handler;
use std::collections::BTreeMap;

#[game_handler]
#[derive(BorshSerialize, BorshDeserialize)]
struct SimpleSettle {
    players: BTreeMap<u64, u64>,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct AccountData {}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Checkpoint;

impl SimpleSettle {
    fn maybe_settle(&mut self, effect: &mut Effect) -> Result<(), HandleError> {
        if self.players.len() > 1 {
            let (ref winner, _) = self.players.pop_first().unwrap();
            let win: u64 = self.players.values().sum();
            effect.settle(Settle::add(*winner, win));
            effect.settle(Settle::eject(*winner));
            for p in self.players.iter() {
                effect.settle(Settle::sub(*p.0, *p.1));
                effect.settle(Settle::eject(*p.0));
            }
            self.players.clear();
        } else {
            effect.start_game();
        }
        Ok(())
    }
}

impl GameHandler for SimpleSettle {
    type Checkpoint = Checkpoint;

    fn init_state(effect: &mut Effect, init_account: InitAccount) -> Result<Self, HandleError> {
        let players = init_account
            .players
            .into_iter()
            .map(|p| (p.id, p.balance))
            .collect();
        let mut ins = Self { players };
        ins.maybe_settle(effect)?;
        Ok(ins)
    }

    fn handle_event(&mut self, effect: &mut Effect, event: Event) -> Result<(), HandleError> {
        match event {
            Event::Sync { new_players, .. } => {
                for p in new_players {
                    self.players.insert(p.id, p.balance);
                }
                self.maybe_settle(effect)?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn into_checkpoint(self) -> Result<Checkpoint, HandleError> {
        Ok(Checkpoint {})
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_settle() {
        let mut effect = Effect::default();
        let event = Event::Sync {
            new_players: vec![
                GamePlayer::new(0, 100, 0),
                GamePlayer::new(1, 100, 1),
            ]
        };
        let mut handler = SimpleSettle::init_state(&mut effect, InitAccount::default()).unwrap();
        handler.handle_event(&mut effect, event).unwrap();
        assert_eq!(effect.settles, vec![
            Settle::add(0, 100),
            Settle::eject(0),
            Settle::sub(1, 100),
            Settle::eject(1),
        ]);
    }
}
