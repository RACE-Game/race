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
    // Handle the sync event so that game kicks off
    holdem.handle_until_no_events(
        &mut ctx,
        &sync_event,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;
    {
        // Randomness and Secrets will be ready before blind bets
        assert_eq!(
            RandomStatus::Ready,
            ctx.get_random_state_unchecked(1).status
        );

        // This is the preflop round, sb will be asked to act
        let state = holdem.get_state();
        assert_eq!(Street::Preflop, state.street);
        assert_eq!(2, ctx.count_players());
        assert_eq!(GameStatus::Running, ctx.get_status());
        assert_eq!(
            Some(DispatchEvent {
                timeout: 30_000,
                event: Event::ActionTimeout { player_addr: "Alice".into() },
            }),
            *ctx.get_dispatch()
        );

    }

    // ------------------------- BLIND BETS -----------------------
    // Alice (SB) calls
    let alice_call = alice.custom_event(GameEvent::Call);
    holdem.handle_until_no_events(
        &mut ctx,
        &alice_call,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;
    // Then Bob will be asked to act
    {
        let state = holdem.get_state();
        assert_eq!(
            Some(Player {
                addr: "Bob".to_string(),
                chips: 9980,
                position: 0,
                status: PlayerStatus::Acting
            }),
            state.acting_player
        );
    }
    // Bob (BB) checks
    let bob_check = bob.custom_event(GameEvent::Check);
    holdem.handle_until_no_events(
        &mut ctx,
        &bob_check,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;
    {
        let state = holdem.get_state();
        // There should be 1 pot of 40 chips, owned by Alice and Bob
        assert_eq!(0, state.street_bet);
        assert_eq!(1, state.pots.len());
        assert_eq!(
            vec!["Alice".to_string(), "Bob".to_string()],
            state.pots[0].owners
        );
        assert_eq!(40, state.pots[0].amount);
        assert_eq!(9980, state.players[0].chips);
        assert_eq!(9980, state.players[1].chips);

        // Then game goes to Flop and Alice will be asked to act
        assert_eq!(Street::Flop, state.street);
        assert_eq!(
            Some(Player {
                addr: "Alice".to_string(),
                chips: 9980,
                position: 0,
                status: PlayerStatus::Acting
            }),
            state.acting_player
        );
        assert_eq!(Street::Flop, state.street);
    }

    // ------------------------- FLOP -----------------------
    {
        // Now both Alice and Bob should be able to see their hole cards + 3 community cards
        println!("Revealed Community Cards from context {:?}", ctx.get_revealed(1));
        // println!("Cards revealed to Alice {:?}", alice.decrypt(&ctx, 1).unwrap());
        println!("Alice Cards {:?}", alice.decrypt(&ctx, 1).unwrap());
        assert_eq!(alice.decrypt(&ctx, 1).unwrap().len(), 5);
        println!("Bob Cards {:?}", bob.decrypt(&ctx, 1).unwrap());
        assert_eq!(bob.decrypt(&ctx, 1).unwrap().len(), 5);
    }

    // Alice SB checks
    let alice_check2 = alice.custom_event(GameEvent::Check);
    holdem.handle_until_no_events(
        &mut ctx,
        &alice_check2,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;
    {
        // Bob is asked to act
        let state = holdem.get_state();
        assert_eq!(
            Some(Player {
                addr: "Bob".to_string(),
                chips: 9980,
                position: 1,
                status: PlayerStatus::Acting
            }),
            state.acting_player
        );

    }
    // Bob BB checks
    let bob_check2 = bob.custom_event(GameEvent::Check);
    holdem.handle_until_no_events(
        &mut ctx,
        &bob_check2,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;
    // Game should move to next street: Turn
    {
        let state = holdem.get_state();
        assert_eq!(Street::Turn, state.street);
    }

    // ------------------------- TURN -----------------------
    {
        // Visible cards: hole cards + 4 community cards
        println!("Revealed Community Cards from context {:?}", ctx.get_revealed(1));
        println!("Alice Cards {:?}", alice.decrypt(&ctx, 1).unwrap());
        assert_eq!(alice.decrypt(&ctx, 1).unwrap().len(), 6);
        println!("Bob Cards {:?}", bob.decrypt(&ctx, 1).unwrap());
        assert_eq!(bob.decrypt(&ctx, 1).unwrap().len(), 6);
    }

    // Alice and Bob keep checking in turn, heading for the last street: River
    let alice_check3 = alice.custom_event(GameEvent::Check);
    holdem.handle_until_no_events(
        &mut ctx,
        &alice_check3,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;
    let bob_check3 = bob.custom_event(GameEvent::Check);
    holdem.handle_until_no_events(
        &mut ctx,
        &bob_check3,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;
    {
        let state = holdem.get_state();
        assert_eq!(Street::River, state.street);

    }

    // ------------------------- RIVER -----------------------
    {
        // Visible cards: hole cards + 5 community cards
        println!("Revealed Community Cards from context {:?}", ctx.get_revealed(1));
        println!("Alice Cards {:?}", alice.decrypt(&ctx, 1).unwrap());
        assert_eq!(alice.decrypt(&ctx, 1).unwrap().len(), 7);
        println!("Bob Cards {:?}", bob.decrypt(&ctx, 1).unwrap());
        assert_eq!(bob.decrypt(&ctx, 1).unwrap().len(), 7);
    }

    // Alice and Bob both check again, thus running into showdown
    let alice_check4 = alice.custom_event(GameEvent::Check);
    holdem.handle_until_no_events(
        &mut ctx,
        &alice_check4,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;
    let bob_check4 = bob.custom_event(GameEvent::Check);
    holdem.handle_until_no_events(
        &mut ctx,
        &bob_check4,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;

    {
        let state = holdem.get_state();
        assert_eq!(Street::Showdown, state.street);
    }

    // ------------------------- Showdown -----------------------
    let event = Event::SecretsReady;
    holdem.handle_until_no_events(
        &mut ctx,
        &event,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;

    Ok(())
}
