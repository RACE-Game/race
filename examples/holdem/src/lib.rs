use borsh::{BorshDeserialize, BorshSerialize};
use std::collections::HashMap;
use race_core::error::{Error, Result};
use race_core::event::{Event, CustomEvent};
use race_core::{context::{GameContext, GameStatus as GeneralStatus}, engine::GameHandler, types::GameAccount};
use serde::{Deserialize, Serialize};
use race_proc_macro::game_handler;

// u8 ~ 256
// u16 ~ 65536
// Byte = 8 bit


// HoldemAccount offers necessary (static) data (serialized in vec) to GameAccount for Holdem games
// This HoldemAccount data go to the (Raw) data field and
// Holdem (the WASM), the actual game, go to the bundle_addr field
#[derive(Default, BorshSerialize, BorshDeserialize, PartialEq, Debug)]
pub struct HoldemAccount {
    pub sb: u64,
    pub bb: u64,
    pub buyin: u64,
    pub btn: u8,                // current btn position, zero-indexed?
    pub rake: f32,
    pub size: u8,               // table size: total number of players
    pub mode: String,           // game type: cash, sng or tourney?
    pub token: String,          // token should be a struct of its own?
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum PlayerStatus {
    #[default]
    Wait,                       // or Idle?
    Acted,
    Acting,                     // or In_Action?
    Allin,
    Fold,
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct Player {             // or <'p>
    pub addr: String,
    pub chips: u64,
    pub position: u8,           // zero indexed
    pub status: PlayerStatus,   // or &'p str?
    // pub online_status
    // pub drop_count
    // pub timebank
    // pub nickname
}

impl Player {
    pub fn new<T: Into<String>>(id: T, bb: u64, pos: u8) -> Player {
        Self {
            addr: id.into(),
            chips: 20 * bb,     // suppose initial chips are 20 bbs
            position: pos,
            status: Default::default(),
        }
    }

    // Whether need to act
    pub fn to_act(&self) -> bool {
        match self.status {
            PlayerStatus::Wait | PlayerStatus::Acted => true,
            _ => false
        }
    }


    pub fn to_remain(&self) -> bool {
        match self.status {
            PlayerStatus::Fold => false,
            _ => true
        }
    }

    pub fn next_to_act(&self) -> bool {
        match self.status {
            PlayerStatus::Allin | PlayerStatus::Fold => false,
            _ => true
        }
    }

    pub fn take_bet(&mut self, bet: u64) {
        // ignore allin for now
        self.chips = self.chips - bet;
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct Pot {
    owners: Vec<String>,
    winners: Vec<String>,
    amount: u64,
}

impl Pot {
    pub fn new() -> Pot {
        Self {
            owners: vec![],
            winners: vec![],
            amount: 0
        }
    }

    pub fn amount(&self) -> u64 {
        self.amount
    }

    pub fn owners(&self) -> Vec<String> {
        self.owners.clone()
    }

    pub fn winners(&self) -> Vec<String> {
        self.winners.clone()
    }

    pub fn update_winners(&mut self, winners: Vec<String>) {
        self.winners = winners;
    }
}

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone)]
pub enum Street {
    Init,
    Preflop,
    Flop,
    Turn,
    River,
    Done,
}

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone)]
pub struct Bet {
    owner: String,
    amount: u64,
}

impl Bet {
    pub fn new<S: Into<String>>(owner: S, amount: u64) -> Self {
        Self {
            owner: owner.into(),
            amount
        }
    }

    pub fn amount(&self) -> u64 {
        self.amount
    }

    pub fn addr(&self) -> String {
        self.owner.clone()
    }
}

// Game status for holdem
#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub enum HoldemStatus {
    Init,
    Encrypt,
    ShareKey,
    Play,
    Runner,
    Settle,
    Showdown,
    Shuffle,

}

#[derive(Serialize, Deserialize)]
pub enum GameEvent {
    Bet(u64),
    Check,
    Call,
    Fold,
    Raise(u64),
}

impl CustomEvent for GameEvent {}


#[game_handler]
#[derive(Deserialize, Serialize, Clone)]
pub struct Holdem {// must have random cards id
    pub sb: u64,
    pub bb: u64,
    pub buyin: u64,
    pub btn: u8,               // current btn position, zero-indexed?
    pub rake: f32,
    pub size: u8,              // table size: total number of players
    pub status: HoldemStatus,
    // mode: String,           // game type: cash, sng or tourney? Treat as CASH GAME
    // token: String,          // token should be a struct of its own?
    pub street: Street,
    pub street_bet: u64,
    pub bet_map: Vec<Bet>,     // each bet's index maps to player's pos (index) in the below player list
    pub players: Vec<Player>,
    pub act_idx: usize,        // position of the player who should take action
    pub pots: Vec<Pot>,        // 1st main pot + rest side pot(s)
}

