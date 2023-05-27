use std::collections::HashMap;
use super::*;
use race_core::{
    context::{DispatchEvent, GameContext, GameStatus},
    error:: Result,
    event::Event,
    random::RandomStatus,
    types::{ClientMode, PlayerJoin},
};
use race_test::{
    transactor_account_addr, TestClient, TestGameAccountBuilder, TestHandler,
};

type Game = (InitAccount, GameContext, TestHandler<Holdem>, TestClient);

fn set_up() -> Game {
    let game_account = TestGameAccountBuilder::default().add_servers(1).build();
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

fn create_sync_event(ctx: &GameContext, players: Vec<String>) -> Event {
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

#[test]
fn test_players_order() -> Result<()> {
    let (game_acct, mut ctx, mut handler, mut transactor) = set_up();

    let mut alice = TestClient::new("Alice".into(), game_acct.addr.clone(), ClientMode::Player);
    let mut bob = TestClient::new("Bob".into(), game_acct.addr.clone(), ClientMode::Player);
    let mut carol = TestClient::new("Carol".into(), game_acct.addr.clone(), ClientMode::Player);
    let mut dave = TestClient::new("Dave".into(), game_acct.addr.clone(), ClientMode::Player);
    let mut eva = TestClient::new("Eva".into(), game_acct.addr.clone(), ClientMode::Player);

    let sync_evt = create_sync_event(
        &ctx,
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Carol".to_string(),
            "Dave".to_string(),
            "Eva".to_string(),
        ],
    );

    // ------------------------- GAMESTART ------------------------
    println!("-- Syncing players --");
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
    println!("-- Syncing done --");

    // BTN is 4 so players should be arranged like below:
    // Alice (SB), Bob (BB), Carol (UTG), Dave (MID), Eva (BTN)
    {
        let state = handler.get_state();
        assert_eq!(
            state.players,
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Carol".to_string(),
                "Dave".to_string(),
                "Eva".to_string(),
            ]
        );
    }

    Ok(())
}

#[test]
fn test_runner() -> Result<()> {
    let (game_acct, mut ctx, mut handler, mut transactor) = set_up();
    let mut alice = TestClient::new("Alice".into(), game_acct.addr.clone(), ClientMode::Player);
    let mut bob = TestClient::new("Bob".into(), game_acct.addr.clone(), ClientMode::Player);
    let sync_evt = create_sync_event(&ctx, vec!["Alice".to_string(), "Bob".to_string()]);

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
    println!("-- Cards {:?}", ctx.get_revealed(holdem_state.deck_random_id)?);

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
                    player_addr: "Alice".into()
                },
            })
        );
        assert!(state.is_acting_player(&"Alice".to_string()));
    }

    // ------------------------- PREFLOP ------------------------
    // Alice is SB and she decides to go all in
    let alice_allin = alice.custom_event(GameEvent::Raise(10_000));
    handler.handle_until_no_events(
        &mut ctx,
        &alice_allin,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;

    // Bob decides to call and thus leads game to runner
    let bob_allin = bob.custom_event(GameEvent::Call);
    handler.handle_until_no_events(
        &mut ctx,
        &bob_allin,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;

    // ------------------------- RUNNER ------------------------
    {
        let state = handler.get_state();
        assert_eq!(state.pots.len(), 1);
        // assert_eq!(2, state.pots[0].owners.len());
        // assert_eq!(1, state.pots[0].winners.len());
    }

    Ok(())
}

#[test]
fn test_play_game() -> Result<()> {
    let (game_acct, mut ctx, mut handler, mut transactor) = set_up();
    let mut alice = TestClient::new("Alice".into(), game_acct.addr.clone(), ClientMode::Player);
    let mut bob = TestClient::new("Bob".into(), game_acct.addr.clone(), ClientMode::Player);
    let mut carol = TestClient::new("Carol".into(), game_acct.addr.clone(), ClientMode::Player);
    let mut dave = TestClient::new("Dave".into(), game_acct.addr.clone(), ClientMode::Player);
    let mut eva = TestClient::new("Eva".into(), game_acct.addr.clone(), ClientMode::Player);

    let sync_evt = create_sync_event(
        &ctx,
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Carol".to_string(),
            "Dave".to_string(),
            "Eva".to_string(),
        ],
    );


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
    println!("-- Cards {:?}", ctx.get_revealed(holdem_state.deck_random_id)?);


    // ------------------------- BLIND BETS ----------------------
    {
        // BTN is 0 so players in the order of action:
        // Dave (UTG), Eva (MID), Alice (BTN), Bob (SB), Carol (BB)
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


        // Wait for 5 secs and game should start again
        handler.handle_dispatch_event(&mut ctx);
        {
            let state = handler.get_state();
            assert_eq!(state.btn, 1);
            // assert_eq!(s)
        }
    }
    Ok(())
}
