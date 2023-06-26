#![allow(dead_code)]
//! Helper functions used in tests
use std::collections::BTreeMap;

use borsh::BorshSerialize;
use race_core::{
    context::GameContext, error::Result, event::Event, prelude::InitAccount, types::PlayerJoin,
};

use race_test::{TestClient, TestGameAccountBuilder, TestHandler};

use crate::essential::*;
use crate::game::*;

// ======================================================
// Heplers for unit tests that focus on holdem game state
// ======================================================
pub fn initial_two_players() -> BTreeMap<String, Player> {
    BTreeMap::from([
        ("Alice".into(), Player::new("Alice".into(), 1000, 0usize)),
        ("Bob".into(), Player::new("Bob".into(), 1000, 1usize)),
    ])
}

pub fn initial_players() -> BTreeMap<String, Player> {
    BTreeMap::from([
        ("Alice".into(), Player::new("Alice".into(), 1000, 0usize)),
        ("Bob".into(), Player::new("Bob".into(), 1000, 1usize)),
        ("Carol".into(), Player::new("Carol".into(), 1000, 2usize)),
        ("Dave".into(), Player::new("Dave".into(), 1000, 3usize)),
        ("Eva".into(), Player::new("Eva".into(), 1000, 4usize)),
        ("Frank".into(), Player::new("Frank".into(), 1000, 5usize)),
    ])
}

impl Player {
    // TODO: consider moving this to lib
    #[allow(dead_code)]
    fn new_with_status(addr: String, chips: u64, position: usize, status: PlayerStatus) -> Player {
        Self {
            addr,
            chips,
            position,
            status,
        }
    }
}

pub fn gaming_players() -> BTreeMap<String, Player> {
    BTreeMap::from([
        (
            "Alice".into(),
            Player::new_with_status("Alice".into(), 1000, 0usize, PlayerStatus::Acting),
        ),
        (
            "Bob".into(),
            Player::new_with_status("Bob".into(), 200, 1usize, PlayerStatus::Acted),
        ),
        (
            "Carol".into(),
            Player::new_with_status("Carol".into(), 0, 2usize, PlayerStatus::Allin),
        ),
        (
            "Dave".into(),
            Player::new_with_status("Dave".into(), 780, 3usize, PlayerStatus::Acted),
        ),
        (
            "Eva".into(),
            Player::new_with_status("Eva".into(), 650, 4usize, PlayerStatus::Acted),
        ),
        (
            "Frank".into(),
            Player::new_with_status("Frank".into(), 800, 5usize, PlayerStatus::Fold),
        ),
    ])
}

pub fn make_even_betmap() -> BTreeMap<String, Bet> {
    BTreeMap::from([
        ("Alice".into(), Bet::new("Alice".into(), 40)),
        ("Bob".into(), Bet::new("Bob".into(), 40)),
        ("Carol".into(), Bet::new("Carol".into(), 40)),
        ("Dave".into(), Bet::new("Dave".into(), 40)),
        ("Eva".into(), Bet::new("Eva".into(), 40)),
    ])
}

pub fn make_uneven_betmap() -> BTreeMap<String, Bet> {
    BTreeMap::from([
        ("Alice".into(), Bet::new("Alice".into(), 20)),
        ("Bob".into(), Bet::new("Bob".into(), 100)),
        ("Carol".into(), Bet::new("Carol".into(), 100)),
        ("Dave".into(), Bet::new("Dave".into(), 60)),
        ("Eva".into(), Bet::new("Eva".into(), 100)),
    ])
}

pub fn make_prize_map() -> BTreeMap<String, u64> {
    BTreeMap::from([("Bob".into(), 220u64), ("Carol".into(), 160u64)])
}

pub fn make_pots() -> Vec<Pot> {
    vec![
        Pot {
            owners: vec![
                "Alice".into(),
                "Bob".into(),
                "Carol".into(),
                "Dave".into(),
                "Eva".into(),
            ],
            winners: vec![],
            amount: 100u64,
        },
        Pot {
            owners: vec!["Bob".into(), "Carol".into(), "Dave".into(), "Eva".into()],
            winners: vec![],
            amount: 120u64,
        },
    ]
}

