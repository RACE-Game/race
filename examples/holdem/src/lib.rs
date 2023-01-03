use borsh::{BorshDeserialize, BorshSerialize};
use race_core::error::{Error, Result};
use race_core::event::Event;
use race_core::{context::{GameContext, GameStatus as GeneralStatus}, engine::GameHandler, types::GameAccount};
use serde::{Deserialize, Serialize};
use race_proc_macro::game_handler;

mod holdem_models;
use holdem_models::{HoldemAccount, Bet, GameEvent, GameStatus as HoldemStatus, Street, Pot, Player, PlayerStatus};
// Imported mods used in test section wont be counted as 'used'
// u8 ~ 256
// u16 ~ 65536
// Byte = 8 bit

#[game_handler]
#[derive(Deserialize, Serialize, Clone)]
pub struct Holdem {// must have random cards id
    sb: u64,
    bb: u64,
    buyin: u64,
    btn: u8,                // current btn position, zero-indexed?
    rake: f32,
    size: u8,               // table size: total number of players
    status: HoldemStatus,
    // mode: String,           // game type: cash, sng or tourney? Treat as CASH GAME
    // token: String,          // token should be a struct of its own?
    street: Street,
    street_bet: u64,
    bet_map: Vec<Bet>,   // each bet's index maps to player's pos (index) in the below player list
    players: Vec<Player>,
    act_idx: usize,         // position of the player who should take action
    pot: Pot,
}

