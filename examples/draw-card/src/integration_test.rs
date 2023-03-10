use crate::{AccountData, DrawCard, GameEvent, Player, GameStage};
use race_core::{
    context::{DispatchEvent, GameContext, GameStatus},
    error::{Error, Result},
    event::Event,
    random::RandomStatus,
    types::{ClientMode, PlayerJoin},
};
use race_test::{transactor_account_addr, TestClient, TestGameAccountBuilder, TestHandler};

#[test]
fn test() -> Result<()> {
    env_logger::builder().is_test(true).try_init().unwrap();

    // Initialize the game account, with 1 player joined.
    // The game account must be served, so we add one server which is the transactor.
    let account_data = AccountData {
        blind_bet: 100,
        min_bet: 100,
        max_bet: 1000,
    };
    println!("Create game account");
    let game_account = TestGameAccountBuilder::default()
        .add_servers(1)
        .add_players(1)
        .with_data(account_data)
        .build();
    let transactor_addr = game_account.transactor_addr.as_ref().unwrap().clone();

    // Initialize player client, which simulates the behavior of player.
    println!("Create test clients");
    let mut alice = TestClient::new(
        "Alice".into(),
        game_account.addr.clone(),
        ClientMode::Player,
    );
    let mut bob = TestClient::new("Bob".into(), game_account.addr.clone(), ClientMode::Player);

    // Initialize the client, which simulates the behavior of transactor.
    let mut transactor = TestClient::new(
        transactor_addr.clone(),
        game_account.addr.clone(),
        ClientMode::Transactor,
    );

    // Create game context and test handler.
    // Initalize the handler state with game account.
    // The game will not start since two players are required.
    println!("Initialize handler state");
    let mut ctx = GameContext::try_new(&game_account)?;
    let mut handler = TestHandler::init_state(&mut ctx, &game_account)?;
    assert_eq!(1, ctx.count_players());

    // Start game
    println!("Start game, without players");
    let first_event = ctx.gen_start_game_event();
    let ret = handler.handle_event(&mut ctx, &first_event);
    assert_eq!(ret, Err(Error::NoEnoughPlayers));

    {
        let state: &DrawCard = handler.get_state();
        assert_eq!(0, state.bet);
    }

    // Another player joined the game.
    // Now we have enough players, an event of `GameStart` should be dispatched.
    println!("Player Bob join the game");
    let av = ctx.get_access_version() + 1;
    let sync_event = Event::Sync {
        new_players: vec![PlayerJoin {
            addr: "Bob".into(),
            balance: 10000,
            position: 1,
            access_version: av,
        }],
        new_servers: vec![],
        transactor_addr: transactor_account_addr(),
        access_version: av,
    };

    handler.handle_event(&mut ctx, &sync_event)?;

    {
        assert_eq!(2, ctx.count_players());
        assert_eq!(GameStatus::Uninit, ctx.get_status());
        assert_eq!(
            Some(DispatchEvent::new(
                Event::GameStart {
                    access_version: ctx.get_access_version()
                },
                0
            )),
            *ctx.get_dispatch()
        );
    }

    // Now the dispatching event should be `GameStart`.
    // By handling this event, a random deck of cards should be created.
    println!("Start game");
    handler.handle_dispatch_event(&mut ctx)?;
    {
        let state: &DrawCard = handler.get_state();
        assert_eq!(GameStatus::Running, ctx.get_status());
        assert_eq!(1, state.random_id);
        assert_eq!(
            vec![
                Player {
                    addr: "Alice".into(),
                    balance: 10000,
                    bet: 0
                },
                Player {
                    addr: "Bob".into(),
                    balance: 10000,
                    bet: 0
                }
            ],
            state.players
        );
        assert_eq!(
            RandomStatus::Masking(transactor_addr.clone()),
            ctx.get_random_state_unchecked(1).status
        );
    }

    // Now we are in the randomization progress. Servers will create the events in turn.
    // But in our test case, we have only one server.
    //
    // Now, Let the client handle the updated context.
    // The corresponding secert state will be initialized, which contains all the secrets.
    // Additionally, one `Mask` event will be created.
    let events = transactor.handle_updated_context(&ctx)?;
    {
        assert_eq!(1, transactor.secret_state().list_random_secrets().len());
        assert_eq!(1, events.len());
    }

    // Send the mask event to handler, we expect the random status to be changed to `Locking`.
    handler.handle_event(&mut ctx, &events[0])?;

    {
        assert_eq!(
            RandomStatus::Locking(transactor_addr.clone()),
            ctx.get_random_state_unchecked(1).status
        );
    }

    // Now, Let the client handle the updated context.
    // One `Lock` event will be created.
    let events = transactor.handle_updated_context(&ctx)?;
    {
        assert_eq!(1, events.len());
    }

    // Send the lock event to handler, we expect the random status to be changed to `Ready`.
    // Since all randomness is ready, an event of `RandomnessReady` will be dispatched.
    handler.handle_event(&mut ctx, &events[0])?;

    {
        assert_eq!(
            RandomStatus::Ready,
            ctx.get_random_state_unchecked(1).status
        );
        assert_eq!(
            Some(DispatchEvent::new(
                Event::RandomnessReady { random_id: 1 },
                0
            )),
            *ctx.get_dispatch()
        );
    }

    // Handle this dispatched `RandomnessReady`.
    // We expect each player to be assigned one card.
    handler.handle_dispatch_event(&mut ctx)?;
    {
        let random_state = ctx.get_random_state_unchecked(1);
        let ciphertexts_for_alice = random_state.list_assigned_ciphertexts("Alice");
        let ciphertexts_for_bob = random_state.list_assigned_ciphertexts("Bob");
        assert_eq!(RandomStatus::WaitingSecrets, random_state.status);
        assert_eq!(1, ciphertexts_for_alice.len());
        assert_eq!(1, ciphertexts_for_bob.len());
    }

    // Let client handle the updated context.
    // `ShareSecret` event should be created.
    println!("Transactor handle updated context, generate `ShareSecrets` event");
    let events = transactor.handle_updated_context(&ctx)?;
    {
        let event = &events[0];
        assert!(
            matches!(event, Event::ShareSecrets { sender, shares } if sender.eq(&transactor_addr) && shares.len() == 2)
        );
    }

    // Handle `ShareSecret` event.
    // Expect the random status to be changed to ready.
    handler.handle_event(&mut ctx, &events[0])?;
    {
        assert_eq!(
            RandomStatus::Ready,
            ctx.get_random_state_unchecked(1).status
        );
        let random_state = ctx.get_random_state_unchecked(1);
        assert_eq!(1, random_state.list_shared_secrets("Alice").unwrap().len());
        assert_eq!(1, random_state.list_shared_secrets("Bob").unwrap().len());
        assert_eq!(
            Some(DispatchEvent::new(Event::SecretsReady, 0)),
            *ctx.get_dispatch()
        );
    }

    println!("Dispatch `SecretsReady` event, update game stage to `Betting`");
    handler.handle_dispatch_event(&mut ctx)?;
    {
        let state =  handler.get_state();
        assert_eq!(GameStage::Betting, state.stage);
    }

    // Now, client should be able to see their cards.
    println!("Checked assigned cards");
    let alice_decryption = alice.decrypt(&ctx, 1)?;
    let bob_decryption = bob.decrypt(&ctx, 1)?;
    {
        println!("Alice decryption: {:?}", alice_decryption);
        println!("Bob decryption: {:?}", bob_decryption);
        assert_eq!(1, alice_decryption.len());
        assert_eq!(1, bob_decryption.len());
    }

    // Now, Alice is the first to act.
    // So, she can send a bet event and we expect the bet amount to be updated to 500.
    println!("Alice bets");
    let event = alice.custom_event(GameEvent::Bet(500));
    handler.handle_event(&mut ctx, &event)?;
    {
        let state = handler.get_state();
        assert_eq!(500, state.bet);
    }

    // Bob calls this.
    // Now, it's time to reveal the cards, so two secrets for hands are required.
    println!("Bob calls");
    let event = bob.custom_event(GameEvent::Call);
    handler.handle_event(&mut ctx, &event)?;
    {
        let random_state = ctx.get_random_state_unchecked(1);
        assert_eq!(
            2,
            random_state
                .list_required_secrets_by_from_addr(&transactor_addr)
                .len()
        );
    }

    // Let the client handle this update.
    // We expect two secrets to be shared.
    let events = transactor.handle_updated_context(&ctx)?;
    {
        let event = &events[0];
        println!(
            "Required ident: {:?}",
            ctx.get_random_state_unchecked(1)
                .list_required_secrets_by_from_addr(&transactor_addr)
        );
        assert_eq!(
            2,
            ctx.get_random_state_unchecked(1)
                .list_required_secrets_by_from_addr(&transactor_addr)
                .len()
        );
        assert!(
            matches!(event, Event::ShareSecrets { sender, shares } if sender.eq(&transactor_addr) && shares.len() == 2)
        );
    }

    // Handle `ShareSecret` event.
    println!("Dispatch ShareSecret event.");
    handler.handle_event(&mut ctx, &events[0])?;
    {
        println!(
            "Required ident: {:?}",
            ctx.get_random_state_unchecked(1)
                .list_required_secrets_by_from_addr(&transactor_addr)
        );
        assert_eq!(
            0,
            ctx.get_random_state_unchecked(1)
                .list_required_secrets_by_from_addr(&transactor_addr)
                .len()
        );
        assert_eq!(
            Some(DispatchEvent::new(Event::SecretsReady, 0)),
            *ctx.get_dispatch()
        );
    }

    // Now, the transactor should be able to reveal all hole cards.
    let decryption = transactor.decrypt(&ctx, 1)?;
    println!("Decryption: {:?}", decryption);
    assert_eq!(2, decryption.len());

    // Now send `SecretReady` to handler.
    handler.handle_dispatch_event(&mut ctx)?;
    {
        assert!(matches!(ctx.get_settles(),
                         Some(settles) if settles.len() == 2
        ));
    }

    Ok(())
}
