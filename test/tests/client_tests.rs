use std::sync::Arc;

use race_core::client::Client;
use race_core::context::GameContext;
use race_core::error::Result;
use race_core::event::{CustomEvent, Event};
use race_core::random::deck_of_cards;
use race_core::types::{ClientMode, GameAccount};
use race_encryptor::Encryptor;

use race_test::*;
use serde::{Deserialize, Serialize};

fn setup() -> (Client, Arc<Encryptor>, Arc<DummyConnection>, GameAccount) {
    let transport = Arc::new(DummyTransport::default());
    let connection = Arc::new(DummyConnection::default());
    let encryptor = Arc::new(Encryptor::default());
    let client = Client::try_new(
        server_account_addr(0),
        game_account_addr(),
        ClientMode::Transactor,
        transport.clone(),
        encryptor.clone(),
        connection.clone(),
    )
    .unwrap();
    let game_account = TestGameAccountBuilder::default()
        .add_players(2)
        .add_servers(2)
        .build();
    (client, encryptor, connection, game_account)
}

#[tokio::test]
async fn test_attach_game() -> Result<()> {
    let (client, _encryptor, connection, _game_account) = setup();
    client.attach_game().await.unwrap();
    assert_eq!(connection.is_attached().await, true);
    Ok(())
}

#[derive(Serialize, Deserialize)]
pub enum MyEvent {
    Foo,
}
impl CustomEvent for MyEvent {}

#[tokio::test]
async fn test_submit_custom_event() -> Result<()> {
    let (client, _encryptor, connection, _game_account) = setup();
    let event = MyEvent::Foo;
    client.submit_custom_event(event).await.unwrap();
    assert_eq!(
        connection.take().await.unwrap(),
        Event::Custom {
            sender: game_account_addr(),
            raw: "\"Foo\"".into()
        }
    );
    Ok(())
}

#[tokio::test]
async fn test_update_secret_state() -> Result<()> {
    let (mut client, _encryptor, _connection, game_account) = setup();
    let mut ctx = GameContext::try_new(&game_account).unwrap();
    let rnd = deck_of_cards();
    ctx.init_random_state(&rnd);
    ctx.init_random_state(&rnd);
    client.update_secret_state(&ctx).unwrap();
    assert_eq!(client.secret_states.len(), 2);
    Ok(())
}


// #[tokio::test]
// async fn test_randomize_and_share() -> Result<()> {
//     let (mut client, _encryptor, _connection, game_account) = setup();
//     let mut ctx = GameContext::try_new(&game_account).unwrap();
//     let rnd = deck_of_cards();
//     let rid = ctx.init_random_state(&rnd);
//     assert_eq!(rid, 1);
//     client.update_secret_state(&ctx).unwrap();
//     ctx.assign(rid, player_account_addr(0), vec![0, 1, 2]).unwrap();
//     let events = client.randomize_and_share(&ctx).unwrap();
//     assert_eq!(events.len(), 1);
//     Ok(())
// }
