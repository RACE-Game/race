//! Test Holdem game and its several key points such as
//! the order of players, the runner stage and hole-card
//! dealing.  The last test shows a complete hand.

use race_core::{
    context::{DispatchEvent, GameStatus},
    error::Result,
    event::Event,
    random::RandomStatus,
};
use race_test::TestClient;
use std::collections::HashMap;

use crate::essential::*;
use crate::tests::helper::{create_sync_event, setup_holdem_game};

#[test]
fn test_players_order() -> Result<()> {
    let (_game_acct, mut ctx, mut handler, mut transactor) = setup_holdem_game();

    let mut alice = TestClient::player("Alice");
    let mut bob = TestClient::player("Bob");
    let mut carol = TestClient::player("Carol");
    let mut dave = TestClient::player("Dave");
    let mut eva = TestClient::player("Eva");

    let sync_evt = create_sync_event(&ctx, &[&alice, &bob, &carol, &dave, &eva], &transactor);

    // ------------------------- GAMESTART ------------------------
    handler.handle_until_no_events(
        &mut ctx,
        &sync_evt,
        vec![
            &mut alice,
            &mut bob,
            &mut carol,
            &mut dave,
            &mut eva,
            &mut transactor,
        ],
    )?;

    // BTN is 4 so players should be arranged like below:
    // Bob (SB), Carol (BB), Dave (UTG), Eva(MID), Alice(BTN)
    {
        let state = handler.get_state();
        assert_eq!(
            state.player_order,
            vec![
                "Bob".to_string(),
                "Carol".to_string(),
                "Dave".to_string(),
                "Eva".to_string(),
                "Alice".to_string(),
            ]
        );
    }

    Ok(())
}

