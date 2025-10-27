//! A minimal rock paper scissors game.  This game is used to
//! demostrate how the immutable decision works.

use std::collections::HashMap;
use std::iter;

use arrayref::{array_ref, array_refs};
use race_core::prelude::*;

#[derive(Serialize, Deserialize)]
struct Player {
    pub balance: u64,
    pub decision_id: usize,
    pub acted: bool,
}

impl Player {
    pub fn new(balance: u64) -> Self {
        Self {
            decision_id: 0,
            balance,
            acted: false,
        }
    }

    pub fn reset(&mut self) {
        self.decision_id = 0;
        self.acted = false;
    }
}

#[derive(Serialize, Deserialize, Default)]
#[game_handler]
struct Roshambo {
    pub players: HashMap<String, Player>,
}

#[repr(u8)]
enum Action {
    Paper = 0,
    Scissors,
    Rock,
}

impl Roshambo {
    fn add_players(&mut self, effect: &mut Effect, ps: Vec<PlayerJoin>) {
        ps.into_iter().for_each(|p| {
            self.players.insert(p.addr, Player::new(p.balance));
        });
        if self.players.len() == 2 {
            effect.start_game();
        }
    }
}

impl GameHandler for Roshambo {
    fn init_state(effect: &mut Effect, init_account: InitAccount) -> Result<Self> {
        let mut ret = Self::default();
        ret.add_players(effect, init_account.players);
        Ok(ret)
    }

    fn handle_event(&mut self, effect: &mut Effect, event: Event) -> Result<()> {
        match event {
            Event::GameStart { .. } => {
                if self.players.len() != 2 {
                    return Err(Error::NoEnoughPlayers);
                }
                // When game starts, we ask each player which action
                // they will make
                for (addr, p) in self.players.iter_mut() {
                    p.decision_id = effect.ask(addr);
                }
            }
            Event::Sync { new_players, .. } => self.add_players(effect, new_players),
            Event::WaitingTimeout => {
                if self.players.len() == 2 {
                    effect.start_game();
                }
            }
            Event::Leave { player_addr } => {
                self.players.remove(&player_addr);
            }
            Event::AnswerDecision { sender, .. } => {
                self.players.get_mut(&sender).map(|p| p.acted = true);
                if self.players.iter().all(|p| p.1.acted) {
                    self.players
                        .iter()
                        .for_each(|p| effect.release(p.1.decision_id))
                }
            }
            Event::SecretsReady => {
                let player_to_opt: Vec<(String, String)>  = self
                    .players
                    .iter()
                    .map(|(addr, p)| (addr.clone(), effect.get_answer(p.decision_id).unwrap().to_owned()))
                    .collect();
                let player_to_opt = array_ref![player_to_opt, 0, 2];
                let [(p0, opt0), (p1, opt1)] = player_to_opt;
                if opt0.eq(opt1) {
                    // draw game

                } else if (opt0 == "1" && opt1 == "2") || (opt0 == "0" && opt1 == "1") || (opt0 == "2" && opt1 == "0") {
                    // p1 win
                    effect.settle(Settle::add(p1, 100));
                    effect.settle(Settle::sub(p0, 100));
                }  else {
                    // p0 win
                    effect.settle(Settle::add(p0, 100));
                    effect.settle(Settle::sub(p1, 100));
                }
                effect.wait_timeout(15_000);
            }
            _ => (),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use race_core::{
        context::{DispatchEvent, GameContext},
        effect::Ask,
        types::ClientMode,
    };
    use race_test::{TestClient, TestGameAccountBuilder, TestHandler};

    use super::*;

    #[test]
    fn test_start_game_without_enough_players_failed() -> Result<()> {
        let mut effect = Effect::default();
        let init_account = InitAccount::default();
        let mut handler = Roshambo::init_state(&mut effect, init_account)?;
        let r = handler.handle_event(&mut effect, Event::GameStart { access_version: 0 });
        assert_eq!(r, Err(Error::NoEnoughPlayers));
        Ok(())
    }

    #[test]
    fn test_start_game() -> Result<()> {
        let mut effect = Effect::default();
        let mut init_account = InitAccount::default();
        init_account.add_player("Alice", 0, 1000);
        init_account.add_player("Bob", 1, 1000);
        let mut handler = Roshambo::init_state(&mut effect, init_account)?;
        handler.handle_event(&mut effect, Event::GameStart { access_version: 2 })?;
        assert_eq!(
            effect.asks,
            vec![
                Ask {
                    player_addr: "Alice".into()
                },
                Ask {
                    player_addr: "Bob".into()
                }
            ]
        );
        Ok(())
    }

    #[test]
    fn integration_test() -> Result<()> {
        // Initialize handler and clients
        let game_account = TestGameAccountBuilder::default()
            .add_players(2)
            .add_servers(1)
            .build();
        let mut context = GameContext::try_new(&game_account)?;
        let mut alice = TestClient::new(
            "Alice".into(),
            game_account.addr.clone(),
            ClientMode::Player,
        );
        let mut bob = TestClient::new("Bob".into(), game_account.addr.clone(), ClientMode::Player);
        let mut transactor = TestClient::new(
            "Foo".into(),
            game_account.addr.clone(),
            ClientMode::Transactor,
        );
        let mut handler: TestHandler<Roshambo> =
            TestHandler::init_state(&mut context, &game_account)?;

        {
            let state = handler.get_state();
            assert_eq!(state.players.len(), 2);
        }

        // start game
        let event = context.gen_start_game_event();
        handler.handle_event(&mut context, &event)?;

        let event = alice.answer(1, "0".into())?;
        handler.handle_event(&mut context, &event)?;
        let event = bob.answer(2, "1".into())?;
        handler.handle_event(&mut context, &event)?;

        // When all decisions are made, players should reveal their decisions
        let events = alice.handle_updated_context(&context)?;
        {
            assert_eq!(events.len(), 1);
        }
        println!("events: {:?}", events);
        handler.handle_event(&mut context, &events[0])?;
        let events = bob.handle_updated_context(&context)?;
        {
            assert_eq!(events.len(), 1);
        }
        println!("events: {:?}", events);
        handler.handle_event(&mut context, &events[0])?;

        assert_eq!(
            context.get_dispatch(),
            &Some(DispatchEvent {
                timeout: 0,
                event: Event::SecretsReady
            })
        );

        handler.handle_dispatch_event(&mut context)?;

        Ok(())
    }
}
