//! Game state machine (or handler) of Holdem: the core of this lib.
use race_core::prelude::*;
use race_proc_macro::game_handler;
use std::collections::BTreeMap;

use crate::essential::{
    ActingPlayer, AwardPot, Display, GameEvent, HoldemAccount, HoldemStage, Player, PlayerResult,
    PlayerStatus, Pot, Street, ACTION_TIMEOUT_POSTFLOP, ACTION_TIMEOUT_PREFLOP,
    ACTION_TIMEOUT_RIVER, ACTION_TIMEOUT_TURN, MAX_ACTION_TIMEOUT_COUNT, WAIT_TIMEOUT_DEFAULT,
    WAIT_TIMEOUT_LAST_PLAYER, WAIT_TIMEOUT_RUNNER, WAIT_TIMEOUT_SHOWDOWN,
};
use crate::evaluator::{compare_hands, create_cards, evaluate_cards, PlayerHand};

// Holdem: the game state
#[cfg_attr(test, derive(Debug, PartialEq))]
#[game_handler]
#[derive(BorshSerialize, BorshDeserialize)]
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
    pub bet_map: BTreeMap<String, u64>,
    pub prize_map: BTreeMap<String, u64>,
    pub player_map: BTreeMap<String, Player>,
    pub player_order: Vec<String>,
    pub pots: Vec<Pot>,
    pub acting_player: Option<ActingPlayer>,
    pub winners: Vec<String>,
    pub display: Vec<Display>,
}

// Methods that mutate or query the game state
impl Holdem {
    // Mark out players.
    // An out player is one with zero chips.
    fn mark_out_players(&mut self) {
        for (_, v) in self.player_map.iter_mut() {
            if v.status != PlayerStatus::Leave && v.chips == 0 {
                v.status = PlayerStatus::Out;
                // Here we use timeout for rebuy timeout.
                v.timeout = 0;
            };
        }
    }

    // Remove players with `Leave` status.
    fn remove_leave_and_out_players(&mut self) -> Vec<String> {
        let mut removed = Vec::with_capacity(self.player_map.len());
        self.player_map.retain(|_, p| {
            if p.status == PlayerStatus::Leave || p.status == PlayerStatus::Out {
                removed.push(p.addr.clone());
                false
            } else {
                true
            }
        });
        removed
    }

    // Make All eligible players Wait
    fn reset_player_map_status(&mut self) -> Result<(), HandleError> {
        for player in self.player_map.values_mut() {
            if player.status == PlayerStatus::Out {
                player.timeout += 1;
            } else {
                player.status = PlayerStatus::Wait;
            }
        }
        Ok(())
    }

    // Clear data that don't belong to a running game, indicating game end
    fn signal_game_end(&mut self) -> Result<(), HandleError> {
        self.street_bet = 0;
        self.min_raise = 0;
        self.acting_player = None;

        Ok(())
    }

    fn reset_holdem_state(&mut self) -> Result<(), HandleError> {
        self.street = Street::Init;
        self.stage = HoldemStage::Init;
        self.pots = Vec::<Pot>::new();
        self.board = Vec::<String>::with_capacity(5);
        self.player_order = Vec::<String>::new();
        self.hand_index_map = BTreeMap::<String, Vec<usize>>::new();
        self.bet_map = BTreeMap::<String, u64>::new();
        self.prize_map = BTreeMap::<String, u64>::new();

        Ok(())
    }

    fn next_action_player(&mut self, next_players: Vec<&String>) -> Option<String> {
        for addr in next_players {
            if let Some(player) = self.player_map.get(addr) {
                let curr_bet: u64 = self.bet_map.get(addr).map(|b| *b).unwrap_or(0);
                if curr_bet < self.street_bet || player.status == PlayerStatus::Wait {
                    return Some(addr.clone());
                }
            }
        }
        None
    }

    pub fn is_acting_player(&self, player_addr: &str) -> bool {
        match &self.acting_player {
            Some(ActingPlayer { addr, .. }) => addr == player_addr,
            None => false,
        }
    }

