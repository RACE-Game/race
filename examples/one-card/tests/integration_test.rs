use std::collections::HashMap;

use race_core::{
    context::{DispatchEvent, GameContext, GameStatus},
    error::Result,
    event::Event,
    random::RandomStatus, types::ClientMode,
};
use race_example_one_card::{GameEvent, OneCard, OneCardGameAccountData};
use race_test::{TestGameAccountBuilder, TestHandler, TestClient};

#[test]
fn test() -> Result<()> {
    // Initialize the game account, with 1 player joined.
    // The game account must be served, so we add one server which is the transactor.
    let game_account = TestGameAccountBuilder::default()
        .add_servers(1)
        .add_players(1)
        .build();
    let transactor_addr = game_account.transactor_addr.as_ref().unwrap().clone();

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
        assert_eq!(0, state.dealer);
        assert_eq!(HashMap::from([("Alice".into(), 10000)]), state.chips);
        assert_eq!(HashMap::new(), state.bets);
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
    // Let the client handle the random event.
    // There should be one secret state created, the secret storage for the random deck.
    // Additionally, `Mask` event should be created.
    let events = client.handle_updated_context(&ctx)?;
    {
        assert_eq!(1, client.secret_states.len());
        assert_eq!(1, events.len());
    }

    // Now we are in the randomization progress, we have only one server in this test.
    // The transactor should send a mask event.
    handler.handle_event(&mut ctx, &events[0])?;

    Ok(())
}
