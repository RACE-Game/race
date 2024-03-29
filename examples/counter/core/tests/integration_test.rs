use race_core::{context::GameContext, event::Event, types::ClientMode};
use race_example_counter::*;
use race_test::*;
use tracing::info;

#[test]
fn test_count() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let account_data = CounterAccountData { init_value: 0 };
    let game_account = TestGameAccountBuilder::default()
        .with_data(account_data)
        .add_players(2)
        .add_servers(1)
        .build();
    let transactor_addr = game_account.transactor_addr.as_ref().unwrap().clone();
    let mut alice = TestClient::new(
        "Alice".into(),
        game_account.addr.clone(),
        ClientMode::Player,
    );
    let mut transactor = TestClient::new(
        transactor_addr.clone(),
        game_account.addr.clone(),
        ClientMode::Transactor,
    );

    let mut ctx = GameContext::try_new(&game_account)?;
    let mut handler = TestHandler::<Counter>::init_state(&mut ctx, &game_account)?;

    // Start the game
    let av = ctx.get_access_version();
    handler.handle_event(&mut ctx, &Event::GameStart { access_version: av })?;

    // // Create randomness

    // handler.handle_event(
    //     &mut ctx,
    //     &Event::custom(player_account_addr(0), &GameEvent::RandomPoker),
    // )?;

    // Mask
    let events = transactor.handle_updated_context(&ctx)?;

    handler.handle_event(&mut ctx, &events[0])?;

    info!("Context after mask: {:?}", ctx);

    // Lock
    let events = transactor.handle_updated_context(&ctx)?;
    for e in events.iter() {
        info!("Event: {}", e);
    }

    handler.handle_event(&mut ctx, &events[0])?;

    info!("Context after lock: {:?}", ctx);

    handler.handle_dispatch_event(&mut ctx)?;

    // ShareSecrets
    let events = transactor.handle_updated_context(&ctx)?;

    for e in events.iter() {
        info!("Event: {}", e);
    }

    handler.handle_event(&mut ctx, &events[0])?;

    let decryption = alice.decrypt(&ctx, handler.get_state().dice_random_id)?;

    info!("Random card: {:?}", decryption.get(&0));

    handler.handle_dispatch_event(&mut ctx)?;

    Ok(())
}

#[test]
fn test_random_card() -> anyhow::Result<()> {
    let account_data = CounterAccountData { init_value: 0 };
    let game_account = TestGameAccountBuilder::default()
        .with_data(account_data)
        .add_players(2)
        .add_servers(1)
        .build();
    let transactor_addr = game_account.transactor_addr.as_ref().unwrap().clone();
    let mut transactor = TestClient::new(
        transactor_addr.clone(),
        game_account.addr.clone(),
        ClientMode::Transactor,
    );

    let mut ctx = GameContext::try_new(&game_account)?;
    let mut handler = TestHandler::<Counter>::init_state(&mut ctx, &game_account)?;

    // Start the game
    let av = ctx.get_access_version();
    handler.handle_event(&mut ctx, &Event::GameStart { access_version: av })?;

    // Create randomness

    handler.handle_event(
        &mut ctx,
        &Event::custom(player_account_addr(0), &GameEvent::RandomPoker),
    )?;

    // Mask
    let events = transactor.handle_updated_context(&ctx)?;

    handler.handle_event(&mut ctx, &events[0])?;
    handler.handle_event(&mut ctx, &events[1])?;

    println!("after mask");
    println!("{:?}", ctx.get_random_state_unchecked(2));

    info!("Context after mask: {:?}", ctx);

    // Lock
    let events = transactor.handle_updated_context(&ctx)?;
    for e in events.iter() {
        info!("Event: {}", e);
    }

    handler.handle_event(&mut ctx, &events[0])?;
    handler.handle_event(&mut ctx, &events[1])?;

    println!("after mask");
    println!("{:?}", ctx.get_random_state_unchecked(2));
    {
        assert_eq!(
            ctx.get_dispatch().as_ref().unwrap().event,
            Event::RandomnessReady { random_id: 2 }
        );
    }

    handler.handle_dispatch_event(&mut ctx)?;
    {
        let state: &Counter = handler.get_state();
        assert_eq!(state.dice_random_id, 1);
        assert_eq!(state.poker_random_id, 2);
    }

    let events = transactor.handle_updated_context(&ctx)?;

    println!("events");
    for e in events.iter() {
        println!("{}", e);
    }
    handler.handle_event(&mut ctx, &events[0])?;
    handler.handle_event(&mut ctx, &events[1])?;

    {
        assert_eq!(
            ctx.get_dispatch().as_ref().unwrap().event,
            Event::SecretsReady
        );
    }

    Ok(())
}
