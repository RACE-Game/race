#![allow(unused_imports)]
use borsh::{BorshDeserialize, BorshSerialize};
use race_core::{
    context::GameContext,
    engine::GameHandler,
    error::{Error, Result},
    event::{CustomEvent, Event},
    random::deck_of_cards,
    types::{GameAccount, RandomId, Settle},
};
use race_proc_macro::game_handler;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};

pub mod evaluator;
use evaluator::{compare_hands, create_cards, evaluate_cards};

// #[macro_use]
// extern crate log;

#[cfg(test)]
mod holdem_tests;
// #[cfg(test)]
// mod integration_test;

// HoldemAccount offers necessary (static) data (serialized in vec) to GameAccount for Holdem games
// This HoldemAccount data go to the (Raw) data field and
// Holdem (the WASM), the actual game, goes to the bundle_addr field
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug)]
pub struct HoldemAccount {
    pub sb: u64,
    pub bb: u64,
    pub buyin: u64,
    pub rake: f32,
    pub size: u8,         // table size: total number of players
    pub mode: HoldemMode, // game type: cash, sng or tourney?
    pub token: String,    // token should be a struct of its own?
}

impl Default for HoldemAccount {
    fn default() -> Self {
        Self {
            sb: 10,
            bb: 20,
            buyin: 400,
            rake: 0.02,
            size: 6,
            mode: HoldemMode::CASH,
            token: "sol".to_string(),
        }
    }
}

#[derive(Default, BorshSerialize, BorshDeserialize, PartialEq, Debug)]
pub enum HoldemMode {
    #[default]
    CASH,
    SNG,
}

#[derive(Deserialize, Serialize, Default, PartialEq, Clone, Debug)]
pub enum PlayerStatus {
    #[default]
    Wait,
    Acted,
    Acting,
    Allin,
    Fold,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub struct Player {
    pub addr: String,
    pub chips: u64,
    pub position: usize, // zero indexed
    pub status: PlayerStatus,
    // pub online_status
    // pub drop_count
    // pub timebank
    // pub nickname
}

impl PartialEq for Player {
    fn eq(&self, other: &Self) -> bool {
        self.addr == other.addr && self.position == other.position
    }
}

impl Player {
    pub fn new<S: Into<String>>(id: S, chips: u64, pos: usize) -> Player {
        Self {
            addr: id.into(),
            chips,
            position: pos,
            status: PlayerStatus::Wait,
        }
    }

    // Whether need to act
    pub fn to_act(&self) -> bool {
        match self.status {
            PlayerStatus::Wait | PlayerStatus::Acted => true,
            _ => false,
        }
    }

    pub fn to_remain(&self) -> bool {
        match self.status {
            PlayerStatus::Fold => false,
            _ => true,
        }
    }

    pub fn next_to_act(&self) -> bool {
        match self.status {
            PlayerStatus::Allin | PlayerStatus::Fold => false,
            _ => true,
        }
    }

    // This fn should:
    // 1. Indicate whether player goes all in
    // 2. Indicate the real bet (<= sb/bb)
    // 3. Update the player's chips in place
    pub fn take_bet(&mut self, bet: u64) -> (bool, u64) {
        if bet < self.chips {
            println!("Take {} from {}", bet, &self.addr);
            self.chips -= bet;
            (false, bet) // real bet
        } else {
            println!("{} ALL IN: {}", &self.addr, bet);
            let chips_left = self.chips;
            self.chips = 0;
            (true, chips_left) // real bet
        }
    }
}

#[derive(Deserialize, Serialize, PartialEq, Default, Clone, Debug)]
pub struct Pot {
    pub owners: Vec<String>,
    pub winners: Vec<String>,
    pub amount: u64,
}

impl Pot {
    pub fn new() -> Pot {
        Self {
            owners: Vec::<String>::new(),
            winners: Vec::<String>::new(),
            amount: 0,
        }
    }
}

#[derive(Deserialize, Serialize, PartialEq, Debug, Default, Clone)]
pub enum Street {
    #[default]
    Init,
    Preflop,
    Flop,
    Turn,
    River,
    Showdown,
    Done,
}

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone)]
pub struct Bet {
    pub owner: String,
    pub amount: u64,
}

