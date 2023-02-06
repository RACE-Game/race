use std::collections::HashMap;

use borsh::BorshSerialize;
use race_core::{
    context::{DispatchEvent, GameContext, GameStatus},
    error::{Error, Result},
    event::Event,
    random::RandomStatus,
    types::{ClientMode, PlayerJoin},
};
use race_test::{transactor_account_addr, TestClient, TestGameAccountBuilder, TestHandler};

#[macro_use]
extern crate log;

use holdem::*;

#[test]
pub fn test_holdem() -> Result<()> {
    // Initialize the game with 1 server added.
    let game_acct = TestGameAccountBuilder::default()
        .add_servers(1)
        .build();

    // Create game context and test handler.
    let mut ctx = GameContext::try_new(&game_acct)?;
    let mut holdem_hlr = TestHandler::<Holdem>::init_state(&mut ctx, &game_acct)?;
    assert_eq!(0, ctx.count_players());


    // Initialize the client, which simulates the behavior of transactor.
    let transactor_addr = game_acct.transactor_addr.as_ref().unwrap().clone();
    let mut transactor = TestClient::new(
        transactor_addr.clone(),
        game_acct.addr.clone(),
        ClientMode::Transactor,
    );

    // Initialize two player clients, which simulate the behavior of player.
    let mut alice = TestClient::new(
        "Alice".into(),
        game_acct.addr.clone(),
        ClientMode::Player,
    );
    let mut bob = TestClient::new(
        "Bob".into(),
        game_acct.addr.clone(),
        ClientMode::Player
    );

    // Try to start the "zero-player" game that will fail
    let fail_to_start = ctx.gen_first_event();
    let result = holdem_hlr.handle_event(&mut ctx, &fail_to_start);
    assert_eq!(result, Err(Error::NoEnoughPlayers));

    // Let players join the game and dispatch the `GameStart' event
    let av = ctx.get_access_version() + 1;
    let sync_event = Event::Sync {
        new_players: vec![
            PlayerJoin {
                addr: "Alice".into(),
                balance: 10000,
                position: 0,
                access_version: av,
            },
            PlayerJoin {
                addr: "Bob".into(),
                balance: 10000,
                position: 1,
                access_version: av,
            }
        ],
        new_servers: vec![],
        transactor_addr: transactor_account_addr(),
        access_version: av,
    };

    // Sync info between transactor and context?
    holdem_hlr.handle_event(&mut ctx, &sync_event)?;

    // Start the game
    DispatchEvent::new(
        Event::GameStart { access_version: ctx.get_access_version() },
        0
    );

    {
        assert_eq!(2, ctx.count_players());
        assert_eq!(GameStatus::Uninit, ctx.get_status());
        assert_eq!(
            Some(DispatchEvent {
                timeout: 0,
                event: Event::GameStart { access_version: ctx.get_access_version() },
            }),
            *ctx.get_dispatch());
    }

    // Use this test-only fn to handle the `GameStart' event, as would a transactor
    holdem_hlr.handle_dispatch_event(&mut ctx)?;
    {
        let state: &Holdem = holdem_hlr.get_state();
        assert_eq!(GameStatus::Running, ctx.get_status());
        assert_eq!(1, state.deck_random_id);
        assert_eq!(
            String::from("Alice"),
            state.players[0].addr.clone()
        );
        assert_eq!(
            RandomStatus::Masking(transactor_addr.clone()),
            ctx.get_random_state_unchecked(1).status
        );
    }

    // Since there is one server only, one event will get returned
    let events = transactor.handle_updated_context(&ctx)?;
    {
        assert_eq!(1, transactor.secret_states().len());
        assert_eq!(1, events.len());
    }

    // Send the mask event to handler for `Locking`.
    holdem_hlr.handle_event(&mut ctx, &events[0])?;
    {
        assert_eq!(
            RandomStatus::Locking(transactor_addr.clone()),
            ctx.get_random_state_unchecked(1).status
        );
    }

    // Let the client handle the updated context.
    // One `Lock` event will be created.
    let events = transactor.handle_updated_context(&ctx)?;
    {
        assert_eq!(1, events.len());
    }

    // Send the lock event to handler, we expect the random status to be changed to `Ready`.
    // Since all randomness is ready, an event of `RandomnessReady` will be dispatched.
    holdem_hlr.handle_event(&mut ctx, &events[0])?;
    {
        assert_eq!(
            RandomStatus::Ready,
            ctx.get_random_state_unchecked(1).status
        );
        assert_eq!(
            Some(DispatchEvent::new(Event::RandomnessReady {random_id: 1}, 0)),
            *ctx.get_dispatch()
        );
    }

    // Handle this dispatched `RandomnessReady`: each player gets two cards
    holdem_hlr.handle_dispatch_event(&mut ctx)?;
    // {
    //     let random_state = ctx.get_random_state_unchecked(1);
    //     let ciphertexts_for_alice = random_state.list_assigned_ciphertexts("Alice");
    //     let ciphertexts_for_bob = random_state.list_assigned_ciphertexts("Bob");
    //     assert_eq!(
    //         RandomStatus::WaitingSecrets(transactor_account_addr()),
    //         random_state.status
    //     );
    //     assert_eq!(1, ciphertexts_for_alice.len());
    //     assert_eq!(1, ciphertexts_for_bob.len());
    // }

    Ok(())

}

