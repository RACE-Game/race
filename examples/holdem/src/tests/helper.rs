#![allow(dead_code)]
//! Helper functions used in tests
use std::collections::BTreeMap;

use borsh::BorshSerialize;
use race_core::{
    context::GameContext,
    error::Result,
    event::Event,
    prelude::InitAccount,
    types::{ClientMode, PlayerJoin},
};

use race_test::{
    transactor_account_addr, TestClient, TestGameAccountBuilder, TestHandler,
};

use crate::essential::*;
use crate::game::*;

// ====================================================
// Heplers for unit tests that focus on game state
// ====================================================
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

pub fn gaming_players() -> [(String, Player); 6] {
    [
        ("Alice".into(), Player::new_with_status("Alice".into(), 1000, 0usize, PlayerStatus::Acting)),
        ("Bob".into(), Player::new_with_status("Bob".into(), 1000, 1usize, PlayerStatus::Acted)),
        ("Carol".into(), Player::new_with_status("Carol".into(), 1000, 2usize, PlayerStatus::Allin)),
        ("Dave".into(), Player::new_with_status("Dave".into(), 1000, 3usize, PlayerStatus::Acted)),
        ("Eva".into(), Player::new_with_status("Eva".into(), 1000, 4usize, PlayerStatus::Acted)),
        ("Frank".into(), Player::new_with_status("Frank".into(), 1000, 5usize, PlayerStatus::Fold)),
    ]
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

pub fn setup_holdem_state() -> Result<Holdem> {
    let players_map = initial_players();
    let mut state = Holdem {
        deck_random_id: 1,
        sb: 10,
        bb: 20,
        min_raise: 20,
        btn: 0,
        rake: 3,
        mode: HoldemMode::CASH,
        stage: HoldemStage::Init,
        street: Street::Init,
        street_bet: 0,
        board: Vec::<String>::with_capacity(5),
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

pub fn setup_context() -> GameContext {
    let game_account = TestGameAccountBuilder::default().add_servers(1).build();
    // let init_account = InitAccount::from_game_account(&game_account);
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
    let mut game_account = TestGameAccountBuilder::default().add_servers(1).build();
    game_account.data = holdem_data;

    let init_account = InitAccount::from_game_account(&game_account);
    let mut context = GameContext::try_new(&game_account).unwrap();
    let handler = TestHandler::<Holdem>::init_state(&mut context, &game_account).unwrap();
    let transactor_addr = game_account.transactor_addr.as_ref().unwrap().clone();
    let transactor = TestClient::new(
        transactor_addr.clone(),
        game_account.addr.clone(),
        ClientMode::Transactor,
    );

    (init_account, context, handler, transactor)
}

pub fn create_sync_event(ctx: &GameContext, players: Vec<String>) -> Event {
    let av = ctx.get_access_version() + 1;

    let mut new_players = Vec::new();
    for (i, p) in players.iter().enumerate() {
        new_players.push(PlayerJoin {
            addr: p.into(),
            balance: 10_000,
            position: i as u16,
            access_version: av,
            verify_key: "".into(),
        })
    }

    Event::Sync {
        new_players,
        new_servers: vec![],
        transactor_addr: transactor_account_addr(),
        access_version: av,
    }
}