impl Bet {
    pub fn new<S: Into<String>>(owner: S, amount: u64) -> Self {
        Self {
            owner: owner.into(),
            amount,
        }
    }
}

#[derive(Default, Serialize, Deserialize, PartialEq, Clone)]
pub enum HoldemStage {
    #[default]
    Init,
    BlindBets,
    // Encrypt,
    ShareKey,
    Play,
    Runner,
    Settle,
    Showdown,
    // Shuffle,
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
#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Holdem {
    // GameState Handler
    pub deck_random_id: RandomId,
    pub dealer_idx: usize,
    pub sb: u64,
    pub bb: u64,
    pub min_raise: u64, // (init == bb, then last raise amount)
    pub buyin: u64,
    pub btn: usize, // current btn position, zero-indexed?
    pub rake: f32,
    pub size: u32,
    pub stage: HoldemStage,
    // mode: HoldeMode,        // game type: cash, sng or tourney? Treat as CASH GAME
    // token: String,          // token should be a struct of its own?
    pub street: Street,
    pub street_bet: u64,
    pub seats_map: HashMap<String, usize>,
    pub board: Vec<String>,
    pub bet_map: HashMap<String, Bet>,
    pub prize_map: HashMap<String, u64>,
    // A map of players in the order of their init positions
    pub player_map: BTreeMap<String, Player>,
    pub players: Vec<Player>,
    pub acting_player: Option<String>, // Acting player's ID or address
    pub pots: Vec<Pot>,                // 1st main pot + rest side pot(s)
}

impl Holdem {
    fn next_action_player(&mut self, players_toact: Vec<&String>) -> Option<String> {
        for addr in players_toact {
            let player = self.player_map.get(addr).unwrap();
            if let Some(bet) = self.bet_map.get(addr) {
                if bet.amount < self.street_bet || player.status == PlayerStatus::Wait {
                    println!("{}' current bet amount {:?}", addr, bet.amount);
                    println!("Current street bet: {}", self.street_bet);
                    return Some(addr.clone());
                }
            } else if player.status == PlayerStatus::Wait {
                return Some(addr.clone());
            }
        }
        None
    }

    pub fn get_player_position(&self, player_addr: &String) -> Option<usize> {
        if let Some(pos) = self.seats_map.get(player_addr) {
            Some(*pos)
        } else {
            None
        }
    }

    pub fn is_acting_player(&self, player_addr: &String) -> bool {
        match &self.acting_player {
            Some(addr) => addr == player_addr,
            None => false,
        }
    }

    pub fn ask_for_action(&mut self, player_addr: String, context: &mut GameContext) -> Result<()> {
        if let Some(player) = self.player_map.get_mut(&player_addr) {
            println!("Asking next player {} to act ... ", player.addr);
            player.status = PlayerStatus::Acting;
            self.acting_player = Some(player.addr.clone());
            // Ask player for action within 30 secs
            context.action_timeout(player_addr, 30_000);
            Ok(())
        } else {
            return Err(Error::Custom("Next player not found in game!".to_string()));
        }
    }

    // Place players in the order of sb, bb, 1st-to-act, 2nd-to-act, ..., btn
    pub fn arrange_players(&mut self) -> Vec<String> {
        // Players sit clockwise,
        self.btn = 2;           // DELETE THIS LINE; FOR TESTING ONLY
        println!("BTN is {}", self.btn);
        let mut player_pos: Vec<(String, usize)> = self.player_map.values()
            .map(|p| {
                if p.position > self.btn {
                    (p.addr.clone(), p.position - self.btn)
                } else {
                    (p.addr.clone(), p.position + 100)
                }
            }).collect();
        player_pos.sort_by(|(_, pos1), (_, pos2)| pos1.cmp(pos2));
        let player_order: Vec<String> =
            player_pos.iter().map(|(addr, _)| addr.clone()).collect();
        println!("Players in order of action {:?}", player_order);
        player_order
    }

