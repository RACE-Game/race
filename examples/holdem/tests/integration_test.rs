use race_core::{
    context::{GameContext, GameStatus as GeneralStatus},
    engine::GameHandler,
    error::{Error, Result},
    event::Event,
    types::GameAccount
};

use holdem::*;
use race_core_test::*;

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

    let init_acct = game_account_with_account_data(holdem_acct);

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
    let event = Event::Join { player_addr: "Alice".into(), balance: 1000 };
    let mut holdem = create_holdem(&mut ctx);
    holdem.handle_event(&mut ctx, event).unwrap();
    // assert_eq!(100, holdem.sb;)
}

#[test]
pub fn test_fns() {
    // Scenario One:
    // 4 players in the flop, Bob goes all in and other three build a side pot
    let mut holdem = Holdem {
        sb: 10,
        bb: 20,
        buyin: 400,
        btn: 0,
        rake: 0.02,
        size: 4,
        status: HoldemStatus::Play,
        street: Street::Flop,
        street_bet: 20,
        bet_map: vec![
            Bet::new("Alice", 100),
            Bet::new("Bob", 45),
            Bet::new("Charlie", 100),
            Bet::new("Gentoo", 50),
        ],
        players: vec![
            Player { addr: String::from("Alice"),
                     chips: 1500,
                     position: 0,
                     status: PlayerStatus::Fold },
            // suppose Bob goes all in
            Player { addr: String::from("Bob"),
                     chips: 0,
                     position: 1,
                     status: PlayerStatus::Allin },
            Player { addr: String::from("Charlie"),
                     chips: 1200,
                     position: 2,
                     status: PlayerStatus::Fold },
            Player { addr: String::from("Gentoo"),
                     chips: 1000,
                     position: 3,
                     status: PlayerStatus::Wait },
        ],
        act_idx: usize::from(2u8),
        pots: vec![],
    };

    // test collecting bets to each pot
    let unsettled_pots = holdem.collect_bets();
    assert_eq!(180, unsettled_pots[0].amount());


    // test assigning winner(s) to each pot
    holdem.pots = unsettled_pots;
    let settled_pots = holdem.assign_winners(
        &vec![
            vec![String::from("Gentoo"), String::from("Bob")],
            vec![String::from("Gentoo"), String::new()]
        ]);

    assert_eq!(vec![String::from("Gentoo")], settled_pots[1].winners());
    //
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
    // holdem.init_players(players_list);
    // holdem.set_act_idx(pos);


    // Custom Event - fold
    let evt = Event::custom("Gentoo", &GameEvent::Fold);
    let can_fold = holdem.handle_event(&mut ctx, evt).unwrap();
    assert_eq!((), can_fold);

    // Custom Event - bet
    // Custom Event - call
    // Custom Event - check
    // Custom Event - raise
}
