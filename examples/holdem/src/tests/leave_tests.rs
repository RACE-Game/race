//! Test varisou types of Leave events.  It is crucial for players to
//! correctly leave the game.

use std::collections::HashMap;

use crate::essential::*;
use crate::tests::helper::{create_sync_event, setup_holdem_game};
use race_core::{error::Result, event::Event};
use race_test::TestClient;

// Two players leave one after another
#[test]
fn test_players_leave() -> Result<()> {
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
            vec!["Alice".to_string(), "Bob".to_string()]
        );
        assert_eq!(
            state.acting_player,
            Some(ActingPlayer {
                addr: "Alice".into(),
                position: 0,
                clock: 30_000
            })
        );
        println!("-- Display {:?}", state.display);
        assert_eq!(state.display.len(), 1);
        assert!(state.display.contains(&Display::DealCards));
    }

    // Alice (SB/BTN) is the acting player and decides to leave
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
        println!("-- Display {:?}", state.display);
        assert_eq!(state.stage, HoldemStage::Play);
        assert_eq!(
            state.acting_player,
            Some(ActingPlayer {
                addr: "Alice".to_string(),
                position: 0,
                clock: 30_000
            })
        );
        assert_eq!(state.player_map.len(), 1);
        assert_eq!(
            state.player_map.get("Bob").unwrap().status,
            PlayerStatus::Winner
        );
    }

    handler.handle_dispatch_event(&mut ctx)?;

    // Bob (BB) decides leaves as well
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
        assert_eq!(state.stage, HoldemStage::Init);
        // println!("-- State {:?}", state);
        // println!("-- Display {:?}", state.display);
        assert_eq!(state.player_map.len(), 0);
    }

    Ok(())
}

