use race_core::{context::GameContext, event::Event, types::ClientMode};
use race_example_counter::*;
use race_test::*;
use tracing::info;

#[test]
fn test() -> anyhow::Result<()> {

    tracing_subscriber::fmt::init();

    let account_data = CounterAccountData { init_value: 0 };
    let game_account = TestGameAccountBuilder::default()
        .with_data(account_data)
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

    let join_event = Event::Join {
        player_addr: "Alice".into(),
        balance: 10000,
        position: 0,
    };
    handler.handle_event(&mut ctx, &join_event)?;

    // Mask
    let events = transactor.handle_updated_context(&ctx)?;

    handler.handle_event(&mut ctx, &events[0])?;

    info!("Context after mask: {:?}", ctx);

    // Lock
    let events = transactor.handle_updated_context(&ctx)?;

    handler.handle_event(&mut ctx, &events[0])?;

    info!("Context after lock: {:?}", ctx);

    handler.handle_dispatch_event(&mut ctx)?;

    // ShareSecrets
    let events = transactor.handle_updated_context(&ctx)?;

    info!("Events: {:?}", events);

    handler.handle_event(&mut ctx, &events[0])?;

    let decryption = alice.decrypt(&ctx, 0)?;

    info!("Random card: {:?}", decryption.get(&0));

    handler.handle_dispatch_event(&mut ctx)?;

    Ok(())
}
