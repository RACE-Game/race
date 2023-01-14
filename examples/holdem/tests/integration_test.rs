use std::collections::HashMap;

use borsh::BorshSerialize;
use race_core::{
    context::{GameContext, GameStatus as GeneralStatus},
    engine::GameHandler,
    error::{Error, Result},
    event::Event,
    types::GameAccount
};

use holdem::*;
use race_test::*;

fn create_holdem(ctx: &mut GameContext) -> Holdem {

    let holdem_acct = HoldemAccount {
        sb: 50,
        bb: 100,
        buyin: 2000,
        rake: 0.02,
        size: 6,
        mode: String::from("cash"),
        token: String::from("USDC1234567890"),
    };

    let v: Vec<u8> = holdem_acct.try_to_vec().unwrap();

    let init_acct = GameAccount {
        addr: String::from("FAKE"),
        bundle_addr: String::from("FAKE"),
        settle_version: 0,
        access_version: 0,
        players: vec![],
        deposits: vec![],
        server_addrs: vec![],
        transactor_addr: Some(String::from("FAKE")),
        max_players: 8,
        data_len: v.len() as _,
        data: v,
    };

    // init_acct (GameAccount + HoldemAccount) + GameContext = Holdem
    Holdem::init_state(ctx, init_acct).unwrap()
}


#[test]
#[ignore]
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
#[ignore]
pub fn test_handle_join() {
    let mut ctx = GameContext::default();
    let event = Event::Join { player_addr: "Alice".into(), balance: 1000, position: 3usize };
    let mut holdem = create_holdem(&mut ctx);
    holdem.handle_event(&mut ctx, event).unwrap();
    // assert_eq!(100, holdem.sb;)
}

#[test]
pub fn test_fns() {
    // Bob goes all in with 45 and Gentoo goes all in with 50
    // The other two call 100 and build side pots
    let mut holdem = Holdem {
        sb: 10,
        bb: 20,
        buyin: 400,
        btn: 3,
        rake: 0.02,
        size: 4,
        status: HoldemStatus::Play,
        street: Street::Preflop,
        street_bet: 20,
        bet_map: vec![
            Bet::new("Alice", 100),
            Bet::new("Bob", 45),
            Bet::new("Carol", 100),
            Bet::new("Gentoo", 50),
        ],
        prize_map: HashMap::new(),
        players: vec![
            Player { addr: String::from("Alice"),
                     chips: 1500,
                     position: 0,
                     status: PlayerStatus::Fold },
            // suppose Bob goes all in
            Player { addr: String::from("Bob"),
                     chips: 45,
                     position: 1,
                     status: PlayerStatus::Allin },
            Player { addr: String::from("Carol"),
                     chips: 1200,
                     position: 2,
                     status: PlayerStatus::Fold },
            Player { addr: String::from("Gentoo"),
                     chips: 50,
                     position: 3,
                     status: PlayerStatus::Wait },
        ],
        acting_player: None,
        act_idx: 2,
        pots: vec![],
    };

    // Test the bets collected in each pot
    let unsettled_pots = holdem.collect_bets();
    assert_eq!(3, unsettled_pots.len());         // passed
    assert_eq!(180, unsettled_pots[0].amount()); // passed
    assert_eq!(15, unsettled_pots[1].amount());  // passed
    assert_eq!(100, unsettled_pots[2].amount()); // passed

    // Test num of pots and owners of each pot
    assert_eq!(4, unsettled_pots[0].owners().len());
    assert_eq!(
        vec!["Alice".to_string(), "Bob".to_string(), "Carol".to_string(), "Gentoo".to_string()],
        unsettled_pots[0].owners()
    ); // passed

    assert_eq!(3, unsettled_pots[1].owners().len());
    assert_eq!(
        vec!["Alice".to_string(), "Carol".to_string(), "Gentoo".to_string()],
        unsettled_pots[1].owners()
    ); // passed

    assert_eq!(2, unsettled_pots[2].owners().len());
    assert_eq!(
        vec!["Alice".to_string(), "Carol".to_string()],
        unsettled_pots[2].owners()
    ); // passed


    // Test assigned winner(s) of each pot
    holdem.pots = unsettled_pots;
    let settled_pots = holdem.assign_winners(
        &vec![
            vec![String::from("Gentoo"), String::from("Bob")],
            vec![String::from("Gentoo"), String::new()],
            vec![String::from("Alice"), String::new()],
        ]);

    // Test num of winners in each pot
    assert_eq!(2, settled_pots[0].winners().len()); // passed
    assert_eq!(1, settled_pots[1].winners().len()); // passed
    assert_eq!(1, settled_pots[2].winners().len()); // passed

    // Test winner(s) of each pot
    assert_eq!(vec![String::from("Bob"), String::from("Gentoo")], settled_pots[0].winners()); // passed
    assert_eq!(vec![String::from("Gentoo")], settled_pots[1].winners()); // passed
    assert_eq!(vec![String::from("Alice") ], settled_pots[2].winners()); // passed

    // Test prize map of each player
    holdem.pots = settled_pots;
    let prize_map = holdem.calc_prize();
    assert_eq!(90u64, prize_map.get(&"Bob".to_string()).copied().unwrap());     // passed
    assert_eq!(105u64, prize_map.get(&"Gentoo".to_string()).copied().unwrap()); // passed
    assert_eq!(100u64, prize_map.get(&"Alice".to_string()).copied().unwrap());  // passed


    // Test chips after applying the prize map
    holdem.prize_map = prize_map;
    holdem.apply_prize();
    assert_eq!(1500, holdem.players[0].chips); // passed
    assert_eq!(90, holdem.players[1].chips);   // passed
    assert_eq!(1100, holdem.players[2].chips); // passed
    assert_eq!(105, holdem.players[3].chips);  // passed
}