    pub fn blind_bets(&mut self, context: &mut GameContext) -> Result<()> {
        let mut players: Vec<String> = self.arrange_players();
        if players.len() == 2 {
            players.reverse();
            // Take bet from BB
            let addr = players.get(0).unwrap();
            if let Some(bb_player) = self.player_map.get_mut(addr) {
                let (allin, real_bb) = bb_player.take_bet(self.bb);
                // FIXME: handle allin
                self.bet_map.insert(
                    bb_player.addr.clone(),
                    Bet::new(bb_player.addr.clone(), real_bb),
                );
            }
            // Take bet from SB (btn)
            let addr = players.get(1).unwrap();
            if let Some(sb_player) = self.player_map.get_mut(addr) {
                println!("SB Player is {:?}", sb_player);
                let (allin, real_sb) = sb_player.take_bet(self.sb);
                // FIXME: handle allin
                self.bet_map.insert(
                    sb_player.addr.clone(),
                    Bet::new(sb_player.addr.clone(), real_sb),
                );
                // SB acts first
                self.ask_for_action((*addr).clone(), context)?;
            }
        } else {
            // Take bet from SB (1st in Player vec)
            let addr = players.get(0).unwrap();
            if let Some(sb_player) = self.player_map.get_mut(addr) {
                println!("SB Player is {:?}", sb_player);
                let (allin, real_sb) = sb_player.take_bet(self.sb);
                if allin {
                    sb_player.status = PlayerStatus::Allin;
                }
                self.bet_map.insert(
                    sb_player.addr.clone(),
                    Bet::new(sb_player.addr.clone(), real_sb),
                );
            }
            // Take bet from BB (2nd in Player vec)
            let addr = players.get(1).unwrap();
            if let Some(bb_player) = self.player_map.get_mut(addr) {
                println!("BB Player is {:?}", bb_player);
                let (allin, real_bb) = bb_player.take_bet(self.bb);
                if allin {
                    bb_player.status = PlayerStatus::Allin;
                }
                self.bet_map.insert(
                    bb_player.addr.clone(),
                    Bet::new(bb_player.addr.clone(), real_bb),
                );
            }

            // Select the next
            {
                players.rotate_left(2);
                let action_players: Vec<String> = players.into_iter()
                    .filter(|addr| {
                        if let Some(player) = self.player_map.get_mut(addr) {
                            player.next_to_act()
                        } else {
                            false
                        }
                    })
                    .collect();
                let player_to_act = action_players.first().unwrap();
                println!("Player {} will act next", player_to_act);
                self.ask_for_action((*player_to_act).clone(), context)?;
            }
        }
        // TODO: add mini raise
        // Update street bet
        self.street_bet = self.bb;
        Ok(())
    }

    // Handle main pot and side pot(s), for example:
    // Players A(100), B(45), C(45), D(50) call or go all in, then the pots become
    // Main:  { amount: 45*4, owners: [A, B, C, D], winners [] }
    // Side1: { amount: 5*2,  owners: [A, D], winners [] }
    // Side2: { amount: 50,   owners: [A], winners [] } <-- should return bet to A
    pub fn collect_bets(&mut self) -> Result<()> {
        // filter bets: arrange from small to big and remove duplicates
        let mut bets: Vec<u64> = self.bet_map.iter().map(|(_, b)| b.amount).collect();
        bets.sort_by(|b1, b2| b1.cmp(b2));
        bets.dedup();

        let mut new_pots: Vec<Pot> = Vec::new();
        // This bet is the minimum or base among the owners of a pot
        let mut acc: u64 = 0;
        for bet in bets {
            let owners: Vec<String> = self.bet_map
                .iter()
                .filter(|(_, b)| b.amount >= bet)
                .map(|(_, b)| b.owner.clone())
                .collect();
            let amount = (bet - acc) * owners.len() as u64;
            // Pot with only 1 owner should return the bet in it to the owner
            if owners.len() == 1 {
                // TODO: replace `expect` method?
                let index = self
                    .get_player_position(&owners[0])
                    .expect("Player NOT found at table");
                if let Some(receiver) = self.players.get_mut(index) {
                    receiver.chips += amount;
                }
                continue;
            } else {
                new_pots.push(Pot {
                    owners,
                    winners: Vec::<String>::new(),
                    amount,
                });
                acc += bet;
            }
        }
        self.bet_map = HashMap::<String, Bet>::new();
        self.pots = new_pots;
        Ok(())
    }

