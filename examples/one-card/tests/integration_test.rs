use std::collections::HashMap;

use race_core::{
    context::{DispatchEvent, GameContext, GameStatus},
    error::Result,
    event::Event,
    random::RandomStatus,
    types::ClientMode,
};
use race_example_one_card::{GameEvent, OneCard, OneCardGameAccountData};
use race_test::{TestClient, TestGameAccountBuilder, TestHandler, TestPlayerClient};

#[macro_use]
extern crate log;

#[test]
fn test() -> Result<()> {
    env_logger::builder().is_test(true).try_init().unwrap();

    // Initialize the game account, with 1 player joined.
    // The game account must be served, so we add one server which is the transactor.
    let game_account = TestGameAccountBuilder::default()
        .add_servers(1)
        .add_players(1)
        .build();
    let transactor_addr = game_account.transactor_addr.as_ref().unwrap().clone();

    // Initialize player client, which simulates the behavior of player.
    let mut alice = TestPlayerClient::new("Alice");
    let mut bob = TestPlayerClient::new("Bob");

    // Initialize the client, which simulates the behavior of transactor.
    let mut client = TestClient::new(ClientMode::Transactor, &game_account);

    // Create game context and test handler.
    // Initalize the handler state with game account.
    // The game will not start since two players are required.
    let mut ctx = GameContext::new(&game_account)?;
    let mut handler = TestHandler::init_state(&mut ctx, &game_account)?;

    {
        assert_eq!(1, ctx.get_players().len());
        let state: &OneCard = handler.get_state();
        assert_eq!(0, state.dealer_idx);
        assert_eq!(HashMap::from([("Alice".into(), 10000)]), state.chips);
        assert_eq!(0, state.bet);
    }

    // Another player joined the game.
    // Now we have enough players, an event of `GameStart` should be dispatched.
    let join_event = Event::Join {
        player_addr: "Bob".into(),
        balance: 10000,
        position: 0,
    };
    handler.handle_event(&mut ctx, &join_event)?;

    {
        let state: &OneCard = handler.get_state();
        assert_eq!(2, ctx.get_players().len());
        assert_eq!(
            Some(DispatchEvent::new(Event::GameStart, 0)),
            *ctx.get_dispatch()
        );
        assert_eq!(GameStatus::Initializing, ctx.get_status());
        assert_eq!(
            HashMap::from([("Alice".into(), 10000), ("Bob".into(), 10000)]),
            state.chips
        );
    }

    // Now the dispatching event should be `GameStart`.
    // By handling this event, a random deck of cards should be created.
    handler.handle_dispatch_event(&mut ctx)?;
    {
        let state: &OneCard = handler.get_state();
        assert_eq!(0, state.deck_random_id);
        assert_eq!(
            RandomStatus::Masking(transactor_addr.clone()),
            ctx.get_random_state_unchecked(0).status
        );
    }

    // Now we are in the randomization progress. Servers will create the events in turn.
    // But in our test case, we have only one server.
    //
    // Now, Let the client handle the updated context.
    // The corresponding secert state will be initialized, which contains all the secrets.
    // Additionally, one `Mask` event will be created.
    let events = client.handle_updated_context(&ctx)?;
    {
        assert_eq!(1, client.secret_states.len());
        assert_eq!(1, events.len());
    }

    // Send the mask event to handler, we expect the random status to be changed to `Locking`.
    handler.handle_event(&mut ctx, &events[0])?;

    {
        assert_eq!(
            RandomStatus::Locking(transactor_addr.clone()),
            ctx.get_random_state_unchecked(0).status
        );
    }

    // Now, Let the client handle the updated context.
    // One `Lock` event will be created.
    let events = client.handle_updated_context(&ctx)?;
    {
        assert_eq!(1, events.len());
    }

    // Send the lock event to handler, we expect the random status to be changed to `Ready`.
    // Since all randomness is ready, an event of `RandomnessReady` will be dispatched.
    handler.handle_event(&mut ctx, &events[0])?;

    {
        assert_eq!(
            RandomStatus::Ready,
            ctx.get_random_state_unchecked(0).status
        );
        assert_eq!(
            Some(DispatchEvent::new(Event::RandomnessReady, 0)),
            *ctx.get_dispatch()
        );
    }

    // Handle this dispatched `RandomnessReady`.
    // We expect each player got one card assigned.
    handler.handle_dispatch_event(&mut ctx)?;
    {
        let random_state = ctx.get_random_state_unchecked(0);
        let ciphertexts_for_alice = random_state.list_assigned_ciphertexts("Alice");
        let ciphertexts_for_bob = random_state.list_assigned_ciphertexts("Bob");
        assert_eq!(RandomStatus::WaitingSecrets, random_state.status);
        assert_eq!(1, ciphertexts_for_alice.len());
        assert_eq!(1, ciphertexts_for_bob.len());
    }

    // Let client handle the updated context.
    // `ShareSecret` event should be created.
    let events = client.handle_updated_context(&ctx)?;
    {
        let event = &events[0];
        assert!(
            matches!(event, Event::ShareSecrets { sender, secrets } if sender.eq(&transactor_addr) && secrets.len() == 2)
        );
    }

    // Handle `ShareSecret` event.
    // Expect the random status to be changed to ready.
    handler.handle_event(&mut ctx, &events[0])?;
    {
        assert_eq!(
            RandomStatus::Ready,
            ctx.get_random_state_unchecked(0).status
        );
        let random_state = ctx.get_random_state_unchecked(0);
        assert_eq!(1, random_state.list_shared_secrets("Alice").unwrap().len());
        assert_eq!(1, random_state.list_shared_secrets("Bob").unwrap().len());
    }

    // Now, client should be able to see their cards.
    let alice_decryption = alice.decrypt(&ctx, 0)?;
    let bob_decryption = bob.decrypt(&ctx, 0)?;
    {
        info!("Alice decryption: {:?}", alice_decryption);
        info!("Bob decryption: {:?}", bob_decryption);
        assert_eq!(1, alice_decryption.len());
        assert_eq!(1, bob_decryption.len());
    }

    // Now, Alice should be the dealer.
    // So, she can send a bet event and we expect the bet amount of Alice to be updated to 500.
    let event = alice.create_custom_event(GameEvent::Bet(500));
    handler.handle_event(&mut ctx, &event)?;
    {
        let state = handler.get_state();
        assert_eq!(500, state.bet);
    }

    // Bob call this.
    // Now, it's time to reveal the cards, so two secrets for hands are required.
    let event = bob.create_custom_event(GameEvent::Call);
    handler.handle_event(&mut ctx, &event)?;
    {
        let random_state = ctx.get_random_state_unchecked(0);
        assert_eq!(
            2,
            random_state
                .list_required_secrets_by_from_addr(&transactor_addr)
                .len()
        );
    }

    // Let the client handle this update.
    // We expect two secrets to be shared.
    let events = client.handle_updated_context(&ctx)?;
    {
        let event = &events[0];
        info!(
            "Required ident: {:?}",
            ctx.get_random_state_unchecked(0)
                .list_required_secrets_by_from_addr(&transactor_addr)
        );
        assert_eq!(
            2,
            ctx.get_random_state_unchecked(0)
                .list_required_secrets_by_from_addr(&transactor_addr)
                .len()
        );
        assert!(
            matches!(event, Event::ShareSecrets { sender, secrets } if sender.eq(&transactor_addr) && secrets.len() == 2)
        );
    }

    // Handle `ShareSecret` event.
    info!("Handle ShareSecret event.");
    handler.handle_event(&mut ctx, &events[0])?;
    {
        info!(
            "Required ident: {:?}",
            ctx.get_random_state_unchecked(0)
                .list_required_secrets_by_from_addr(&transactor_addr)
        );
        assert_eq!(
            0,
            ctx.get_random_state_unchecked(0)
                .list_required_secrets_by_from_addr(&transactor_addr)
                .len()
        );
        assert_eq!(
            Some(DispatchEvent::new(Event::SecretsReady, 0)),
            *ctx.get_dispatch()
        );
    }

    // Now, the transactor should be able to reveal all hole cards.
    let decryption = client.decrypt(&ctx, 0)?;
    info!("Decryption: {:?}", decryption);
    assert_eq!(2, decryption.len());
    ctx.add_revealed(0, decryption)?;

    // Now send `SecretReady` to handler.
    handler.handle_dispatch_event(&mut ctx)?;
    {
        assert!(matches!(ctx.get_settles(),
                         Some(settles) if settles.len() == 2
        ));
    }

    Ok(())
}
