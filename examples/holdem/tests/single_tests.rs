#![allow(unused_variables)]               // Remove these two later
#![allow(warnings)]
use holdem::*;
use race_core::context::GameContext;
use std::collections::{BTreeMap, HashMap};

// The unit tests in this file test Holdem specific single functions,
// such as fns modifying pots, players, chips and so on.
// For the complete testing of Holdem games, see integration_tests.rs in the same dir.

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
        min_raise: 20,
        buyin: 400,
        btn: 3,
        rake: 0.02,
        size: 4,
        stage: HoldemStage::Play,
        street: Street::Flop,
        street_bet: 20,
        board: Vec::with_capacity(5),
        seats_map: HashMap::<String, usize>::new(),
        player_map: BTreeMap::<String, Player>::new(),
        bet_map: HashMap::new(),
        // bets: vec![
        //     Bet::new("Alice", 100),
        //     Bet::new("Bob", 45),
        //     Bet::new("Carol", 100),
        //     Bet::new("Gentoo", 50),
        // ],
        prize_map: HashMap::new(),
        players: vec![
            Player {
                addr: String::from("Alice"),
                chips: 1500,
                position: 0,
                status: PlayerStatus::Fold,
            },
            // suppose Bob goes all in
            Player {
                addr: String::from("Bob"),
                chips: 45,
                position: 1,
                status: PlayerStatus::Allin,
            },
            Player {
                addr: String::from("Carol"),
                chips: 1200,
                position: 2,
                status: PlayerStatus::Fold,
            },
            Player {
                addr: String::from("Gentoo"),
                chips: 50,
                position: 3,
                status: PlayerStatus::Wait,
            },
        ],
        acting_player: None,
        pots: vec![],
    };

    // Test the bets collected in each pot
    holdem.collect_bets().unwrap();
    assert_eq!(3, holdem.pots.len());
    assert_eq!(180, holdem.pots[0].amount);
    assert_eq!(15, holdem.pots[1].amount);
    assert_eq!(100, holdem.pots[2].amount);

    // Test num of pots and owners of each pot
    assert_eq!(4, holdem.pots[0].owners.len());
    assert_eq!(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Carol".to_string(),
            "Gentoo".to_string()
        ],
        holdem.pots[0].owners
    );

    assert_eq!(3, holdem.pots[1].owners.len());
    assert_eq!(
        vec![
            "Alice".to_string(),
            "Carol".to_string(),
            "Gentoo".to_string()
        ],
        holdem.pots[1].owners
    );

    assert_eq!(2, holdem.pots[2].owners.len());
    assert_eq!(
        vec!["Alice".to_string(), "Carol".to_string()],
        holdem.pots[2].owners
    );

    // Test assigned winner(s) of each pot
    holdem
        .assign_winners(&vec![
            vec![String::from("Gentoo"), String::from("Bob")],
            vec![String::from("Gentoo"), String::new()],
            vec![String::from("Alice"), String::new()],
        ])
        .unwrap();

    // Test num of winners in each pot
    assert_eq!(2, holdem.pots[0].winners.len());
    assert_eq!(1, holdem.pots[1].winners.len());
    assert_eq!(1, holdem.pots[2].winners.len());

    // Test winner(s) of each pot
    assert_eq!(
        vec![String::from("Bob"), String::from("Gentoo")],
        holdem.pots[0].winners
    );
    assert_eq!(vec![String::from("Gentoo")], holdem.pots[1].winners);
    assert_eq!(vec![String::from("Alice")], holdem.pots[2].winners);

    // Test prize map of each player
    holdem.calc_prize().unwrap();
    assert_eq!(
        90u64,
        holdem.prize_map.get(&"Bob".to_string()).copied().unwrap()
    );
    assert_eq!(
        105u64,
        holdem
            .prize_map
            .get(&"Gentoo".to_string())
            .copied()
            .unwrap()
    );
    assert_eq!(
        100u64,
        holdem.prize_map.get(&"Alice".to_string()).copied().unwrap()
    );

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
    let mut ctx = GameContext::default();
    let mut holdem = Holdem {
        deck_random_id: 0,
        dealer_idx: 0,
        sb: 10,
        bb: 20,
        min_raise: 20,
        buyin: 400,
        btn: 3,
        rake: 0.02,
        size: 4,
        stage: HoldemStage::Play,
        street: Street::Preflop,
        board: Vec::with_capacity(5),
        seats_map: HashMap::<String, usize>::new(),
        player_map: BTreeMap::<String, Player>::new(),
        street_bet: 0,
        bet_map: HashMap::new(),
        prize_map: HashMap::new(),
        players: vec![
            Player {
                addr: String::from("Alice"),
                chips: 400,
                position: 0,
                status: PlayerStatus::Wait,
            },
            // suppose Bob goes all in
            Player {
                addr: String::from("Bob"),
                chips: 400,
                position: 1,
                status: PlayerStatus::Wait,
            },
            Player {
                addr: String::from("Carol"),
                chips: 400,
                position: 2,
                status: PlayerStatus::Wait,
            },
            Player {
                addr: String::from("Gentoo"),
                chips: 400,
                position: 3,
                status: PlayerStatus::Wait,
            },
        ],
        acting_player: None,
        pots: vec![],
    };

    // Test blind bets
    // Before blind bets:
    // assert_eq!(0, holdem.street_bet);
    // assert_eq!(None, holdem.actinbet_mapayer.unwrap());
    let init_bet_map: Vec<Bet> = vec![];
    // assert_eq!(init_bet_map, holdem.bets);

    // After blind bets
    assert_eq!((), holdem.blind_bets(&mut ctx).unwrap());
    assert_eq!(20, holdem.street_bet);
    // assert_eq!(bet_map
    //     vec![Bet::new("Alice", 10), Bet::new("Bob", 20)],
    //     holdem.bets
    // );
    assert_eq!(String::from("Carol"), holdem.acting_player.unwrap());
}

