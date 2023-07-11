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
fn test_eject_timeout() -> Result<()> {
    let (_game_acct, mut ctx, mut handler, mut transactor) = setup_holdem_game();
    let mut charlie = TestClient::player("Charlie");
    let mut alice = TestClient::player("Alice");
    let mut bob = TestClient::player("Bob");

    let sync_evt = create_sync_event(&ctx, &[&charlie, &alice, &bob], &transactor);
    handler.handle_until_no_events(
        &mut ctx,
        &sync_evt,
        vec![&mut charlie, &mut alice, &mut bob, &mut transactor],
    )?;

    // --------------------- INIT ------------------------
    {
        let state = handler.get_mut_state();
        let charlie = state.player_map.get_mut("Charlie").unwrap();
        charlie.timeout = 3;
        assert_eq!(
            state.player_order,
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string() // UTG + BTN
            ]
        );

        for p in state.player_map.values() {
            if p.addr == "Alice".to_string() || p.addr == "Bob".to_string() {
                assert_eq!(p.timeout, 0)
            } else {
                assert_eq!(p.timeout, 3)
            }
        }

        assert_eq!(
            state.acting_player,
            Some(ActingPlayer {
                addr: "Charlie".to_string(),
                position: 0,
                clock: 30_000
            })
        );
    }

    // --------------------- PREFLOP ------------------------
    // Charlie (UTG/BTN) reaches action timeout, meets 3 action timeout
    let charlie_timeout = Event::ActionTimeout {
        player_addr: "Charlie".to_string(),
    };
    handler.handle_until_no_events(
        &mut ctx,
        &charlie_timeout,
        vec![&mut charlie, &mut alice, &mut bob, &mut transactor],
    )?;

    {
        let state = handler.get_state();
        assert_eq!(
            state.acting_player,
            Some(ActingPlayer {
                addr: "Alice".to_string(),
                position: 1,
                clock: 30_000
            })
        );
        for p in state.player_map.values() {
            if p.addr == "Alice".to_string() {
                assert_eq!(p.timeout, 0);
            } else if p.addr == "Bob".to_string() {
                assert_eq!(p.status, PlayerStatus::Wait);
                assert_eq!(p.timeout, 0);
            } else {
                assert_eq!(p.status, PlayerStatus::Leave);
                assert_eq!(p.timeout, 3);
            }
        }
    }

    // and will be marked `Leave'
    // Alice (SB) folds, and Bob (BB) wins
    let alice_fold = alice.custom_event(GameEvent::Fold);
    handler.handle_until_no_events(
        &mut ctx,
        &alice_fold,
        vec![&mut charlie, &mut alice, &mut bob, &mut transactor],
    )?;

    {
        let state = handler.get_state();
        assert_eq!(state.player_map.len(), 2);
        assert!(state.player_map.contains_key(&"Alice".to_string()));
        assert!(state.player_map.contains_key(&"Bob".to_string()));
    }

    Ok(())
}