    pub fn change_street(&mut self, context: &mut GameContext, street: Street) -> Result<()> {
        // Reset acted to wait
        for player in &mut self.players {
            if player.status == PlayerStatus::Acted {
                player.status = PlayerStatus::Wait;
            }
        }
        println!("Moving to next street {:?}", street);
        self.collect_bets()?;
        self.stage = HoldemStage::Play;
        self.street = street;
        self.street_bet = 0;
        self.acting_player = None;

        let players_cnt = self.players.len() * 2;

        match self.street {
            Street::Flop => {
                context.reveal(
                    self.deck_random_id,
                    (players_cnt..(players_cnt + 3)).collect::<Vec<usize>>(),
                )?;
                Ok(())
            }

            Street::Turn => {
                context.reveal(
                    self.deck_random_id,
                    (players_cnt..(players_cnt + 4)).collect::<Vec<usize>>(),
                )?;
                Ok(())
            }

            Street::River => {
                context.reveal(
                    self.deck_random_id,
                    (players_cnt..(players_cnt + 5)).collect::<Vec<usize>>(),
                )?;
                Ok(())
            }

            _ => Ok(()),
        }
    }

    pub fn next_street(&mut self) -> Street {
        match self.street {
            Street::Init => Street::Preflop,
            Street::Preflop => Street::Flop,
            Street::Flop => Street::Turn,
            Street::Turn => Street::River,
            _ => Street::Showdown,
        }
    }

    pub fn apply_prize(&mut self) -> Result<()> {
        for ply in &mut self.players {
            match self.prize_map.get(&ply.addr) {
                // If player in the prize map, chips increase by the amt in the map
                Some(prize) => {
                    ply.chips += *prize;
                    println!("Player {} won {} chips", ply.addr, *prize);
                }
                // else chips decrease by the amt in the bet map?
                None => {
                    println!("Player {} lost bet", ply.addr);
                    // player.take_bet() aleady reduced the chips
                }
            }
        }
        Ok(())
    }

    pub fn calc_prize(&mut self) -> Result<()> {
        let pots = &mut self.pots;
        let mut prize_map = HashMap::<String, u64>::new();
        for pot in pots {
            let cnt: u64 = pot.winners.len() as u64;
            let prize: u64 = pot.amount / cnt;
            for wnr in &pot.winners {
                prize_map
                    .entry(wnr.clone())
                    .and_modify(|p| *p += prize)
                    .or_insert(prize);
            }
        }
        self.prize_map = prize_map;
        Ok(())
    }

    // TODO: Use HashSet::intersection?
    pub fn assign_winners(&mut self, winner_sets: &Vec<Vec<String>>) -> Result<()> {
        let mut pots = self.pots.clone();
        for (idx, pot) in pots.iter_mut().enumerate() {
            let real_wnrs: Vec<String> = pot
                .owners
                .iter()
                .filter(|w| winner_sets[idx].contains(w))
                .map(|w| (*w).clone())
                .collect();
            pot.winners = real_wnrs;
        }
        self.pots = pots;
        Ok(())
    }

    pub fn single_player_win(&mut self, winner: &Vec<Vec<String>>) -> Result<()> {
        self.collect_bets()?;
        self.assign_winners(winner)?;
        self.calc_prize()?;
        self.apply_prize()?;
        // TODO:
        // 1. submit game result
        // 2. remove non alive players
        // 3. dispatch reset event
        Ok(())
    }

    pub fn settle(&mut self) -> Result<()> {
        Ok(())
    }