// TODO:
// handle game start event

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
        let street = self.street.clone();
        match street {
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

    fn next_action_player(street_bet: u64, bet_map: Vec<Bet>, players_toact: Vec<&Player>) -> Player {
        for p in players_toact {
            if let Some(player) = bet_map.get(usize::from(p.position)) {
                if player.get_bet_amount() < street_bet || p.status == PlayerStatus::Wait {
                    return p.clone();
                }
            }
        }
        return Player::default();
    }

    fn next_state(&mut self, context: &mut GameContext) -> Result<()> {
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
        match self.street {
            Street::Preflop => {
                if self.bet_map.is_empty() {
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
                        .filter(|p| p.to_take_action())
                        .collect();

                    if let Some(player) = action_players.get(0) {
                        // skip pot for now
                        println!("Blind bets!");
                        todo!()
                    }
                }
            }
            _ => {
                println!("Unhandled street");
                todo!()
            }
        }

        // Single player wins
        if remain_players.len() == 1 {
            println!("All other players folded and the one left won the game!");
        } else if all_players.len() == 1 {
            println!("Only one player left and won the game!");
        }

        // Next player to act
        if !next_action_player.addr.is_empty() {
            println!("Ask next player to act!");
        }

        // Runner
        match self.status {
            HoldemStatus::Runner => { println!("Already runner"); },
            _ => {
                if remain_players.len() == allin_players.len() + 1 {
                    println!("Entering runner state");
                }
            }
        }

        // New street
        {
            let new_street = self.next_street();
            // change_street(new_street);
            todo!()
        }
        // Showdown
    }

    // implement methods for Holdem as handlers for custom events
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

                // 4. go to next game state, which is one of the following:
                // 4.1 detect current street: preflop + no bets => blind bets
                // 4.2 detect player number: 1 or a few?
                // 4.3 single player win? (All other players folded)
                // 4.4 next player to act (none of the above)
                // 4.5 runner
                // 4.6 new street or
                // 4.7 showdown

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
            pot: Pot {owners: vec![], winners: vec![], amount: 0},
            act_idx: usize::from(0u8), // starts from last in Vec<Player>
        })
    }

    // for side effects, return either nothing or error
    fn handle_event(&mut self, context: &mut GameContext, event: Event) -> Result<()> {
        match event {
            // Handle custom events
            Event::Custom { sender, raw } => {
                // 1. Check if event sender is in the players
                if let Some(_) = context.players().iter().find(|&p| p.addr == sender) {
                    println!("Valid player");
                } else {
                    return Err(Error::Custom(String::from("Unknown player!")));
                }

                // 2. Check game status is valid (running)?
                match context.status() {
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

// -- tests ----------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn create_holdem(ctx: &mut GameContext) -> Holdem {

        let holdem_acct = HoldemAccount {
            sb: 50,
            bb: 100,
            buyin: 2000,
            btn: 0,
            rake: 0.02,
            size: 6,
            mode: String::from("cash"),
            token: String::from("USDC1234567890"),
        };

        let v: Vec<u8> = holdem_acct.try_to_vec().unwrap();

        let init_acct = GameAccount {
            addr: "FAKE_ADDR_ON_CHAIN".into(),      // A game's on-chian address
            bundle_addr: String::from("FAKE ADDR"), // Address linked to Holdem game (players, street, pots, etc)
            settle_version: 0,
            access_version: 0,
            players: vec![],                        // Vec<Option<Player>>
            transactors: vec![],
            max_players: 6,
            data_len: v.len() as _,                 // ?
            data: v,
            served: true,
        };
        // init_acct (GameAccount + HoldemAccount) + GameContext = Holdem
        Holdem::init_state(ctx, init_acct).unwrap()
    }


    #[test]
    pub fn test_init_state() {

        let mut ctx = GameContext::default();
        let holdem = create_holdem(&mut ctx);
        assert_eq!(50, holdem.sb);
        assert_eq!(100, holdem.bb);
        // assert_eq!(String::from("gentoo"), holdem.player_nick);
        // assert_eq!(String::from("qwertyuiop"), holdem.player_id);
        assert_eq!(Street::Init, holdem.street);
    }

    #[test]
    pub fn test_handle_join() {
        let mut ctx = GameContext::default();
        let event = Event::Join { player_addr: "Alice".into(), balance: 1000 };
        let mut holdem = create_holdem(&mut ctx);
        holdem.handle_event(&mut ctx, event).unwrap();
        // assert_eq!(100, holdem.sb;)
    }

    #[test]
    pub fn test_next_state() {
        // Set up the game context
        let mut ctx = GameContext::default();
        ctx.set_game_status(GeneralStatus::Running);
        ctx.add_player("Alice", 10000).unwrap();
        ctx.add_player("Bob", 10000).unwrap();
        ctx.add_player("Charlie", 10000).unwrap();
        ctx.add_player("Gentoo", 10000).unwrap();

        // Set up the holdem game state
        let mut holdem = create_holdem(&mut ctx);

        let mut players_list: Vec<Player> = vec![]; // player list
        let mut pos: u8 = 0;                        // player position
        for p in ctx.players() {
            players_list.push(
                Player::new(p.addr.clone(), holdem.bb, pos)
            );
            pos += 1;
        }
        holdem.init_players(players_list);
        holdem.set_act_idx(pos);
        holdem.street = Street::Preflop;
        // match holdem.street {
        //     Street::River => {
        //         println!("River already");
        //     },
        //     Street::Done => {
        //         println!("Should settle");
        //     },
        //     _ => { holdem.street = holdem.next_street();}
        // }

        // blind bets
        let next_game_state = holdem.next_state(&mut ctx).unwrap();
        assert_eq!((), next_game_state);
        // single player win (1 player left)

        // single player win (All other players folded)

        // next player to act

        // runner

        // new street

        // showdown
    }

    #[test]
    pub fn test_player_event() {
        // Set up the game context
        let mut ctx = GameContext::default();
        ctx.set_game_status(GeneralStatus::Running);
        ctx.add_player("Alice", 10000).unwrap();
        ctx.add_player("Bob", 10000).unwrap();
        ctx.add_player("Charlie", 10000).unwrap();
        ctx.add_player("Gentoo", 10000).unwrap();

        // Set up the holdem game state
        let mut holdem = create_holdem(&mut ctx);

        let mut players_list: Vec<Player> = vec![]; // player list
        let mut pos: u8 = 0;                        // player position
        for p in ctx.players() {
            players_list.push(
                Player::new(p.addr.clone(), holdem.bb, pos)
            );
            pos += 1;
        }
        holdem.init_players(players_list);
        holdem.set_act_idx(pos);


        // Custom Event - fold
        let evt = Event::custom("Gentoo", &GameEvent::Fold);
        let can_fold = holdem.handle_event(&mut ctx, evt).unwrap();
        assert_eq!((), can_fold);

        // Custom Event - bet
        // Custom Event - call
        // Custom Event - check
        // Custom Event - raise
    }
}
