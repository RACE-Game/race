use std::collections::HashMap;

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use race_proc_macro::game_handler;
use race_core::{
    context::GameContext,
    engine::GameHandler,
    error::{Error, Result},
    event::{CustomEvent, Event},
    random::deck_of_cards,
    types::{GameAccount, RandomId, Settle},
};

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
    pub size: u8,               // table size: total number of players
    pub mode: String,           // game type: cash, sng or tourney?
    pub token: String,          // token should be a struct of its own?
}

impl Default for HoldemAccount {
    fn default() -> Self {
        Self {
            sb: 10,
            bb: 20,
            buyin: 400,
            rake: 0.02,
            size: 6,
            mode: "cash".to_string(),
            token: "sol".to_string(),
        }
    }
}

#[derive(Deserialize, Serialize, Default, PartialEq, Clone, Debug)]
pub enum PlayerStatus {
    #[default]
    Wait,                       // or Idle?
    Acted,
    Acting,                     // or InAction?
    Allin,
    Fold,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub struct Player {
    pub addr: String,
    pub chips: u64,
    pub position: usize,        // zero indexed
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
    pub fn new<T: Into<String>>(id: T, bb: u64, pos: usize) -> Player {
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

    // This fn should do:
    // 1. Indicate whether the sb/bb goes all in due to short of chips
    // 2. Indicate the real bet (<= sb/bb)
    // 3. Update the player's chips in place
    pub fn take_bet(&mut self, bet: u64) -> u64 {
        if bet < self.chips {
            self.chips -= bet;
            bet                // real bet
        } else {
            let chips_left = self.chips;
            self.status = PlayerStatus::Allin;
            self.chips = 0;
            chips_left        // real bet
        }
    }
}


#[derive(Deserialize, Serialize, Default, Clone)]
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

#[derive(Default, Serialize, Deserialize, PartialEq, Clone)]
pub enum HoldemStage {
    #[default]
    Init,
    BlindBet,
    // Encrypt,
    // ShareKey,
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
pub struct Holdem {// GameState Handler
    pub deck_random_id: RandomId,
    pub dealer_idx: usize,
    pub sb: u64,
    pub bb: u64,
    pub buyin: u64,
    pub btn: usize,            // current btn position, zero-indexed?
    pub rake: f32,
    pub size: u8,              // table size: total number of players
    pub stage: HoldemStage,
    // mode: String,           // game type: cash, sng or tourney? Treat as CASH GAME
    // token: String,          // token should be a struct of its own?
    pub street: Street,
    pub street_bet: u64,
    pub seats_map: HashMap<String, usize>,
    // pub community_cards: &'a str,
    pub bets: Vec<Bet>,     // each bet's index maps to player's pos (index) in the below player list
    pub prize_map: HashMap<String, u64>,
    pub players: Vec<Player>,
    pub acting_player: Option<Player>,
    pub pots: Vec<Pot>,        // 1st main pot + rest side pot(s)
}


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

    // pub fn get_acting_player(&mut self) -> Result<Player> {
    //     if let Some(player) = self.acting_player {
    //         Ok(player.clone())
    //     } else {
    //         Err(None)
    //     }
    // }

    fn update_streetbet(&mut self, new_sbet: u64) {
        self.street_bet = new_sbet;
    }

    fn update_bets(&mut self, new_bmap: Vec<Bet>) {
        self.bets.extend_from_slice(&new_bmap);
    }

    pub fn change_street(&mut self, street: Street) -> Result<()> {
        // Reset acted to wait
        for player in &mut self.players {
            if player.status == PlayerStatus::Acted {
                player.status = PlayerStatus::Wait;
            }
        }

        self.collect_bets()?;

        self.street = street;
        self.street_bet = 0;
        self.acting_player = None;

        // TODO:
        // update-require-key-idents
        // take-released-keys
        // dispatch-key-share-timeout
        Ok(())
    }

    pub fn next_street(&mut self) -> Street {
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

    pub fn apply_prize(&mut self) -> Result<()> {
        for ply in &mut self.players {
            match self.prize_map.get(&ply.addr) {
                // If player in the prize map, chips increase by the amt in the map
                Some(prize) => {
                    println!("Found the player and applying prize to their chips ... ");
                    ply.chips += *prize - &self.bets[ply.position].amount();
                },
                // else chips decrease by the amt in the bet map?
                None => {
                    println!("Lost chips");
                    ply.chips = ply.chips - &self.bets[ply.position].amount();
                }
            }
        }
        Ok(())
    }

    pub fn calc_prize(&mut self) -> Result<()> {
        let pots = &self.pots;
        let mut prize_map: HashMap<String, u64> = HashMap::new();

        for pot in pots {
            let cnt: u64 = pot.winners().len() as u64;
            let prize: u64 = pot.amount() / cnt;
            for wnr in pot.winners() {
                prize_map.entry(wnr.clone()).and_modify(|p| *p += prize).or_insert(prize);
            }
        }
        self.prize_map = prize_map;
        Ok(())
    }

    pub fn assign_winners(&mut self, winner_sets: &Vec<Vec<String>>) -> Result<()> {
        let mut pots = self.pots.clone();

        for (idx, pot) in pots.iter_mut().enumerate() {
            let real_wnrs: Vec<String> = pot.owners().iter()
                    .filter(|w| winner_sets[idx].contains(w))
                    .map(|w| (*w).clone())
                    .collect();
            pot.update_winners(real_wnrs);
        }
        self.pots = pots;
        Ok(())
    }

    pub fn collect_bets(&mut self) -> Result<()> {
        // filter bets: arrange from small to big and remove duplicates
        let mut bets: Vec<u64> = self.bets.iter().map(|b| b.amount()).collect();
        bets.sort_by(|b1,b2| b1.cmp(b2));
        bets.dedup();

        let mut pots: Vec<Pot> = vec![];
        let mut prev_bet: u64 = 0;
        // group players by bets and make pots: those who bet the same in the same pot(s)
        for bet in bets {
            let mut owners: Vec<String> = vec![];
            let mut amount: u64 = 0;
            for ply in self.bets.clone() {
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
        self.pots = pots;
        Ok(())
    }

    pub fn ask_for_action(&mut self, player: &mut Player) -> Result<()> {
        player.status = PlayerStatus::Acting;
        self.acting_player = Some(player.clone());
        // TODO: dispatch action timeout event
        Ok(())
    }

    pub fn blind_bets(&mut self) -> Result<()> {
        let players = self.players.clone();

        // sb is btn when only two players
        if players.len() == 2 {
            let mut sb_player = players[1].clone();
            let mut bb_player = players[0].clone();
            let real_sb = sb_player.take_bet(self.sb);
            let real_bb = bb_player.take_bet(self.bb);

            // Update bet map
            self.update_bets(
                vec![Bet::new(sb_player.addr.clone(), real_sb),
                     Bet::new(bb_player.addr.clone(), real_bb)]
        );
            // Next-to-act player is sb player
            self.ask_for_action(&mut sb_player)?;
        } else {
            let mut sb_player = players[0].clone();
            let mut bb_player = players[1].clone();
            let real_sb = sb_player.take_bet(self.sb);
            let real_bb = bb_player.take_bet(self.bb);

            // Update bet map
            self.update_bets(
            vec![Bet::new(sb_player.addr.clone(), real_sb),
                 Bet::new(bb_player.addr.clone(), real_bb)]
            );

            // Select the next
            let mut rest_players = vec![];
            rest_players.extend_from_slice(&players[2..]);
            rest_players.extend_from_slice(&vec![sb_player, bb_player]);

            let action_players: Vec<&Player> = rest_players.iter()
                .filter(|p| p.next_to_act())
                .collect();
            let mut action_player = action_players[0].clone();

            // Ask next player to take act
            self.ask_for_action(&mut action_player)?;
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

    pub fn next_state(&mut self, _context: &mut GameContext) -> Result<()> {
        let all_players = self.players.clone();
        let remain_players: Vec<&Player> = all_players.iter().filter(|p| p.to_remain()).collect();
        let players_toact: Vec<&Player> = all_players.iter().filter(|p| p.to_act()).collect();
        let allin_players: Vec<&Player> = remain_players.iter()
            .filter(|&p| p.status == PlayerStatus::Allin)
            .map(|p| *p)
            .collect();

        let bet_map = self.bets.clone();
        let mut next_action_player = Holdem::next_action_player(self.street_bet, bet_map, players_toact);
        let new_street = self.next_street();

        // Blind bets
        if self.street == Street::Preflop && self.bets.is_empty() {
            self.blind_bets()?;
            Ok(())
        }
        // Single player wins because there are one player only
        else if all_players.len() == 1 {
            let winner = all_players[0].addr.clone();
            println!("Only one player left and won the game!");
            self.single_player_win(&vec![vec![winner]])?;
            Ok(())
        }
        // Singple players wins because others folded
        else if remain_players.len() == 1 {
            let winner = remain_players[0].addr.clone();
            println!("All other players folded and the one left won the game!");
            self.single_player_win(&vec![vec![winner]])?;
            Ok(())
        }
        // Next player to act
        else if !next_action_player.addr.is_empty() {
            println!("Ask next player to act!");
            self.ask_for_action(&mut next_action_player)?;
            Ok(())
        }
        // Runner
        else if self.stage != HoldemStage::Runner && remain_players.len() == allin_players.len() + 1 {
            println!("Entering runner state");
            todo!()
        }
        // Next Street
        else if new_street != Street::Done {
            println!("Move to next street");
            self.change_street(new_street)?;
            Ok(())
        }
        // Showdown
        else {
            todo!()
        }
    }

    fn handle_custom_event(
        &mut self,
        context: &mut GameContext,
        event: GameEvent,
        sender: String
    ) -> Result<()> {
        // let next_action_player: Option<&Player> = remain_players.get(self.act_idx);
        match event {
            GameEvent::Bet(amount) => {
                // TODO: handle other Errors
                if self.street_bet > amount {
                    return Err(Error::Custom(String::from("Player's bet is too small")));}

                if let Some(player_pos) = self.seats_map.get(&sender) {
                    let mut player = self.players[*player_pos].clone();
                    player.take_bet(amount);
                    // TODO: handle other amout of street_bet
                    self.street_bet = amount;
                }
                Ok(())
            }

            GameEvent::Call => {
                // TODO: handle errors

                if let Some(player_pos) = self.seats_map.get(&sender) {
                    let mut player = self.players[*player_pos].clone();
                    // player.take_bet();
                    // TODO: handle other amout of street_bet
                    // self.street_bet = amount;
                }

                Ok(())
            }

            GameEvent::Fold  => {
                // FIXME: refactor this part
                let acting_player = self.acting_player.clone();
                match acting_player {
                    Some(mut player) => {
                        if player.addr == sender {
                            println!("Player is at the action position");
                            player.status = PlayerStatus::Fold;
                            self.acting_player = Some(player);
                        }
                    },
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
            _ => todo!()
        }
    }
}

impl GameHandler for Holdem {
    fn init_state(context: &mut GameContext, init_account: GameAccount) -> Result<Self> {
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
            seats_map: HashMap::new(),
            // community_cards: vec![],
            bets: vec![],    // p[0] is sb, p[1] at bb, p[2] the first to act, p[n-1] (last) btn
            prize_map: HashMap::new(),
            players: vec![],
            pots: vec![
                Pot {owners: vec![], winners: vec![], amount: 0}
            ],
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
                if context.get_players().len() < 2 {
                    return Err(Error::NoEnoughPlayers);
                }
                // TODO: check server numers
                let rnd_spec = deck_of_cards();
                self.deck_random_id = context.init_random_state(&rnd_spec)?;

                // Initializing the game state
                self.stage = HoldemStage::Play;
                self.street = Street::Preflop;
                self.next_state(context)?;
                Ok(())
            }

            Event::Sync { new_players, .. } => {
                // TODO
                for p in new_players.iter() {
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

            Event::RandomnessReady { .. } => {
                let addr0 = context.get_player_by_index(0).unwrap().addr.clone();
                let addr1 = context.get_player_by_index(1).unwrap().addr.clone();
                context.assign(self.deck_random_id, addr0, vec![0, 1])?;
                context.assign(self.deck_random_id, addr1, vec![2, 3])?;
                Ok(())
            }


            // Event::SecretsReady => match self.stage {
            //     HoldemStage::Showdown | HoldemStage::Runner => {
            //         let decryption = context.get_revealed(self.deck_random_id)?;
            //         let player_idx: usize = if self.dealer_idx == 0 { 1 } else { 0 };
            //         let dealer_addr = context
            //             .get_player_by_index(self.dealer_idx)
            //             .unwrap()
            //             .addr
            //             .clone();
            //         let player_addr = context
            //             .get_player_by_index(player_idx)
            //             .unwrap()
            //             .addr
            //             .clone();
            //         let dealer_card = decryption.get(&self.dealer_idx).unwrap();
            //         let player_card = decryption.get(&player_idx).unwrap();
            //         let (winner, loser) = if is_better_than(dealer_card, player_card) {
            //             (dealer_addr, player_addr)
            //         } else {
            //             (player_addr, dealer_addr)
            //         };
            //         context.settle(vec![
            //             Settle::add(winner, self.bet),
            //             Settle::sub(loser, self.bet),
            //         ]);
            //     },
            // _ => {}
            // }
            // Ignore other types of events for now
            _ => Ok(())
        }

    }
}
