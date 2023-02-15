#![allow(unused_variables)]               // Remove these two later
#![allow(warnings)]

use std::collections::{HashMap, HashSet};

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use race_core::{
    context::GameContext,
    engine::GameHandler,
    error::{Error, Result},
    event::{CustomEvent, Event},
    random::deck_of_cards,
    types::{GameAccount, RandomId, Settle},
};
use race_proc_macro::game_handler;

pub mod evaluator;

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
    Wait, // or Idle?
    Acted,
    Acting, // or InAction?
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
        self.addr == other.addr
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

    pub fn chips(&self) -> u64 {
        self.chips
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
    // 1. Indicate whether the sb/bb goes all in due to short of chips
    // 2. Indicate the real bet (<= sb/bb)
    // 3. Update the player's chips in place
    // TODO: return an indicator to show if the player goes all in
    pub fn take_bet(&mut self, bet: u64) -> u64 {
        if bet < self.chips {
            self.chips -= bet;
            self.status = PlayerStatus::Acted;
            bet // real bet
        } else {
            let chips_left = self.chips;
            self.status = PlayerStatus::Allin;
            self.chips = 0;
            chips_left // real bet
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

#[derive(Deserialize, Serialize, PartialEq, Debug, Default, Clone)]
pub enum Street {
    #[default]
    Init,
    Preflop,
    Flop,
    Turn,
    River,
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

    pub fn amount(&self) -> u64 {
        self.amount
    }

    pub fn owner(&self) -> String {
        self.owner.clone()
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
    pub buyin: u64,
    pub btn: usize, // current btn position, zero-indexed?
    pub rake: f32,
    pub size: u8, // table size: total number of players
    pub stage: HoldemStage,
    // mode: HoldeMode,        // game type: cash, sng or tourney? Treat as CASH GAME
    // token: String,          // token should be a struct of its own?
    pub street: Street,
    // TODO: add min_raise
    pub street_bet: u64,
    pub seats_map: HashMap<String, usize>,
    // pub community_cards: &'a str,
    pub bets: Vec<Bet>, // each bet's index maps to player's pos (index) in the below player list
    pub prize_map: HashMap<String, u64>,
    pub players: Vec<Player>,
    pub acting_player: Option<Player>,
    pub pots: Vec<Pot>, // 1st main pot + rest side pot(s)
}

// Methods related to Game and often with side effects
impl Holdem {
    fn next_action_player(&mut self, players_toact: Vec<&Player>) -> Option<String> {
        for p in players_toact {
            if let Some(bet) = self.bets.get(p.position) {
                println!("Bet amount {:?}", bet.amount);
                println!("Street bet {:?}", self.street_bet);
                if bet.amount < self.street_bet || p.status == PlayerStatus::Wait {
                    return Some(p.addr.clone());
                }
            } else if p.status == PlayerStatus::Wait {
                return Some(p.addr.clone());
            }
        }
        None
    }

    pub fn get_player(&self, index: usize) -> Option<&Player> {
        self.players.get(index)
    }

    pub fn get_player_mut(&mut self, index: usize) -> Option<&mut Player> {
        self.players.get_mut(index)
    }

    pub fn get_seats_map(&self) -> HashMap<String, usize> {
        self.seats_map.clone()
    }

    pub fn get_player_position(&self, player_addr: &String) -> Option<usize> {
        if let Some(pos) = self.seats_map.get(player_addr) {
            Some(*pos)
        } else {
            None
        }
    }

    // Get Bet of Player at seat (index)
    pub fn get_player_bet(&self, index: usize) -> Bet {
        self.bets[index].clone()
    }

    fn update_streetbet(&mut self, new_sbet: u64) {
        self.street_bet = new_sbet;
    }

    fn update_bets(&mut self, new_bmap: Vec<Bet>) {
        self.bets.extend_from_slice(&new_bmap);
    }

    pub fn change_street(&mut self, context: &mut GameContext, street: Street) -> Result<()> {
        // Reset acted to wait
        for player in &mut self.players {
            if player.status == PlayerStatus::Acted {
                player.status = PlayerStatus::Wait;
            }
        }

        self.collect_bets()?;
        self.stage = HoldemStage::ShareKey;
        self.street = street;
        self.street_bet = 0;
        self.acting_player = None;

        let players_cnt = self.players.len() * 2;

        match self.street {
            Street::Flop => {
                context.reveal(self.deck_random_id,
                               (players_cnt..(players_cnt+3)).collect::<Vec<usize>>())?;
                Ok(())
            }
            _ => {
                Ok(())
            }
        }

    }

    pub fn next_street(&mut self) -> Street {
        match self.street {
            Street::Init => Street::Preflop,
            Street::Preflop => Street::Flop,
            Street::Flop => Street::Turn,

            Street::Turn => Street::River,
            _ => Street::Done,
        }
    }

    pub fn apply_prize(&mut self) -> Result<()> {
        for ply in &mut self.players {
            match self.prize_map.get(&ply.addr) {
                // If player in the prize map, chips increase by the amt in the map
                Some(prize) => {
                    println!("Found the player and applying prize to their chips ... ");
                    // ply.chips += *prize - &self.bets[ply.position].amount();
                    ply.chips += *prize;
                }
                // else chips decrease by the amt in the bet map?
                None => {
                    println!("Lost chips");
                    // player.take_bet() aleady reduced the chips
                    // ply.chips = ply.chips - &self.bets[ply.position].amount();
                }
            }
        }
        Ok(())
    }

    pub fn calc_prize(&mut self) -> Result<()> {
        let pots = &self.pots;
        let mut prize_map = HashMap::<String, u64>::new();
        for pot in pots {
            let cnt: u64 = pot.winners().len() as u64;
            let prize: u64 = pot.amount() / cnt;
            for wnr in pot.winners() {
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
                .owners()
                .iter()
                .filter(|w| winner_sets[idx].contains(w))
                .map(|w| (*w).clone())
                .collect();
            pot.update_winners(real_wnrs);
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
        let mut bets: Vec<u64> = self.bets.iter().map(|b| b.amount()).collect();
        bets.sort_by(|b1, b2| b1.cmp(b2));
        bets.dedup();

        let mut new_pots: Vec<Pot> = Vec::new();
        // This bet is the minimum or base among the owners of a pot
        let mut acc: u64 = 0;
        for bet in bets {
            let owners: Vec<String> = self
                .bets
                .iter()
                .filter(|b| b.amount() >= bet)
                .map(|b| b.owner())
                .collect();
            let amount = (bet - acc) * owners.len() as u64;
            // Pot with only 1 owner should return the bet in it to the owner
            if owners.len() == 1 {
                // TODO: replace `expect` method?
                let index = self
                    .get_player_position(&owners[0])
                    .expect("Player NOT found at table");
                if let Some(receiver) = self.get_player_mut(index) {
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

    pub fn ask_for_action(&mut self, player_addr: String, context: &mut GameContext) -> Result<()> {
        let player_pos: usize = self
            .seats_map
            .get(&player_addr)
            .expect("Player not found in game!")
            .clone();
        let player = &mut self.players[player_pos];
        player.status = PlayerStatus::Acting;
        self.acting_player = Some(player.clone());
        // Ask player for action within 30 secs
        context.action_timeout(player_addr, 30 * 1000);
        Ok(())
    }

    pub fn blind_bets(&mut self, context: &mut GameContext) -> Result<()> {
        // sb is btn when only two players
        if self.players.len() == 2 {
            // players.reverse();
            // let players = &mut self.players;
            // FIXME: Use slice
            {
            let players = &mut self.players;
                let bb_player = players.get_mut(0).unwrap();
                let real_bb = bb_player.take_bet(self.bb);
                println!("BB Player {:?}", bb_player);
                self.bets.push(
                    Bet::new(bb_player.addr.clone(), real_bb),
                );
            }
            {
            let players = &mut self.players;
                let sb_player = players.get_mut(1).unwrap();
                let real_sb = sb_player.take_bet(self.sb);
                println!("SB Player {:?}", sb_player);
                self.bets.push(
                    Bet::new(sb_player.addr.clone(), real_sb),
                );
            }
            self.ask_for_action("Bob".to_string(), context)?;


            // Next-to-act player is sb player
        } else {
            let mut sb_player = self.players[0].clone();
            let mut bb_player = self.players[1].clone();
            let real_sb = sb_player.take_bet(self.sb);
            let real_bb = bb_player.take_bet(self.bb);

            // Update bet map
            self.update_bets(vec![
                Bet::new(sb_player.addr.clone(), real_sb),
                Bet::new(bb_player.addr.clone(), real_bb),
            ]);

            // Select the next
            let mut rest_players = vec![];
            rest_players.extend_from_slice(&self.players[2..]);
            rest_players.extend_from_slice(&vec![sb_player, bb_player]);

            let action_players: Vec<&Player> =
                rest_players.iter().filter(|p| p.next_to_act()).collect();
            let action_player = action_players[0].addr.clone();

            // Ask next player to take act
            self.ask_for_action(action_player, context)?;
        }
        // Update street bet
        self.update_streetbet(self.bb);
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

    pub fn next_state(&mut self, context: &mut GameContext) -> Result<()> {
        let all_players = self.players.clone();
        let remain_players: Vec<&Player> = all_players.iter().filter(|p| p.to_remain()).collect();
        let players_toact: Vec<&Player> = all_players.iter().filter(|p| p.to_act()).collect();
        let allin_players: Vec<&Player> = remain_players
            .iter()
            .filter(|&p| p.status == PlayerStatus::Allin)
            .map(|p| *p)
            .collect();

        // let bet_map = self.bets.clone();
        let next_player = self.next_action_player(players_toact);
        let new_street = self.next_street();

        // Blind bets
        if self.street == Street::Preflop && self.bets.is_empty() {
            self.stage = HoldemStage::BlindBets;
            self.blind_bets(context)?;
            Ok(())
        }
        // Single player wins because there are one player only
        else if all_players.len() == 1 {
            let winner = all_players[0].addr.clone();
            println!("Next state: Only one player left and won the game!");
            self.single_player_win(&vec![vec![winner]])?;
            Ok(())
        }
        // Singple players wins because others all folded
        else if remain_players.len() == 1 {
            let winner = remain_players[0].addr.clone();
            println!(
                "Next state: All others folded and single winner is {}",
                winner.clone()
            );
            self.single_player_win(&vec![vec![winner]])?;
            Ok(())
        }
        // Next player to act
        else if next_player.is_some() {
            println!("Next state: Ask next player to act!");
            let next_action_player = next_player.unwrap();
            self.ask_for_action(next_action_player, context)?;
            Ok(())
        }
        // Runner
        else if self.stage != HoldemStage::Runner
            && remain_players.len() == allin_players.len() + 1
        {
            println!("Next state: Runner");
            todo!()
        }
        // Next Street
        else if new_street != Street::Done {
            println!("Next state: Move to next street");
            self.change_street(context, new_street)?;
            Ok(())
        }
        // Showdown
        else {
            todo!()
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
                // TODO: Log more detailed error info?
                let player_pos: usize = self
                    .seats_map
                    .get(&sender)
                    .expect("Player not found in game!")
                    .clone();
                // FIXME: update condition after refactoring bet_map
                if self.bets.get(player_pos).is_some() {
                    return Err(Error::Custom("Player cant bet!".to_string()));
                }

                // TODO: amount must be less than player's remained chips
                if self.street_bet > amount {
                    return Err(Error::Custom("Player's bet is too small".to_string()));
                }

                if let Some(player) = self.players.get_mut(player_pos) {
                    let player_bet = player.take_bet(amount);
                    self.bets.push(Bet::new(player.addr.clone(), player_bet));
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
                        let real_bet = player.take_bet(call_amount);
                        println!("After calls Player: {:?}", player);
                        betted.amount += real_bet;
                        self.next_state(context)?;
                        Ok(())
                    } else {
                        let call_amount = self.street_bet - 0;
                        let real_bet = player.take_bet(call_amount);
                        self.bets.push(Bet::new(player.addr.clone(), real_bet));
                        self.next_state(context)?;
                        Ok(())
                    }
                } else {
                    return Err(Error::Custom("Player not found in game".to_string()));
                }
            }

            GameEvent::Check => {
                let player_pos: usize = self
                    .seats_map
                    .get(&sender)
                    .expect("Player not found in game!")
                    .clone();
                if let Some(player_bet) = self.bets.get(player_pos) {
                    if self.street_bet != player_bet.amount {
                        return Err(Error::Custom("Player cant check!".to_string()));
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

            GameEvent::Fold => {
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

                // single player wins: only one player left
                // if self.players.len() == 1 {
                //     todo!()
                // }
                // single player wins: others all folded

                // next player to act

                // runner
            }

            GameEvent::Raise(amount) => {
                todo!()
            }
        }
    }
}

impl GameHandler for Holdem {
    fn init_state(context: &mut GameContext, _init_account: GameAccount) -> Result<Self> {
        // Skip this account part for now
        // let account = HoldemAccount::try_from_slice(&init_account.data).unwrap();
        Ok(Self {
            deck_random_id: 0,
            dealer_idx: 0,
            sb: 10,
            bb: 20,
            buyin: 400,
            btn: context.get_players().len(),
            size: 6,
            rake: 0.2,
            stage: HoldemStage::Init,
            street: Street::Init,
            street_bet: 0,
            seats_map: HashMap::<String, usize>::new(),
            // community_cards: vec![],
            bets: Vec::<Bet>::new(), // p[0] sb, p[1] bb, p[2] the first to act, p[n-1] (last) btn
            prize_map: HashMap::<String, u64>::new(),
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
                // TODO: count servers/players
                if context.get_players().len() < 2 {
                    context.wait_timeout(10_000);
                } else {
                    let rnd_spec = deck_of_cards();
                    self.deck_random_id = context.init_random_state(&rnd_spec)?;

                    // Initializing the game state
                    // self.stage = HoldemStage::Play;
                    self.street = Street::Preflop;
                }
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

                if context.count_players() >= 2 {
                    context.start_game();
                }
                Ok(())
            }

            Event::ActionTimeout { player_addr } => {
                // TODO: Handle player's default preference (always check)?
                if let Some(index) = self.get_player_position(&player_addr) {
                    let player_bet = self.get_player_bet(index);
                    let street_bet = self.street_bet;
                    if let Some(player) = self.get_player_mut(index) {
                        if player.addr.clone() == player_bet.owner() {
                            if player_bet.amount() == street_bet {
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
                } else {
                    Err(Error::Custom("Player not found in game!".to_string()))
                }
            }

            Event::RandomnessReady { .. } => {
                let addr0 = context.get_player_by_index(0).unwrap().addr.clone();
                let addr1 = context.get_player_by_index(1).unwrap().addr.clone();
                context.assign(self.deck_random_id, addr0, vec![0, 1])?;
                context.assign(self.deck_random_id, addr1, vec![2, 3])?;
                Ok(())
            }

            Event::SecretsReady => match self.stage {
                HoldemStage::Init => {
                    self.next_state(context)?;
                    Ok(())
                }

                // HoldemStage::Showdown | HoldemStage::Runner => {
                //     let decryption = context.get_revealed(self.deck_random_id)?;
                //     let player_idx: usize = if self.dealer_idx == 0 { 1 } else { 0 };
                //     let dealer_addr = context
                //         .get_player_by_index(self.dealer_idx)
                //         .unwrap()
                //         .addr
                //         .clone();
                //     let player_addr = context
                //         .get_player_by_index(player_idx)
                //         .unwrap()
                //         .addr
                //         .clone();
                //     let dealer_card = decryption.get(&self.dealer_idx).unwrap();
                //     let player_card = decryption.get(&player_idx).unwrap();
                //     let (winner, loser) = if is_better_than(dealer_card, player_card) {
                //         (dealer_addr, player_addr)
                //     } else {
                //         (player_addr, dealer_addr)
                //     };
                //     context.settle(vec![
                //         Settle::add(winner, self.bet),
                //         Settle::sub(loser, self.bet),
                //         ]);
                //     },
                // _ => {}
                _ => {
                    self.next_state(context)?;
                    Ok(())
                },
            }
            _ => Ok(())
        }
    }
}