// TODO:
// handle game start event

// Associated fns
impl Holdem {
    fn next_action_player(street_bet: u64, bet_map: Vec<Bet>, players_toact: Vec<&Player>) -> Player {
        for p in players_toact {
            if let Some(bet) = bet_map.get(usize::from(p.position)) {
                if bet.amount() < street_bet || p.status == PlayerStatus::Wait {
                    return p.clone();
                }
            }
        }
        return Player::default();
    }
}

// Methods
impl Holdem {
    fn init_players(&mut self, players: Vec<Player>) {
        self.players = players;
    }

    fn set_act_idx(&mut self, lens: u8) {
        self.act_idx = usize::from(lens - 1);
    }

    fn update_streetbet(&mut self, new_sbet: u64) {
        self.street_bet = new_sbet;
    }

    fn update_betmap(&mut self, new_bmap: Vec<Bet>) {
        self.bet_map.extend_from_slice(&new_bmap);
    }

    fn next_street(&mut self) -> Street {
        match self.street {
            Street::Init => {
                Street::Preflop
            },
            Street::Preflop => {
                Street::Flop
            },
            Street::Flop => {
                Street::Turn
            },

            Street::Turn => {
                Street::River
            },
            _ => {
                Street::Done
            }
        }
    }

    pub fn calc_prize(&mut self) -> HashMap<String, u64> {
        let pots = &self.pots;
        let mut prize_map: HashMap<String, u64> = HashMap::new();

        for pot in pots {
            let cnt: u64 = pot.winners().len() as u64;
            let prize: u64 = pot.amount() / cnt;
            for wnr in pot.winners() {
                prize_map.entry(wnr.clone()).and_modify(|p| *p += prize).or_insert(prize);
            }
        }
        prize_map
    }

    pub fn assign_winners(&mut self, winner_sets: &Vec<Vec<String>>) -> Vec<Pot> {
        let mut pots = self.pots.clone();

        for (idx, pot) in pots.iter_mut().enumerate() {
            let real_wnrs: Vec<String> = pot.owners().iter()
                    .filter(|w| winner_sets[idx].contains(w))
                    .map(|w| (*w).clone())
                    .collect();
            pot.update_winners(real_wnrs);
        }
        pots
    }

    pub fn collect_bets(&mut self) -> Vec<Pot> {
        // filter bets: arrange from small to big and remove duplicates
        let mut bets: Vec<u64> = self.bet_map.iter().map(|b| b.amount()).collect();
        bets.sort_by(|b1,b2| b1.cmp(b2));
        bets.dedup();

        let mut pots: Vec<Pot> = vec![];
        let mut prev_bet: u64 = 0;
        // group players by bets and make pots: those who bet the same in the same pot(s)
        for bet in bets {
            let mut owners: Vec<String> = vec![];
            let mut amount: u64 = 0;
            for ply in self.bet_map.clone() {
                if ply.amount() >= bet {
                    owners.push(ply.addr());
                    amount += bet - prev_bet;
                }
            }
            pots.push(
                Pot {
                    owners,
                    winners: vec![],
                    amount,
                });

            prev_bet = bet;
        }
        pots
    }

