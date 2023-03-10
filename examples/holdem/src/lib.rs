#![allow(unused_imports)]
use borsh::{BorshDeserialize, BorshSerialize};
use race_core::prelude::*;
use race_proc_macro::game_handler;
use serde::{Deserialize, Serialize};
// use core::num::flt2dec::decoder;
use std::cmp::Ordering;
use std::collections::BTreeMap;

pub mod evaluator;
use evaluator::{compare_hands, create_cards, evaluate_cards};

// #[macro_use]
// extern crate log;

#[cfg(test)]
mod holdem_tests;
// #[cfg(test)]
// mod integration_test;

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

    // This fn indicates
    // 1. whether player goes all in
    // 2. the real bet
    // 3. the player's remaining chips after the bet
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

type ActingPlayer = (String, usize);

#[derive(Deserialize, Serialize, PartialEq, Default, Clone, Debug)]
pub struct Pot {
    pub owners: Vec<String>,
    pub winners: Vec<String>,
    pub amount: u64,
}

impl Pot {
    pub fn new() -> Self {
        Self {
            owners: Vec::<String>::new(),
            winners: Vec::<String>::new(),
            amount: 0,
        }
    }

    pub fn merge(&mut self, other: &Pot) -> Result<()> {
        self.amount += other.amount;
        Ok(())
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
    pub fn new(owner: String, amount: u64) -> Self {
        Self { owner, amount }
    }
}

#[derive(Default, Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum HoldemStage {
    #[default]
    Init,
    // Encrypt,
    ShareKey,
    Play,
    Runner,
    Settle,
    Showdown,
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
    pub seats_map: BTreeMap<String, usize>,
    pub board: Vec<String>,
    pub bet_map: BTreeMap<String, Bet>,
    pub prize_map: BTreeMap<String, u64>,
    // A map of players in the order of their init positions
    pub player_map: BTreeMap<String, Player>,
    pub players: Vec<String>,
    // TODO: make this into Option<(String, position)> | type ActingPlayer = (String, usize)
    pub acting_player: Option<ActingPlayer>, // Acting player's ID or address
    pub pots: Vec<Pot>,                      // 1st main pot + rest side pot(s)
}

