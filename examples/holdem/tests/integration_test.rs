use std::collections::HashMap;
use borsh::BorshSerialize;
use race_core::{
    context::{DispatchEvent, GameContext, GameStatus},
    error::{Error, Result},
    event::Event,
    random::RandomStatus,
    types::{ClientMode, PlayerJoin},
};
use race_test::{transactor_account_addr, TestClient, TestGameAccountBuilder, TestHandler};

#[macro_use]
extern crate log;
use holdem::*;
mod single_tests;

// In a game, there are generally two types of events:
// 1. Events dispatched from within the context, such as GameStart, ShareScrets, etc.
// 2. Events dispatched by clients or transactors, because they see the updated context
// Note the handle_event function in the below test.
// It accepts an event as ref, borrowing it. This is TestHandler's impl.
#[test]
pub fn test_holdem() -> Result<()> {
    // ------------------------- SETUP ------------------------
    // Initialize the game with 1 server added.
    let game_acct = TestGameAccountBuilder::default()
        .add_servers(1)
        .build();

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
    let mut alice = TestClient::new(
        "Alice".into(),
        game_acct.addr.clone(),
        ClientMode::Player,
    );
    let mut bob = TestClient::new(
        "Bob".into(),
        game_acct.addr.clone(),
        ClientMode::Player
    );

    // ------------------------- INITTEST ------------------------
    // Try to start the "zero-player" game that will fail
    let fail_to_start = ctx.gen_first_event();
    let result = holdem.handle_event(&mut ctx, &fail_to_start);
    assert_eq!(result, Err(Error::NoEnoughPlayers));

    // ------------------------- PLAYERJOIN ------------------------
    // Let players join the game
    let av = ctx.get_access_version() + 1;
    let sync_event = Event::Sync {
        new_players: vec![
            PlayerJoin {
                addr: "Alice".into(),
                balance: 10000,
                position: 0,
                access_version: av,
            },
            PlayerJoin {
                addr: "Bob".into(),
                balance: 10000,
                position: 1,
                access_version: av,
            }
        ],
        new_servers: vec![],
        transactor_addr: transactor_account_addr(),
        access_version: av,
    };

    // Sync event will cause context to dispatch a GameStart Event, if possible
    holdem.handle_event(&mut ctx, &sync_event)?;
    {
        assert_eq!(2, ctx.count_players());
        assert_eq!(GameStatus::Uninit, ctx.get_status());
        assert_eq!(
            Some(DispatchEvent {
                timeout: 0,
                event: Event::GameStart { access_version: ctx.get_access_version() },
            }),
            *ctx.get_dispatch());
    }

    // ------------------------- GAMESTART ------------------------
    // ------------------------- BLINDBETS ------------------------
    // Handle GameStart event and next_state should lead to blind bets
    holdem.handle_dispatch_event(&mut ctx)?;
    {
        let state: &Holdem = holdem.get_state();
        assert_eq!(GameStatus::Running, ctx.get_status());
        assert_eq!(1, state.deck_random_id);
        assert_eq!(
            String::from("Alice"),
            state.players[0].addr.clone()
        );
        assert_eq!(
            RandomStatus::Masking(transactor_addr.clone()),
            ctx.get_random_state_unchecked(1).status
        );
        assert_eq!(Street::Preflop, state.street);
        assert_eq!(2, state.bets.len());
        assert_eq!(20, state.bets[0].amount());
        assert_eq!(20, state.street_bet);
        // Bob should be at sb and thus the acting player (via ask_for_action)
        assert!(!state.acting_player.is_none());
        assert_eq!(
            Some(Player { addr: "Bob".to_string(),
                          chips: 10000,
                          position: 1,
                          status: PlayerStatus::Acting}),
            state.acting_player
        );
    }

    // ------------------------- 1st TO ACT -----------------------
    // In this 2-player game, Bob is at SB and asked for action within 30 secs
    holdem.handle_dispatch_event(&mut ctx)?;

    // Test ActionTimeout Event: Player either checks or folds automatically
    // Since Bob hasnt bet yet, so he will be forced to fold
    // This will lead to the result where the left single player, Alice, wins.
    // {
    //     let state: &Holdem = holdem.get_state();
    //     assert!(state.acting_player.is_none());
    //     assert_eq!(1, state.pots.len());
    //     assert_eq!(
    //         vec!["Alice".to_string(), "Bob".to_string()],
    //         state.pots[0].owners()
    //     );
    //     assert_eq!(
    //         vec!["Alice".to_string()],
    //         state.pots[0].winners()
    //     );
    //     assert_eq!(20, state.pots[0].amount());
    //
    // }

    // Bob decides to call
    let event = bob.custom_event(GameEvent::Bet(10));
    holdem.handle_event(&mut ctx, &event)?;
    {
        let state = holdem.get_state();
        assert_eq!(20, state.street_bet);
    }

    // let events = transactor.handle_updated_context(&ctx)?;
    // {
    //     assert_eq!(1, transactor.secret_states().len());
    //     assert_eq!(1, events.len());
    // }
    //
    // // Send the mask event to handler for `Locking`.
    // holdem_hlr.handle_event(&mut ctx, &events[0])?;
    // {
    //     assert_eq!(
    //         RandomStatus::Locking(transactor_addr.clone()),
    //         ctx.get_random_state_unchecked(1).status
    //     );
    // }
    //
    // // Let the client handle the updated context.
    // let events = transactor.handle_updated_context(&ctx)?;
    // {
    //     assert_eq!(1, events.len());
    // }
    //
    // // Send the lock event to handler, the random status to be changed to `Ready`.
    // // Since all randomness is ready, an event of `RandomnessReady` will be dispatched.
    // holdem_hlr.handle_event(&mut ctx, &events[0])?;
    // {
    //     assert_eq!(
    //         RandomStatus::Ready,
    //         ctx.get_random_state_unchecked(1).status
    //     );
    //     assert_eq!(
    //         Some(DispatchEvent::new(Event::RandomnessReady {random_id: 1}, 0)),
    //         *ctx.get_dispatch()
    //     );
    // }
    //
    // // Handle this dispatched `RandomnessReady`: each player gets two cards
    // holdem_hlr.handle_dispatch_event(&mut ctx)?;
    // {
    //     let random_state = ctx.get_random_state_unchecked(1);
    //     let ciphertexts_for_alice = random_state.list_assigned_ciphertexts("Alice");
    //     let ciphertexts_for_bob = random_state.list_assigned_ciphertexts("Bob");
    //     assert_eq!(
    //         RandomStatus::WaitingSecrets(transactor_account_addr()),
    //         random_state.status
    //     );
    //     assert_eq!(2, ciphertexts_for_alice.len());
    //     assert_eq!(2, ciphertexts_for_bob.len());
    // }
    //
    // // Let client handle the updated context
    // let events = transactor.handle_updated_context(&ctx)?;
    // {
    //     let event = &events[0];
    //     assert!(
    //         matches!(
    //             event,
    //             Event::ShareSecrets { sender, secrets } if sender.eq(&transactor_addr) && secrets.len() == 4)
    //     );
    // }
    //
    // // Handle `ShareSecret` event.
    // // Expect the random status to be changed to ready.
    // holdem_hlr.handle_event(&mut ctx, &events[0])?;
    // {
    //     assert_eq!(
    //         RandomStatus::Ready,
    //         ctx.get_random_state_unchecked(1).status
    //     );
    //     let random_state = ctx.get_random_state_unchecked(1);
    //     assert_eq!(2, random_state.list_shared_secrets("Alice").unwrap().len());
    //     assert_eq!(2, random_state.list_shared_secrets("Bob").unwrap().len());
    // }
    //
    // // Cards are visible to clients now
    // let alice_decryption = alice.decrypt(&ctx, 1)?;
    // let bob_decryption = bob.decrypt(&ctx, 1)?;
    // {
    //     info!("Alice decryption: {:?}", alice_decryption);
    //     info!("Bob decryption: {:?}", bob_decryption);
    //     assert_eq!(2, alice_decryption.len());
    //     assert_eq!(2, bob_decryption.len());
    // }
    //
    // // Players start to act: Alice bets
    // let event = alice.custom_event(GameEvent::Bet(500));
    // holdem_hlr.handle_event(&mut ctx, &event)?;
    // {
    //     let state = holdem_hlr.get_state();
    //     assert_eq!(500, state.street_bet);
    // }

    // Bob calls

    Ok(())

}