// Set up a holdem state with multi players joined
pub fn setup_holdem_state() -> Result<Holdem> {
    let players_map = initial_players();
    let mut state = Holdem {
        deck_random_id: 1,
        sb: 10,
        bb: 20,
        min_raise: 20,
        btn: 0,
        rake: 3,
        stage: HoldemStage::Init,
        street: Street::Init,
        street_bet: 20,
        board: Vec::<String>::with_capacity(5),
        hand_index_map: BTreeMap::<String, Vec<usize>>::new(),
        bet_map: BTreeMap::<String, Bet>::new(),
        prize_map: BTreeMap::<String, u64>::new(),
        player_map: players_map,
        players: Vec::<String>::new(),
        pots: Vec::<Pot>::new(),
        acting_player: None,
    };
    state.arrange_players(0usize)?;
    Ok(state)
}

// Set up a holdem state with two players joined
pub fn setup_two_player_holdem() -> Result<Holdem> {
    let players_map = initial_two_players();
    let mut state = Holdem {
        deck_random_id: 1,
        sb: 10,
        bb: 20,
        min_raise: 20,
        btn: 0,
        rake: 3,
        stage: HoldemStage::Init,
        street: Street::Init,
        street_bet: 20,
        board: Vec::<String>::with_capacity(5),
        hand_index_map: BTreeMap::<String, Vec<usize>>::new(),
        bet_map: BTreeMap::<String, Bet>::new(),
        prize_map: BTreeMap::<String, u64>::new(),
        player_map: players_map,
        players: Vec::<String>::new(),
        pots: Vec::<Pot>::new(),
        acting_player: None,
    };
    state.arrange_players(0usize)?;
    Ok(state)
}

// Set up a holdem scene simiar to those in real world
pub fn setup_real_holdem() -> Holdem {
    let mut holdem = setup_holdem_state().unwrap();
    let player_map = gaming_players();
    let bet_map = make_even_betmap();
    let pots = make_pots();
    let board = vec![
        "sa".into(),
        "dt".into(),
        "c9".into(),
        "c2".into(),
        "hq".into(),
    ];
    let prize_map = make_prize_map();
    holdem.bet_map = bet_map;
    holdem.board = board;
    holdem.player_map = player_map;
    holdem.prize_map = prize_map;
    holdem.pots = pots;
    holdem.acting_player = Some(("Bob".into(), 1usize));
    holdem
}

pub fn setup_context() -> GameContext {
    let transactor = TestClient::transactor("foo");
    let game_account = TestGameAccountBuilder::default()
        .set_transactor(&transactor)
        .build();
    let context = GameContext::try_new(&game_account).unwrap();
    context
}

// ====================================================
// Helpers for testing Holdem with the protocol
// ====================================================
type Game = (InitAccount, GameContext, TestHandler<Holdem>, TestClient);

pub fn setup_holdem_game() -> Game {
    let holdem_account = HoldemAccount::default();
    let holdem_data = holdem_account.try_to_vec().unwrap();
    let transactor = TestClient::transactor("foo");
    let mut game_account = TestGameAccountBuilder::default()
        .set_transactor(&transactor)
        .build();
    game_account.data = holdem_data;

    let init_account = InitAccount::from_game_account(&game_account);
    let mut context = GameContext::try_new(&game_account).unwrap();
    let handler = TestHandler::<Holdem>::init_state(&mut context, &game_account).unwrap();
    (init_account, context, handler, transactor)
}

pub fn create_sync_event(
    ctx: &GameContext,
    players: &[&TestClient],
    transactor: &TestClient,
) -> Event {
    let av = ctx.get_access_version() + 1;

    let mut new_players = Vec::new();
    for (i, p) in players.iter().enumerate() {
        new_players.push(PlayerJoin {
            addr: p.get_addr(),
            balance: 10_000,
            position: i as u16,
            access_version: av,
            verify_key: "".into(),
        })
    }

    Event::Sync {
        new_players,
        new_servers: vec![],
        transactor_addr: transactor.get_addr(),
        access_version: av,
    }
}
