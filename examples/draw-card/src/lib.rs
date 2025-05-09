//! A minimal poker game to demonstrate how the protocol works.
//!
//! The game is for two players.  In the dealing, each player gets one
//! random card as hand.  And each player put some assets into the
//! pot.  Then player A can bet with an amount, player B can either
//! call or fold.  If player B calls, both players' hands will be
//! revealed.  The one with better hand win the pot(B wins if both got
//! the same hands).  If player B folds, player A wins the pot.
//! Players switch positions in each round.

use std::collections::BTreeMap;

use race_api::prelude::*;
use race_proc_macro::game_handler;

const ACTION_TIMEOUT: u64 = 30_000;
const NEXT_GAME_TIMEOUT: u64 = 3000;

#[derive(BorshSerialize, BorshDeserialize)]
pub enum GameEvent {
    Bet(u64),
    Call,
    Fold,
}

impl CustomEvent for GameEvent {}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct AccountData {
    pub blind_bet: u64,
    pub min_bet: u64,
    pub max_bet: u64,
}

#[derive(Default, Debug, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub enum GameStage {
    #[default]
    Dealing,
    Betting,
    Reacting,
    Revealing,
    Ending,
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(BorshSerialize, BorshDeserialize)]
pub struct Player {
    pub id: u64,
    pub balance: u64,
    pub bet: u64,
}

#[game_handler]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(BorshSerialize, BorshDeserialize)]
pub struct DrawCard {
    pub last_winner: Option<String>,
    pub random_id: RandomId,
    pub player_map: BTreeMap<u64, Player>,
    pub action_order: Vec<u64>, // player ids in action order
    pub stage: GameStage,
    pub pot: u64,
    pub bet: u64,
    pub blind_bet: u64,
    pub min_bet: u64,
    pub max_bet: u64,
}

impl DrawCard {

    fn custom_handle_event(
        &mut self,
        effect: &mut Effect,
        sender: u64,
        event: GameEvent,
    ) -> Result<(), HandleError> {
        match event {
            GameEvent::Bet(amount) => {
                if self.stage == GameStage::Betting {
                    let player = self
                        .player_map
                        .get_mut(&self.action_order[0])
                        .ok_or(HandleError::InvalidPlayer)?;
                    if sender != player.id {
                        return Err(HandleError::InvalidPlayer);
                    }
                    if amount < self.min_bet || amount > self.max_bet || amount > player.balance {
                        return Err(HandleError::InvalidAmount);
                    }
                    player.bet += amount;
                    player.balance -= amount;
                    self.bet = amount;
                    self.pot += amount;
                    self.stage = GameStage::Reacting;
                    effect.action_timeout(player.id.clone(), ACTION_TIMEOUT)?;
                } else {
                    return Err(HandleError::Custom("Can't bet".into()));
                }
            }
            GameEvent::Call => {
                if self.stage == GameStage::Reacting {
                    let player = self
                        .player_map
                        .get_mut(&self.action_order[1])
                        .ok_or(HandleError::InvalidPlayer)?;
                    if sender.ne(&player.id) {
                        return Err(HandleError::InvalidPlayer);
                    }
                    if self.bet > player.balance {
                        player.bet += player.balance;
                        player.balance = 0;
                        self.pot += player.balance;
                    } else {
                        player.bet += self.bet;
                        player.balance -= self.bet;
                        self.pot += self.bet;
                    }
                    self.stage = GameStage::Revealing;
                    effect.reveal(self.random_id, vec![0, 1]);
                } else {
                    return Err(HandleError::Custom("Can't call".into()));
                }
            }
            GameEvent::Fold => {
                if self.stage == GameStage::Reacting {
                    self.stage = GameStage::Ending;
                    self.set_winner(effect, 0)?;
                } else {
                    return Err(HandleError::Custom("Can't fold".into()));
                }
            }
        }

        Ok(())
    }
}

// A simple function used to compare cards
fn is_better_than(card_a: &str, card_b: &str) -> bool {
    let ranking = vec![
        '2', '3', '4', '5', '6', '7', '8', '9', 't', 'j', 'q', 'k', 'a',
    ];
    let rank_a = ranking
        .iter()
        .rposition(|r| r.eq(&card_a.chars().nth_back(0).unwrap()));
    let rank_b = ranking
        .iter()
        .rposition(|r| r.eq(&card_b.chars().nth_back(0).unwrap()));
    rank_a > rank_b
}

