//! Game state machine (or handler) of Holdem: the core of this lib.
use race_core::prelude::*;
use race_proc_macro::game_handler;
use std::collections::BTreeMap;

use crate::essential::{
    ActingPlayer, Bet, GameEvent, HoldemAccount, HoldemStage, Player, PlayerStatus, Pot, Street,
    ACTION_TIMEOUT, WAIT_TIMEOUT,
};
use crate::evaluator::{compare_hands, create_cards, evaluate_cards, PlayerHand};

// Holdem: the game state
#[cfg_attr(test, derive(Debug, PartialEq))]
#[game_handler]
#[derive(BorshSerialize, BorshDeserialize, Clone, Default)]
pub struct Holdem {
    pub deck_random_id: RandomId,
    pub sb: u64,
    pub bb: u64,
    pub min_raise: u64,
    pub btn: usize,
    pub rake: u16,
    pub stage: HoldemStage,
    pub street: Street,
    pub street_bet: u64,
    pub board: Vec<String>,
    pub hand_index_map: BTreeMap<String, Vec<usize>>,
    pub bet_map: BTreeMap<String, Bet>,
    pub prize_map: BTreeMap<String, u64>,
    pub player_map: BTreeMap<String, Player>,
    pub players: Vec<String>, // up-to-date player order
    pub acting_player: Option<ActingPlayer>,
    pub pots: Vec<Pot>,
}

// Methods that mutate or query the game state
impl Holdem {
    fn reset_holdem_state(&mut self) -> Result<(), HandleError> {
        self.street = Street::Init;
        self.stage = HoldemStage::Init;
        self.street_bet = 0;
        self.min_raise = 0;
        self.acting_player = None;
        self.pots = Vec::<Pot>::new();
        self.board = Vec::<String>::with_capacity(5);
        self.hand_index_map = BTreeMap::<String, Vec<usize>>::new();
        self.bet_map = BTreeMap::<String, Bet>::new();
        self.prize_map = BTreeMap::<String, u64>::new();

        Ok(())
    }

    fn next_action_player(&mut self, players_toact: Vec<&String>) -> Option<String> {
        for addr in players_toact {
            if let Some(player) = self.player_map.get(addr) {
                if let Some(bet) = self.bet_map.get(addr) {
                    if bet.amount < self.street_bet || player.status == PlayerStatus::Wait {
                        println!("== {}' current bet amount {}", addr, bet.amount);
                        println!("== Current street bet: {}", self.street_bet);
                        return Some(addr.clone());
                    }
                } else if player.status == PlayerStatus::Wait {
                    return Some(addr.clone());
                }
            };
        }
        None
    }

    pub fn is_acting_player(&self, player_addr: &String) -> bool {
        match &self.acting_player {
            Some((addr, _)) => addr == player_addr,
            None => false,
        }
    }

    // Return either acting player position or btn for reference
    pub fn get_ref_position(&self) -> usize {
        if let Some((_, position)) = self.acting_player {
            position
        } else {
            self.btn
        }
    }

    // BTN moves clockwise.  The next BTN is calculated base on the current one
    pub fn get_next_btn(&mut self) -> Result<usize, HandleError> {
        let ref_pos = if self.btn == self.players.len() - 1 {
            0
        } else {
            self.btn + 1
        };

        let next_positions: Vec<usize> = self
            .player_map
            .values()
            .filter(|p| p.position >= ref_pos)
            .map(|p| p.position)
            .collect();

        if next_positions.is_empty() {
            if let Some((_, first_player)) = self.player_map.first_key_value() {
                Ok(first_player.position)
            } else {
                return Err(HandleError::Custom(
                    "Failed to find a player for the next button".to_string(),
                ));
            }
        } else {
            if let Some(pos) = next_positions.first() {
                Ok(*pos)
            } else {
                return Err(HandleError::Custom(
                    "Failed to find a proper position for the next button".to_string(),
                ));
            }
        }
    }

    pub fn ask_for_action(
        &mut self,
        player_addr: String,
        effect: &mut Effect,
    ) -> Result<(), HandleError> {
        if let Some(player) = self.player_map.get_mut(&player_addr) {
            println!("== Asking {} to act", player.addr);
            player.status = PlayerStatus::Acting;
            self.acting_player = Some((player.addr.clone(), player.position));
            effect.action_timeout(player_addr, ACTION_TIMEOUT); // in secs
            Ok(())
        } else {
            return Err(HandleError::Custom(
                "Next player not found in game".to_string(),
            ));
        }
    }

    /// Place players (sitting clockwise) in the following order:
    /// SB, BB, UTG (1st-to-act), MID (2nd-to-act), ..., BTN (last-to-act).
    pub fn arrange_players(&mut self, last_pos: usize) -> Result<(), HandleError> {
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
        // println!("== BTN is {}", self.btn);
        println!("== Player order {:?}", player_order);
        self.players = player_order;
        Ok(())
    }

