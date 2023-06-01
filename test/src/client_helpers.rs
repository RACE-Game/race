use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use race_client::Client;
use race_core::{
    connection::ConnectionT,
    context::GameContext,
    error::Result,
    event::{CustomEvent, Event},
    secret::SecretState,
    types::{AttachGameParams, ClientMode, DecisionId, ExitGameParams, SubmitEventParams},
};
use race_encryptor::Encryptor;
use tokio::sync::{mpsc, Mutex};

use crate::{transport_helpers::DummyTransport, test_game_addr};

pub struct TestClient {
    client: Client,
}

pub struct DummyConnection {
    rx: Mutex<mpsc::Receiver<Event>>,
    tx: mpsc::Sender<Event>,
    pub attached: Mutex<bool>,
}

impl Default for DummyConnection {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel(1);
        Self {
            tx,
            rx: Mutex::new(rx),
            attached: Mutex::new(false),
        }
    }
}

impl DummyConnection {
    pub async fn take(&self) -> Option<Event> {
        self.rx.lock().await.recv().await
    }

    pub async fn is_attached(&self) -> bool {
        *self.attached.lock().await
    }
}

#[async_trait]
impl ConnectionT for DummyConnection {
    async fn attach_game(&self, _game_addr: &str, _params: AttachGameParams) -> Result<()> {
        let mut attached = self.attached.lock().await;
        *attached = true;
        Ok(())
    }
    async fn submit_event(&self, _game_addr: &str, params: SubmitEventParams) -> Result<()> {
        self.tx.send(params.event).await.unwrap();
        Ok(())
    }
    async fn exit_game(&self, _game_addr: &str, _params: ExitGameParams) -> Result<()> {
        Ok(())
    }
}

impl TestClient {
    pub fn new<S: Into<String>>(addr: S, mode: ClientMode) -> Self {
        let addr = addr.into();
        let transport = Arc::new(DummyTransport::default());
        let encryptor = Arc::new(Encryptor::default());
        let connection = Arc::new(DummyConnection::default());
        Self {
            client: Client::new(
                addr,
                test_game_addr(),
                mode,
                transport,
                encryptor,
                connection,
            ),
        }
    }

    pub fn player<S: Into<String>>(addr: S) -> Self {
        Self::new(addr, ClientMode::Player)
    }

    pub fn transactor<S: Into<String>>(addr: S) -> Self {
        Self::new(addr, ClientMode::Transactor)
    }

    pub fn validator<S: Into<String>>(addr: S) -> Self {
        Self::new(addr, ClientMode::Validator)
    }

    pub fn handle_updated_context(&mut self, ctx: &GameContext) -> Result<Vec<Event>> {
        self.client.handle_updated_context(ctx)
    }

    pub fn get_mode(&self) -> ClientMode {
        self.client.mode.clone()
    }

    pub fn get_addr(&self) -> String {
        self.client.addr.clone()
    }

    pub fn decrypt(
        &mut self,
        ctx: &GameContext,
        random_id: usize,
    ) -> Result<HashMap<usize, String>> {
        self.client.decrypt(ctx, random_id)
    }

    pub fn secret_state(&self) -> &SecretState {
        &self.client.secret_state
    }

    pub fn custom_event<E: CustomEvent>(&self, custom_event: E) -> Event {
        Event::Custom {
            sender: self.client.addr.to_owned(),
            raw: custom_event.try_to_vec().unwrap(),
        }
    }

    pub fn answer(&mut self, decision_id: DecisionId, answer: String) -> Result<Event> {
        self.client.answer_event(decision_id, answer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dummy_connection() -> Result<()> {
        let conn = DummyConnection::default();
        let event = Event::GameStart { access_version: 1 };
        conn.submit_event(
            "",
            SubmitEventParams {
                event: event.clone(),
            },
        )
        .await?;
        let event_1 = conn.take().await.unwrap();
        assert_eq!(event, event_1);
        Ok(())
    }
}
