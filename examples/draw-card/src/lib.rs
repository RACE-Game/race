//! A minimal game to demonstrate how the protocol works.

use std::collections::BTreeMap;

use race_core::prelude::*;

#[derive(BorshSerialize, BorshDeserialize)]
pub enum GameEvent {
    Bet(u64),
    Call,
    Fold,
}

impl CustomEvent for GameEvent {}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct MinimalAccountData {}

#[derive(Default, BorshSerialize, BorshDeserialize)]
pub enum GameStage {
    #[default]
    Dealing,
    Revealing,
}

#[game_handler]
#[derive(BorshSerialize, BorshDeserialize)]
pub struct MinimalHandler {
    pub deck_random_id: RandomId,

    // Current dealer position
    pub dealer_idx: usize,

    // Real-time chips
    pub chips: BTreeMap<String, u64>,

    pub stage: GameStage,

    // Bet amount
    pub bet: u64,
}

impl MinimalHandler {
    fn custom_handle_event(
        &mut self,
        context: &mut Effect,
        _sender: String,
        event: GameEvent,
    ) -> Result<()> {
        match event {
            GameEvent::Bet(amount) => {
                if self.chips.values().any(|c| *c < amount) {
                    return Err(Error::InvalidAmount);
                }
                self.bet = amount;
            }
            GameEvent::Call => {
                context.reveal(self.deck_random_id, vec![0, 1]);
                self.stage = GameStage::Revealing;
            }
            GameEvent::Fold => {}
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

impl GameHandler for MinimalHandler {
    fn init_state(context: &mut Effect, init_account: InitAccount) -> Result<Self> {
        Ok(Self {
            deck_random_id: 0,
            dealer_idx: 0,
            chips: init_account
                .players
                .iter()
                .map(|p| (p.addr, p.balance))
                .collect(),
            bet: 0,
            stage: GameStage::Dealing,
        })
    }

    fn handle_event(&mut self, context: &mut Effect, event: Event) -> Result<()> {
        match event {
            // Custom events are the events we defined for this game particularly
            // See [[GameEvent]].
            Event::Custom { sender, raw } => {
                let event = GameEvent::try_from_slice(&raw)?;
                self.custom_handle_event(context, sender, event)?;
            }

            // Reset current game state.  Set up randomness
            Event::GameStart { .. } => {
                if context.count_players() < 2 {
                    return Err(Error::NoEnoughPlayers);
                }
                for p in context.get_players().iter() {
                    self.chips.insert(p.addr.clone(), p.balance);
                }
                let rnd_spec = deck_of_cards();
                self.deck_random_id = context.init_random_state(&rnd_spec)?;
                self.stage = GameStage::Dealing;
            }

            Event::RandomnessReady { .. } => {
                let addr0 = context.get_player_by_index(0).unwrap().addr.clone();
                let addr1 = context.get_player_by_index(1).unwrap().addr.clone();
                context.assign(self.deck_random_id, addr0, vec![0])?;
                context.assign(self.deck_random_id, addr1, vec![1])?;
            }

            // Start game when there are two players.
            Event::Sync { .. } => {
                if context.count_players() == 2 {
                    context.start_game();
                }
            }

            Event::SecretsReady => match self.stage {
                GameStage::Dealing => {}
                GameStage::Revealing => {
                    let decryption = context.get_revealed(self.deck_random_id)?;
                    let player_idx: usize = if self.dealer_idx == 0 { 1 } else { 0 };
                    let dealer_addr = self.chips.keys().nth(self.dealer_idx).unwrap().to_owned();
                    let player_addr = self.chips.keys().nth(player_idx).unwrap().to_owned();
                    let dealer_card = decryption.get(&self.dealer_idx).unwrap();
                    let player_card = decryption.get(&player_idx).unwrap();
                    let (winner, loser) = if is_better_than(dealer_card, player_card) {
                        (dealer_addr, player_addr)
                    } else {
                        (player_addr, dealer_addr)
                    };
                    context.settle(Settle::add(winner, self.bet));
                    context.settle(Settle::sub(loser, self.bet));
                }
            },
            _ => (),
        }

        Ok(())
    }
}

#[cfg(test)]
mod integration_test;
