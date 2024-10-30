use crate::{AccountData, DrawCard, GameEvent, GameStage, Player};
use race_api::prelude::*;
use race_test::prelude::*;

#[test]
fn test() -> anyhow::Result<()> {
    // Initialize player clients, which simulates the behavior of players.
    let mut alice = TestClient::player("Alice");
    let mut bob = TestClient::player("Bob");

    // Initialize the client, which simulates the behavior of transactor.
    let transactor_addr = "Transactor".to_string();
    let mut transactor = TestClient::transactor(&transactor_addr);

    // Initialize the test context, which 1 player joined.
    // The game must be served by at least one server, so we add a transactor here.
    let (mut ctx, _) = TestContextBuilder::default()
        .with_data(AccountData {
        blind_bet: 100,
        min_bet: 100,
        max_bet: 1000,
        })
        .with_max_players(2)
        .set_transactor(&mut transactor)
        .add_player(&mut alice, 10000)
        .build_with_init_state::<DrawCard>()?;

    // The game will not start since two players are required.
    // Start game should fail
    println!("Start game, without players");
    let ret = ctx.handle_event(&Event::GameStart);
    assert_eq!(ret, Err(Error::HandleError(HandleError::NoEnoughPlayers)));

    {
        let state: &DrawCard = ctx.state();
        assert_eq!(0, state.bet);
    }

    // Another player joined the game.
    // Now we have enough players, an event of `GameStart` should be dispatched.
    println!("Player Bob join the game");
    let join_event = ctx.join(&mut bob, 10000);
    let ee = ctx.handle_event(&join_event)?;

    {
        println!("State: {:?}", ctx.state());
        assert!(ee.start_game);
    }

    // Now the dispatching event should be `GameStart`.
    // By handling this event, a random deck of cards should be created.
    println!("Start game");
    ctx.handle_dispatch_event()?;
    {
        let state: &DrawCard = ctx.state();
        println!("State: {:?}", state);
        assert_eq!(1, state.random_id);
        assert_eq!(
            vec![
                Player {
                    id: bob.id(),
                    balance: 10000,
                    bet: 0
                },
                Player {
                    id: alice.id(),
                    balance: 10000,
                    bet: 0
                },
            ],
            state.players
        );
        assert_eq!(
            RandomStatus::Masking(transactor_addr.clone()),
            ctx.random_state(1)?.status
        );
    }

    // Now we are in the randomization progress. Servers will create the events in turn.
    // But in our test case, we have only one server.
    //
    // Now, Let the client handle the updated context.
    // The corresponding secert state will be initialized, which contains all the secrets.
    // Additionally, one `Mask` event will be created.
    let events = ctx.client_events(&mut transactor)?;
    {
        assert_eq!(1, transactor.secret_state().list_random_secrets().len());
        assert_eq!(1, events.len());
    }

    // Send the mask event to handler, we expect the random status to be changed to `Locking`.
    ctx.handle_event(&events[0])?;

    {
        assert_eq!(
            RandomStatus::Locking(transactor_addr.clone()),
            ctx.random_state(1)?.status
        );
    }

    // Now, Let the client handle the updated context.
    // One `Lock` event will be created.
    let events = ctx.client_events(&mut transactor)?;
    {
        assert_eq!(1, events.len());
    }

    // Send the lock event to handler, we expect the random status to be changed to `Ready`.
    // Since all randomness is ready, an event of `RandomnessReady` will be dispatched.
    ctx.handle_event(&events[0])?;

    {
        assert_eq!(
            RandomStatus::Ready,
            ctx.random_state(1)?.status
        );
        assert_eq!(
            Some(DispatchEvent::new(
                Event::RandomnessReady { random_id: 1 },
                0
            )),
            ctx.current_dispatch()
        );
    }

    // Handle this dispatched `RandomnessReady`.
    // We expect each player to be assigned one card.
    ctx.handle_dispatch_event()?;
    {
        let random_state = ctx.random_state(1)?;
        let ciphertexts_for_alice = random_state.list_assigned_ciphertexts("Alice");
        let ciphertexts_for_bob = random_state.list_assigned_ciphertexts("Bob");
        assert_eq!(RandomStatus::WaitingSecrets, random_state.status);
        assert_eq!(1, ciphertexts_for_alice.len());
        assert_eq!(1, ciphertexts_for_bob.len());
    }

    // Let client handle the updated context.
    // `ShareSecret` event should be created.
    println!("Transactor handle updated context, generate `ShareSecrets` event");
    let events = ctx.client_events(&mut transactor)?;
    {
        let event = &events[0];
        assert!(
            matches!(event, Event::ShareSecrets { sender, shares } if *sender == transactor.id() && shares.len() == 2)
        );
    }

    // Handle `ShareSecret` event.
    // Expect the random status to be changed to ready.
    ctx.handle_event(&events[0])?;
    {
        assert_eq!(
            RandomStatus::Ready,
            ctx.random_state(1)?.status
        );
        let random_state = ctx.random_state(1)?;
        assert_eq!(1, random_state.list_shared_secrets("Alice").unwrap().len());
        assert_eq!(1, random_state.list_shared_secrets("Bob").unwrap().len());
        assert_eq!(
            Some(DispatchEvent::new(Event::SecretsReady { random_ids: vec![1] }, 0)),
            ctx.current_dispatch()
        );
    }

    println!("Dispatch `SecretsReady` event, update game stage to `Betting`");
    ctx.handle_dispatch_event()?;
    {
        let state = ctx.state();
        assert_eq!(GameStage::Betting, state.stage);
    }

    // Now, client should be able to see their cards.
    println!("Checked assigned cards");
    let alice_decryption = ctx.client_decrypt(&alice, 1)?;
    let bob_decryption = ctx.client_decrypt(&bob, 1)?;
    {
        println!("Alice decryption: {:?}", alice_decryption);
        println!("Bob decryption: {:?}", bob_decryption);
        assert_eq!(1, alice_decryption.len());
        assert_eq!(1, bob_decryption.len());
    }

    // Now, Bob is the first to act.
    // So, she can send a bet event and we expect the bet amount to be updated to 500.
    println!("Bob bets");
    let event = bob.custom_event(GameEvent::Bet(500));
    ctx.handle_event(&event)?;
    {
        let state = ctx.state();
        assert_eq!(500, state.bet);
    }

    // Alice calls this.
    // Now, it's time to reveal the cards, so two secrets for hands are required.
    println!("Alice calls");
    let event = alice.custom_event(GameEvent::Call);
    ctx.handle_event(&event)?;
    {
        let random_state = ctx.random_state(1)?;
        assert_eq!(
            2,
            random_state
                .list_required_secrets_by_from_addr(&transactor_addr)
                .len()
        );
    }

    // Let the client handle this update.
    // We expect two secrets to be shared.
    let events = ctx.client_events(&mut transactor)?;
    {
        let event = &events[0];
        println!(
            "Required ident: {:?}",
            ctx.random_state(1)?
                .list_required_secrets_by_from_addr(&transactor_addr)
        );
        assert_eq!(
            2,
            ctx.random_state(1)?
                .list_required_secrets_by_from_addr(&transactor_addr)
                .len()
        );
        assert!(
            matches!(event, Event::ShareSecrets { sender, shares } if *sender == transactor.id() && shares.len() == 2)
        );
    }

    // Handle `ShareSecret` event.
    println!("Dispatch ShareSecret event.");

    ctx.handle_event(&events[0])?;
    {
        println!(
            "Required ident: {:?}",
            ctx.random_state(1)?
                .list_required_secrets_by_from_addr(&transactor_addr)
        );
        assert_eq!(
            0,
            ctx.random_state(1)?
                .list_required_secrets_by_from_addr(&transactor_addr)
                .len()
        );
        assert_eq!(
            Some(DispatchEvent::new(Event::SecretsReady { random_ids: vec![1] }, 0)),
            ctx.current_dispatch()
        );
    }

    // Now, the transactor should be able to reveal all hole cards.
    let decryption = ctx.client_decrypt(&transactor, 1)?;
    println!("Decryption: {:?}", decryption);
    println!("Secrets Ready Event: {:?}", ctx.current_dispatch());
    assert_eq!(2, decryption.len());

    // Now send `SecretReady` to handler.
    ctx.handle_dispatch_event()?;

    Ok(())
}
