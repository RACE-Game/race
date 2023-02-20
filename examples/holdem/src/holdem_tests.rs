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

type Game = (GameAccount, GameContext, TestHandler<Holdem>, TestClient);

fn set_up() -> Game {
    let game_account = TestGameAccountBuilder::default().add_servers(1).build();
    let mut context = GameContext::try_new(&game_account).unwrap();
    let handler = TestHandler::<Holdem>::init_state(&mut context, &game_account).unwrap();
    let transactor_addr = game_account.transactor_addr.as_ref().unwrap().clone();
    let transactor = TestClient::new(
        transactor_addr.clone(),
        game_account.addr.clone(),
        ClientMode::Transactor,
    );

    (game_account, context, handler, transactor)
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
        assert!(state.is_acting_player("Alice".to_string()));

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


    Ok(())
}