    pub fn blind_bets(&mut self, effect: &mut Effect) -> Result<(), HandleError> {
        let mut players: Vec<String> = self.players.clone();
        if players.len() == 2 {
            players.reverse();
            // Take bet from BB
            if let Some(bb_addr) = players.first() {
                if let Some(bb_player) = self.player_map.get_mut(bb_addr) {
                    let (allin, real_bb) = bb_player.take_bet(self.bb);
                    self.bet_map
                        .insert(bb_addr.clone(), Bet::new(bb_addr.clone(), real_bb));
                    if allin {
                        bb_player.status = PlayerStatus::Allin;
                    }
                }
            } else {
                return Err(HandleError::Custom(
                    "No first player found in 2 players for big blind".to_string(),
                ));
            }

            // Take bet from SB (btn)
            if let Some(sb_addr) = players.last() {
                if let Some(sb_player) = self.player_map.get_mut(sb_addr) {
                    println!("== SB Player is {:?}", sb_player);
                    let (allin, real_sb) = sb_player.take_bet(self.sb);
                    self.bet_map
                        .insert(sb_addr.clone(), Bet::new(sb_addr.clone(), real_sb));
                    if allin {
                        sb_player.status = PlayerStatus::Allin;
                    }
                    // SB acts first
                    self.ask_for_action(sb_addr.clone(), effect)?;
                }
            } else {
                return Err(HandleError::Custom(
                    "No second player found in 2 players for small blind".to_string(),
                ));
            }
        } else {
            // Take bet from SB (1st in Player vec)
            if let Some(addr) = players.get(0) {
                if let Some(sb_player) = self.player_map.get_mut(addr) {
                    println!("== SB Player is {:?}", sb_player);
                    let (allin, real_sb) = sb_player.take_bet(self.sb);
                    if allin {
                        sb_player.status = PlayerStatus::Allin;
                    }
                    self.bet_map.insert(
                        sb_player.addr.clone(),
                        Bet::new(sb_player.addr.clone(), real_sb),
                    );
                }
            } else {
                return Err(HandleError::Custom(
                    "No first player found in multi players for small blind".to_string(),
                ));
            };

            // Take bet from BB (2nd in Player vec)
            if let Some(addr) = players.get(1) {
                if let Some(bb_player) = self.player_map.get_mut(addr) {
                    println!("== BB Player is {:?}", bb_player);
                    let (allin, real_bb) = bb_player.take_bet(self.bb);
                    if allin {
                        bb_player.status = PlayerStatus::Allin;
                    }
                    self.bet_map.insert(
                        bb_player.addr.clone(),
                        Bet::new(bb_player.addr.clone(), real_bb),
                    );
                }
            } else {
                return Err(HandleError::Custom(
                    "No second player found in multi players for big blind".to_string(),
                ));
            }

            // Select next to act

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

            match action_players.first() {
                Some(player_to_act) => {
                    self.ask_for_action((*player_to_act).clone(), effect)?;
                }
                None => {
                    return Err(HandleError::Custom(
                        "Failed to find the next player to act".to_string(),
                    ));
                }
            }
        }
        self.min_raise = self.bb;
        self.street_bet = self.bb;
        Ok(())
    }

    /// Handle main pot and side pot(s), for example:
    /// Players A(100), B(45), C(45), D(50) call or go all in, then the pots become
    /// Main:  { amount: 45*4, owners: [A, B, C, D], winners [] }
    /// Side1: { amount: 5*2,  owners: [A, D], winners [] }
    /// Side2: { amount: 50,   owners: [A], winners [] } <-- should return bet to A
    ///
    /// Note: in reality, if two consecutive streets have different numbers of players,
    /// there will also be multiple pots.  For example, 4 players bet in preflop, and
    /// one of them folds in flop. As a result, there will be pots of preflop and flop.
    pub fn collect_bets(&mut self) -> Result<(), HandleError> {
        // Filter bets: arrange from small to big and remove duplicates
        let mut bets: Vec<u64> = self.bet_map.iter().map(|(_, b)| b.amount).collect();
        bets.sort_by(|b1, b2| b1.cmp(b2));
        bets.dedup();
        println!(
            "== In Street {:?} and Bets: {:?}",
            self.street, self.bet_map
        );

        let mut new_pots = Vec::<Pot>::new();
        let mut acc: u64 = 0;
        for bet in bets {
            let owners: Vec<String> = self
                .bet_map
                .iter()
                .filter(|(_, b)| b.amount >= bet)
                .map(|(_, b)| b.owner.clone())
                .collect();
            let actual_bet = bet - acc;
            let amount = actual_bet * owners.len() as u64;
            // Pot with only 1 owner should return the bet in it to the owner
            if owners.len() == 1 {
                let Some(owner) = owners.first() else {
                    return Err(HandleError::Custom(
                        "Failed to get the only owner".to_string()
                    ));
                };
                let Some(receiver) = self.player_map.get_mut(owner) else {
                    return Err(HandleError::Custom(
                        "Failed to find the owner in player map".to_string()
                    ));
                };
                receiver.chips += amount;
                continue;
            } else {
                new_pots.push(Pot {
                    owners,
                    winners: Vec::<String>::new(),
                    amount,
                });
                acc += actual_bet;
            }
        }

        // Merge pots with same (num of) owners
        if self.pots.len() == 0 {
            self.pots = new_pots;
        } else {
            let mut diff_pots = Vec::<Pot>::new();
            if let Some(last_pot) = self.pots.last_mut() {
                for npot in new_pots.iter() {
                    if npot.owners.len() == last_pot.owners.len() {
                        last_pot.merge(npot)?;
                    } else {
                        diff_pots.push(npot.clone());
                    }
                }
                self.pots.extend_from_slice(&diff_pots);
            } else {
                return Err(HandleError::Custom("No last pot found".to_string()));
            }
        }

        println!("== Pots after collecting bets: {:?}", self.pots);
        self.bet_map = BTreeMap::<String, Bet>::new();
        Ok(())
    }

