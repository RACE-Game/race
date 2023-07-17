//! Test handling various events in Holdem.  There are two types of events:
//! 1. General events such as Sync, GameStart, WaitTimeout, etc;
//! 2. Custom events that are exclusively relevant to Holdem:
//! Call, Bet, Raise, Leave, etc.
//! The pivot function next_state is also covered in this test file.

use race_core::{
    error::Result as CoreResult,
    prelude::{Effect, HandleError}
};
use race_test::TestClient;
use std::collections::BTreeMap;

use crate::essential::*;
use crate::tests::helper::{create_sync_event, setup_context, setup_two_player_holdem,  setup_holdem_game};

#[test]
fn test_preflop_fold() -> CoreResult<()> {
    let (_game_acct, mut ctx, mut handler, mut transactor) = setup_holdem_game();

    let mut alice = TestClient::player("Alice");
    let mut bob = TestClient::player("Bob");

    let sync_evt = create_sync_event(&ctx, &[&alice, &bob], &transactor);

    handler.handle_until_no_events(
        &mut ctx,
        &sync_evt,
        vec![&mut alice, &mut bob, &mut transactor]
    )?;

    // Regular tests to make sure holdem has been set up properly
    {
        let state = handler.get_state();
        assert_eq!(state.street, Street::Preflop);
        assert_eq!(state.btn, 1);
        assert!(state.is_acting_player(&"Alice".to_string()));
    }

    // SB(Alice) folds so BB(Bob), the single player, wins
    let alice_fold = alice.custom_event(GameEvent::Fold);
    handler.handle_until_no_events(
        &mut ctx,
        &alice_fold,
        vec![&mut alice, &mut bob, &mut transactor],
    )?;

    {
        let state = handler.get_state();
        let alice = state.player_map.get("Alice").unwrap();
        let bob = state.player_map.get("Bob").unwrap();
        // Street should remain unchanged
        assert_eq!(state.street, Street::Preflop);
        assert_eq!(alice.chips, 9990);
        assert_eq!(bob.chips, 10_010);
        assert_eq!(state.player_map.get("Bob").unwrap().status, PlayerStatus::Wait);
    }

    // Game should be able to start again
    handler.handle_dispatch_event(&mut ctx)?;
    {
        let state = handler.get_state();
        assert_eq!(state.btn, 1);
    }

    Ok(())
}

#[test]
fn test_next_state() -> Result<(), HandleError> {
    let mut state = setup_two_player_holdem()?;
    let ctx = setup_context();
    let mut effect = Effect::from_context(&ctx);
    // SB folds so next state: single player wins
    {
        let bet_map: BTreeMap<String, u64> = BTreeMap::from([
          ("Alice".into(), 20u64), // BB
          ("Bob".into(), 10u64),     // SB
        ]);
        state.bet_map = bet_map;
        state.street = Street::Preflop;

        let bob = state.player_map.get_mut(&("Bob".to_string())).unwrap();
        bob.status = PlayerStatus::Fold;

        // Effect is primarily for `settle' and `assign', which is
        // outside the scope of this unit test
        state.next_state(&mut effect)?;

        assert_eq!(state.street, Street::Preflop);

    }
    Ok(())
}
