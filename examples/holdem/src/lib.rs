use borsh::{BorshDeserialize, BorshSerialize};
use std::collections::HashMap;
use race_core::error::{Error, Result};
use race_core::event::Event;
use race_core::{context::GameContext, engine::GameHandler, types::GameAccount};

mod custom_handlers;
mod holdem_models;
use holdem_models::{HoldemAccount, BetMap, Table, Street, Pots};
// Imported mods used in test section wont be counted as 'used'
// u8 ~ 256
// u16 ~ 65536
// Byte = 8 bit

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Holdem {
    sb: u64,
    bb: u64,
    table: Table,
    street: Street,
    street_bet: BetMap,         // actual street bet is street_bet.street_bet
    pot: Pots,
    // player_id: String,
    // player_nick: String,
    // player_pos: u8,
    // player_chips: u64,
    // player_status: PlayerStatus,
}

// implementing traits GameHandler
impl GameHandler for Holdem {
    fn init_state(_context: &mut GameContext, init_account: GameAccount) -> Result<Self> {
        let account = HoldemAccount::try_from_slice(&init_account.data).unwrap();
        Ok(Self {
            sb: account.sb,
            bb: account.bb,
            table: Table { nft: "abcdefg".into(),
                           btn: 0,
                           name: "RACE".into(),
                           size: 6,
                           rake: 0.02,
                           mode: "cash".into(),
                           token: "USDC11111111111".into(),
            },
            street: Street::Init,
            street_bet: BetMap { street_bet: account.bb, bet_map: HashMap::new()},
            pot: Pots {owner_ids: vec![], winner_ids: vec![], amount: 0},
        })
    }

    // for side effects, return either nothing or error
    fn handle_event(&mut self, context: &mut GameContext, event: Event) -> Result<()> {
        match event {
            // Handle custom events
            Event::Custom(action) => {
                if action == String::from("fold") {
                    // 1. check if player_id in Vec<Player>
                    todo!();
                    // custom_handlers::player_fold(self, context)
                } else {
                    return Err(Error::Custom(String::from("Unknow event type!")));
                }
            },
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
    use race_core::context::{GameStatus, Player};
    use super::*;

    fn create_holdem(ctx: &mut GameContext) -> Holdem {
        let holdem_acct = HoldemAccount {
            sb: 100,
            bb: 200,
            buyin: 2000,
        };
        let v: Vec<u8> = holdem_acct.try_to_vec().unwrap();
        let init_acct = GameAccount {
            addr: "FAKE_ADDR_ON_CHAIN".into(),      // A game's on-chian address
            bundle_addr: String::from("FAKE ADDR"), // ?
            settle_serial: 0,                       // ?
            access_serial: 0,                       // ?
            max_players: 2,
            transactors: vec![],
            players: vec![],                        // Vec<Option<Player>>
            data_len: v.len() as _,                 // ?
            data: v,                                // HoldemAccount data
        };
        // init_acct (GameAccount + HoldemAccount) + GameContext = Holdem
        Holdem::init_state(ctx, init_acct).unwrap()
    }


    #[test]
    pub fn test_init_state() {
        // test
        // let mut test_ctx = GameContext {
        //     game_addr: "fake".into(),
        //     status: todo!(),
        //     players: todo!(),
        //     transactors: todo!(),
        //     dispatch: todo!(),
        //     state_json: todo!(),
        //     timestamp: todo!(),
        //
        // };

        let mut ctx = GameContext::default();

        let holdem = create_holdem(&mut ctx);
        assert_eq!(100, holdem.sb);
        assert_eq!(200, holdem.bb);
        // assert_eq!(String::from("gentoo"), holdem.player_nick);
        // assert_eq!(String::from("qwertyuiop"), holdem.player_id);
        assert_eq!(Street::Init, holdem.street);
    }

    #[test]
    pub fn test_handle_join() {
        let mut ctx = GameContext::default();
        let event = Event::Join { player_addr: "Alice".into(), timestamp: 0 };
        let mut holdem = create_holdem(&mut ctx);
        holdem.handle_event(&mut ctx, event).unwrap();
        // assert_eq!(100, holdem.sb;)
    }

    #[test]
    pub fn test_player_action() {
        // Prepare a fake game context
        let players_list = vec![Player::new(String::from("ALICE")),
                                Player::new(String::from("Bob")),
                                Player::new(String::from("Carol")),
                                Player::new(String::from("qwertyuiop"))];
        let mut ctx = GameContext::default();
        // 3 most relevant fields at the moment
        ctx.game_addr = String::from("fake_race_game_onchain_id");
        ctx.players = players_list;
        ctx.status = GameStatus::Running;

        // Create the holdem game for various events below
        let mut holdem = create_holdem(&mut ctx);

        // Custom Event - fold
        let player_fold = String::from("fold");
        let event_fold = Event::Custom(player_fold);
        let can_fold = holdem.handle_event(&mut ctx, event_fold).unwrap();
        assert_eq!((), can_fold);

        // Custom Event - bet
        // Custom Event - call
        // Custom Event - check
        // Custom Event - raise
    }
}