    pub fn change_street(
        &mut self,
        effect: &mut Effect,
        new_street: Street,
    ) -> Result<(), HandleError> {
        for player in self.player_map.values_mut() {
            if player.status == PlayerStatus::Acted {
                player.status = PlayerStatus::Wait;
            }
        }
        self.collect_bets()?;
        self.street = new_street;
        println!("== Street changes to {:?}", self.street);
        self.min_raise = self.bb;
        self.street_bet = 0;
        self.acting_player = None;
        self.update_board(effect)?;

        Ok(())
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

    // Reveal community cards according to current street
    pub fn update_board(&mut self, effect: &mut Effect) -> Result<(), HandleError> {
        let players_cnt = self.players.len() * 2;
        match self.street {
            Street::Flop => {
                effect.reveal(
                    self.deck_random_id,
                    (players_cnt..(players_cnt + 3)).collect::<Vec<usize>>(),
                );
                self.stage = HoldemStage::ShareKey;
                println!("== Board is {:?}", self.board);
            }
            Street::Turn => {
                effect.reveal(
                    self.deck_random_id,
                    (players_cnt..(players_cnt + 4)).collect::<Vec<usize>>(),
                );
                self.stage = HoldemStage::ShareKey;
                println!("== Board is {:?}", self.board);
            }
            Street::River => {
                effect.reveal(
                    self.deck_random_id,
                    (players_cnt..(players_cnt + 5)).collect::<Vec<usize>>(),
                );
                self.stage = HoldemStage::ShareKey;
                println!("== Board is {:?}", self.board);
            }
            Street::Showdown => {
                let decryption = effect.get_revealed(self.deck_random_id)?;
                for i in players_cnt..(players_cnt + 5) {
                    if let Some(card) = decryption.get(&i) {
                        self.board.push(card.clone());
                    } else {
                        return Err(HandleError::Custom(
                            "Failed to reveal the board for showdown".to_string(),
                        ));
                    }
                }
                println!("== Board is {:?}", self.board);
            }
            _ => {}
        }

        Ok(())
    }

    /// Build the prize map for rewarding
    pub fn calc_prize(&mut self) -> Result<(), HandleError> {
        let pots = &mut self.pots;
        let mut prize_map = BTreeMap::<String, u64>::new();
        // TODO: discuss the smallest unit
        let smallest_bet = 1u64;
        let mut odd_chips = 0u64;
        for pot in pots {
            let cnt: u64 = pot.winners.len() as u64;
            let remainder = pot.amount % (smallest_bet * cnt);
            odd_chips += remainder;
            let prize: u64 = (pot.amount - remainder) / cnt;
            for winner in pot.winners.iter() {
                prize_map
                    .entry(winner.clone())
                    .and_modify(|p| *p += prize)
                    .or_insert(prize);
            }
        }

        // Giving odd chips to btn (if present) or sb
        let mut remainder_player = "none".to_string();
        if let Some(addr) = self.players.get(self.btn) {
            remainder_player = addr.clone();
        } else {
            if let Some(addr) = self.players.first() {
                remainder_player = addr.clone();
            };
        };
        println!(
            "== Player {} to get the {} odd chips",
            remainder_player, odd_chips
        );
        prize_map
            .entry(remainder_player)
            .and_modify(|prize| *prize += odd_chips)
            .or_insert(odd_chips);

        self.prize_map = prize_map;

        Ok(())
    }

    /// Increase player chips according to prize map.
    /// Chips of those who lost will be left untouched as
    /// their chips will be updated later by update_chips_map.
    pub fn apply_prize(&mut self) -> Result<(), HandleError> {
        for player in self.player_map.values_mut() {
            match self.prize_map.get(&player.addr) {
                Some(prize) => {
                    player.chips += *prize;
                    println!("== Player {} won {} chips", player.addr, *prize);
                }
                None => {
                    println!("== Player {} lost bet", player.addr);
                }
            }
        }
        Ok(())
    }

    pub fn assign_winners(&mut self, winner_sets: Vec<Vec<String>>) -> Result<(), HandleError> {
        for pot in self.pots.iter_mut() {
            for winner_set in winner_sets.iter() {
                let real_winners: Vec<String> = winner_set
                    .iter()
                    .filter(|&w| pot.owners.contains(w))
                    .map(|w| w.clone())
                    .collect();
                // A pot should have at least one winner
                if real_winners.len() >= 1 {
                    pot.winners = real_winners;
                    break;
                } else {
                    continue;
                }
            }

            // If a pot fails to have any winners, its owners split the pot
            if pot.winners.len() == 0 {
                pot.winners = pot.owners.clone();
            }
        }

        Ok(())
    }

    /// Update the map that records players chips change (increased or decreased)
    /// Used for settlement
    pub fn update_chips_map(&mut self) -> Result<BTreeMap<String, i64>, HandleError> {
        let mut chips_change_map: BTreeMap<String, i64> = self
            .player_map
            .keys()
            .map(|addr| (addr.clone(), 0))
            .collect();

        println!("== Chips map before awarding: {:?}", chips_change_map);

        // Player's chips change = prize - betted
        for pot in self.pots.iter() {
            let bet = pot.amount / pot.owners.len() as u64;
            for owner in pot.owners.iter() {
                chips_change_map
                    .entry(owner.clone())
                    .and_modify(|chips| *chips -= bet as i64)
                    .or_insert(0 - bet as i64);
            }
        }

        for (player, prize) in self.prize_map.iter() {
            chips_change_map
                .entry(player.clone())
                .and_modify(|chips| *chips += *prize as i64);
        }

        println!("== Chips map after awarding: {:?}", chips_change_map);

        Ok(chips_change_map)
    }

    pub fn single_player_win(
        &mut self,
        effect: &mut Effect,
        winners: Vec<Vec<String>>,
    ) -> Result<(), HandleError> {
        self.collect_bets()?;
        self.assign_winners(winners)?;
        self.calc_prize()?;
        self.apply_prize()?;

        let chips_change_map = self.update_chips_map()?;

        // Add or reduce players chips according to chips change map
        for (player, chips_change) in chips_change_map.iter() {
            if *chips_change > 0 {
                effect.settle(Settle::add(player, *chips_change as u64))
            } else if *chips_change < 0 {
                effect.settle(Settle::sub(player, -*chips_change as u64))
            }
        }
        // Eject those whose lost all chips
        for player in self.player_map.values() {
            if player.chips == 0 {
                effect.settle(Settle::eject(&player.addr));
            }
        }

        effect.wait_timeout(WAIT_TIMEOUT);
        Ok(())
    }

    pub fn settle(&mut self, effect: &mut Effect) -> Result<(), HandleError> {
        let decryption = effect.get_revealed(self.deck_random_id)?;
        // Board
        let board: Vec<&str> = self.board.iter().map(|c| c.as_str()).collect();
        // Player hands
        let mut player_hands: Vec<(String, PlayerHand)> = Vec::with_capacity(self.players.len());
        for (addr, idxs) in self.hand_index_map.iter() {
            if idxs.len() != 2 {
                return Err(HandleError::Custom(
                    "Invalid hole-card idx vec: length not equal to 2".to_string(),
                ));
            }

            let Some(player) = self.player_map.get(addr) else {
                return Err(HandleError::Custom(
                    "Player not found in game".to_string()
                ));
            };

            if player.status != PlayerStatus::Fold {
                let Some(first_card_idx) = idxs.first() else {
                    return Err(HandleError::Custom(
                        "Failed to extract index for 1st hole card".to_string()
                    ));
                };
                let Some(first_card) = decryption.get(first_card_idx) else {
                    return Err(HandleError::Custom(
                        "Failed to get 1st hole card from the revealed info".to_string()
                    ));
                };
                let Some(second_card_idx) = idxs.last() else {
                    return Err(HandleError::Custom(
                        "Failed to extract index for 2nd hole card".to_string()
                    ));
                };
                let Some(second_card) = decryption.get(second_card_idx) else {
                    return Err(HandleError::Custom(
                        "Failed to get 2nd hole card from the revealed info".to_string()
                    ));
                };
                let hole_cards = [first_card.as_str(), second_card.as_str()];
                let cards = create_cards(board.as_slice(), &hole_cards);
                let hand = evaluate_cards(cards);
                player_hands.push((player.addr.clone(), hand));
            }
        }
        player_hands.sort_by(|(_, h1), (_, h2)| compare_hands(&h2.value, &h1.value));
        println!("Player Hands from strong to weak {:?}", player_hands);

        // Winners example: [[w1], [w2, w3], ... ] where w2 == w3, i.e. a draw/tie
        let mut winners: Vec<Vec<String>> = Vec::new();
        let mut weaker: Vec<Vec<String>> = Vec::new();
        // Players in a draw will be in the same set
        let mut draws = Vec::<String>::new();
        // Each hand is either equal to or weaker than winner (1st)
        let Some((winner, highest_hand)) = player_hands.first() else {
            return Err(HandleError::Custom(
                "Failed to spot the strongest hand".to_string()
            ));
        };

        for (player, hand) in player_hands.iter().skip(1) {
            if highest_hand.value.iter().eq(hand.value.iter()) {
                draws.push(player.clone());
            } else {
                weaker.push(vec![player.clone()]);
            }
        }

        if draws.len() > 0 {
            draws.push(winner.clone());
            winners.extend_from_slice(&vec![draws]);
        } else {
            winners.push(vec![winner.clone()]);
        }

        if weaker.len() > 0 {
            winners.extend_from_slice(&weaker);
        }

        println!("== Player rankings in order: {:?}", winners);
        self.assign_winners(winners)?;
        self.calc_prize()?;
        self.apply_prize()?;
        let chips_change_map = self.update_chips_map()?;

        // Add or reduce players chips according to chips change map
        for (player, chips_change) in chips_change_map.iter() {
            if *chips_change > 0 {
                effect.settle(Settle::add(player, *chips_change as u64))
            } else if *chips_change < 0 {
                effect.settle(Settle::sub(player, -*chips_change as u64))
            }
        }
        // Eject those whose lost all chips
        for player in self.player_map.values() {
            if player.chips == 0 {
                effect.settle(Settle::eject(&player.addr));
            }
        }

        Ok(())
    }

    // De facto entry point of Holdem
    pub fn next_state(&mut self, effect: &mut Effect) -> Result<(), HandleError> {
        let last_pos = self.get_ref_position();
        self.arrange_players(last_pos)?;
        let all_players = self.players.clone();
        let mut players_to_stay = Vec::<&String>::new();
        let mut players_to_act = Vec::<&String>::new();
        let mut players_allin = Vec::<&String>::new();
        // TODO: consider using a block so that all_players can be removed
        for addr in all_players.iter() {
            if let Some(player) = self.player_map.get(addr) {
                match player.status {
                    PlayerStatus::Acting => {
                        players_to_stay.push(addr);
                    }
                    PlayerStatus::Wait | PlayerStatus::Acted => {
                        players_to_stay.push(addr);
                        players_to_act.push(addr);
                    }
                    PlayerStatus::Allin => {
                        players_to_stay.push(addr);
                        players_allin.push(addr);
                    }
                    PlayerStatus::Fold => {}
                }
            }
        }

        let next_player = self.next_action_player(players_to_act);
        let new_street = self.next_street();

        println!("== Current GAME BETS are {:?}", self.bet_map);

        // Blind bets
        if self.street == Street::Preflop && self.bet_map.is_empty() {
            println!("[Next State]: Blind bets");
            self.blind_bets(effect)?;
            Ok(())
        }
        // Single player wins because there is one player only
        else if all_players.len() == 1 {
            let Some(winner) = all_players.first() else {
                return Err(HandleError::Custom(
                    "Failed to get the only player".to_string()
                ));
            };
            println!("[Next State]: Single winner : {}", winner);
            self.single_player_win(effect, vec![vec![(*winner).clone()]])?;
            Ok(())
        }
        // Singple players wins because others all folded
        else if players_to_stay.len() == 1 {
            let Some(winner) = players_to_stay.first() else {
                return Err(HandleError::Custom(
                    "Failed to get the single winner left".to_string()
                ));
            };
            println!(
                "[Next State]: All others folded and single winner is {}",
                winner
            );
            self.single_player_win(effect, vec![vec![(*winner).clone()]])?;
            Ok(())
        }
        // Ask next player to act
        else if next_player.is_some() {
            let Some(next_action_player) = next_player else {
                return Err(HandleError::Custom(
                    "Failed to get the next-to-act player".to_string()
                ));
            };
            println!(
                "[Next State]: Next-to-act player is: {}",
                next_action_player
            );
            self.ask_for_action(next_action_player, effect)?;
            Ok(())
        }
        // Runner
        else if self.stage != HoldemStage::Runner
            && players_allin.len() + 1 >= players_to_stay.len()
        {
            println!("[Next State]: Runner");
            self.street = Street::Showdown;
            self.stage = HoldemStage::Runner;
            self.collect_bets()?;

            // Reveal all cards at once
            let players_cnt = self.players.len() * 2;
            let mut idxs = Vec::<usize>::new();
            for (idx, player) in self.player_map.values().enumerate() {
                if player.status != PlayerStatus::Fold {
                    idxs.push(idx * 2);
                    idxs.push(idx * 2 + 1);
                }
            }
            idxs.extend_from_slice(&(players_cnt..(players_cnt + 5)).collect::<Vec<usize>>());
            effect.reveal(self.deck_random_id, idxs);

            Ok(())
        }
        // Next Street
        else if new_street != Street::Showdown {
            println!("[Next State]: Move to next street: {:?}", new_street);
            self.change_street(effect, new_street)?;
            Ok(())
        }
        // Showdown
        else {
            println!("[Next State]: Showdown");
            self.stage = HoldemStage::Showdown;
            self.collect_bets()?;

            // Reveal players' hole cards
            let mut idxs = Vec::<usize>::new();
            for (idx, player) in self.player_map.values().enumerate() {
                if player.status != PlayerStatus::Fold {
                    idxs.push(idx * 2);
                    idxs.push(idx * 2 + 1);
                }
            }
            effect.reveal(self.deck_random_id, idxs);
            // self.settle(effect)?;
            // self.reset_holdem_state()?;
            // effect.wait_timeout(WAIT_TIMEOUT);

            Ok(())
        }
    }

    fn handle_custom_event(
        &mut self,
        effect: &mut Effect,
        event: GameEvent,
        sender: String,
    ) -> Result<(), HandleError> {
        match event {
            GameEvent::Bet(amount) => {
                if !self.is_acting_player(&sender) {
                    return Err(HandleError::Custom(
                        "Player is NOT the acting player so can't bet".to_string(),
                    ));
                }
                if self.bet_map.get(&sender).is_some() {
                    return Err(HandleError::Custom("Player already betted".to_string()));
                }
                // Freestyle betting not allowed in the preflop
                if self.street_bet != 0 {
                    return Err(HandleError::Custom("Player can't bet".to_string()));
                }
                if self.bb > amount {
                    return Err(HandleError::Custom("Bet must be >= bb".to_string()));
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
                    self.next_state(effect)?;
                    Ok(())
                } else {
                    return Err(HandleError::Custom(
                        "Player not found in game or has not joined yet".to_string(),
                    ));
                }
            }

            GameEvent::Call => {
                // TODO: handle errors
                if !self.is_acting_player(&sender) {
                    return Err(HandleError::Custom(
                        "Player is NOT the acting player so can't call".to_string(),
                    ));
                }
                if let Some(player) = self.player_map.get_mut(&sender) {
                    // Have betted but needs to increase the bet
                    if let Some(betted) = self.bet_map.get_mut(&sender) {
                        let betted_amount = betted.amount;
                        let call_amount = self.street_bet - betted_amount;
                        println!(
                            "== {} needs to call with amount: {}",
                            player.addr, call_amount
                        );
                        let (allin, real_bet) = player.take_bet(call_amount);
                        betted.amount += real_bet;
                        player.status = if allin {
                            PlayerStatus::Allin
                        } else {
                            PlayerStatus::Acted
                        };
                        println!("== {} chips after calls: {}", player.addr, player.chips);
                        self.next_state(effect)?;
                        Ok(())
                    } else {
                        // First time to call in this street
                        let call_amount = self.street_bet - 0;
                        let (allin, real_bet) = player.take_bet(call_amount);
                        self.bet_map
                            .insert(player.addr.clone(), Bet::new(player.addr.clone(), real_bet));
                        player.status = if allin {
                            PlayerStatus::Allin
                        } else {
                            PlayerStatus::Acted
                        };
                        self.next_state(effect)?;
                        Ok(())
                    }
                } else {
                    return Err(HandleError::Custom("Player not found in game".to_string()));
                }
            }

            GameEvent::Check => {
                if !self.is_acting_player(&sender) {
                    return Err(HandleError::Custom(
                        "Player is NOT the acting player so can't check".to_string(),
                    ));
                }
                match self.street {
                    Street::Preflop => {
                        if let Some(player_bet) = self.bet_map.get(&sender) {
                            if self.street_bet != player_bet.amount {
                                return Err(HandleError::Custom("Player can't check".to_string()));
                            }

                            if let Some(player) = self.player_map.get_mut(&sender) {
                                player.status = PlayerStatus::Acted;
                                self.next_state(effect)?;
                                Ok(())
                            } else {
                                return Err(HandleError::Custom(
                                    "Player not found [Check, Preflop]".to_string(),
                                ));
                            }
                        } else {
                            return Err(HandleError::Custom("Player hasnt bet yet".to_string()));
                        }
                    }

                    Street::Flop | Street::Turn | Street::River => {
                        if self.bet_map.is_empty() {
                            let Some(player) = self.player_map.get_mut(&sender) else {
                                return Err(HandleError::Custom(
                                    "Player not found [Check, Flop|Turn|River]".to_string()
                                ));
                            };
                            player.status = PlayerStatus::Acted;
                            self.next_state(effect)?;
                            Ok(())
                        } else {
                            return Err(HandleError::Custom("Player hasnt bet yet".to_string()));
                        }
                    }
                    _ => Err(HandleError::Custom(
                        "Invalid Street so player can't check".to_string(),
                    )),
                }
            }

            GameEvent::Fold => {
                if !self.is_acting_player(&sender) {
                    return Err(HandleError::Custom(
                        "Player is NOT the acting player so can't fold".to_string(),
                    ));
                }

                if let Some(player) = self.player_map.get_mut(&sender) {
                    println!("== Player {} folds", sender);
                    player.status = PlayerStatus::Fold;
                    self.next_state(effect)?;
                    Ok(())
                } else {
                    return Err(HandleError::Custom("Player NOT found in game".to_string()));
                }
            }

            GameEvent::Raise(amount) => {
                if !self.is_acting_player(&sender) {
                    return Err(HandleError::Custom(
                        "Player is NOT the acting player so can't raise".to_string(),
                    ));
                }

                if self.street_bet == 0 || self.bet_map.is_empty() {
                    return Err(HandleError::Custom(
                        "Street bet is 0 so raising is not allowed".to_string(),
                    ));
                }

                if amount == 0 || amount < self.street_bet {
                    return Err(HandleError::Custom(
                        "Invalid raise amount: 0 or less than street bet".to_string(),
                    ));
                }

                if amount < self.street_bet + self.min_raise {
                    return Err(HandleError::Custom("Player raises too small".to_string()));
                }

                if let Some(player) = self.player_map.get_mut(&sender) {
                    if let Some(betted) = self.bet_map.get_mut(&sender) {
                        // let added_bet = amount - betted.amount;
                        let (allin, real_bet) = player.take_bet(amount);
                        let new_street_bet = betted.amount + real_bet;
                        let new_min_raise = new_street_bet - self.street_bet;
                        self.street_bet = new_street_bet;
                        self.min_raise = new_min_raise;
                        player.status = if allin {
                            PlayerStatus::Allin
                        } else {
                            PlayerStatus::Acted
                        };
                        betted.amount += real_bet;
                        self.next_state(effect)?;
                    }
                    else {
                        // let added_bet = amount;
                        let (allin, real_bet) = player.take_bet(amount);
                        let new_min_raise = real_bet - self.street_bet;
                        self.street_bet = real_bet;
                        self.min_raise = new_min_raise;
                        player.status = if allin {
                            PlayerStatus::Allin
                        } else {
                            PlayerStatus::Acted
                        };
                        self.bet_map
                            .insert(sender, Bet::new(player.addr.clone(), real_bet));
                        self.next_state(effect)?;
                    }
                    Ok(())
                } else {
                    return Err(HandleError::Custom("Player not found in game".to_string()));
                }
            }
        }
    }
}

impl GameHandler for Holdem {
    fn init_state(_effect: &mut Effect, init_account: InitAccount) -> Result<Self, HandleError> {
        let HoldemAccount { sb, bb, rake } = init_account.data()?;
        Ok(Self {
            deck_random_id: 0,
            sb,
            bb,
            min_raise: bb,
            btn: 0,
            rake,
            stage: HoldemStage::Init,
            street: Street::Init,
            street_bet: 0,
            board: Vec::<String>::with_capacity(5),
            hand_index_map: BTreeMap::<String, Vec<usize>>::new(),
            bet_map: BTreeMap::<String, Bet>::new(),
            prize_map: BTreeMap::<String, u64>::new(),
            player_map: BTreeMap::<String, Player>::new(),
            players: Vec::<String>::new(),
            pots: Vec::<Pot>::new(),
            acting_player: None,
        })
    }