    // De facto entry point of Holdem
    pub fn next_state(&mut self, context: &mut GameContext) -> Result<()> {
        // let mut all_players: Vec<&Player> = self.player_map.values().collect();
        let all_players: Vec<String> = self.arrange_players();
        let remained_players: Vec<&String> = all_players.iter()
            .filter(|addr| {
                if let Some(player) = self.player_map.get(*addr) {
                    player.to_remain()
                } else {
                    false
                }
            })
            .collect();

        let mut toact_players: Vec<&String> = all_players.iter()
            .filter(|addr| {
                if let Some(player) = self.player_map.get(*addr) {
                    player.to_act()
                } else {
                    false
                }
            })
            .collect();

        let allin_players: Vec<&String> = remained_players.iter()
            .filter(|addr| {
                if let Some(player) = self.player_map.get(**addr) {
                    player.status == PlayerStatus::Allin
                } else {
                    false
                }
            })
            .map(|p| *p)
            .collect();

        let next_player = self.next_action_player(toact_players);
        let new_street = self.next_street();

        println!("Current GAME BETS are {:?}", self.bet_map);

        // Blind bets
        if self.street == Street::Preflop && self.bet_map.is_empty() {
            println!("[Next State]: Blind bets");
            self.stage = HoldemStage::BlindBets;
            self.blind_bets(context)?;
            Ok(())
        }
        // Single player wins because there are one player only
        else if all_players.len() == 1 {
            let winner = all_players.first().unwrap();
            println!("[Next State]: Only {} left and wins.", winner);
            self.single_player_win(&vec![vec![(*winner).clone()]])?;
            Ok(())
        }
        // Singple players wins because others all folded
        else if remained_players.len() == 1 {
            let winner = all_players.first().unwrap();
            println!(
                "[Next State]: All others folded and single winner is {}",
                winner
            );
            self.single_player_win(&vec![vec![(*winner).clone()]])?;
            Ok(())
        }
        // Next player to act
        else if next_player.is_some() {
            let next_action_player = next_player.unwrap();
            println!(
                "[Next State]: Next-to-act player is: {}",
                next_action_player
            );
            self.ask_for_action(next_action_player, context)?;
            Ok(())
        }
        // Runner
        else if self.stage != HoldemStage::Runner
            && allin_players.len() + 1 >= remained_players.len()
        {
            println!("[Next State]: Runner");
            self.street = Street::Showdown;

            // Reveal players' hole cards
            let mut indexes = Vec::<usize>::new();
            for (idx, player) in self.players.iter().enumerate() {
                if player.status != PlayerStatus::Fold {
                    indexes.push(idx * 2);
                    indexes.push(idx * 2 + 1);
                }
            }
            context.reveal(self.deck_random_id, indexes)?;

            // Reveal community cards
            let players_cnt = self.players.len() * 2;
            context.reveal(
                self.deck_random_id,
                (players_cnt..(players_cnt + 5)).collect::<Vec<usize>>(),
            )?;
            Ok(())
        }
        // Next Street
        else if new_street != Street::Showdown {
            println!("[Next State]: Move to next street: {:?}", new_street);
            self.change_street(context, new_street)?;
            Ok(())
        }
        // Showdown
        else {
            println!("[Next State]: Showdown");
            self.street = Street::Showdown;
            // Reveal all hole cards for players who have not folded
            let mut indexes = Vec::new();
            for (idx, player) in self.players.iter().enumerate() {
                if player.status != PlayerStatus::Fold {
                    indexes.push(idx * 2);
                    indexes.push(idx * 2 + 1);
                }
            }
            context.reveal(self.deck_random_id, indexes)?;

            Ok(())
        }
    }