#[test]
fn test_player_leave() -> Result<()> {
    let (_game_acct, mut ctx, mut handler, mut transactor) = setup_holdem_game();
    let mut alice = TestClient::player("Alice");
    let mut bob = TestClient::player("Bob");

    let sync_evt = create_sync_event(&ctx, &[&alice, &bob], &transactor);
    handler.handle_until_no_events(
        &mut ctx,
        &sync_evt,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;

    {
        let state = handler.get_state();
        assert_eq!(
            state.player_order,
            vec!["Bob".to_string(), "Alice".to_string()]
        );
        assert_eq!(
            state.acting_player,
            Some(ActingPlayer {
                addr: "Bob".into(),
                position: 1,
                clock: 30_000
            })
        );
    }

    // Bob (SB/BTN) is the acting player and decides to leave
    let bob_leave = Event::Leave {
        player_addr: "Bob".to_string(),
    };
    handler.handle_until_no_events(
        &mut ctx,
        &bob_leave,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;

    {
        let state = handler.get_state();
        assert_eq!(state.player_map.len(), 1);
        assert_eq!(
            state.player_map.get("Alice").unwrap().status,
            PlayerStatus::Winner
        );
        // assert_eq!(state.stage, HoldemStage::Init);
    }

    // Handle the dispatched wait timeout event
    handler.handle_dispatch_event(&mut ctx)?;

    // Alice (BB) wins and leaves as well
    let alice_leave = Event::Leave {
        player_addr: "Alice".to_string(),
    };
    handler.handle_until_no_events(
        &mut ctx,
        &alice_leave,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;

    {
        let state = handler.get_state();
        println!("state {:?}", state);
        assert_eq!(state.player_map.len(), 0);
    }

    Ok(())
}
#[test]
fn test_get_holecards_idxs() -> Result<()> {
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

    let holdem_state = handler.get_state();
    {
        println!("-- Player hand index map {:?}", holdem_state.hand_index_map);
        let alice_hole_cards = alice.decrypt(&ctx, holdem_state.deck_random_id);
        println!("Alice hole cards {:?}", alice_hole_cards);

        let alice_hand_index = holdem_state
            .hand_index_map
            .get(&"Alice".to_string())
            .unwrap();
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
        assert_eq!(
            state.acting_player,
            Some(ActingPlayer {
                addr: "Bob".to_string(),
                position: 1usize,
                clock: 30_000u64,
            })
        );
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

        let alice = state.player_map.get("Alice").unwrap();
        let bob = state.player_map.get("Bob").unwrap();
        assert_eq!(alice.status, PlayerStatus::Winner);
        assert_eq!(bob.status, PlayerStatus::Winner);

        println!("-- Display {:?}", state.display);
        assert_eq!(state.board.len(), 5);
        assert!(state.display.len() >= 1);
        assert!(state.display.contains(&Display::DealBoard {
            prev: 0,
            board: vec![
                "s5".to_string(),
                "c6".to_string(),
                "h2".to_string(),
                "h8".to_string(),
                "d7".to_string(),
            ]
        }));
        assert!(state.display.contains(&Display::AwardPots {
            pots: vec![AwardPot {
                winners: vec!["Bob".to_string(), "Alice".to_string()],
                amount: 20000
            }]
        }))
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

        // UTG decides to leave
        println!("Dave is going to leave");
        let dave_leave = Event::Leave {
            player_addr: "Dave".to_string(),
        };
        handler.handle_until_no_events(
            &mut ctx,
            &dave_leave,
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

            for p in state.player_map.values() {
                println!("-- Player {} position {}", p.addr, p.position);
            }

            assert_eq!(state.street, Street::Preflop,);
            assert_eq!(
                state.player_map.get(&"Dave".to_string()).unwrap().status,
                PlayerStatus::Leave
            );
            // Acting player is the next player, BB, Carol
            assert!(state.acting_player.is_some());
            assert_eq!(
                state.acting_player,
                Some(ActingPlayer {
                    addr: "Carol".to_string(),
                    position: 2usize,
                    clock: 30_000u64
                })
            );
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
            println!("-- Display {:?}", state.display);
            assert!(state.display.contains(&Display::DealBoard {
                prev: 0,
                board: vec!["s5".to_string(), "c6".to_string(), "h2".to_string(),]
            }));
        }

        // Frank Joins:
        // 1. Frank's status should be `Init'
        // 2. Frank should be in player_map but not in player_order
        // 3. Frank should not be assgined any cards, i.e., not in hand_index_map
        let mut frank = TestClient::player("Frank");
        let frank_join = create_sync_event(&ctx, &[&frank], &transactor);

        handler.handle_until_no_events(
            &mut ctx,
            &frank_join,
            vec![
                &mut alice,
                &mut bob,
                &mut carol,
                &mut dave,
                &mut eva,
                &mut frank,
                &mut transactor,
            ],
        )?;
        {
            let state = handler.get_state();
            assert_eq!(state.player_map.len(), 6);
            assert_eq!(state.player_order.len(), 5);
            assert!(matches!(
                state.player_map.get("Frank").unwrap().status,
                PlayerStatus::Init
            ));
            assert_eq!(state.hand_index_map.get("Frank"), None);
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
            assert_eq!(
                state.board,
                vec![
                    "s5".to_string(),
                    "c6".to_string(),
                    "h2".to_string(),
                    "h8".to_string(),
                ]
            );
            assert!(state.display.contains(&Display::DealBoard {
                prev: 3,
                board: vec![
                    "s5".to_string(),
                    "c6".to_string(),
                    "h2".to_string(),
                    "h8".to_string(),
                ]
            }));
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
            assert_eq!(
                state.board,
                vec![
                    "s5".to_string(),
                    "c6".to_string(),
                    "h2".to_string(),
                    "h8".to_string(),
                    "d7".to_string(),
                ]
            );
            assert!(state.display.contains(&Display::DealBoard {
                prev: 4,
                board: vec![
                    "s5".to_string(),
                    "c6".to_string(),
                    "h2".to_string(),
                    "h8".to_string(),
                    "d7".to_string(),
                ]
            }));
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
            assert_eq!(state.player_map.len(), 5);
            // Player order has not been cleared yet
            assert_eq!(state.player_order.len(), 5);
        }

        // Handle GameStart
        handler.handle_dispatch_event(&mut ctx)?;
        {
            let state = handler.get_state();
            assert_eq!(state.player_map.len(), 5);
            assert_eq!(state.player_order.len(), 0);
            assert_eq!(state.hand_index_map.len(), 0);
        }
    }
    Ok(())
}