    fn handle_event(&mut self, effect: &mut Effect, event: Event) -> Result<(), HandleError> {
        match event {
            // Handle holdem specific (custom) events
            Event::Custom { sender, raw } => {
                let event: GameEvent = GameEvent::try_parse(&raw)?;
                self.handle_custom_event(effect, event, sender.clone())?;
                Ok(())
            }

            Event::ActionTimeout { player_addr } => {
                let Some(player) = self.player_map.get_mut(&player_addr) else {
                    return Err(HandleError::Custom("Player not found in game".to_string()));
                };

                let street_bet = self.street_bet;
                let bet = if let Some(player_bet) = self.bet_map.get(&player_addr) {
                    player_bet.amount
                } else {
                    0
                };

                if bet == street_bet {
                    player.status = PlayerStatus::Acted;
                    self.acting_player = None;
                    self.next_state(effect)?;
                    Ok(())
                } else {
                    player.status = PlayerStatus::Fold;
                    self.acting_player = None;
                    self.next_state(effect)?;
                    Ok(())
                }
            }

            Event::WaitingTimeout => {
                let next_btn = self.get_next_btn()?;
                println!("Next BTN: {}", next_btn);

                for player in self.player_map.values_mut() {
                    player.status = PlayerStatus::Wait
                }

                if effect.count_players() >= 2 && effect.count_servers() >= 1 {
                    self.btn = next_btn;
                    effect.start_game();
                }
                println!("== Game starts again");
                Ok(())
            }

            Event::Sync { new_players, .. } => {
                for p in new_players.iter() {
                    let player = Player::new(p.addr.clone(), p.balance, p.position as usize);
                    self.players.push(player.addr.clone());
                    self.player_map.insert(player.addr.clone(), player);
                }

                // Must check num of players and servers
                if effect.count_players() >= 2 && effect.count_servers() >= 1 {
                    effect.start_game();
                }

                Ok(())
            }

            Event::GameStart { .. } => {
                self.reset_holdem_state()?;
                self.stage = HoldemStage::Play;
                let player_num = self.player_map.len();
                println!("== {} players join game", player_num);

                // let btn = self.get_next_btn()?;
                // println!("Next BTN: {}", btn);
                // self.btn = btn;

                // Get the randomness (shuffled and dealt cards) ready
                let rnd_spec = RandomSpec::deck_of_cards();
                self.deck_random_id = effect.init_random_state(rnd_spec);
                Ok(())
            }

            Event::Leave { player_addr } => {
                // TODO: Leaving is not allowed in SNG game
                if self.player_map.get(&player_addr).is_none() {
                    return Err(HandleError::CantLeave);
                }

                match self.stage {
                    HoldemStage::Init | HoldemStage::Settle | HoldemStage::Showdown => {
                        // TODO: Allow leaving when game is not running
                    }
                    _ => {
                        return Err(HandleError::CantLeave);
                    }
                }

                let Some(leaving_player) = self.player_map.get_mut(&player_addr) else {
                    return Err(HandleError::Custom(
                       "Player not found [Leave]".to_string()
                    ));
                };
                leaving_player.status = PlayerStatus::Fold;
                let remained_players: Vec<String> = self
                    .player_map
                    .values()
                    .filter(|p| match p.status {
                        PlayerStatus::Allin
                        | PlayerStatus::Acted
                        | PlayerStatus::Acting
                        | PlayerStatus::Wait => true,
                        _ => false,
                    })
                    .map(|p| p.addr.clone())
                    .collect();

                if remained_players.len() == 1 {
                    self.single_player_win(effect, vec![remained_players])?;
                } else {
                    // TODO: remove the leaving player and let game continue?
                }

                Ok(())
            }

            Event::RandomnessReady { .. } => {
                // Cards are dealt to players but remain invisible to them
                for (idx, (addr, _)) in self.player_map.iter().enumerate() {
                    effect.assign(self.deck_random_id, addr, vec![idx * 2, idx * 2 + 1]);
                    self.hand_index_map
                        .insert(addr.clone(), vec![idx * 2, idx * 2 + 1]);
                }

                Ok(())
            }

            Event::SecretsReady => match self.stage {
                HoldemStage::ShareKey => {
                    let players_cnt = self.players.len() * 2;
                    self.stage = HoldemStage::Play;

                    match self.street {
                        Street::Preflop => {
                            self.next_state(effect)?;
                        }

                        Street::Flop => {
                            let decryption = effect.get_revealed(self.deck_random_id)?;
                            for i in players_cnt..(players_cnt + 3) {
                                if let Some(card) = decryption.get(&i) {
                                    self.board.push(card.clone())
                                } else {
                                    return Err(HandleError::Custom(
                                        "Failed to reveal the 3 flop cards".to_string(),
                                    ));
                                }
                            }

                            self.next_state(effect)?;
                        }

                        Street::Turn => {
                            let decryption = effect.get_revealed(self.deck_random_id)?;
                            let card_index = players_cnt + 3;
                            if let Some(card) = decryption.get(&card_index) {
                                self.board.push(card.clone());
                            } else {
                                return Err(HandleError::Custom(
                                    "Failed to reveal the turn card".to_string(),
                                ));
                            }

                            self.next_state(effect)?;
                        }

                        Street::River => {
                            let decryption = effect.get_revealed(self.deck_random_id)?;
                            let card_index = players_cnt + 4;
                            if let Some(card) = decryption.get(&card_index) {
                                self.board.push(card.clone());
                            } else {
                                return Err(HandleError::Custom(
                                    "Failed to reveal the river card".to_string(),
                                ));
                            }
                            self.next_state(effect)?;
                        }

                        _ => {}
                    }
                    Ok(())
                }

                HoldemStage::Play => {
                    match self.street {
                        Street::Init => {
                            self.street = Street::Preflop;
                            self.stage = HoldemStage::ShareKey;
                            self.next_state(effect)?;
                            Ok(())
                        }

                        // if other streets, keep playing
                        _ => Ok(()),
                    }
                }

                // TODO: Stage should be upper class and include street
                HoldemStage::Runner | HoldemStage::Showdown => {
                    self.update_board(effect)?;
                    self.settle(effect)?;

                    effect.wait_timeout(WAIT_TIMEOUT);
                    Ok(())
                }

                // Other Holdem Stages
                _ => Ok(()),
            },

            // Other events
            _ => Ok(()),
        }
    }
}
