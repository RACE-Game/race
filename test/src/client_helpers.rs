use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use race_core::{
    client::Client,
    connection::ConnectionT,
    context::GameContext,
    error::Result,
    event::{CustomEvent, Event},
    secret::SecretState,
    types::{AttachGameParams, ClientMode, ExitGameParams, SubmitEventParams, SubscribeEventParams},
};
use race_encryptor::Encryptor;

use crate::DummyTransport;

pub struct TestClient {
    client: Client,
}

#[derive(Default)]
pub struct DummyConnection {}
#[async_trait]
impl ConnectionT for DummyConnection {
    async fn attach_game(&self, _game_addr: &str, _params: AttachGameParams) -> Result<()> {
        Ok(())
    }
    async fn submit_event(&self, _game_addr: &str, _params: SubmitEventParams) -> Result<()> {
        Ok(())
    }
    async fn exit_game(&self, _game_addr: &str, _params: ExitGameParams) -> Result<()> {
        Ok(())
    }
}

impl TestClient {
    pub fn new(addr: String, game_addr: String, mode: ClientMode) -> Self {
        let transport = Arc::new(DummyTransport::default());
        let encryptor = Arc::new(Encryptor::default());
        let connection = Arc::new(DummyConnection::default());
        Self {
            client: Client::try_new(addr, game_addr, mode, transport, encryptor, connection)
                .expect("Failed to test client"),
        }
    }

    pub fn handle_updated_context(&mut self, ctx: &GameContext) -> Result<Vec<Event>> {
        self.client.handle_updated_context(ctx)
    }

    pub fn decrypt(
        &mut self,
        ctx: &GameContext,
        random_id: usize,
    ) -> Result<HashMap<usize, String>> {
        self.client.decrypt(ctx, random_id)
    }

    pub fn secret_states(&self) -> &Vec<SecretState> {
        &self.client.secret_states
    }

    pub fn custom_event<E: CustomEvent>(&self, custom_event: E) -> Event {
        Event::Custom {
            sender: self.client.addr.to_owned(),
            raw: serde_json::to_string(&custom_event).expect("Failed to serialize custom event"),
        }
    }
}