    #[allow(unused_variables)]
    fn handle_custom_event(
        &mut self,
        context: &mut GameContext,
        event: GameEvent,
        sender: String,
    ) -> Result<()> {
        match event {
            GameEvent::Bet(amount) => {
                // TODO: handle other Errors
                if !self.is_acting_player(&sender) {
                    return Err(Error::Custom(
                        "Player is NOT the acting player so can't raise!".to_string(),
                    ));
                }
                // TODO: Log more detailed error info?
                // TODO: amount must be dividable by sb!
                let player_pos: usize = self
                    .seats_map
                    .get(&sender)
                    .expect("Player not found in game!")
                    .clone();
                // FIXME: update condition after refactoring bet_map
                if self.bet_map.get(&sender).is_some() {
                    return Err(Error::Custom("Player already betted!".to_string()));
                }

                // TODO: amount must be less than player's remained chips
                if self.street_bet > amount {
                    return Err(Error::Custom("Player's bet is too small".to_string()));
                }

                if let Some(player) = self.players.get_mut(player_pos) {
                    let (allin, real_bet) = player.take_bet(amount);
                    self.bet_map
                        .insert(player.addr.clone(), Bet::new(player.addr.clone(), real_bet));
                    player.status = if allin {
                        PlayerStatus::Allin
                    } else {
                        PlayerStatus::Acted
                    };
                    self.next_state(context)?;
                    Ok(())
                } else {
                    return Err(Error::Custom(
                        "Player not found in game or has not joined yet!".to_string(),
                    ));
                }
            }

            GameEvent::Call => {
                // TODO: handle errors
                if !self.is_acting_player(&sender) {
                    return Err(Error::Custom(
                        "Player is NOT the acting player so can't raise!".to_string(),
                    ));
                }
                let player_pos: usize = self
                    .seats_map
                    .get(&sender)
                    .expect("Player not found in game!")
                    .clone();
                // TODO: update condition after refactoring bet map
                if let Some(player) = self.players.get_mut(player_pos) {
                    if let Some(betted) = self.bet_map.get_mut(&sender) {
                        let betted_amount = betted.amount;
                        let call_amount = self.street_bet - betted_amount;
                        println!("{} needs to call with amount {}", player.addr, call_amount);
                        let (allin, real_bet) = player.take_bet(call_amount);
                        betted.amount += real_bet;
                        player.status = if allin {
                            PlayerStatus::Allin
                        } else {
                            PlayerStatus::Acted
                        };
                        println!("After calls: {:?}", player);
                        self.next_state(context)?;
                        Ok(())
                    } else {
                        let call_amount = self.street_bet - 0;
                        let (allin, real_bet) = player.take_bet(call_amount);
                        self.bet_map
                            .insert(player.addr.clone(), Bet::new(player.addr.clone(), real_bet));
                        player.status = if allin {
                            PlayerStatus::Allin
                        } else {
                            PlayerStatus::Acted
                        };
                        self.next_state(context)?;
                        Ok(())
                    }
                } else {
                    return Err(Error::Custom("Player not found in game".to_string()));
                }
            }

            GameEvent::Check => {
                if !self.is_acting_player(&sender) {
                    return Err(Error::Custom(
                        "Player is NOT the acting player so can't raise!".to_string(),
                    ));
                }
                let player_pos: usize = self
                    .seats_map
                    .get(&sender)
                    .expect("Player not found in game!")
                    .clone();
                match self.street {
                    Street::Preflop => {
                        if let Some(player_bet) = self.bet_map.get(&sender) {
                            if self.street_bet != player_bet.amount {
                                return Err(Error::Custom("Player can't check!".to_string()));
                            }

                            if let Some(player) = self.players.get_mut(player_pos) {
                                player.status = PlayerStatus::Acted;
                                self.next_state(context)?;
                                Ok(())
                            } else {
                                return Err(Error::Custom("Player not found in game!".to_string()));
                            }
                        } else {
                            return Err(Error::Custom("Player hasnt bet yet!".to_string()));
                        }
                    }

                    Street::Flop | Street::Turn | Street::River => {
                        if self.bet_map.is_empty() {
                            let player = self.players.get_mut(player_pos).unwrap();
                            player.status = PlayerStatus::Acted;
                            self.next_state(context)?;
                            Ok(())
                        } else {
                            return Err(Error::Custom("Player hasnt bet yet!".to_string()));
                        }
                    }
                    _ => Err(Error::Custom(
                        "Invalid Street so player can't check!".to_string(),
                    )),
                }
            }

            GameEvent::Fold => {
                if !self.is_acting_player(&sender) {
                    return Err(Error::Custom(
                        "Player is NOT the acting player so can't raise!".to_string(),
                    ));
                }

                if let Some(player) = self.player_map.get_mut(&sender) {
                    println!("Player {} folds", sender);
                    player.status = PlayerStatus::Fold;
                    self.next_state(context)?;
                    Ok(())
                } else {
                    return Err(Error::Custom("Player NOT found in game!".to_string()));
                }
            }

            GameEvent::Raise(amount) => {
                if !self.is_acting_player(&sender) {
                    return Err(Error::Custom(
                        "Player is the acting player so can't raise!".to_string(),
                    ));
                }
                if self.street_bet == 0 || self.bet_map.is_empty() {
                    return Err(Error::Custom(
                        "Street bet is 0 so raising is not allowed!".to_string(),
                    ));
                }
                if amount == 0 || amount < self.street_bet {
                    return Err(Error::Custom(
                        "Invalid raise amount: 0 or less than street bet".to_string(),
                    ));
                }
                // TODO: handle raise too small
                let player_pos: usize = self
                    .seats_map
                    .get(&sender)
                    .expect("Player not found in game!")
                    .clone();
                if let Some(player) = self.players.get_mut(player_pos) {
                    let betted = self.bet_map.get_mut(&sender).unwrap();
                    let added_bet = amount - betted.amount;
                    let (allin, real_bet) = player.take_bet(added_bet);
                    self.street_bet = betted.amount + real_bet;
                    // TODO: update mini raise amount
                    player.status = if allin {
                        PlayerStatus::Allin
                    } else {
                        PlayerStatus::Acted
                    };
                    if let Some(player_bet) = self.bet_map.get_mut(&sender) {
                        player_bet.amount += real_bet;
                    } else {
                        self.bet_map
                            .insert(sender, Bet::new(player.addr.clone(), real_bet));
                    }
                    self.next_state(context)?;
                    Ok(())
                } else {
                    return Err(Error::Custom("Player not found in game!".to_string()));
                }
            }
        }
    }
}