    pub fn next_state(&mut self, context: &mut GameContext) -> Result<()> {
        // 4. go to next game state, which is one of the following:
        // 4.1 detect current street: preflop + no bets => blind bets
        // 4.2 detect player number: 1 or a few?
        // 4.3 single player win? (All other players folded)
        // 4.4 next player to act (none of the above)
        // 4.5 runner
        // 4.6 new street or
        // 4.7 showdown
        let all_players = self.players.clone();
        let remain_players: Vec<&Player> = all_players.iter().filter(|p| p.to_remain()).collect();
        let players_toact: Vec<&Player> = all_players.iter().filter(|p| p.to_act()).collect();
        let allin_players: Vec<&Player> = remain_players.iter()
            .filter(|&p| p.status == PlayerStatus::Allin)
            .map(|p| *p)
            .collect();

        let bet_map = self.bet_map.clone();
        let next_action_player = Holdem::next_action_player(self.street_bet, bet_map, players_toact);

        // blind bets
        if self.street == Street::Preflop && self.bet_map.is_empty() {
            // take bets (sb and bb) from sb/bb players
            let mut bb_player = all_players[0].clone();
            let mut sb_player = all_players[1].clone();
            bb_player.take_bet(self.bb);
            sb_player.take_bet(self.sb);

            // update bet map
            self.update_betmap(
                vec![Bet::new(bb_player.addr, self.bb),
                     Bet::new(sb_player.addr, self.sb)]
            );

            // update street bet
            self.update_streetbet(self.street_bet);

            let action_players: Vec<&Player> = all_players[2..].iter()
                .filter(|p| p.next_to_act())
                .collect();

            if let Some(player) = action_players.get(0) {
                // skip pot for now
                println!("Blind bets!");
                todo!()
            } else {
                todo!()
            }
        } else if all_players.len() == 1 {
            // single player wins: one player in the game
            println!("Only one player left and won the game!");
            todo!()
        } else if remain_players.len() == 1 {
            // single player wins: one player bets and all others folded
            println!("All other players folded and the one left won the game!");
            let new_pots: Vec<Pot> = self.collect_bets();
            for pot in new_pots {
                println!("The pot amount is {}", pot.amount);
            }
            todo!()
        } else if !next_action_player.addr.is_empty() {
            println!("Ask next player to act!");
            let new_pots: Vec<Pot> = self.collect_bets();
            for pot in new_pots {
                println!("The pot amount is {}", pot.amount);
            }
            todo!()
        } else if self.status != HoldemStatus::Runner && remain_players.len() == allin_players.len() + 1 {
                // Runner
            println!("Entering runner state");
            todo!()
        } else {
            let new_street = self.next_street();
            // change_street(new_street);
            todo!()
        }

        // Showdown
    }

    // implement methods for Holdem to handle custom events
    fn handle_custom_event(&mut self, context: &mut GameContext, event: GameEvent, sender: String) -> Result<()> {
        // let next_action_player: Option<&Player> = remain_players.get(self.act_idx);
        match event {
            GameEvent::Fold  => {
                // Check player is at the action position?
                if self.players[self.act_idx].addr == sender {
                    println!("Player is at the action position");
                    // update player's status to fold
                    self.players[self.act_idx].status = PlayerStatus::Fold;

                } else {
                    return Err(Error::Custom(String::from("Not the player's turn to act!")));
                }

                Ok(())

                // single player wins: only one player left
                // if self.players.len() == 1 {
                //     todo!()
                // }
                // single player wins: others all folded

                // next player to act

                // runner
            }
            _ => todo!()
        }
    }
}

// implementing traits GameHandler
impl GameHandler for Holdem {
    fn init_state(context: &mut GameContext, init_account: GameAccount) -> Result<Self> {
        let account = HoldemAccount::try_from_slice(&init_account.data).unwrap();
        // Get players

        Ok(Self {
            sb: account.sb,
            bb: account.bb,
            buyin: account.buyin,
            btn: account.btn,
            size: account.size,
            rake: account.rake,
            status: HoldemStatus::Init,
            street: Street::Init,
            street_bet: account.bb,
            bet_map: vec![],    // suppose p[0] at bb, p[1] at sb, p[3] at btn, p[n-1] (last) frist to act
            players: vec![],
            pots: vec![
                Pot {owners: vec![], winners: vec![], amount: 0}
            ],
            act_idx: usize::from(0u8), // starts from last in Vec<Player>
        })
    }

    // for side effects, return either nothing or error
    fn handle_event(&mut self, context: &mut GameContext, event: Event) -> Result<()> {
        match event {
            // Handle custom events
            Event::Custom { sender, raw } => {
                // 1. Check if event sender is in the players
                if let Some(_) = context.get_players().iter().find(|&p| p.addr == sender) {
                    println!("Valid player");
                } else {
                    return Err(Error::Custom(String::from("Unknown player!")));
                }

                // 2. Check game status is valid (running)?
                match context.get_status() {
                    GeneralStatus::Running => {
                        println!("Valid game status: Running");
                    },
                    _ => {
                        return Err(Error::Custom(String::from("Invalid GameStatus: Game is not running!")));
                    }
                }
                // 3. handle this custom event
                let evt: GameEvent = serde_json::from_str(&raw)?;
                self.handle_custom_event(context, evt, sender)
            }
            // Ignore other types of events for now
            _ => {
                println!("Error");
                return Err(Error::Custom(String::from("Unknow event type!")));
            }
        }
    }
}