#[test]
#[ignore]
pub fn test_handle_join() {
    // let mut ctx = GameContext::default();
    // let event = Event::Join { player_addr: "Alice".into(), balance: 1000, position: 3usize };
    // let mut holdem = create_holdem(&mut ctx);
    // holdem.handle_event(&mut ctx, event).unwrap();
    // assert_eq!(100, holdem.sb;)
}

#[test]
#[ignore]
pub fn test_fns() {
    // Bob goes all in with 45 and Gentoo goes all in with 50
    // The other two call 100 and build side pots
    let mut holdem = Holdem {
        deck_random_id: 0,
        dealer_idx: 0,
        sb: 10,
        bb: 20,
        buyin: 400,
        btn: 3,
        rake: 0.02,
        size: 4,
        stage: HoldemStage::Play,
        street: Street::Flop,
        street_bet: 20,
        bets: vec![
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
        pots: vec![],
    };

    // Test the bets collected in each pot
    holdem.collect_bets().unwrap();
    assert_eq!(3, holdem.pots.len());
    assert_eq!(180, holdem.pots[0].amount());
    assert_eq!(15, holdem.pots[1].amount());
    assert_eq!(100, holdem.pots[2].amount());

    // Test num of pots and owners of each pot
    assert_eq!(4, holdem.pots[0].owners().len());
    assert_eq!(
        vec!["Alice".to_string(), "Bob".to_string(), "Carol".to_string(), "Gentoo".to_string()],
        holdem.pots[0].owners()
    );

    assert_eq!(3, holdem.pots[1].owners().len());
    assert_eq!(
        vec!["Alice".to_string(), "Carol".to_string(), "Gentoo".to_string()],
        holdem.pots[1].owners()
    );

    assert_eq!(2, holdem.pots[2].owners().len());
    assert_eq!(
        vec!["Alice".to_string(), "Carol".to_string()],
        holdem.pots[2].owners()
    );


    // Test assigned winner(s) of each pot
    holdem.assign_winners(
        &vec![
            vec![String::from("Gentoo"), String::from("Bob")],
            vec![String::from("Gentoo"), String::new()],
            vec![String::from("Alice"), String::new()],
        ]).unwrap();

    // Test num of winners in each pot
    assert_eq!(2, holdem.pots[0].winners().len());
    assert_eq!(1, holdem.pots[1].winners().len());
    assert_eq!(1, holdem.pots[2].winners().len());

    // Test winner(s) of each pot
    assert_eq!(vec![String::from("Bob"), String::from("Gentoo")], holdem.pots[0].winners());
    assert_eq!(vec![String::from("Gentoo")], holdem.pots[1].winners());
    assert_eq!(vec![String::from("Alice") ], holdem.pots[2].winners());

    // Test prize map of each player
    holdem.calc_prize().unwrap();
    assert_eq!(90u64, holdem.prize_map.get(&"Bob".to_string()).copied().unwrap());
    assert_eq!(105u64, holdem.prize_map.get(&"Gentoo".to_string()).copied().unwrap());
    assert_eq!(100u64, holdem.prize_map.get(&"Alice".to_string()).copied().unwrap());


    // Test chips after applying the prize map
    holdem.apply_prize().unwrap();
    assert_eq!(1500, holdem.players[0].chips);
    assert_eq!(90, holdem.players[1].chips);
    assert_eq!(1100, holdem.players[2].chips);
    assert_eq!(105, holdem.players[3].chips);
}

#[test]
#[ignore]
pub fn test_blind_bets() {
    let mut holdem = Holdem {
        deck_random_id: 0,
        dealer_idx: 0,
        sb: 10,
        bb: 20,
        buyin: 400,
        btn: 3,
        rake: 0.02,
        size: 4,
        stage: HoldemStage::Play,
        street: Street::Preflop,
        street_bet: 0,
        bets: vec![],
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
        pots: vec![],
    };

    // Test blind bets
    // Before blind bets:
    assert_eq!(0, holdem.street_bet);
    // assert_eq!(None, holdem.acting_player.unwrap());
    let init_bet_map: Vec<Bet> = vec![];
    assert_eq!(init_bet_map, holdem.bets);

    // After blind bets
    assert_eq!((), holdem.blind_bets().unwrap());
    assert_eq!(20, holdem.street_bet);
    assert_eq!(
        vec![Bet::new("Alice", 10), Bet::new("Bob", 20)],
        holdem.bets
    );
    assert_eq!(
        String::from("Carol") ,
        holdem.acting_player.unwrap().addr
    );
}

#[test]
#[ignore]
pub fn test_single_player_wins() {
    let mut holdem = Holdem {
        deck_random_id: 0,
        dealer_idx: 0,
        sb: 10,
        bb: 20,
        buyin: 400,
        btn: 3,
        rake: 0.02,
        size: 4,
        stage: HoldemStage::Play,
        street: Street::Preflop,
        street_bet: 0,
        bets: vec![
            Bet::new("Alice", 40),
            Bet::new("Bob", 40),
            Bet::new("Carol", 40),
            Bet::new("Gentoo", 40),
        ],
        prize_map: HashMap::new(),
        players: vec![
            Player { addr: String::from("Alice"),
                     chips: 400,
                     position: 0,
                     status: PlayerStatus::Acted },
            // suppose Bob goes all in
            Player { addr: String::from("Bob"),
                     chips: 400,
                     position: 1,
                     status: PlayerStatus::Acted },
            Player { addr: String::from("Carol"),
                     chips: 400,
                     position: 2,
                     status: PlayerStatus::Acted },
            Player { addr: String::from("Gentoo"),
                     chips: 400,
                     position: 3,
                     status: PlayerStatus::Acted },
        ],
        acting_player: None,
        pots: vec![],
    };

    assert_eq!((), holdem.single_player_win(&vec![vec!["Gentoo".to_string()]]).unwrap());
    assert_eq!(1, holdem.pots.len());
    assert_eq!(4, holdem.pots[0].owners().len());
    assert_eq!(160, holdem.pots[0].amount());
    assert_eq!(160, holdem.prize_map.get(&"Gentoo".to_string()).copied().unwrap());
    assert_eq!(520, holdem.players[3].chips);
}

#[test]
#[ignore]
pub fn tstageange_street() {
    let mut holdem = Holdem {
        deck_random_id: 0,
        dealer_idx: 0,
        sb: 10,
        bb: 20,
        buyin: 400,
        btn: 3,
        rake: 0.1,
        size: 4,
        stage: HoldemStage::Play,
        street: Street::Preflop,
        street_bet: 20,
        bets: vec![
            // Bet::new("Alice", 40),
            Bet::new("Bob", 40),
            Bet::new("Carol", 40),
            Bet::new("Gentoo", 40),
        ],
        prize_map: HashMap::new(),
        players: vec![
            Player { addr: String::from("Alice"),
                     chips: 400,
                     position: 0,
                     status: PlayerStatus::Fold },
            // suppose Bob goes all in
            Player { addr: String::from("Bob"),
                     chips: 400,
                     position: 1,
                     status: PlayerStatus::Acted },
            Player { addr: String::from("Carol"),
                     chips: 400,
                     position: 2,
                     status: PlayerStatus::Acted },
            Player { addr: String::from("Gentoo"),
                     chips: 400,
                     position: 3,
                     status: PlayerStatus::Acted },
        ],
        acting_player: Some(Player
                            { addr: String::from("Gentoo"),
                              chips: 400,
                              position: 3,
                              status: PlayerStatus::Acted }),
        pots: vec![],
    };

    let next_street = holdem.next_street();
    assert_eq!((), holdem.change_street(next_street).unwrap());
    assert_eq!(PlayerStatus::Wait, holdem.players[2].status);

    assert_eq!(0, holdem.street_bet);
    assert_eq!(Street::Flop, holdem.street);
}

#[test]
#[ignore]
pub fn test_next_state() {
    let mut ctx = GameContext::default();
    // Modify the fields to fall into different states
    // Below is an example for tesing blind bets, similiar to test_blind_bets() above
    let mut holdem = Holdem {
        deck_random_id: 0,
        dealer_idx: 0,
        sb: 10,
        bb: 20,
        buyin: 400,
        btn: 3,
        rake: 0.1,
        size: 4,
        stage: HoldemStage::Play,
        street: Street::Preflop,
        street_bet: 0,
        bets: vec![
            // Bet::new("Alice", 40),
            // Bet::new("Bob", 40),
            // Bet::new("Carol", 40),
            // Bet::new("Gentoo", 40),
        ],
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
        pots: vec![],
    };

    assert_eq!((), holdem.next_state(&mut ctx).unwrap());

}

#[test]
#[ignore]
pub fn test_player_event() {}