impl GameHandler for Holdem {
    fn init_state(context: &mut GameContext, _init_account: GameAccount) -> Result<Self> {
        // Skip this account part for now
        // let account = HoldemAccount::try_from_slice(&init_account.data).unwrap();
        Ok(Self {
            deck_random_id: 1,
            dealer_idx: 0,
            sb: 10,
            bb: 20,
            min_raise: 20,
            buyin: 400,
            btn: 0,
            size: 6,
            rake: 0.2,
            stage: HoldemStage::Init,
            street: Street::Init,
            street_bet: 0,
            seats_map: HashMap::<String, usize>::new(),
            board: Vec::<String>::with_capacity(5),
            bet_map: HashMap::<String, Bet>::new(),
            prize_map: HashMap::<String, u64>::new(),
            player_map: BTreeMap::<String, Player>::new(),
            // p[0] sb, p[1] bb, p[2] 1st to act, p[n-1] (last) btn; when 2 players, btn == sb
            players: Vec::<Player>::new(),
            pots: Vec::<Pot>::new(),
            acting_player: None,
        })
    }

    fn handle_event(&mut self, context: &mut GameContext, event: Event) -> Result<()> {
        match event {
            // Handle holdem specific (custom) events
            Event::Custom { sender, raw } => {
                let event: GameEvent = serde_json::from_str(&raw)?;
                self.handle_custom_event(context, event, sender.clone())?;
                Ok(())
            }

            // Timeout, reveal, assign
            Event::GameStart { .. } => {
                // After a player joins
                let rnd_spec = deck_of_cards();
                self.deck_random_id = context.init_random_state(&rnd_spec)?;

                // Initializing the game state
                // self.stage = HoldemStage::Play;
                let player_num = context.get_players().len();
                self.btn = player_num - 1;
                println!("There are {} players in game", player_num);
                self.street = Street::Preflop;
                Ok(())
            }

            Event::Sync { new_players, .. } => {
                // TODO
                for p in new_players.iter() {
                    // TODO: balance == chips?
                    let player = Player::new(p.addr.clone(), p.balance, p.position);
                    // TODO: Check diff of p.position and the current positions of game
                    self.player_map.insert(player.addr.clone(), player);
                }

                // Must detect num of players and servers
                if context.count_players() >= 2 && context.count_servers() >= 1 {
                    context.start_game();
                }
                Ok(())
            }

            Event::ActionTimeout { player_addr } => {
                // TODO: Handle player's default preference (always check)?

                let player_pos: usize = self
                    .seats_map
                    .get(&player_addr)
                    .expect("Player not found in game!")
                    .clone();
                let player_bet = self
                    .bet_map
                    .get(&player_addr)
                    .expect("No bet found for player!")
                    .clone();
                let street_bet = self.street_bet;

                if let Some(player) = self.players.get_mut(player_pos) {
                    if player.addr.clone() == player_bet.owner {
                        if player_bet.amount == street_bet {
                            player.status = PlayerStatus::Acted;
                            self.acting_player = None;
                            self.next_state(context)?;
                            Ok(())
                        } else {
                            player.status = PlayerStatus::Fold;
                            self.acting_player = None;
                            self.next_state(context)?;
                            Ok(())
                        }
                    } else {
                        Err(Error::Custom("Player not found in bets array!".to_string()))
                    }
                } else {
                    Err(Error::Custom("Player not found in game!".to_string()))
                }
            }

            Event::RandomnessReady { .. } => {
                // TODO: handle multi-player table
                let addr0 = context.get_player_by_index(0).unwrap().addr.clone();
                let addr1 = context.get_player_by_index(1).unwrap().addr.clone();
                context.assign(self.deck_random_id, addr0, vec![0, 1])?;
                context.assign(self.deck_random_id, addr1, vec![2, 3])?;
                Ok(())
            }

            // TODO: Use Stage?
            Event::SecretsReady => match self.street {
                Street::Init | Street::Preflop | Street::Flop | Street::Turn | Street::River => {
                    self.next_state(context)?;
                    Ok(())
                }

                Street::Showdown => {
                    let decryption = context.get_revealed(self.deck_random_id)?;
                    // let player_idx: usize = 0;
                    // let dealer_addr = context
                    //     .get_player_by_index(0)
                    //     .unwrap()
                    //     .addr
                    //     .clone();
                    // let player_addr = context
                    //     .get_player_by_index(1)
                    //     .unwrap()
                    //     .addr
                    //     .clone();
                    let bob_hole = [
                        &decryption.get(&0usize).unwrap(),
                        decryption.get(&1usize).unwrap().as_str(),
                    ];
                    let alice_hole = [
                        decryption.get(&2usize).unwrap().as_str(),
                        decryption.get(&3usize).unwrap().as_str(),
                    ];
                    let board = [
                        decryption.get(&4usize).unwrap().as_str(),
                        decryption.get(&5usize).unwrap().as_str(),
                        decryption.get(&6usize).unwrap().as_str(),
                        decryption.get(&7usize).unwrap().as_str(),
                        decryption.get(&8usize).unwrap().as_str(),
                    ];

                    let alice_cards = create_cards(&board, &alice_hole);
                    let bob_cards = create_cards(&board, &bob_hole);

                    let alice_hand = evaluate_cards(alice_cards);
                    let bob_hand = evaluate_cards(bob_cards);

                    let result = compare_hands(&alice_hand.value, &bob_hand.value);
                    match result {
                        Ordering::Greater => {
                            println!("Winner is Alice")
                        }
                        Ordering::Less => {
                            println!("Winner is Bob")
                        }
                        Ordering::Equal => {
                            println!("A Tie!")
                        }
                    };
                    // let (winner, loser) = if is_better_than(dealer_card, player_card) {
                    //     (dealer_addr, player_addr)
                    // } else {
                    //     (player_addr, dealer_addr)
                    // };
                    // context.settle(vec![
                    //     Settle::add(winner, self.bet),
                    //     Settle::sub(loser, self.bet),
                    //     ]);

                    Ok(())
                }

                //TODO: handle opertaiontimeout event
                //
                _ => Ok(()),
            },
            _ => Ok(()),
        }
    }
}