#[test]
#[ignore]
pub fn test_single_player_wins() {
    let mut holdem = Holdem {
        deck_random_id: 0,
        dealer_idx: 0,
        sb: 10,
        bb: 20,
        min_raise: 20,
        buyin: 400,
        btn: 3,
        rake: 0.02,
        size: 4,
        stage: HoldemStage::Play,
        board: Vec::with_capacity(5),
        player_map: BTreeMap::<String, Player>::new(),
        street: Street::Preflop,
        street_bet: 0,
        bet_map: HashMap::new(),
        seats_map: HashMap::new(),
        // bets: vec![
        //     Bet::new("Alice", 40),
        //     Bet::new("Bob", 40),
        //     Bet::new("Carol", 40),
        //     Bet::new("Gentoo", 40),
        // ],

        prize_map: HashMap::new(),
        players: vec![
            Player {
                addr: String::from("Alice"),
                chips: 400,
                position: 0,
                status: PlayerStatus::Acted,
            },
            // suppose Bob goes all in
            Player {
                addr: String::from("Bob"),
                chips: 400,
                position: 1,
                status: PlayerStatus::Acted,
            },
            Player {
                addr: String::from("Carol"),
                chips: 400,
                position: 2,
                status: PlayerStatus::Acted,
            },
            Player {
                addr: String::from("Gentoo"),
                chips: 400,
                position: 3,
                status: PlayerStatus::Acted,
            },
        ],
        acting_player: None,
        pots: vec![],
    };

    assert_eq!(
        (),
        holdem
            .single_player_win(&vec![vec!["Gentoo".to_string()]])
            .unwrap()
    );
    assert_eq!(1, holdem.pots.len());
    assert_eq!(4, holdem.pots[0].owners.len());
    assert_eq!(160, holdem.pots[0].amount);
    assert_eq!(
        160,
        holdem
            .prize_map
            .get(&"Gentoo".to_string())
            .copied()
            .unwrap()
    );
    assert_eq!(520, holdem.players[3].chips);
}

#[test]
#[ignore]
pub fn test_new_street() {
    let mut holdem = Holdem {
        deck_random_id: 0,
        dealer_idx: 0,
        sb: 10,
        bb: 20,
        min_raise: 20,
        buyin: 400,
        btn: 3,
        rake: 0.1,
        size: 4,
        stage: HoldemStage::Play,
        street: Street::Preflop,
        street_bet: 20,
        seats_map: HashMap::new(),
        board: Vec::<String>::with_capacity(5),
        bet_map: HashMap::<String, Bet>::new(),
        player_map: BTreeMap::<String, Player>::new(),
        // bets: vec![
        //     // Bet::new("Alice", 40),
        //     Bet::new("Bob", 40),
        //     Bet::new("Carol", 40),
        //     Bet::new("Gentoo", 40),
        // ],
        prize_map: HashMap::new(),
        players: vec![
            Player {
                addr: String::from("Alice"),
                chips: 400,
                position: 0,
                status: PlayerStatus::Fold,
            },
            // suppose Bob goes all in
            Player {
                addr: String::from("Bob"),
                chips: 400,
                position: 1,
                status: PlayerStatus::Acted,
            },
            Player {
                addr: String::from("Carol"),
                chips: 400,
                position: 2,
                status: PlayerStatus::Acted,
            },
            Player {
                addr: String::from("Gentoo"),
                chips: 400,
                position: 3,
                status: PlayerStatus::Acted,
            },
        ],
        acting_player: Some("Gentoo".to_string()),
        pots: vec![],
    };

    let next_street = holdem.next_street();
    // assert_eq!((), holdem.change_street(ctx, next_street).unwrap());
    // assert_eq!(PlayerStatus::Wait, holdem.players[2].status);
    //
    // assert_eq!(0, holdem.street_bet);
    // assert_eq!(Street::Flop, holdem.street);
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
        min_raise: 20,
        buyin: 400,
        size: 6,
        btn: 3,
        rake: 0.1,
        stage: HoldemStage::Play,
        street: Street::Preflop,
        street_bet: 0,
        seats_map: HashMap::new(),
        player_map: BTreeMap::<String, Player>::new(),
        board: Vec::<String>::with_capacity(5),
        bet_map: HashMap::<String, Bet>::new(),
        // bets: vec![
            // Bet::new("Alice", 40),
            // Bet::new("Bob", 40),
            // Bet::new("Carol", 40),
            // Bet::new("Gentoo", 40),
        // ],
        prize_map: HashMap::new(),
        players: vec![
            Player {
                addr: String::from("Alice"),
                chips: 400,
                position: 0,
                status: PlayerStatus::Wait,
            },
            // suppose Bob goes all in
            Player {
                addr: String::from("Bob"),
                chips: 400,
                position: 1,
                status: PlayerStatus::Wait,
            },
            Player {
                addr: String::from("Carol"),
                chips: 400,
                position: 2,
                status: PlayerStatus::Wait,
            },
            Player {
                addr: String::from("Gentoo"),
                chips: 400,
                position: 3,
                status: PlayerStatus::Wait,
            },
        ],
        acting_player: None,
        pots: vec![],
    };
    assert_eq!((), holdem.next_state(&mut ctx).unwrap());
}
