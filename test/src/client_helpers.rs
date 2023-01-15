use std::{collections::HashMap, sync::Arc};

use race_core::{
    client::Client,
    context::GameContext,
    error::Result,
    event::{CustomEvent, Event},
    secret::SecretState,
    types::ClientMode,
};
use race_encryptor::Encryptor;

use crate::DummyTransport;

pub struct TestClient {
    client: Client,
}

impl TestClient {
    pub fn new(addr: String, mode: ClientMode) -> Self {
        let transport = Arc::new(DummyTransport::default());
        let encryptor = Arc::new(Encryptor::default());
        Self {
            client: Client::try_new(addr, mode, transport, encryptor).expect("Failed to test client"),
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