// Test one player leaving in settle
// Two players in game: Alice(SB/BTN) and Bob(BB)
// Alice folds then BB wins
// Alice leaves the game
// Expect Alice to leave instantly
#[test]
fn test_settle_leave() -> Result<()> {
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
            vec!["Alice".to_string(), "Bob".to_string()]
        );
        assert_eq!(
            state.acting_player,
            Some(ActingPlayer {
                addr: "Alice".into(),
                position: 0,
                clock: 30_000
            })
        );
    }

    // Alice (SB/BTN) is the acting player and decides to fold
    let sb_fold = alice.custom_event(GameEvent::Fold);
    handler.handle_until_no_events(
        &mut ctx,
        &sb_fold,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;

    // Bob (BB) should be Winner and game is in Settle
    {
        let state = handler.get_state();
        assert_eq!(state.stage, HoldemStage::Settle);
        assert_eq!(state.player_map.len(), 2);
        assert_eq!(
            state.player_map.get("Bob").unwrap().status,
            PlayerStatus::Winner
        );
    }

    // Alice then decides to leave
    let sb_leave = Event::Leave {
        player_addr: "Alice".to_string(),
    };
    handler.handle_until_no_events(
        &mut ctx,
        &sb_leave,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;

    {
        let state = handler.get_state();
        assert_eq!(state.stage, HoldemStage::Settle);
        assert_eq!(state.acting_player, None);
        assert_eq!(state.player_map.len(), 1);
        assert!(state.player_map.contains_key(&"Bob".to_string()));
        println!("Game state {:?}", state);
    }

    Ok(())
}

// Test player leaving in runner
// Two players in the game: Alice(SB/BTN) and Bob(BB).
// Alice goes all-in, then Bob do a hero call.
// Alice leaves the game while the stage is Runner.
// Expect alice to leave instantly.
#[test]
fn test_runner_leave() -> Result<()> {
    let (_game_acct, mut ctx, mut handler, mut transactor) = setup_holdem_game();
    let mut alice = TestClient::player("Alice");
    let mut bob = TestClient::player("Bob");
    let revealed = HashMap::from([
        // Bob
        (0, "ck".to_string()),
        (1, "ca".to_string()),
        // Alice
        (2, "c2".to_string()),
        (3, "c7".to_string()),
        // Board
        (4, "sa".to_string()),
        (5, "sk".to_string()),
        (6, "h3".to_string()),
        (7, "ha".to_string()),
        (8, "d4".to_string()),
    ]);

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
            vec!["Alice".to_string(), "Bob".to_string()]
        );
        assert_eq!(
            state.acting_player,
            Some(ActingPlayer {
                addr: "Alice".into(),
                position: 0,
                clock: 30_000
            })
        );
    }
    ctx.add_revealed_random(1, revealed)?;

    // Alice (SB/BTN) is the acting player and decides to go allin
    let sb_allin = alice.custom_event(GameEvent::Raise(9990));
    handler.handle_until_no_events(
        &mut ctx,
        &sb_allin,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;

    {
        let state = handler.get_state();
        assert_eq!(state.stage, HoldemStage::Play);
        assert_eq!(state.player_map.len(), 2);
        assert_eq!(
            state.player_map.get("Bob").unwrap().status,
            PlayerStatus::Acting
        );
    }

    // BB makes a hero call
    let bb_herocall = bob.custom_event(GameEvent::Call);
    handler.handle_until_no_events(
        &mut ctx,
        &bb_herocall,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;

    {
        let state = handler.get_state();
        assert_eq!(state.street, Street::Showdown);
        assert_eq!(state.stage, HoldemStage::Runner);
        assert_eq!(state.acting_player, None);
        assert_eq!(state.player_map.len(), 1);
    }

    // Alice then decides to leave
    let sb_leave = Event::Leave {
        player_addr: "Alice".to_string(),
    };
    handler.handle_until_no_events(
        &mut ctx,
        &sb_leave,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;

    {
        let state = handler.get_state();
        assert_eq!(state.stage, HoldemStage::Runner);
        assert_eq!(state.acting_player, None);
        assert_eq!(state.player_map.len(), 0);
        println!("Game state {:?}", state);
    }

    Ok(())
}

// Test player leaving in showdown
#[test]
fn test_showdown_leave() -> Result<()> {
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
            vec!["Alice".to_string(), "Bob".to_string()]
        );
        assert_eq!(
            state.acting_player,
            Some(ActingPlayer {
                addr: "Alice".into(),
                position: 0,
                clock: 30_000
            })
        );
    }

    // Alice (SB/BTN) is the acting player and calls
    let sb_call = alice.custom_event(GameEvent::Call);
    handler.handle_until_no_events(
        &mut ctx,
        &sb_call,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;

    {
        let state = handler.get_state();
        assert_eq!(state.stage, HoldemStage::Play);
        assert_eq!(state.street, Street::Preflop);
        assert_eq!(state.player_map.len(), 2);
        assert_eq!(
            state.player_map.get("Bob").unwrap().status,
            PlayerStatus::Acting
        );
    }

    // Bob decides to check and street --> Flop
    let bb_check = bob.custom_event(GameEvent::Check);
    handler.handle_until_no_events(
        &mut ctx,
        &bb_check,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;

    {
        let state = handler.get_state();
        assert_eq!(state.stage, HoldemStage::Play);
        assert_eq!(state.street, Street::Flop);
        // Acting player is now Alice
        assert_eq!(
            state.player_map.get("Alice").unwrap().status,
            PlayerStatus::Acting
        );
    }

    // From this point on, two players keep checking until showdown
    // Flop -> Turn
    let sb_check = alice.custom_event(GameEvent::Check);
    handler.handle_until_no_events(
        &mut ctx,
        &sb_check,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;

    let bb_check = bob.custom_event(GameEvent::Check);
    handler.handle_until_no_events(
        &mut ctx,
        &bb_check,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;

    {
        let state = handler.get_state();
        assert_eq!(state.stage, HoldemStage::Play);
        assert_eq!(state.street, Street::Turn);
        assert_eq!(
            state.player_map.get("Alice").unwrap().status,
            PlayerStatus::Acting
        );
    }

    // Turn -> River
    let sb_check = alice.custom_event(GameEvent::Check);
    handler.handle_until_no_events(
        &mut ctx,
        &sb_check,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;
    let bb_check = bob.custom_event(GameEvent::Check);
    handler.handle_until_no_events(
        &mut ctx,
        &bb_check,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;
    {
        let state = handler.get_state();
        assert_eq!(state.stage, HoldemStage::Play);
        assert_eq!(state.street, Street::River);
        assert_eq!(
            state.acting_player,
            Some(ActingPlayer {
                addr: "Alice".to_string(),
                position: 0,
                clock: 30_000
            })
        );
    }

    // River -> Showdown
    let sb_check = alice.custom_event(GameEvent::Check);
    handler.handle_until_no_events(
        &mut ctx,
        &sb_check,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;
    let bb_check = bob.custom_event(GameEvent::Check);
    handler.handle_until_no_events(
        &mut ctx,
        &bb_check,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;
    {
        let state = handler.get_state();
        assert_eq!(state.stage, HoldemStage::Showdown);
        assert_eq!(state.street, Street::Showdown);
        assert_eq!(state.acting_player, None);
    }

    // Alice decides to leave
    let sb_leave = Event::Leave {
        player_addr: "Alice".to_string(),
    };
    handler.handle_until_no_events(
        &mut ctx,
        &sb_leave,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;
    {
        let state = handler.get_state();
        assert_eq!(state.stage, HoldemStage::Showdown);
        assert_eq!(state.player_map.len(), 1);
        assert!(!state.player_map.contains_key("Alice"));
    }

    Ok(())
}

#[test]
fn test_play_leave() -> Result<()> {
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
            vec!["Alice".to_string(), "Bob".to_string()]
        );
        assert_eq!(
            state.acting_player,
            Some(ActingPlayer {
                addr: "Alice".into(),
                position: 0,
                clock: 30_000
            })
        );
    }

    // Alice (SB/BTN) is the acting player and decides to fold
    let sb_fold = alice.custom_event(GameEvent::Fold);
    handler.handle_until_no_events(
        &mut ctx,
        &sb_fold,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;

    // Bob (BB) should be Winner and game is in Settle
    {
        let state = handler.get_state();
        assert_eq!(state.stage, HoldemStage::Settle);
        assert_eq!(state.player_map.len(), 2);
        assert_eq!(
            state.player_map.get("Bob").unwrap().status,
            PlayerStatus::Winner
        );
    }

    // Alice then decides to leave
    let sb_leave = Event::Leave {
        player_addr: "Alice".to_string(),
    };
    handler.handle_until_no_events(
        &mut ctx,
        &sb_leave,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;

    {
        let state = handler.get_state();
        assert_eq!(state.stage, HoldemStage::Settle);
        assert_eq!(state.acting_player, None);
        assert_eq!(state.player_map.len(), 1);
        assert!(state.player_map.contains_key(&"Bob".to_string()));
        println!("Game state {:?}", state);
    }

    Ok(())
}
