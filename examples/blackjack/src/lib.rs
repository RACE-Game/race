//! A blackjack demo

use borsh::{BorshDeserialize, BorshSerialize};
use race_core::{
    context::GameContext,
    engine::GameHandler,
    error::{Error, Result},
    event::Event,
    random::deck_of_cards,
    types::{Addr, Amount, GameAccount, RandomId},
};
use race_proc_macro::game_handler;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
enum GameEvent {
    Bet(Amount),
    Stand,
    Hit,
    DoubleDown,
    Surrender,
    Split,
}

#[derive(Serialize, Deserialize)]
enum Stage {
    Idle,
    Dealing,
    AskingSideRule,
    // parameter: the index of splits
    PlayerActing(usize),
    DealerActing,
}

#[derive(BorshSerialize, BorshDeserialize)]
 struct AccountData {
    min_bet: Amount,
    max_bet: Amount,
}


#[derive(Serialize, Deserialize)]
 struct Player {
    pub addr: Addr,
    pub chips: Amount,
    pub position: usize,
}

#[derive(Serialize, Deserialize)]
pub enum GameResult {
    Lose,
    Win,
    Push,
    Blackjack,
}

#[derive(Serialize, Deserialize)]
 struct Split {
    pub cards: Vec<String>,
    pub points: u32,
}

#[derive(Serialize, Deserialize)]
struct Cards {
    splits: Vec<Split>,
    num_of_splits: usize,
}

#[game_handler]
#[derive(Serialize, Deserialize)]
struct Handler {
    min_bet: Amount,
    max_bet: Amount,
    dealer_pos: usize,
    players: Vec<Player>,
    stage: Stage,
    random_id: RandomId,
}

impl Handler {
    fn handle_custom_event(
        &mut self,
        context: &mut GameContext,
        sender: String,
        event: GameEvent,
    ) -> Result<()> {
        Ok(())
    }

    fn get_player(&self, pos: usize) -> Result<&Player> {
        self.players
            .get(pos)
            .ok_or(Error::Custom("Get player failed".into()))
    }
}

impl GameHandler for Handler {
    fn init_state(context: &mut GameContext, init_account: GameAccount) -> Result<Self> {
        let account_data = AccountData::try_from_slice(&init_account.data)
            .map_err(|_| Error::MalformedData("Failed to deseralize account data".into()))?;

        if account_data.min_bet == 0 || account_data.max_bet < account_data.min_bet {
            return Err(Error::MalformedData("Invalid range for bet amount".into()));
        }

        Ok(Self {
            min_bet: account_data.min_bet,
            max_bet: account_data.max_bet,
            dealer_pos: 0,
            players: Vec::new(),
            stage: Stage::Idle,
            random_id: 0,
        })
    }

    fn handle_event(&mut self, context: &mut GameContext, event: Event) -> Result<()> {
        match event {
            Event::Custom { sender, raw } => {
                let event = serde_json::from_str(&raw)?;
                self.handle_custom_event(context, sender, event)
            }
            Event::Sync { .. } => {
                // Start the game when we have two players and at least one server
                if context.count_players() == 2 && context.count_servers() >= 1 {
                    context.start_game();
                }
                // Do nothing when no enough players or servers
                Ok(())
            }
            Event::GameStart { .. } => {
                context.get_players().iter().for_each(|p| {
                    let player = Player {
                        addr: p.addr.clone(),
                        chips: p.balance,
                        position: p.position,
                    };
                    self.players.push(player);
                });
                let deck = deck_of_cards();
                self.random_id = context.init_random_state(&deck)?;
                self.stage = Stage::Dealing;
                Ok(())
            }
            Event::RandomnessReady { .. } => {
                // Deal cards:
                // Dealer: 0, 1 - 1 will be revealed by default
                // Player: 2, 3 - both will be revealed.
                let dealer_addr = &self.get_player(self.dealer_pos)?.addr;
                context.reveal(self.random_id, vec![1, 2, 3])?;
                context.assign(self.random_id, dealer_addr, vec![0])?;
                Ok(())
            }
            Event::SecretsReady => {

            }
            Event::ActionTimeout { player_addr } => {
                Ok(())
            }
            Event::OperationTimeout { addr } => todo!(),
            Event::Leave { player_addr } => todo!(),
            _ => Ok(()),
        }
    }
}
