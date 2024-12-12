use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use race_core::error::{Error, Result};
use race_api::event::{CustomEvent, Event};
use race_api::types::GamePlayer;
use race_client::Client;
use race_core::types::{GameAccount, PlayerJoin};
use race_core::{
    connection::ConnectionT,
    context::GameContext,
    secret::SecretState,
    types::{AttachGameParams, ClientMode, DecisionId, ExitGameParams, SubmitEventParams},
};
use race_encryptor::Encryptor;
use tokio::sync::{mpsc, Mutex};

use crate::misc::test_game_addr;
use crate::transport_helpers::DummyTransport;

pub struct TestClient {
    id: Option<u64>,
    client: Client,
}

pub(crate) trait AsGameContextRef {
    fn as_game_context_ref(&self) -> &GameContext;
}

impl AsGameContextRef for GameContext {
    fn as_game_context_ref(&self) -> &GameContext {
        &self
    }
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
            id: None,
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

    pub(crate) fn set_id(&mut self, id: u64) {
        self.id = Some(id)
    }

    pub(crate) fn join(
        &mut self,
        game_context: &mut GameContext,
        game_account: &mut GameAccount,
        balance: u64,
    ) -> Result<GamePlayer> {
        if self.client.mode != ClientMode::Player {
            panic!("TestClient can only join with Player mode");
        }

        if game_account.players.len() >= game_account.max_players as _ {
            return Err(Error::GameIsFull(game_account.max_players as _));
        }

        game_account.access_version += 1;
        let id = game_account.access_version;

        let mut position = 0;
        for i in 0..game_account.max_players {
            if game_account
                .players
                .iter()
                .find(|p| p.position == i)
                .is_none()
            {
                position = i;
                break;
            }
        }

        game_account.players.push(PlayerJoin {
            addr: self.client.addr.clone(),
            position,
            access_version: id,
            verify_key: "".into(),
        });
        game_account.deposits.push(race_core::types::PlayerDeposit {
            addr: self.client.addr.clone(),
            amount: balance,
            access_version: game_account.access_version,
            settle_version: game_account.settle_version,
        });
        self.set_id(id);
        game_context.add_node(self.client.addr.clone(), id, ClientMode::Player);

        Ok(GamePlayer::new(id, position))
    }

    pub fn transactor<S: Into<String>>(addr: S) -> Self {
        Self::new(addr, ClientMode::Transactor)
    }

    pub fn validator<S: Into<String>>(addr: S) -> Self {
        Self::new(addr, ClientMode::Validator)
    }

    pub(crate) fn handle_updated_context<T: AsGameContextRef>(
        &mut self,
        ctx: &T,
    ) -> Result<Vec<Event>> {
        self.client
            .handle_updated_context(ctx.as_game_context_ref())
    }

    pub fn mode(&self) -> ClientMode {
        self.client.mode.clone()
    }

    pub fn addr(&self) -> String {
        self.client.addr.clone()
    }

    pub(crate) fn decrypt<T: AsGameContextRef>(
        &self,
        ctx: &T,
        random_id: usize,
    ) -> Result<HashMap<usize, String>> {
        self.client.decrypt(ctx.as_game_context_ref(), random_id)
    }

    pub fn secret_state(&self) -> &SecretState {
        &self.client.secret_state
    }

    pub fn id(&self) -> u64 {
        self.id
            .expect(&format!("Client {} is not in game", self.client.addr))
    }

    pub fn custom_event<E: CustomEvent>(&self, custom_event: E) -> Event {
        Event::Custom {
            sender: self.id(),
            raw: borsh::to_vec(&custom_event).unwrap(),
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
        let event = Event::GameStart;
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