    // Return either acting player position or btn for reference
    pub fn get_ref_position(&self) -> usize {
        if let Some(ActingPlayer {
            addr: _,
            position,
            clock: _,
        }) = self.acting_player
        {
            position
        } else {
            self.btn
        }
    }

    // BTN moves clockwise.  The next BTN is calculated base on the current one
    pub fn get_next_btn(&mut self) -> Result<usize, HandleError> {
        let mut player_positions: Vec<usize> =
            self.player_map.values().map(|p| p.position).collect();
        player_positions.sort();

        let next_positions: Vec<usize> = player_positions
            .iter()
            .filter(|pos| **pos > self.btn)
            .map(|p| *p)
            .collect();

        if next_positions.is_empty() {
            let Some(next_btn) = player_positions.first() else {
                return Err(HandleError::Custom(
                    "Failed to find a player for the next button".to_string(),
                ));
            };
            Ok(*next_btn)
        } else {
            if let Some(next_btn) = next_positions.first() {
                Ok(*next_btn)
            } else {
                return Err(HandleError::Custom(
                    "Failed to find a proper position for the next button".to_string(),
                ));
            }
        }
    }

    fn get_action_time(&self) -> u64 {
        match self.street {
            Street::Turn => ACTION_TIMEOUT_TURN,
            Street::River => ACTION_TIMEOUT_RIVER,
            Street::Flop => ACTION_TIMEOUT_POSTFLOP,
            Street::Preflop => {
                if self.street_bet == self.bb {
                    ACTION_TIMEOUT_PREFLOP
                } else {
                    ACTION_TIMEOUT_POSTFLOP
                }
            }
            _ => 0,
        }
    }

