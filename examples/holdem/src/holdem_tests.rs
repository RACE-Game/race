// use std::collections::HashMap;
use race_core::{
    context::{DispatchEvent, GameContext, GameStatus},
    error::{Error, Result},
    event::Event,
    random::RandomStatus,
    types::{ClientMode, PlayerJoin},
};
use race_test::{transactor_account_addr, TestClient, TestGameAccountBuilder, TestHandler};
use super::*;

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
        new_players.push(
            PlayerJoin {
                addr: p.into(),
                balance: 10_000,
                position: i,
                access_version: av,
            }
        )
    }

    Event::Sync {
        new_players,
        new_servers: vec![],
        transactor_addr: transactor_account_addr(),
        access_version: av,
    }

}

#[test]
#[ignore]
fn test_runner() -> Result<()> {
    let (game_acct, mut ctx, mut handler, mut transactor) = set_up();
    let mut alice = TestClient::new("Alice".into(), game_acct.addr.clone(), ClientMode::Player);
    let mut bob = TestClient::new("Bob".into(), game_acct.addr.clone(), ClientMode::Player);
    let sync_evt = create_sync_event(&ctx, vec!["Alice".to_string(), "Bob".to_string()]);

    // ------------------------- GAMESTART ------------------------
    handler.handle_until_no_events(
        &mut ctx,
        &sync_evt,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;
    {
        // Randomness and Secrets will be ready before blind bets
        assert_eq!(
            RandomStatus::Ready,
            ctx.get_random_state_unchecked(1).status
        );

        // This is the preflop round, sb will be asked to act
        let state = handler.get_state();
        assert_eq!(Street::Preflop, state.street);
        assert_eq!(2, ctx.count_players());
        assert_eq!(GameStatus::Running, ctx.get_status());
        assert_eq!(
            Some(DispatchEvent {
                timeout: 30_000,
                event: Event::ActionTimeout {
                    player_addr: "Alice".into()
                },
            }),
            *ctx.get_dispatch()
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
    // {
    //     let state = handler.get_state();
    //     assert_eq!(2, state.pots.len());
    //     assert_eq!(2, state.pots[0].owners.len());
    //     assert_eq!(1, state.pots[0].winners.len());
    // }

    Ok(())
}

#[test]
#[ignore]
fn test_players_order() -> Result<()> {
    let (game_acct, mut ctx, mut handler, mut transactor) = set_up();

    let mut alice = TestClient::new("Alice".into(), game_acct.addr.clone(), ClientMode::Player);
    let mut bob = TestClient::new("Bob".into(), game_acct.addr.clone(), ClientMode::Player);
    let mut carol = TestClient::new("Carol".into(), game_acct.addr.clone(), ClientMode::Player);
    let mut dave = TestClient::new("Dave".into(), game_acct.addr.clone(), ClientMode::Player);
    let mut eva = TestClient::new("Eva".into(), game_acct.addr.clone(), ClientMode::Player);

    let sync_evt = create_sync_event(
        &ctx,
        vec!["Alice".to_string(),
             "Bob".to_string(),
             "Carol".to_string(),
             "Dave".to_string(),
             "Eva".to_string(),
        ]
    );

    // ------------------------- GAMESTART ------------------------
    handler.handle_until_no_events(
        &mut ctx,
        &sync_evt,
        vec![&mut alice, &mut bob, &mut carol, &mut dave, &mut eva, &mut transactor]
    )?;

    // BTN is 4 so players should be arranged like below
    // Alice (SB), Bob (BB), Carol (UTG), Dave (MID), Eva (BTN)
    {
        let state = handler.get_state();
        assert_eq!(
            vec!["Alice".to_string(), "Bob".to_string(),
                 "Carol".to_string(), "Dave".to_string(), "Eva".to_string(),
            ],
            state.players
        );
    }

    // ------------------------- BLIND BETS ------------------------

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
        vec!["Alice".to_string(),
             "Bob".to_string(),
             "Carol".to_string(),
             "Dave".to_string(),
             "Eva".to_string(),
        ]
    );

    // ------------------------- GAMESTART ------------------------
    handler.handle_until_no_events(
        &mut ctx,
        &sync_evt,
        vec![&mut alice, &mut bob, &mut carol, &mut dave, &mut eva, &mut transactor]
    )?;

    // ------------------------- BLIND BETS ----------------------
    {
        // BTN is 4 so players in the order of action:
        // Alice (SB), Bob (BB), Carol (UTG), Dave (MID), Eva (BTN)
        // UTG folds
        let carol_fold = carol.custom_event(GameEvent::Fold);
        handler.handle_until_no_events(
            &mut ctx,
            &carol_fold,
            vec![&mut alice, &mut bob, &mut carol, &mut dave, &mut eva, &mut transactor]
        )?;

        // MID calls
        let dave_call = dave.custom_event(GameEvent::Call);
        handler.handle_until_no_events(
            &mut ctx,
            &dave_call,
            vec![&mut alice, &mut bob, &mut carol, &mut dave, &mut eva, &mut transactor]
        )?;

        // BTN calls
        let eva_call = eva.custom_event(GameEvent::Call);
        handler.handle_until_no_events(
            &mut ctx,
            &eva_call,
            vec![&mut alice, &mut bob, &mut carol, &mut dave, &mut eva, &mut transactor]
        )?;

        // SB calls
        let alice_call = alice.custom_event(GameEvent::Call);
        handler.handle_until_no_events(
            &mut ctx,
            &alice_call,
            vec![&mut alice, &mut bob, &mut carol, &mut dave, &mut eva, &mut transactor]
        )?;

        {
            let state = handler.get_state();
            assert_eq!(Street::Preflop, state.street);
            assert_eq!(PlayerStatus::Fold,
                       state.player_map.get(&"Carol".to_string()).unwrap().status);
            assert!(state.acting_player.is_some());
            assert!(matches!(state.acting_player.clone(),
                             Some(player) if player == ("Bob".to_string(), 1)));
        }

        // BB checks then game goes to flop
        let bob_check = bob.custom_event(GameEvent::Check);
        handler.handle_until_no_events(
            &mut ctx,
            &bob_check,
            vec![&mut alice, &mut bob, &mut carol, &mut dave, &mut eva, &mut transactor]
        )?;
    }

    // ------------------------- THE FLOP ------------------------
    {
        // Test pots
        {
            let state = handler.get_state();
            assert_eq!(Street::Flop, state.street);
            assert_eq!(0, state.street_bet);
            assert_eq!(20, state.min_raise);
            assert_eq!(1, state.pots.len());
            assert_eq!(80, state.pots[0].amount);
            assert_eq!(4, state.pots[0].owners.len());
        }

        // Alice (SB) bets 1BB
        let alice_bet = alice.custom_event(GameEvent::Bet(20));
        handler.handle_until_no_events(
            &mut ctx,
            &alice_bet,
            vec![&mut alice, &mut bob, &mut carol, &mut dave, &mut eva, &mut transactor]
        )?;

        // Bob (BB) calls
        let bob_call = bob.custom_event(GameEvent::Call);
        handler.handle_until_no_events(
            &mut ctx,
            &bob_call,
            vec![&mut alice, &mut bob, &mut carol, &mut dave, &mut eva, &mut transactor]
        )?;

        // Dave calls (Carol folded in the preflop)
        let dave_call = dave.custom_event(GameEvent::Call);
        handler.handle_until_no_events(
            &mut ctx,
            &dave_call,
            vec![&mut alice, &mut bob, &mut carol, &mut dave, &mut eva, &mut transactor]
        )?;
        {
            let state = handler.get_state();
            assert_eq!(4, state.get_ref_positon());
        }

        // Eva calls and then game goes to Turn
        let eva_call =eva.custom_event(GameEvent::Call);
        handler.handle_until_no_events(
            &mut ctx,
            &eva_call,
            vec![&mut alice, &mut bob, &mut carol, &mut dave, &mut eva, &mut transactor]
        )?;
    }

    // ------------------------- THE TURN ------------------------
    {
        {
            // Test pots
            let state = handler.get_state();
            assert_eq!(Street::Turn, state.street);
            assert_eq!(0, state.street_bet);
            assert_eq!(20, state.min_raise);
            assert_eq!(1, state.pots.len());
            assert_eq!(160, state.pots[0].amount);
            assert_eq!(4, state.pots[0].owners.len());
            assert!(state.pots[0].owners.contains(&"Alice".to_string()));
            assert!(state.pots[0].owners.contains(&"Bob".to_string()));
            assert!(state.pots[0].owners.contains(&"Dave".to_string()));
            assert!(state.pots[0].owners.contains(&"Eva".to_string()));
        }

        // Alice decides to c-bet 1BB
        let alice_bet = alice.custom_event(GameEvent::Bet(20));
        handler.handle_until_no_events(
            &mut ctx,
            &alice_bet,
            vec![&mut alice, &mut bob, &mut carol, &mut dave, &mut eva, &mut transactor]
        )?;

        {
            // Test min raise
            let state = handler.get_state();
            assert_eq!(20, state.street_bet);
            assert_eq!(20, state.min_raise);
        }

        // Bob decides to raise
        let bob_raise = bob.custom_event(GameEvent::Raise(60));
        handler.handle_until_no_events(
            &mut ctx,
            &bob_raise,
            vec![&mut alice, &mut bob, &mut carol, &mut dave, &mut eva, &mut transactor]
        )?;

        {
            // Test min raise
            let state = handler.get_state();
            assert_eq!(60, state.street_bet);
            assert_eq!(40, state.min_raise);
        }

        // Dave folds
        let dave_fold = dave.custom_event(GameEvent::Fold);
        handler.handle_until_no_events(
            &mut ctx,
            &dave_fold,
            vec![&mut alice, &mut bob, &mut carol, &mut dave, &mut eva, &mut transactor]
        )?;

        // Eva calls
        let eva_call = eva.custom_event(GameEvent::Call);
        handler.handle_until_no_events(
            &mut ctx,
            &eva_call,
            vec![&mut alice, &mut bob, &mut carol, &mut dave, &mut eva, &mut transactor]
        )?;

        // Alice can call, re-raise, or fold
        // She decides to call and games enter the last street: River
        let alice_call = alice.custom_event(GameEvent::Call);
        handler.handle_until_no_events(
            &mut ctx,
            &alice_call,
            vec![&mut alice, &mut bob, &mut carol, &mut dave, &mut eva, &mut transactor]
        )?;
    }

    // ------------------------- THE RIVER ------------------------
    {
        {
            // Test pots
            let state = handler.get_state();
            assert_eq!(Street::River, state.street);
            assert_eq!(0, state.street_bet);
            assert_eq!(20, state.min_raise);
            assert_eq!(2, state.pots.len());
            // Pot 1
            assert_eq!(160, state.pots[0].amount);
            assert_eq!(4, state.pots[0].owners.len());
            assert!(state.pots[0].owners.contains(&"Alice".to_string()));
            assert!(state.pots[0].owners.contains(&"Bob".to_string()));
            assert!(state.pots[0].owners.contains(&"Dave".to_string()));
            assert!(state.pots[0].owners.contains(&"Eva".to_string()));
            // Pot 2
            assert_eq!(180, state.pots[1].amount);
            assert_eq!(3, state.pots[1].owners.len());
            assert!(state.pots[0].owners.contains(&"Alice".to_string()));
            assert!(state.pots[0].owners.contains(&"Bob".to_string()));
            assert!(state.pots[0].owners.contains(&"Eva".to_string()));
        }

        // Alice continues to bet
        let alice_bet = alice.custom_event(GameEvent::Bet(40));
        handler.handle_until_no_events(
            &mut ctx,
            &alice_bet,
            vec![&mut alice, &mut bob, &mut carol, &mut dave, &mut eva, &mut transactor]
        )?;

        // Bob calls
        let bob_call = bob.custom_event(GameEvent::Call);
        handler.handle_until_no_events(
            &mut ctx,
            &bob_call,
            vec![&mut alice, &mut bob, &mut carol, &mut dave, &mut eva, &mut transactor]
        )?;

        // Eva calls so it's showdown time
        let eva_call = eva.custom_event(GameEvent::Call);
        handler.handle_until_no_events(
            &mut ctx,
            &eva_call,
            vec![&mut alice, &mut bob, &mut carol, &mut dave, &mut eva, &mut transactor]
        )?;

        // {
        //     let state = handler.get_state();
        //     assert_eq!(Street::Showdown, state.street);
        //     assert_eq!(40, state.street_bet);
        //     assert_eq!(20, state.min_raise);
        //     assert_eq!(2, state.pots.len());
        //     // Pot 1
        //     assert_eq!(160, state.pots[0].amount);
        //     assert_eq!(4, state.pots[0].owners.len());
        //     assert!(state.pots[0].owners.contains(&"Alice".to_string()));
        //     assert!(state.pots[0].owners.contains(&"Bob".to_string()));
        //     assert!(state.pots[0].owners.contains(&"Dave".to_string()));
        //     assert!(state.pots[0].owners.contains(&"Eva".to_string()));
        //     // Pot 2
        //     assert_eq!(300, state.pots[1].amount);
        //     assert_eq!(3, state.pots[1].owners.len());
        //     assert!(state.pots[0].owners.contains(&"Alice".to_string()));
        //     assert!(state.pots[0].owners.contains(&"Bob".to_string()));
        //     assert!(state.pots[0].owners.contains(&"Eva".to_string()));
        // }
        // Alice got hole cards [s7, s3] A High
        // Bob got hole cards [ca, cj]   Two Pairs
        // Carol got hole cards [dj, st] Fold J Pair
        // Dave got hole cards [sk, dt]  Fold A High
        // Eva got hole cards [sa, s4]   A Pair

        // ["da", "d9", "c6", "hj", "d8"]
        // Bob,

    }
    Ok(())
}