#[test]
fn test_get_holecards() -> Result<()> {
    let (_game_acct, mut ctx, mut handler, mut transactor) = setup_holdem_game();
    let mut alice = TestClient::player("Alice");
    let mut bob = TestClient::player("Bob");
    let sync_evt = create_sync_event(&ctx, &[&alice, &bob], &transactor);
    // Syncing players to the game, i.e. they join the game and game kicks start
    handler.handle_until_no_events(
        &mut ctx,
        &sync_evt,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;

    // let runner_revealed = HashMap::from([
    //     // Alice
    //     (0, "st".to_string()),
    //     (1, "ct".to_string()),
    //     // Bob
    //     (2, "ht".to_string()),
    //     (3, "dt".to_string()),
    //     // Board
    //     (4, "s5".to_string()),
    //     (5, "c6".to_string()),
    //     (6, "h2".to_string()),
    //     (7, "h8".to_string()),
    //     (8, "d7".to_string()),
    // ]);
    let holdem_state = handler.get_state();
    // ctx.add_revealed_random(holdem_state.deck_random_id, runner_revealed)?;

    {
        println!(
            "-- Player hand index map {:?}",
            holdem_state.hand_index_map
        );
        let alice_hole_cards = alice.decrypt(&ctx, holdem_state.deck_random_id);
        println!("Alice hole cards {:?}", alice_hole_cards);

        let alice_hand_index = holdem_state.hand_index_map.get(&"Alice".to_string()).unwrap();
        assert_eq!(alice_hand_index, &vec![0, 1]);

    }
    Ok(())
}

#[test]
fn test_runner() -> Result<()> {
    let (_game_acct, mut ctx, mut handler, mut transactor) = setup_holdem_game();
    let mut alice = TestClient::player("Alice");
    let mut bob = TestClient::player("Bob");
    let sync_evt = create_sync_event(&ctx, &[&alice, &bob], &transactor);

    // Syncing players to the game, i.e. they join the game and game kicks start
    handler.handle_until_no_events(
        &mut ctx,
        &sync_evt,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;

    let runner_revealed = HashMap::from([
        // Alice
        (0, "st".to_string()),
        (1, "ct".to_string()),
        // Bob
        (2, "ht".to_string()),
        (3, "dt".to_string()),
        // Board
        (4, "s5".to_string()),
        (5, "c6".to_string()),
        (6, "h2".to_string()),
        (7, "h8".to_string()),
        (8, "d7".to_string()),
    ]);
    let holdem_state = handler.get_state();
    ctx.add_revealed_random(holdem_state.deck_random_id, runner_revealed)?;
    println!(
        "-- Cards {:?}",
        ctx.get_revealed(holdem_state.deck_random_id)?
    );

    // With everything ready, game enters preflop
    {
        assert_eq!(
            RandomStatus::Ready,
            ctx.get_random_state_unchecked(1).status
        );

        let state = handler.get_state();
        assert_eq!(state.street, Street::Preflop,);
        assert_eq!(ctx.count_players(), 2);
        assert_eq!(ctx.get_status(), GameStatus::Running);
        assert_eq!(
            *ctx.get_dispatch(),
            Some(DispatchEvent {
                timeout: 30_000,
                event: Event::ActionTimeout {
                    player_addr: "Bob".into()
                },
            })
        );
        assert!(state.is_acting_player(&"Bob".to_string()));
    }

    // ------------------------- PREFLOP ------------------------
    // Bob decides to go all in
    let bob_allin = bob.custom_event(GameEvent::Raise(9990));
    handler.handle_until_no_events(
        &mut ctx,
        &bob_allin,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;

    // Alice is BB and she decides to make a hero call
    let alice_allin = alice.custom_event(GameEvent::Call);
    handler.handle_until_no_events(
        &mut ctx,
        &alice_allin,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;

    // ------------------------- RUNNER ------------------------
    {
        let state = handler.get_state();
        assert_eq!(state.pots.len(), 1);
        assert_eq!(state.pots[0].owners.len(), 2);
        assert_eq!(state.pots[0].winners.len(), 2); // a draw
    }

    Ok(())
}

#[test]
fn test_play_game() -> Result<()> {
    let (_game_acct, mut ctx, mut handler, mut transactor) = setup_holdem_game();
    let mut alice = TestClient::player("Alice");
    let mut bob = TestClient::player("Bob");
    let mut carol = TestClient::player("Carol");
    let mut dave = TestClient::player("Dave");
    let mut eva = TestClient::player("Eva");

    let sync_evt = create_sync_event(&ctx, &[&alice, &bob, &carol, &dave, &eva], &transactor);

    // ------------------------- GAMESTART ------------------------
    handler.handle_until_no_events(
        &mut ctx,
        &sync_evt,
        vec![
            &mut alice,
            &mut bob,
            &mut carol,
            &mut dave,
            &mut eva,
            &mut transactor,
        ],
    )?;

    // After game starts, a random state will be initialized and ready for game
    // In this stage, players are assigned/dealt folded cards
    // Pin the randomness for testing purposes
    let revealed = HashMap::from([
        // Alice
        (0, "c9".to_string()),
        (1, "d3".to_string()),
        // Bob
        (2, "ht".to_string()),
        (3, "d8".to_string()),
        // Carol
        (4, "st".to_string()),
        (5, "ct".to_string()),
        // Dave
        (6, "sq".to_string()),
        (7, "d2".to_string()),
        // Eva
        (8, "h3".to_string()),
        (9, "dk".to_string()),
        // Board
        (10, "s5".to_string()),
        (11, "c6".to_string()),
        (12, "h2".to_string()),
        (13, "h8".to_string()),
        (14, "d7".to_string()),
    ]);
    let holdem_state = handler.get_state();
    ctx.add_revealed_random(holdem_state.deck_random_id, revealed)?;
    println!(
        "-- Cards {:?}",
        ctx.get_revealed(holdem_state.deck_random_id)?
    );

    // ------------------------- BLIND BETS ----------------------
    {
        // BTN is 0 so players in the order of action:
        // Dave (UTG), Eva (MID), Alice (BTN), Bob (SB), Carol (BB)
        // In state: [Bob, Carol, Dave, Eva, Alice]
        // UTG folds
        let dave_fold = dave.custom_event(GameEvent::Fold);
        handler.handle_until_no_events(
            &mut ctx,
            &dave_fold,
            vec![
                &mut alice,
                &mut bob,
                &mut carol,
                &mut dave,
                &mut eva,
                &mut transactor,
            ],
        )?;

        // MID calls
        let eva_call = eva.custom_event(GameEvent::Call);
        handler.handle_until_no_events(
            &mut ctx,
            &eva_call,
            vec![
                &mut alice,
                &mut bob,
                &mut carol,
                &mut dave,
                &mut eva,
                &mut transactor,
            ],
        )?;

        // BTN calls
        let alice_call = alice.custom_event(GameEvent::Call);
        handler.handle_until_no_events(
            &mut ctx,
            &alice_call,
            vec![
                &mut alice,
                &mut bob,
                &mut carol,
                &mut dave,
                &mut eva,
                &mut transactor,
            ],
        )?;

        // SB calls
        let bob_call = bob.custom_event(GameEvent::Call);
        handler.handle_until_no_events(
            &mut ctx,
            &bob_call,
            vec![
                &mut alice,
                &mut bob,
                &mut carol,
                &mut dave,
                &mut eva,
                &mut transactor,
            ],
        )?;

        {
            let state = handler.get_state();
            assert_eq!(state.street, Street::Preflop,);
            assert_eq!(
                state.player_map.get(&"Dave".to_string()).unwrap().status,
                PlayerStatus::Fold
            );
            assert!(state.acting_player.is_some());
            // assert!(matches!(state.acting_player.clone(),
            //                  Some(player) if player == ("Carol".to_string(), 1)));
        }

        // BB checks then game goes to flop
        let carol_check = carol.custom_event(GameEvent::Check);
        handler.handle_until_no_events(
            &mut ctx,
            &carol_check,
            vec![
                &mut alice,
                &mut bob,
                &mut carol,
                &mut dave,
                &mut eva,
                &mut transactor,
            ],
        )?;
    }

    // ------------------------- THE FLOP ------------------------
    {
        // Test pots
        {
            let state = handler.get_state();
            assert_eq!(state.street, Street::Flop);
            assert_eq!(state.street_bet, 0);
            assert_eq!(state.min_raise, 20);
            assert_eq!(state.pots.len(), 1);
            assert_eq!(state.pots[0].amount, 80);
            assert_eq!(state.pots[0].owners.len(), 4);
        }

        // Bob (SB) bets 1BB
        let bob_bet = bob.custom_event(GameEvent::Bet(20));
        handler.handle_until_no_events(
            &mut ctx,
            &bob_bet,
            vec![
                &mut alice,
                &mut bob,
                &mut carol,
                &mut dave,
                &mut eva,
                &mut transactor,
            ],
        )?;

        // Carol (BB) calls
        let carol_call = carol.custom_event(GameEvent::Call);
        handler.handle_until_no_events(
            &mut ctx,
            &carol_call,
            vec![
                &mut alice,
                &mut bob,
                &mut carol,
                &mut dave,
                &mut eva,
                &mut transactor,
            ],
        )?;

        // Eva calls
        let eva_call = eva.custom_event(GameEvent::Call);
        handler.handle_until_no_events(
            &mut ctx,
            &eva_call,
            vec![
                &mut alice,
                &mut bob,
                &mut carol,
                &mut dave,
                &mut eva,
                &mut transactor,
            ],
        )?;
        {
            let state = handler.get_state();
            assert_eq!(state.get_ref_position(), 0);
        }

        // Alice(BTN) calls and then game goes to Turn
        let alice_call = alice.custom_event(GameEvent::Call);
        handler.handle_until_no_events(
            &mut ctx,
            &alice_call,
            vec![
                &mut alice,
                &mut bob,
                &mut carol,
                &mut dave,
                &mut eva,
                &mut transactor,
            ],
        )?;
    }

    // ------------------------- THE TURN ------------------------
    {
        {
            // Test pots
            let state = handler.get_state();
            assert_eq!(state.street, Street::Turn);
            assert_eq!(state.street_bet, 0);
            assert_eq!(state.min_raise, 20);
            assert_eq!(state.pots.len(), 1);
            assert_eq!(state.pots[0].amount, 160);
            assert_eq!(state.pots[0].owners.len(), 4);
            assert!(state.pots[0].owners.contains(&"Alice".to_string()));
            assert!(state.pots[0].owners.contains(&"Bob".to_string()));
            assert!(state.pots[0].owners.contains(&"Carol".to_string()));
            assert!(state.pots[0].owners.contains(&"Eva".to_string()));
        }

        // Bob (SB) decides to c-bet 1BB
        let bob_bet = bob.custom_event(GameEvent::Bet(20));
        handler.handle_until_no_events(
            &mut ctx,
            &bob_bet,
            vec![
                &mut alice,
                &mut bob,
                &mut carol,
                &mut dave,
                &mut eva,
                &mut transactor,
            ],
        )?;

        {
            // Test min raise
            let state = handler.get_state();
            assert_eq!(20, state.street_bet);
            assert_eq!(20, state.min_raise);
        }

        // Carol (BB) decides to raise
        let carol_raise = carol.custom_event(GameEvent::Raise(60));
        handler.handle_until_no_events(
            &mut ctx,
            &carol_raise,
            vec![
                &mut alice,
                &mut bob,
                &mut carol,
                &mut dave,
                &mut eva,
                &mut transactor,
            ],
        )?;

        {
            // Test min raise
            let state = handler.get_state();
            assert_eq!(state.street_bet, 60);
            assert_eq!(state.min_raise, 40);
        }

        // eva folds
        let eva_fold = eva.custom_event(GameEvent::Fold);
        handler.handle_until_no_events(
            &mut ctx,
            &eva_fold,
            vec![
                &mut alice,
                &mut bob,
                &mut carol,
                &mut dave,
                &mut eva,
                &mut transactor,
            ],
        )?;

        // Alice calls
        let alice_call = alice.custom_event(GameEvent::Call);
        handler.handle_until_no_events(
            &mut ctx,
            &alice_call,
            vec![
                &mut alice,
                &mut bob,
                &mut carol,
                &mut dave,
                &mut eva,
                &mut transactor,
            ],
        )?;

        // Bob (SB) can call, re-raise, or fold
        // He decides to call and games enter the last street: River
        let bob_call = bob.custom_event(GameEvent::Call);
        handler.handle_until_no_events(
            &mut ctx,
            &bob_call,
            vec![
                &mut alice,
                &mut bob,
                &mut carol,
                &mut dave,
                &mut eva,
                &mut transactor,
            ],
        )?;
    }

    // ------------------------- THE RIVER ------------------------
    {
        {
            // Test pots
            let state = handler.get_state();
            assert_eq!(state.street, Street::River);
            assert_eq!(state.street_bet, 0);
            assert_eq!(state.min_raise, 20);
            assert_eq!(state.pots.len(), 2);
            // Pot 1
            assert_eq!(state.pots[0].amount, 160);
            assert_eq!(state.pots[0].owners.len(), 4);
            assert!(state.pots[0].owners.contains(&"Alice".to_string()));
            assert!(state.pots[0].owners.contains(&"Bob".to_string()));
            assert!(state.pots[0].owners.contains(&"Carol".to_string()));
            assert!(state.pots[0].owners.contains(&"Eva".to_string()));
            // Pot 2
            assert_eq!(state.pots[1].amount, 180);
            assert_eq!(state.pots[1].owners.len(), 3);
            assert!(state.pots[0].owners.contains(&"Alice".to_string()));
            assert!(state.pots[0].owners.contains(&"Bob".to_string()));
            assert!(state.pots[0].owners.contains(&"Carol".to_string()));
        }

        // Bob continues to bet
        let bob_bet = bob.custom_event(GameEvent::Bet(40));
        handler.handle_until_no_events(
            &mut ctx,
            &bob_bet,
            vec![
                &mut alice,
                &mut bob,
                &mut carol,
                &mut dave,
                &mut eva,
                &mut transactor,
            ],
        )?;

        // Carol (BB) calls
        let carol_call = carol.custom_event(GameEvent::Call);
        handler.handle_until_no_events(
            &mut ctx,
            &carol_call,
            vec![
                &mut alice,
                &mut bob,
                &mut carol,
                &mut dave,
                &mut eva,
                &mut transactor,
            ],
        )?;

        // Alice calls so it's showdown time
        let alice_call = alice.custom_event(GameEvent::Call);
        handler.handle_until_no_events(
            &mut ctx,
            &alice_call,
            vec![
                &mut alice,
                &mut bob,
                &mut carol,
                &mut dave,
                &mut eva,
                &mut transactor,
            ],
        )?;

        // Wait for 10 secs and game should start again
        handler.handle_dispatch_event(&mut ctx)?;
        {
            let state = handler.get_state();
            assert_eq!(state.btn, 1);
        }
    }
    Ok(())
}