    pub fn ask_for_action(
        &mut self,
        player_addr: String,
        effect: &mut Effect,
    ) -> Result<(), HandleError> {
        let timeout = self.get_action_time();
        if let Some(player) = self.player_map.get_mut(&player_addr) {
            println!("== Asking {} to act", player.addr);
            player.status = PlayerStatus::Acting;
            self.acting_player = Some(ActingPlayer {
                addr: player.addr(),
                position: player.position,
                clock: effect.timestamp() + timeout,
            });
            effect.action_timeout(player_addr, timeout); // in secs
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
            .filter(|p| p.status != PlayerStatus::Init)
            .map(|p| {
                if p.position > last_pos {
                    (p.addr(), p.position - last_pos)
                } else {
                    (p.addr(), p.position + 100)
                }
            })
            .collect();
        player_pos.sort_by(|(_, pos1), (_, pos2)| pos1.cmp(pos2));
        let player_order: Vec<String> = player_pos.into_iter().map(|(addr, _)| addr).collect();
        println!("== Player order {:?}", player_order);
        self.player_order = player_order;
        Ok(())
    }

    pub fn blind_bets(&mut self, effect: &mut Effect) -> Result<(), HandleError> {
        let mut players: Vec<String> = self.player_order.clone();
        if players.len() == 2 {
            players.reverse();
            // Take bet from BB
            if let Some(bb_addr) = players.first() {
                if let Some(bb_player) = self.player_map.get_mut(bb_addr) {
                    let (allin, real_bb) = bb_player.take_bet(self.bb);
                    self.bet_map.insert(bb_addr.clone(), real_bb);
                    if allin {
                        bb_player.status = PlayerStatus::Allin;
                    }
                }
            } else {
                return Err(HandleError::Custom(
                    "No first player found in 2 players for big blind".to_string(),
                ));
            }

            // Take bet from SB (BTN)
            if let Some(sb_addr) = players.last() {
                if let Some(sb_player) = self.player_map.get_mut(sb_addr) {
                    println!("== SB Player is {:?}", sb_player);
                    let (allin, real_sb) = sb_player.take_bet(self.sb);
                    self.bet_map.insert(sb_addr.clone(), real_sb);
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
                    self.bet_map.insert(sb_player.addr(), real_sb);
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
                    self.bet_map.insert(bb_player.addr(), real_bb);
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
        // TODO: Move display a bit earlier than first SecretsReady
        self.display.push(Display::DealCards);
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
        let mut bets: Vec<u64> = self.bet_map.iter().map(|(_, b)| *b).collect();
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
                .filter(|(_, b)| **b >= bet)
                .map(|(owner, _)| owner.clone())
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
        self.display.push(Display::CollectBets {
            bet_map: self.bet_map.clone(),
        });
        self.bet_map = BTreeMap::<String, u64>::new();
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

    // Count players whose status is not `Init`
    pub fn count_ingame_players(&self) -> usize {
        self.player_map
            .values()
            .filter(|p| p.status != PlayerStatus::Init)
            .count()
    }

    // Reveal community cards according to current street
    pub fn update_board(&mut self, effect: &mut Effect) -> Result<(), HandleError> {
        let players_cnt = self.count_ingame_players() * 2;
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
                effect.reveal(self.deck_random_id, vec![players_cnt + 3]);
                self.stage = HoldemStage::ShareKey;
                println!("== Board is {:?}", self.board);
            }

            Street::River => {
                effect.reveal(self.deck_random_id, vec![players_cnt + 4]);
                self.stage = HoldemStage::ShareKey;
                println!("== Board is {:?}", self.board);
            }

            // For Runner, update 5 community cards at once
            Street::Showdown => {
                self.board.clear();
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

    /// Build the prize map for awarding chips
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
        if let Some(addr) = self.player_order.get(self.btn) {
            remainder_player = addr.clone();
        } else {
            if let Some(addr) = self.player_order.first() {
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
                    for w in real_winners.iter() {
                        let Some(_player) = self.player_map.get_mut(w) else {
                            return Err(HandleError::Custom(
                                "Winner not found in player map".to_string()
                            ));
                        };
                    }
                    pot.winners = real_winners;
                    break;
                } else {
                    continue;
                }
            }

            // If a pot fails to have any winners, its owners split the pot
            // FIXME: This should never happen.
            if pot.winners.len() == 0 {
                pot.winners = pot.owners.clone();
                for w in pot.winners.iter() {
                    let Some(_player) = self.player_map.get_mut(w) else {
                        return Err(HandleError::Custom(
                            "Winner not found in player map".to_string()
                        ));
                    };
                }
            }
        }

        let award_pots = self
            .pots
            .iter()
            .map(|pot| {
                let winners = pot.winners.clone();
                let amount = pot.amount;
                AwardPot { winners, amount }
            })
            .collect();
        self.display.push(Display::AwardPots { pots: award_pots });

        Ok(())
    }

    /// Update the map that records players chips change (increased or decreased)
    /// Used for settlement
    pub fn update_chips_map(&mut self) -> Result<BTreeMap<String, i64>, HandleError> {
        // The i64 change for each player.  The amount = total pots
        // earned - total bet.  This map will be returned for furture
        // calculation.
        let mut chips_change_map: BTreeMap<String, i64> = self
            .player_map
            .keys()
            .map(|addr| (addr.clone(), 0))
            .collect();

        // The players for game result information.  The `chips` is
        // the amount before the settlement, the `prize` is the sum of
        // pots earned during the settlement.  This map will be added
        // to display.
        let mut result_player_map = BTreeMap::<String, PlayerResult>::new();

        self.winners = Vec::<String>::with_capacity(self.player_map.len());

        println!("== Chips map before awarding: {:?}", chips_change_map);
        println!("== Prize map: {:?}", self.prize_map);

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
            if *prize > 0 {
                self.winners.push(player.clone());
            }
            chips_change_map
                .entry(player.clone())
                .and_modify(|chips| *chips += *prize as i64);
        }

        println!("== Chips map after awarding: {:?}", chips_change_map);

        for (addr, player) in self.player_map.iter() {
            let prize = if let Some(p) = self.prize_map.get(addr).copied() {
                if p == 0 {
                    None
                } else {
                    Some(p)
                }
            } else {
                None
            };

            let result = PlayerResult {
                addr: addr.clone(),
                position: player.position,
                status: player.status,
                chips: player.chips,
                prize,
            };

            result_player_map.insert(addr.clone(), result);
        }

        self.display.push(Display::GameResult {
            player_map: result_player_map,
        });

        Ok(chips_change_map)
    }

    pub fn single_player_win(
        &mut self,
        effect: &mut Effect,
        winner: String,
    ) -> Result<(), HandleError> {
        self.collect_bets()?;
        self.assign_winners(vec![vec![winner]])?;
        self.calc_prize()?;
        let chips_change_map = self.update_chips_map()?;
        self.apply_prize()?;

        // Add or reduce players chips according to chips change map
        for (player, chips_change) in chips_change_map.iter() {
            if *chips_change > 0 {
                effect.settle(Settle::add(player, *chips_change as u64));
            } else if *chips_change < 0 {
                effect.settle(Settle::sub(player, -*chips_change as u64));
            }
        }

        self.mark_out_players();

        let removed_addrs = self.remove_leave_and_out_players();
        for addr in removed_addrs {
            effect.settle(Settle::eject(addr));
        }

        effect.wait_timeout(WAIT_TIMEOUT_LAST_PLAYER);
        Ok(())
    }

    pub fn settle(&mut self, effect: &mut Effect) -> Result<(), HandleError> {
        let decryption = effect.get_revealed(self.deck_random_id)?;
        // Board
        let board: Vec<&str> = self.board.iter().map(|c| c.as_str()).collect();
        // Player hands
        let mut player_hands: Vec<(String, PlayerHand)> =
            Vec::with_capacity(self.player_order.len());
        for (addr, idxs) in self.hand_index_map.iter() {
            if idxs.len() != 2 {
                return Err(HandleError::Custom(
                    "Invalid hole-card idx vec: length not equal to 2".to_string(),
                ));
            }

            let Some(player) = self.player_map.get(addr) else {
                return Err(HandleError::Custom(
                    "Player not found in game [settle]".to_string()
                ));
            };

            if player.status != PlayerStatus::Fold
                && player.status != PlayerStatus::Init
                && player.status != PlayerStatus::Leave
            {
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
                player_hands.push((player.addr(), hand));
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
        let chips_change_map = self.update_chips_map()?;
        self.apply_prize()?;

        // Add or reduce players chips according to chips change map
        for (player, chips_change) in chips_change_map.iter() {
            if *chips_change > 0 {
                effect.settle(Settle::add(player, *chips_change as u64))
            } else if *chips_change < 0 {
                effect.settle(Settle::sub(player, -*chips_change as u64))
            }
        }

        self.mark_out_players();
        let removed_addrs = self.remove_leave_and_out_players();

        for addr in removed_addrs {
            effect.settle(Settle::eject(addr));
        }

        Ok(())
    }

    // De facto entry point of Holdem
    pub fn next_state(&mut self, effect: &mut Effect) -> Result<(), HandleError> {
        let last_pos = self.get_ref_position();
        self.arrange_players(last_pos)?;
        // ingame_players exclude anyone with `Init` status
        let ingame_players = self.player_order.clone();
        let mut players_to_stay = Vec::<&String>::new();
        let mut players_to_act = Vec::<&String>::new();
        let mut players_allin = Vec::<&String>::new();

        for addr in ingame_players.iter() {
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
                    _ => {}
                }
            }
        }

        let next_player = self.next_action_player(players_to_act);
        let next_street = self.next_street();

        // Blind bets
        if self.street == Street::Preflop && self.bet_map.is_empty() {
            println!("[Next State]: Blind bets");
            self.blind_bets(effect)?;
            Ok(())
        }
        // Single player wins because there is one player only
        else if ingame_players.len() == 1 {
            self.stage = HoldemStage::Settle;
            self.signal_game_end()?;
            let Some(winner) = ingame_players.first() else {
                return Err(HandleError::Custom(
                    "Failed to get the only player".to_string()
                ));
            };
            println!("[Next State]: Single winner: {}", winner);
            self.single_player_win(effect, winner.clone())?;
            Ok(())
        }
        // Single players wins because others all folded
        else if players_to_stay.len() == 1 {
            self.stage = HoldemStage::Settle;
            self.signal_game_end()?;
            let Some(winner) = players_to_stay.first() else {
                return Err(HandleError::Custom(
                    "Failed to get the single winner left".to_string()
                ));
            };
            println!(
                "[Next State]: All others folded and single winner is {}",
                winner
            );
            self.single_player_win(effect, (*winner).clone())?;
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
            self.signal_game_end()?;
            self.collect_bets()?;

            // Reveal all cards for eligible players: not folded and without init status
            let mut idxs = Vec::<usize>::new();
            for (idx, player) in self.player_map.values().enumerate() {
                match player.status {
                    PlayerStatus::Init | PlayerStatus::Fold | PlayerStatus::Leave => {}
                    _ => {
                        idxs.push(idx * 2);
                        idxs.push(idx * 2 + 1);
                    }
                }
            }

            let players_cnt = self.count_ingame_players() * 2;
            idxs.extend_from_slice(&(players_cnt..(players_cnt + 5)).collect::<Vec<usize>>());
            effect.reveal(self.deck_random_id, idxs);

            Ok(())
        }
        // Next Street
        else if next_street != Street::Showdown {
            println!("[Next State]: Move to next street: {:?}", next_street);
            self.change_street(effect, next_street)?;
            Ok(())
        }
        // Showdown
        else {
            println!("[Next State]: Showdown");
            self.stage = HoldemStage::Showdown;
            self.street = Street::Showdown;
            self.signal_game_end()?;
            self.collect_bets()?;

            // Reveal players' hole cards
            for (addr, idxs) in self.hand_index_map.iter() {
                let Some(player) = self.player_map.get(addr) else {
                    return Err(HandleError::Custom(
                        "Player not found in game but assigned cards".to_string()
                    ));
                };
                if matches!(player.status, PlayerStatus::Acted | PlayerStatus::Allin) {
                    effect.reveal(self.deck_random_id, idxs.clone());
                }
            }

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
                    return Err(HandleError::Custom(
                        "Player can't freestyle bet".to_string(),
                    ));
                }
                if self.bb > amount {
                    return Err(HandleError::Custom("Bet must be >= bb".to_string()));
                }
                if let Some(player) = self.player_map.get_mut(&sender) {
                    let (allin, real_bet) = player.take_bet(amount);
                    self.bet_map.insert(player.addr(), real_bet);
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
                if !self.is_acting_player(&sender) {
                    return Err(HandleError::Custom(
                        "Player is NOT the acting player so can't call".to_string(),
                    ));
                }
                if let Some(player) = self.player_map.get_mut(&sender) {
                    // Have betted but needs to increase the bet
                    if let Some(betted) = self.bet_map.get_mut(&sender) {
                        let betted_amount = *betted;
                        let call_amount = self.street_bet - betted_amount;
                        println!(
                            "== {} needs to call with amount: {}",
                            player.addr, call_amount
                        );
                        let (allin, real_bet) = player.take_bet(call_amount);
                        *betted += real_bet;
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
                        self.bet_map.insert(player.addr(), real_bet);
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

                // Check is only available when player's current bet equals street bet.
                let curr_bet: u64 = self.bet_map.get(&sender).map(|x| *x).unwrap_or(0);
                if curr_bet == self.street_bet {
                    let Some(player) = self.player_map.get_mut(&sender) else {
                        return Err(HandleError::Custom("Player not found".to_string()));
                    };
                    player.status = PlayerStatus::Acted;
                    self.next_state(effect)?;
                    Ok(())
                } else {
                    return Err(HandleError::Custom(
                        "Player can't check because of not enough bet".to_string(),
                    ));
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
                        let new_street_bet = *betted + real_bet;
                        let new_min_raise = new_street_bet - self.street_bet;
                        self.street_bet = new_street_bet;
                        self.min_raise = new_min_raise;
                        player.status = if allin {
                            PlayerStatus::Allin
                        } else {
                            PlayerStatus::Acted
                        };
                        *betted += real_bet;
                        self.next_state(effect)?;
                    } else {
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
                        self.bet_map.insert(sender, real_bet);
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
        let player_map: BTreeMap<String, Player> = init_account
            .players
            .iter()
            .map(|p| {
                let addr = p.addr.clone();
                let player = Player::new(p.addr.clone(), p.balance, p.position);
                (addr, player)
            })
            .collect();

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
            bet_map: BTreeMap::<String, u64>::new(),
            prize_map: BTreeMap::<String, u64>::new(),
            player_map,
            player_order: Vec::<String>::new(),
            pots: Vec::<Pot>::new(),
            acting_player: None,
            winners: Vec::<String>::new(),
            display: Vec::<Display>::new(),
        })
    }

    fn handle_event(&mut self, effect: &mut Effect, event: Event) -> Result<(), HandleError> {
        match event {
            // Handle holdem specific (custom) events
            Event::Custom { sender, raw } => {
                self.display.clear();
                let event: GameEvent = GameEvent::try_parse(&raw)?;
                println!("== Player action event: {:?}, sender: {:?}", event, sender);
                self.handle_custom_event(effect, event, sender.clone())?;
                Ok(())
            }

            Event::ActionTimeout { player_addr } => {
                self.display.clear();
                let Some(player) = self.player_map.get_mut(&player_addr) else {
                    return Err(HandleError::Custom("Player not found in game".to_string()));
                };

                // Mark those who've reached T/O for
                // MAX_ACTION_TIMEOUT_COUNT times with `Leave` status
                if player.timeout > MAX_ACTION_TIMEOUT_COUNT {
                    player.status = PlayerStatus::Leave;
                    self.acting_player = None;
                    self.next_state(effect)?;
                    return Ok(());
                } else {
                    player.timeout += 1;
                }

                let street_bet = self.street_bet;
                let bet = if let Some(player_bet) = self.bet_map.get(&player_addr) {
                    *player_bet
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
                self.display.clear();
                self.reset_holdem_state()?;
                self.reset_player_map_status()?;

                if effect.count_players() >= 2 && effect.count_servers() >= 1 {
                    effect.start_game();
                }

                println!("== Game starts again");
                Ok(())
            }

            Event::Sync { new_players, .. } => {
                self.display.clear();
                effect.allow_exit(true);
                println!("Game allows exit? {}", effect.allow_exit);
                match self.stage {
                    HoldemStage::Init => {
                        for p in new_players.into_iter() {
                            let PlayerJoin {
                                addr,
                                position,
                                balance,
                                ..
                            } = p;
                            let player = Player::new(addr, balance, position);
                            self.player_map.insert(player.addr(), player);
                        }

                        if effect.count_players() >= 2 && effect.count_servers() >= 1 {
                            effect.start_game();
                        }
                    }

                    _ => {
                        for p in new_players.into_iter() {
                            let PlayerJoin {
                                addr,
                                position,
                                balance,
                                ..
                            } = p;
                            let player = Player::init(addr, balance, position);
                            self.player_map.insert(player.addr(), player);
                        }
                    }
                }

                Ok(())
            }

            Event::GameStart { .. } => {
                self.display.clear();

                let next_btn = self.get_next_btn()?;
                println!("Next BTN: {}", next_btn);
                self.btn = next_btn;

                let player_num = self.player_map.len();
                println!("== {} players in game", player_num);

                // Prepare randomness (shuffling cards)
                let rnd_spec = RandomSpec::deck_of_cards();
                self.deck_random_id = effect.init_random_state(rnd_spec);
                Ok(())
            }

            Event::Leave { player_addr } => {
                // TODO: Leaving is not allowed in SNG game
                self.display.clear();
                println!("== Player {} decides to leave game", player_addr);

                let Some(leaving_player) = self.player_map.get_mut(&player_addr) else {
                    return Err(HandleError::Custom(
                        "Player not found in game [Leave]".to_string()
                    ));
                };
                leaving_player.status = PlayerStatus::Leave;

                match self.stage {
                    // If current stage is not playing, the player can
                    // leave with a settlement instantly.
                    HoldemStage::Init
                    | HoldemStage::Settle
                    | HoldemStage::Runner
                    | HoldemStage::Showdown => {
                        self.player_map.remove_entry(&player_addr);
                        effect.settle(Settle::eject(&player_addr));
                        effect.wait_timeout(WAIT_TIMEOUT_DEFAULT);
                        self.signal_game_end()?;
                    }

                    // If current stage is playing, the player will be
                    // marked as `Leave`.  There are 3 cases to
                    // handle:
                    //
                    // 1. The leaving player is the
                    // second last player, so the remaining player
                    // just wins.
                    //
                    // 2. The leaving player is in acting.  In such
                    // case, we just fold this player and do next
                    // state calculation.
                    //
                    // 3. The leaving player is not the acting player,
                    // and the game can continue.
                    //
                    // All these cases are handled in next state.
                    HoldemStage::Play | HoldemStage::ShareKey => {
                        self.next_state(effect)?;
                    }
                }

                Ok(())
            }

            Event::RandomnessReady { .. } => {
                self.display.clear();
                // Cards are dealt to players but remain invisible to them
                for (idx, (addr, player)) in self.player_map.iter().enumerate() {
                    if player.status != PlayerStatus::Init {
                        effect.assign(self.deck_random_id, addr, vec![idx * 2, idx * 2 + 1]);
                        self.hand_index_map
                            .insert(addr.clone(), vec![idx * 2, idx * 2 + 1]);
                    }
                }

                Ok(())
            }

            Event::SecretsReady => match self.stage {
                HoldemStage::ShareKey => {
                    self.display.clear();
                    let players_cnt = self.count_ingame_players() * 2;
                    let board_prev_cnt = self.board.len();
                    self.stage = HoldemStage::Play;

                    match self.street {
                        Street::Preflop => {
                            self.next_state(effect)?;
                        }

                        Street::Flop => {
                            let decryption = effect.get_revealed(self.deck_random_id)?;
                            for i in players_cnt..(players_cnt + 3) {
                                if let Some(card) = decryption.get(&i) {
                                    self.board.push(card.clone());
                                } else {
                                    return Err(HandleError::Custom(
                                        "Failed to reveal the 3 flop cards".to_string(),
                                    ));
                                }
                            }
                            self.display.push(Display::DealBoard {
                                prev: board_prev_cnt,
                                board: self.board.clone(),
                            });

                            self.next_state(effect)?;
                        }

                        Street::Turn => {
                            let decryption = effect.get_revealed(self.deck_random_id)?;
                            let card_index = players_cnt + 3;
                            if let Some(card) = decryption.get(&card_index) {
                                self.board.push(card.clone());
                                self.display.push(Display::DealBoard {
                                    prev: board_prev_cnt,
                                    board: self.board.clone(),
                                });
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
                                self.display.push(Display::DealBoard {
                                    prev: board_prev_cnt,
                                    board: self.board.clone(),
                                });
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

                // Shuffling deck
                HoldemStage::Init => {
                    self.display.clear();
                    match self.street {
                        Street::Init => {
                            self.street = Street::Preflop;
                            self.stage = HoldemStage::Play;
                            self.next_state(effect)?;
                            Ok(())
                        }

                        // if other streets, keep playing
                        _ => Ok(()),
                    }
                }

                // Ending, comparing cards
                HoldemStage::Runner => {
                    self.display.clear();
                    self.update_board(effect)?;
                    self.display.push(Display::DealBoard {
                        prev: 0,
                        board: self.board.clone(),
                    });
                    self.settle(effect)?;

                    effect.wait_timeout(WAIT_TIMEOUT_RUNNER);
                    Ok(())
                }

                // Ending, comparing cards
                HoldemStage::Showdown => {
                    self.display.clear();
                    self.settle(effect)?;
                    effect.wait_timeout(WAIT_TIMEOUT_SHOWDOWN);
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
