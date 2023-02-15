#![allow(unused_variables)]               // Remove these two later
#![allow(warnings)]

// use borsh::BorshSerialize;
use race_core::{
    context::{DispatchEvent, GameContext, GameStatus},
    error::{Error, Result},

    event::Event,
    random::RandomStatus,
    types::{ClientMode, PlayerJoin},
};
use race_test::{transactor_account_addr, TestClient, TestGameAccountBuilder, TestHandler};
use std::collections::HashMap;

#[macro_use]
extern crate log;
use holdem::*;

// In a game, there are generally two types of events:
// 1. Events dispatched from within the context, such as GameStart, ShareScrets, etc.
// 2. Events dispatched by clients or transactors, because they see the updated context
// Note the handle_event function in the below test.
// It accepts an event as ref, borrowing it. This is TestHandler's impl.
#[test]
pub fn test_holdem() -> Result<()> {
    // ------------------------- SETUP ------------------------
    // Initialize the game with 1 server added.
    let game_acct = TestGameAccountBuilder::default().add_servers(1).build();

    // Create game context and test handler.
    let mut ctx = GameContext::try_new(&game_acct)?;
    let mut holdem = TestHandler::<Holdem>::init_state(&mut ctx, &game_acct)?;
    assert_eq!(0, ctx.count_players());

    // Initialize the client, which simulates the behavior of transactor.
    let transactor_addr = game_acct.transactor_addr.as_ref().unwrap().clone();
    let mut transactor = TestClient::new(
        transactor_addr.clone(),
        game_acct.addr.clone(),
        ClientMode::Transactor,
    );

    // Initialize two player clients, which simulate the behavior of player.
    let mut alice = TestClient::new("Alice".into(), game_acct.addr.clone(), ClientMode::Player);
    let mut bob = TestClient::new("Bob".into(), game_acct.addr.clone(), ClientMode::Player);

    // ------------------------- INITTEST ------------------------
    // Try to start the "zero-player" game that will fail
    let fail_to_start = ctx.gen_first_event();
    holdem.handle_event(&mut ctx, &fail_to_start)?;
    assert_eq!(
        *ctx.get_dispatch(),
        Some(DispatchEvent {
            timeout: 10_000,
            event: Event::WaitingTimeout
        })
    );

    // ------------------------- PLAYERJOIN ------------------------
    // Let players join the game
    let av = ctx.get_access_version() + 1;
    let sync_event = Event::Sync {
        new_players: vec![
            PlayerJoin {
                addr: "Alice".into(),
                balance: 10_000,
                position: 0,
                access_version: av,
            },
            PlayerJoin {
                addr: "Bob".into(),
                balance: 10_000,
                position: 1,
                access_version: av,
            },
        ],
        new_servers: vec![],
        transactor_addr: transactor_account_addr(),
        access_version: av,
    };

    // ------------------------- GAMESTART ------------------------
    // Sync event will cause context to dispatch a GameStart Event
    // holdem.handle_event(&mut ctx, &sync_event)?;

    holdem.handle_until_no_events(
        &mut ctx,
        &sync_event,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;
    {
        let state = holdem.get_state();
        assert_eq!(Street::Preflop, state.street);
        assert_eq!(2, ctx.count_players());
        assert_eq!(GameStatus::Running, ctx.get_status());
        assert_eq!(
            Some(DispatchEvent {
                timeout: 30_000,
                event: Event::ActionTimeout { player_addr: "Bob".into() },
            }),
            *ctx.get_dispatch()
        );
        println!("Alice Cards {:?}", alice.decrypt(&ctx, 1).unwrap());
        assert_eq!(alice.decrypt(&ctx, 1).unwrap().len(), 2);

    }

    // ------------------------- SB CALLS -----------------------
    // Bob decides to call
    let bob_call = bob.custom_event(GameEvent::Call);
    holdem.handle_until_no_events(
        &mut ctx,
        &bob_call,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;
    {
        let state = holdem.get_state();
        assert_eq!(0, state.street_bet);
        assert_eq!(9980, state.players[1].chips);
        assert_eq!(
            Some(Player {
                addr: "Alice".to_string(),
                chips: 10000,
                position: 1,
                status: PlayerStatus::Acting
            }),
            state.acting_player
        );
        println!("Revealed Community Cards from context {:?}", ctx.get_revealed(1));
        println!("Cards revealed to Alice {:?}", alice.decrypt(&ctx, 1).unwrap());
    }
    // // ------------------------- BB CHECKS -----------------------
    // let event = alice.custom_event(GameEvent::Check);
    // holdem.handle_event(&mut ctx, &event)?;
    // {
    //     // let mut clients: Vec<&mut TestClient> = vec![&mut alice, &mut bob, &mut transactor];
    //     // holdem.handle_until_no_events(&mut ctx, &event, &mut clients[..])?;
    //     let state = holdem.get_state();
    //     assert_eq!(0, state.street_bet);
    //     assert_eq!(1, state.pots.len());
    //     assert_eq!(
    //         vec!["Alice".to_string(), "Bob".to_string()],
    //         state.pots[0].owners
    //     );
    //     assert_eq!(40, state.pots[0].amount);
    //     assert_eq!(Street::Flop, state.street);
    // }

    // ------------------------- PREFLOP -----------------------
    // ------------------------- SHUFFLE -----------------------

    Ok(())
}