impl Holdem {
    fn next_action_player(&mut self, players_toact: Vec<&String>) -> Option<String> {
        for addr in players_toact {
            let player = self.player_map.get(addr).unwrap();
            if let Some(bet) = self.bet_map.get(addr) {
                if bet.amount < self.street_bet || player.status == PlayerStatus::Wait {
                    println!("== {}' current bet amount {:?} ==", addr, bet.amount);
                    println!("== Current street bet: {} ==", self.street_bet);
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
            Some((addr, _)) => addr == player_addr,
            None => false,
        }
    }

    // Either the position of acting player or btn
    pub fn get_ref_positon(&self) -> usize {
        if let Some((_, position)) = self.acting_player {
            position
        } else {
            self.btn
        }
    }

    pub fn ask_for_action(&mut self, player_addr: String, context: &mut Effect) -> Result<()> {
        if let Some(player) = self.player_map.get_mut(&player_addr) {
            println!("== Asking {} to act ==", player.addr);
            player.status = PlayerStatus::Acting;
            self.acting_player = Some((player.addr.clone(), player.position));
            // Ask player for action within 30 secs
            context.action_timeout(player_addr, 30_000);
            Ok(())
        } else {
            return Err(Error::Custom("Next player not found in game!".to_string()));
        }
    }

    // Place players in the order of sb, bb, 1st-to-act, 2nd-to-act, ..., btn
    pub fn arrange_players(&mut self, last_pos: usize) -> Result<()> {
        // Players sit clockwise,
        let mut player_pos: Vec<(String, usize)> = self
            .player_map
            .values()
            .map(|p| {
                if p.position > last_pos {
                    (p.addr.clone(), p.position - last_pos)
                } else {
                    (p.addr.clone(), p.position + 100)
                }
            })
            .collect();
        player_pos.sort_by(|(_, pos1), (_, pos2)| pos1.cmp(pos2));
        let player_order: Vec<String> = player_pos.iter().map(|(addr, _)| addr.clone()).collect();
        // println!("== BTN is {} ==", self.btn);
        println!("== Player order {:?} ==", player_order);
        self.players = player_order;
        Ok(())
    }

    pub fn blind_bets(&mut self, context: &mut Effect) -> Result<()> {
        let mut players: Vec<String> = self.players.clone();
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
                println!("== SB Player is {:?} ==", sb_player);
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
                println!("== BB Player is {:?} ==", bb_player);
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
                let action_players: Vec<&String> = players
                    .iter()
                    .filter(|addr| {
                        if let Some(player) = self.player_map.get_mut(*addr) {
                            player.next_to_act()
                        } else {
                            false
                        }
                    })
                    .collect();
                let player_to_act = action_players.first().unwrap();
                self.ask_for_action((*player_to_act).clone(), context)?;
            }
        }
        // self.players = players;
        self.min_raise = self.bb;
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

        // Build new pots
        let mut new_pots: Vec<Pot> = Vec::new();
        let mut acc: u64 = 0;
        for bet in bets {
            let owners: Vec<String> = self
                .bet_map
                .iter()
                .filter(|(_, b)| b.amount >= bet)
                .map(|(_, b)| b.owner.clone())
                .collect();
            let amount = (bet - acc) * owners.len() as u64;
            // Pot with only 1 owner should return the bet in it to the owner
            if owners.len() == 1 {
                if let Some(receiver) = self.player_map.get_mut(&owners[0]) {
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

        // Update old pots
        if self.pots.len() == 0 {
            // First time to collect bets to pots
            self.pots = new_pots;
        } else {
            // Merge pots with same (num of) owners
            let mut diff_pots = Vec::<Pot>::new();
            for pot in self.pots.iter_mut() {
                for npot in new_pots.iter() {
                    if pot.owners.len() == npot.owners.len() {
                        pot.merge(npot)?;
                    } else {
                        diff_pots.push(npot.clone())
                    }
                }
            }
            self.pots.extend_from_slice(&diff_pots);
        }
        self.bet_map = BTreeMap::<String, Bet>::new();
        Ok(())
    }

    pub fn change_street(&mut self, context: &mut Effect, new_street: Street) -> Result<()> {
        // Reset acted to wait
        for player in self.player_map.values_mut() {
            if player.status == PlayerStatus::Acted {
                player.status = PlayerStatus::Wait;
            }
        }
        self.collect_bets()?;
        // self.stage = HoldemStage::Play;
        self.street = new_street;
        println!("Street now is {:?}", self.street);
        self.min_raise = self.bb;
        self.street_bet = 0;
        self.acting_player = None;

        let players_cnt = self.players.len() * 2;
        match self.street {
            Street::Flop => {
                println!("Reveal cards for street {:?}", self.street);
                context.reveal(
                    self.deck_random_id,
                    (players_cnt..(players_cnt + 3)).collect::<Vec<usize>>(),
                );
                self.stage = HoldemStage::ShareKey;
                Ok(())
            }

            Street::Turn => {
                context.reveal(
                    self.deck_random_id,
                    (players_cnt..(players_cnt + 4)).collect::<Vec<usize>>(),
                );
                self.stage = HoldemStage::ShareKey;
                Ok(())
            }

            Street::River => {
                context.reveal(
                    self.deck_random_id,
                    (players_cnt..(players_cnt + 5)).collect::<Vec<usize>>(),
                );
                self.stage = HoldemStage::ShareKey;
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
        for ply in self.player_map.values_mut() {
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
    pub fn next_state(&mut self, context: &mut Effect) -> Result<()> {
        // Or self.players.rotate_left(self.btn - 1);
        let last_pos = self.get_ref_positon();
        self.arrange_players(last_pos)?;
        let all_players = self.players.clone();
        let remained_players: Vec<&String> = all_players
            .iter()
            .filter(|addr| {
                if let Some(player) = self.player_map.get(*addr) {
                    player.to_remain()
                } else {
                    false
                }
            })
            .collect();

        let mut toact_players: Vec<&String> = all_players
            .iter()
            .filter(|addr| {
                if let Some(player) = self.player_map.get(*addr) {
                    player.to_act()
                } else {
                    false
                }
            })
            .collect();

        let allin_players: Vec<&String> = remained_players
            .iter()
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

        println!("== Current GAME BETS are {:?} ==", self.bet_map);

        // Blind bets
        if self.street == Street::Preflop && self.bet_map.is_empty() {
            println!("[Next State]: Blind bets");
            // self.stage = HoldemStage::BlindBets;
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
            let winner = remained_players.first().unwrap();
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
            for (idx, player) in self.player_map.values().enumerate() {
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
            self.collect_bets()?;

            // Collect players' hands
            let board: Vec<&str> = self.board.iter().map(|c| c.as_str()).collect();
            let mut player_hands: Vec<(String, evaluator::PlayerHand)> =
                Vec::with_capacity(self.players.len());
            let decryption = context.get_revealed(self.deck_random_id)?;
            for player in self.player_map.values() {
                if player.status != PlayerStatus::Fold {
                    let hole_cards = [
                        decryption.get(&player.position).unwrap().as_str(),
                        decryption.get(&(player.position + 1)).unwrap().as_str(),
                    ];
                    let cards = evaluator::create_cards(board.as_slice(), &hole_cards);
                    let hand = evaluate_cards(cards);
                    player_hands.push((player.addr.clone(), hand));
                }
            }
            // Compare players' hands
            // From low to high
            player_hands.sort_by(|(_, h1), (_, h2)| evaluator::compare_hands(&h1.value, &h2.value));
            println!("Player Hands in ascending order {:?}", player_hands);

            let (loser, lowest_hand) = player_hands.first().unwrap();
            let mut winners: Vec<Vec<String>> = Vec::new();
            let mut draw_players = Vec::<String>::new();
            // each hand is either equal to or higher than the loser hand
            for (player, hand) in player_hands.iter().skip(1) {
                if lowest_hand.value.iter().eq(hand.value.iter()) {
                    draw_players.push(player.clone());
                    draw_players.push(loser.clone());
                } else {
                    // player's hand higher than loser's
                    winners.push(vec![player.clone()]);
                }
            }
            // Move the strongest to 1st, second strongest 2nd, and so on
            winners.reverse();
            // Add equal-hand players if there are
            if draw_players.len() > 0 {
                winners.extend_from_slice(&vec![draw_players]);
            }
            // Append loser to the last
            winners.push(vec![loser.clone()]);
            println!("Winners in order are {:?}", winners);
            Ok(())
        }
    }

    fn handle_custom_event(
        &mut self,
        context: &mut Effect,
        event: GameEvent,
        sender: String,
    ) -> Result<()> {
        match event {
            GameEvent::Bet(amount) => {
                if !self.is_acting_player(&sender) {
                    return Err(Error::Custom(
                        "Player is NOT the acting player so can't raise!".to_string(),
                    ));
                }
                if self.bet_map.get(&sender).is_some() {
                    return Err(Error::Custom("Player already betted!".to_string()));
                }
                // Freestyle betting not allowed in the preflop
                if self.street_bet != 0 {
                    return Err(Error::Custom("Player can't bet!".to_string()));
                }
                if self.bb > amount {
                    return Err(Error::Custom("Bet must be >= bb".to_string()));
                }
                // TODO: amount must be less than player's remained chips
                if let Some(player) = self.player_map.get_mut(&sender) {
                    let (allin, real_bet) = player.take_bet(amount);
                    self.bet_map
                        .insert(player.addr.clone(), Bet::new(player.addr.clone(), real_bet));
                    player.status = if allin {
                        PlayerStatus::Allin
                    } else {
                        PlayerStatus::Acted
                    };
                    self.min_raise = amount;
                    self.street_bet = amount;
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
                if let Some(player) = self.player_map.get_mut(&sender) {
                    // player betted but need to increase the bet by calling
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
                        // player's first time to call in this street
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
                match self.street {
                    Street::Preflop => {
                        if let Some(player_bet) = self.bet_map.get(&sender) {
                            if self.street_bet != player_bet.amount {
                                return Err(Error::Custom("Player can't check!".to_string()));
                            }

                            if let Some(player) = self.player_map.get_mut(&sender) {
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
                            let player = self.player_map.get_mut(&sender).unwrap();
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
                if amount < self.street_bet + self.min_raise {
                    return Err(Error::Custom("Player raises too small".to_string()));
                }
                // TODO: handle raise too small
                if let Some(player) = self.player_map.get_mut(&sender) {
                    if let Some(betted) = self.bet_map.get_mut(&sender) {
                        let added_bet = amount - betted.amount;
                        let (allin, real_bet) = player.take_bet(added_bet);
                        let new_street_bet = betted.amount + real_bet;
                        let new_min_raise = new_street_bet - self.street_bet;
                        self.street_bet = new_street_bet;
                        self.min_raise = new_min_raise;
                        // self.min_raise =
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
                    } else {
                        let added_bet = amount - 0;
                        let (allin, real_bet) = player.take_bet(added_bet);
                        let new_min_raise = real_bet - self.street_bet;
                        self.street_bet = real_bet;
                        self.min_raise = new_min_raise;
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
                    }
                    Ok(())
                } else {
                    return Err(Error::Custom("Player not found in game!".to_string()));
                }
            }
        }
    }
}

impl GameHandler for Holdem {
    fn init_state(context: &mut Effect, _init_account: InitAccount) -> Result<Self> {
        // TODO: Use GameAccount to initialize the Game State
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
            seats_map: BTreeMap::<String, usize>::new(),
            board: Vec::<String>::with_capacity(5),
            bet_map: BTreeMap::<String, Bet>::new(),
            prize_map: BTreeMap::<String, u64>::new(),
            player_map: BTreeMap::<String, Player>::new(),
            // p[0] sb, p[1] bb, p[2] 1st to act, p[n-1] (last) btn; when 2 players, btn == sb
            players: Vec::<String>::new(),
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
                // Initializing the game state
                self.street = Street::Init;
                self.stage = HoldemStage::Play;
                let player_num = self.player_map.len();
                self.btn = player_num - 1;
                println!("== There are {} players in game ==", player_num);
                // Arrange action order for players
                self.arrange_players(self.btn)?;

                // Get the randomness (shuffled and dealt cards) ready
                let rnd_spec = RandomSpec::deck_of_cards();
                self.deck_random_id = context.init_random_state(rnd_spec);

                Ok(())
            }

            Event::Sync { new_players, .. } => {
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
                let player_bet = self
                    .bet_map
                    .get(&player_addr)
                    .expect("No bet found for player!")
                    .clone();
                let street_bet = self.street_bet;

                if let Some(player) = self.player_map.get_mut(&player_addr) {
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

            // random_id: RandomId
            Event::RandomnessReady { ref random_id } => {
                self.deck_random_id = *random_id;
                let mut idx: usize = 0;
                for (idx, (addr, player)) in self.player_map.iter().enumerate() {
                    context.assign(self.deck_random_id, addr, vec![idx * 2, idx * 2 + 1]);
                }
                Ok(())
            }

            // Whenever context updated, ShareSecrets and SecretsReady will be dispatched
            // Then the program falls into this variant arm
            // Before handling SecretsReady, the game is halted for a short while
            Event::SecretsReady => match self.stage {
                // SecretsReady dispatched because of revealing cards, restore the game
                HoldemStage::ShareKey => {
                    self.stage = HoldemStage::Play;

                    // After revealing hole cards, makes the game enter the preflop
                    match self.street {
                        Street::Init => {
                            let decryption = context.get_revealed(self.deck_random_id)?;
                            for (idx, player) in self.players.iter().enumerate() {
                                // let hole_cards = vec![
                                //     decryption.get(&idx).unwrap(),
                                //     decryption.get(&(idx + 1)).unwrap(),
                                // ];
                                let base = idx * 2;
                                println!(
                                    "{} got hole cards [{}, {}]",
                                    player,
                                    decryption.get(&base).unwrap(),
                                    decryption.get(&(base + 1)).unwrap(),
                                );
                            }

                            self.street = Street::Preflop;
                            self.next_state(context)?;
                        }

                        // Street::Preflop  => {
                        //     let decryption = context.get_revealed(self.deck_random_id)?;
                        //     for (idx, player) in self.players.iter().enumerate() {
                        //         // let hole_cards = vec![
                        //         //     decryption.get(&idx).unwrap(),
                        //         //     decryption.get(&(idx + 1)).unwrap(),
                        //         // ];
                        //
                        //         println!("{} got hole cards [{}, {}]",
                        //                  player,
                        //                  decryption.get(&idx).unwrap(),
                        //                  decryption.get(&(idx + 1)).unwrap(),
                        //         );
                        //     }
                        //
                        //     self.next_state(context)?;
                        // }
                        Street::Flop => {
                            let players_cnt = self.players.len() * 2;
                            let decryption = context.get_revealed(self.deck_random_id)?;
                            let mut board = Vec::<String>::with_capacity(3);
                            for i in players_cnt..(players_cnt + 3) {
                                let card = decryption.get(&i).unwrap().clone();
                                board.push(card);
                            }
                            self.board = board;
                            println!("== Board is {:?} ==", self.board);

                            self.next_state(context)?;
                        }

                        Street::Turn => {
                            let decryption = context.get_revealed(self.deck_random_id)?;
                            let card_index = self.players.len() * 2 + 3;
                            let card = decryption.get(&card_index).unwrap().clone();
                            self.board.push(card);
                            println!("== Board is {:?} ==", self.board);

                            self.next_state(context)?;
                        }

                        Street::River => {
                            let decryption = context.get_revealed(self.deck_random_id)?;
                            let card_index = self.players.len() * 2 + 4;
                            let card = decryption.get(&card_index).unwrap().clone();
                            self.board.push(card);
                            println!("== Board is {:?} ==", self.board);

                            self.next_state(context)?;
                        }

                        _ => {}
                    }
                    Ok(())
                }

                HoldemStage::Play => {
                    match self.street {
                        Street::Init => {
                            // Reveal players hole cards
                            let end = self.players.len() * 2;
                            context.reveal(self.deck_random_id, (0..end).collect::<Vec<usize>>());
                            self.stage = HoldemStage::ShareKey;
                            Ok(())
                        }

                        // let (winner, loser) = if is_better_than(dealer_card, player_card) {
                        //     (dealer_addr, player_addr)
                        // } else {
                        //     (player_addr, dealer_addr)
                        // };
                        // context.settle(vec![
                        //     Settle::add(winner, self.bet),
                        //     Settle::sub(loser, self.bet),
                        //     ]);
                        _ => Ok(()),
                    }
                }

                // Other Holdem Stages
                _ => Ok(()),
            },

            // Other events
            _ => Ok(()),
        }
    }
}