impl GameHandler for DrawCard {

    fn init_state(init_account: InitAccount) -> Result<Self, HandleError> {
        let AccountData {
            blind_bet,
            min_bet,
            max_bet,
        } = init_account.data()?;
        Ok(Self {
            last_winner: None,
            random_id: 0,
            players: vec![],
            bet: 0,
            pot: 0,
            stage: GameStage::Dealing,
            min_bet,
            max_bet,
            blind_bet,
        })
    }

    fn handle_event(&mut self, effect: &mut Effect, event: Event) -> Result<(), HandleError> {
        match event {
            // Custom events are the events we defined for this game particularly
            // See [[GameEvent]].
            Event::Custom { sender, raw } => {
                let event = GameEvent::try_parse(&raw)?;
                self.custom_handle_event(effect, sender, event)?;
            }

            // Waiting timeout usually sent after each game.  Here we
            // can trigger the next game.
            Event::WaitingTimeout => {
                if self.players.len() == 2 {
                    effect.start_game();
                }
            }

            // Reset current game state.  Set up randomness
            Event::GameStart { .. } => {
                if self.players.len() < 2 {
                    return Err(HandleError::NoEnoughPlayers);
                }

                if effect.count_nodes() == 0 {
                    return Err(HandleError::NoEnoughServers);
                }

                let rnd_spec = RandomSpec::deck_of_cards();
                // Reset the state when starting
                self.pot = 0;
                self.bet = 0;
                for p in self.players.iter_mut() {
                    p.bet = 0;
                }
                self.stage = GameStage::Dealing;
                self.random_id = effect.init_random_state(rnd_spec);
                self.players.rotate_right(1);
            }

            Event::RandomnessReady { .. } => {
                effect.assign(self.random_id, self.players[0].id, vec![0])?;
                effect.assign(self.random_id, self.players[1].id, vec![1])?;
            }

            // Start game when there are two players.
            Event::Join { players } => {
                for p in players.into_iter() {
                    self.players.push(Player {
                        id: p.id(),
                        balance: 0,
                        bet: 0,
                    });
                }
                // Start the game when there're enough players
                if self.players.len() == 2 {
                    effect.start_game();
                }
            }

            Event::SecretsReady { .. } => {
                match self.stage {
                    GameStage::Dealing => {
                        // Now it's the first player's turn to act.
                        // So we dispatch an action timeout event.
                        self.stage = GameStage::Betting;
                        effect.action_timeout(self.players[0].id.clone(), ACTION_TIMEOUT)?;
                    }
                    GameStage::Revealing => {
                        // Reveal and compare the hands to decide who is the winner
                        let revealed = effect.get_revealed(self.random_id)?;
                        println!("Revealed from wasm: {:?}", effect);
                        let card_0 = revealed
                            .get(&0)
                            .ok_or(HandleError::Custom("Can't get revealed card".into()))?;
                        let card_1 = revealed
                            .get(&1)
                            .ok_or(HandleError::Custom("Can't get revealed card".into()))?;
                        if is_better_than(card_0, card_1) {
                            self.set_winner(effect, 0)?;
                        } else {
                            self.set_winner(effect, 1)?;
                        }
                    }
                    _ => (),
                }
            }

            Event::Leave { player_id } => {
                if let Some(player_idx) = self.players.iter().position(|p| p.id.eq(&player_id))
                {
                    let player = self.players.remove(player_idx);
                    effect.settle(player.id, player.balance)?;
                    effect.wait_timeout(NEXT_GAME_TIMEOUT);
                    effect.checkpoint();
                } else {
                    return Err(HandleError::InvalidPlayer);
                }
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
    fn test_account_data() {
        let account_data = AccountData {
            blind_bet: 100,
            min_bet: 100,
            max_bet: 1000,
        };

        let data = borsh::to_vec(&account_data).unwrap();
        println!("data: {:?}", data);
        println!("data len: {}", data.len());
    }
}

#[cfg(test)]
mod integration_test;