#[test]
pub fn test_blind_bets() {
    let mut holdem = Holdem {
        sb: 10,
        bb: 20,
        buyin: 400,
        btn: 3,
        rake: 0.02,
        size: 4,
        status: HoldemStatus::Play,
        street: Street::Preflop,
        street_bet: 0,
        bet_map: vec![],
        prize_map: HashMap::new(),
        players: vec![
            Player { addr: String::from("Alice"),
                     chips: 400,
                     position: 0,
                     status: PlayerStatus::Wait },
            // suppose Bob goes all in
            Player { addr: String::from("Bob"),
                     chips: 400,
                     position: 1,
                     status: PlayerStatus::Wait },
            Player { addr: String::from("Carol"),
                     chips: 400,
                     position: 2,
                     status: PlayerStatus::Wait },
            Player { addr: String::from("Gentoo"),
                     chips: 400,
                     position: 3,
                     status: PlayerStatus::Wait },
        ],
        acting_player: None,
        act_idx: 0,
        pots: vec![],
    };

    // Test blind bets
    // Before blind bets:
    assert_eq!(0, holdem.street_bet);
    // assert_eq!(None, holdem.acting_player.unwrap());
    let init_bet_map: Vec<Bet> = vec![];
    assert_eq!(init_bet_map, holdem.bet_map);

    // After blind bets
    assert_eq!((), holdem.blind_bets().unwrap()); // passed
    assert_eq!(0, holdem.street_bet);             // passed
    assert_eq!(
        vec![Bet::new("Alice", 10), Bet::new("Bob", 20)],
        holdem.bet_map
    );                                            // passed
    assert_eq!(
        String::from("Carol") ,
        holdem.acting_player.unwrap().addr
    );                                            // passed
}

#[test]
#[ignore]
pub fn test_next_state() {
    // Initialize the game context
    // let mut ctx = GameContext::default();
    // ctx.set_game_status(GeneralStatus::Running);
    // ctx.add_player("Alice", 10000).unwrap();
    // ctx.add_player("Bob", 10000).unwrap();
    // ctx.add_player("Charlie", 10000).unwrap();
    // ctx.add_player("Gentoo", 10000).unwrap();

    // let next_game_state = holdem.next_state(&mut ctx).unwrap();
    // assert_eq!((), next_game_state);

}

#[test]
#[ignore]
pub fn test_player_event() {}
