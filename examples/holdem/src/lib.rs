use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;

use race_core::prelude::*;

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
    // pub mini_rasie: u64, (init == bb, then last raise amount)
    pub buyin: u64,
    pub btn: usize, // current btn position, zero-indexed?
    pub rake: f32,
    pub size: u32,
    pub stage: HoldemStage,
    // mode: HoldeMode,        // game type: cash, sng or tourney? Treat as CASH GAME
    // token: String,          // token should be a struct of its own?
    pub street: Street,
    // TODO: add min_raise
    pub street_bet: u64,
    pub seats_map: BTreeMap<String, usize>,
    // pub community_cards: &'a str, Rust: COW/to_owned()
    pub bets: Vec<Bet>, // each bet's index maps to player's pos (index) in the below player list
    pub prize_map: BTreeMap<String, u64>,
    pub players: Vec<Player>,
    // TODO: use player addr <String> to represent acting player
    pub acting_player: Option<Player>,
    pub pots: Vec<Pot>, // 1st main pot + rest side pot(s)
}

impl Holdem {
    fn next_action_player(&mut self, players_toact: Vec<&Player>) -> Option<String> {
        // FIXME: re-order the players
        // Sort players_to_act
        // players_toact.sort_by(|p1, p2| -> Ordering {
        //     if p1.position >= self.btn && p2.position <= self.btn { Ordering::Less }
        //     else if p1.position <= self.btn && p2.position >= self.btn { Ordering::Greater }
        //     else { Ordering::Equal }
        // });
        for p in players_toact {
            if let Some(bet) = self.bets.get(p.position) {
                if bet.amount < self.street_bet || p.status == PlayerStatus::Wait {
                    println!("{}' current bet amount {:?}", p.addr, bet.amount);
                    println!("Current street bet: {}", self.street_bet);
                    return Some(p.addr.clone());
                }
            } else if p.status == PlayerStatus::Wait {
                return Some(p.addr.clone());
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

    pub fn is_acting_player(&self, player_addr: String) -> bool {
        match &self.acting_player {
            Some(player) => player.addr == player_addr,
            None => false,
        }
    }

    pub fn ask_for_action(&mut self, player_addr: String, context: &mut Effect) -> Result<()> {
        let player_pos: usize = self
            .seats_map
            .get(&player_addr)
            .expect("Player not found in game!")
            .clone();
        let player = self.players.get_mut(player_pos).unwrap();
        println!("Asking next player {} to act ... ", player.addr);
        player.status = PlayerStatus::Acting;
        self.acting_player = Some(player.clone());
        // Ask player for action within 30 secs
        context.action_timeout(player_addr, 30_000);
        Ok(())
    }

    pub fn change_street(&mut self, context: &mut Effect, street: Street) -> Result<()> {
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
                );
                Ok(())
            }

            Street::Turn => {
                context.reveal(
                    self.deck_random_id,
                    (players_cnt..(players_cnt + 4)).collect::<Vec<usize>>(),
                );
                Ok(())
            }

            Street::River => {
                context.reveal(
                    self.deck_random_id,
                    (players_cnt..(players_cnt + 5)).collect::<Vec<usize>>(),
                );
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
        let mut prize_map = BTreeMap::<String, u64>::new();
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

    // Handle main pot and side pot(s), for example:
    // Players A(100), B(45), C(45), D(50) call or go all in, then the pots become
    // Main:  { amount: 45*4, owners: [A, B, C, D], winners [] }
    // Side1: { amount: 5*2,  owners: [A, D], winners [] }
    // Side2: { amount: 50,   owners: [A], winners [] } <-- should return bet to A
    pub fn collect_bets(&mut self) -> Result<()> {
        // filter bets: arrange from small to big and remove duplicates
        let mut bets: Vec<u64> = self.bets.iter().map(|b| b.amount).collect();
        bets.sort_by(|b1, b2| b1.cmp(b2));
        bets.dedup();

        let mut new_pots: Vec<Pot> = Vec::new();
        // This bet is the minimum or base among the owners of a pot
        let mut acc: u64 = 0;
        for bet in bets {
            let owners: Vec<String> = self
                .bets
                .iter()
                .filter(|b| b.amount >= bet)
                .map(|b| b.owner.clone())
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
        self.bets = Vec::<Bet>::new();
        self.pots = new_pots;
        Ok(())
    }

    pub fn blind_bets(&mut self, context: &mut Effect) -> Result<()> {
        let players = &mut self.players;
        #[allow(unused_assignments)]
        let mut player_to_act = String::new();
        if players.len() == 2 {
            // Take bet from SB (btn)
            {
                self.btn = 0; // SB is btn
                let sb_player = players.get_mut(self.btn).unwrap();
                // sb_player.position = self.btn;
                self.seats_map.insert(sb_player.addr.clone(), self.btn);
                player_to_act = sb_player.addr.clone();
                println!("SB Player is {:?}", sb_player);
                let (_allin, real_sb) = sb_player.take_bet(self.sb);
                self.bets.push(Bet::new(sb_player.addr.clone(), real_sb));
            }
            // Take bet from BB
            {
                let bb_player = players.get_mut(1).unwrap();
                // bb_player.position = 0;
                self.seats_map.insert(bb_player.addr.clone(), 1);
                println!("BB Player is {:?}", bb_player);
                let (_allin, real_bb) = bb_player.take_bet(self.bb);
                self.bets.push(Bet::new(bb_player.addr.clone(), real_bb));
            }
            // SB acts first
            self.ask_for_action(player_to_act, context)?;
        } else {
            // Take bet from SB (1st in Player vec)
            {
                let sb_player = players.get_mut(0).unwrap();
                println!("SB Player is {:?}", sb_player);
                let (_allin, real_sb) = sb_player.take_bet(self.sb);
                self.bets.push(Bet::new(sb_player.addr.clone(), real_sb));
            }
            // Take bet from BB (2nd in Player vec)
            {
                let bb_player = players.get_mut(1).unwrap();
                println!("BB Player is {:?}", bb_player);
                let (_allin, real_bb) = bb_player.take_bet(self.bb);
                self.bets.push(Bet::new(bb_player.addr.clone(), real_bb));
            }

            // Select the next
            {
                let mut new_players = vec![];
                new_players.extend_from_slice(&players[2..]);
                new_players.extend_from_slice(&players[0..3]);
                let action_players: Vec<&Player> =
                    new_players.iter().filter(|p| p.next_to_act()).collect();
                player_to_act = action_players[0].addr.clone();
                // Update seats_map
                for (pos, ply) in new_players.iter().enumerate() {
                    let old_pos = self.seats_map.insert(ply.addr.clone(), pos);
                    // TODO: old_pos can be None
                    println!(
                        "Player {} old position: {}",
                        ply.addr,
                        old_pos.unwrap_or(9999usize)
                    );
                    println!("Player {} new position: {}", ply.addr, pos);
                }
                // TODO: update players vec
                // self.players = new_players
            }
            self.ask_for_action(player_to_act, context)?;
        }
        // TODO: add mini raise
        // Update street bet
        self.street_bet = self.bb;
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

    pub fn next_state(&mut self, context: &mut Effect) -> Result<()> {
        let all_players = self.players.clone();
        let remained_players: Vec<&Player> = all_players.iter().filter(|p| p.to_remain()).collect();
        let toact_players: Vec<&Player> = all_players.iter().filter(|p| p.to_act()).collect();
        let allin_players: Vec<&Player> = remained_players
            .iter()
            .filter(|&p| p.status == PlayerStatus::Allin)
            .map(|p| *p)
            .collect();

        let next_player = self.next_action_player(toact_players);
        let new_street = self.next_street();

        println!("Current GAME BETS are {:?}", self.bets);

        // Blind bets
        if self.street == Street::Preflop && self.bets.is_empty() {
            println!("[Next State]: Blind bets");
            self.stage = HoldemStage::BlindBets;
            self.blind_bets(context)?;
            Ok(())
        }
        // Single player wins because there are one player only
        else if all_players.len() == 1 {
            let winner = all_players[0].addr.clone();
            println!("[Next State]: Only {} left and wins.", winner);
            self.single_player_win(&vec![vec![winner]])?;
            Ok(())
        }
        // Singple players wins because others all folded
        else if remained_players.len() == 1 {
            let winner = remained_players[0].addr.clone();
            println!(
                "[Next State]: All others folded and single winner is {}",
                winner
            );
            self.single_player_win(&vec![vec![winner]])?;
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
            context.reveal(self.deck_random_id, indexes);

            // Reveal community cards
            let players_cnt = self.players.len() * 2;
            context.reveal(
                self.deck_random_id,
                (players_cnt..(players_cnt + 5)).collect::<Vec<usize>>(),
            );
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
            context.reveal(self.deck_random_id, indexes);

            Ok(())
        }
    }

    #[allow(unused_variables)]
    fn handle_custom_event(
        &mut self,
        context: &mut Effect,
        event: GameEvent,
        sender: String,
    ) -> Result<()> {
        match event {
            GameEvent::Bet(amount) => {
                // TODO: handle other Errors
                if !self.is_acting_player(sender.clone()) {
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
                if self.bets.get(player_pos).is_some() {
                    return Err(Error::Custom("Player already betted!".to_string()));
                }

                // TODO: amount must be less than player's remained chips
                if self.street_bet > amount {
                    return Err(Error::Custom("Player's bet is too small".to_string()));
                }

                if let Some(player) = self.players.get_mut(player_pos) {
                    let (allin, real_bet) = player.take_bet(amount);
                    self.bets.push(Bet::new(player.addr.clone(), real_bet));
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
                if !self.is_acting_player(sender.clone()) {
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
                    if let Some(betted) = self.bets.get_mut(player_pos) {
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
                        self.bets.push(Bet::new(player.addr.clone(), real_bet));
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
                if !self.is_acting_player(sender.clone()) {
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
                        if let Some(player_bet) = self.bets.get(player_pos) {
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
                        if self.bets.is_empty() {
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
                if !self.is_acting_player(sender.clone()) {
                    return Err(Error::Custom(
                        "Player is NOT the acting player so can't raise!".to_string(),
                    ));
                }
                // FIXME: refactor this part
                let acting_player = self.acting_player.clone();
                match acting_player {
                    Some(mut player) => {
                        if player.addr == sender {
                            println!("Player is at the action position");
                            player.status = PlayerStatus::Fold;
                            self.acting_player = Some(player);
                        }
                    }
                    None => {
                        return Err(Error::Custom(String::from("Not the player's turn to act!")));
                    }
                }

                Ok(())
            }

            GameEvent::Raise(amount) => {
                if !self.is_acting_player(sender.clone()) {
                    return Err(Error::Custom(
                        "Player is the acting player so can't raise!".to_string(),
                    ));
                }
                if self.street_bet == 0 || self.bets.is_empty() {
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
                    let betted = self.bets.get_mut(player_pos).unwrap();
                    let added_bet = amount - betted.amount;
                    let (allin, real_bet) = player.take_bet(added_bet);
                    self.street_bet = betted.amount + real_bet;
                    // TODO: update mini raise amount
                    player.status = if allin {
                        PlayerStatus::Allin
                    } else {
                        PlayerStatus::Acted
                    };
                    if let Some(player_bet) = self.bets.get_mut(player_pos) {
                        player_bet.amount += real_bet;
                    } else {
                        self.bets.push(Bet::new(player.addr.clone(), real_bet))
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
    fn init_state(_context: &mut Effect, _init_account: InitAccount) -> Result<Self> {
        // Skip this account part for now
        // let account = HoldemAccount::try_from_slice(&init_account.data).unwrap();
        Ok(Self {
            deck_random_id: 1,
            dealer_idx: 0,
            sb: 10,
            bb: 20,
            buyin: 400,
            btn: 0,
            size: 6,
            rake: 0.2,
            stage: HoldemStage::Init,
            street: Street::Init,
            street_bet: 0,
            seats_map: BTreeMap::<String, usize>::new(),
            // community_cards: vec![],
            bets: Vec::<Bet>::new(),
            prize_map: BTreeMap::<String, u64>::new(),
            // p[0] sb, p[1] bb, p[2] 1st to act, p[n-1] (last) btn; when 2 players, btn == sb
            players: Vec::<Player>::new(),
            pots: Vec::<Pot>::new(),
            acting_player: None,
        })
    }

    fn handle_event(&mut self, context: &mut Effect, event: Event) -> Result<()> {
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
                let player_num = self.players.len();
                let rnd_spec = RandomSpec::deck_of_cards();
                self.deck_random_id = context.init_random_state(rnd_spec);

                // Initializing the game state
                // self.stage = HoldemStage::Play;
                println!("There are {} players in game", context.count_players());
                self.btn = player_num - 1;
                self.street = Street::Preflop;
                Ok(())
            }

            Event::Sync { new_players, .. } => {
                // TODO
                for p in new_players.iter() {
                    // TODO: balance == chips?
                    let player = Player::new(p.addr.clone(), p.balance, p.position);
                    // TODO: Check diff of p.position and the current positions of game
                    self.seats_map.insert(player.addr.clone(), player.position);
                    self.players.push(player);
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
                    .bets
                    .get(player_pos)
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
                let addr0 = self.players[0].addr.clone();
                let addr1 = self.players[1].addr.clone();
                context.assign(self.deck_random_id, addr0, vec![0, 1]);
                context.assign(self.deck_random_id, addr1, vec![2, 3]);
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
