//! This component will handle the events sharing between
//! transactors/validators.  Also it will handle the decryption for
//! hidden information when there are enough secrets available.
//! If the client is running as Validator mode, it will create the rpc client to
//! connect to the Transactor.
//! Following events will be handled by this component:
//! - ContextUpdated

use std::sync::Arc;

use crate::frame::EventFrame;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use race_core::context::GameContext;
use race_core::error::{Error, Result};
use race_core::event::Event;
use race_core::random::{CipherStatus, RandomMode};
use race_core::transport::TransportT;
use race_core::types::{GameAccount, TransactorAccount};
use race_crypto::SecretState;
use tokio::sync::{mpsc, oneshot, watch};

use crate::component::traits::{Attachable, Component, Named};

use super::event_bus::CloseReason;

pub enum ClientMode {
    Transactor,
    Validator,
}

pub struct Client {
    pub input_tx: mpsc::Sender<EventFrame>,
    pub output_rx: watch::Receiver<EventFrame>,
    pub closed_rx: oneshot::Receiver<CloseReason>,
    pub ctx: Option<ClientContext>,
}

pub struct ClientContext {
    pub input_rx: mpsc::Receiver<EventFrame>,
    pub output_tx: watch::Sender<EventFrame>,
    pub closed_tx: oneshot::Sender<CloseReason>,
    pub transport: Arc<dyn TransportT>,
    pub mode: ClientMode,        // client running mode
    pub transactor_addr: String, // address of current transactor
    pub server_addr: String, // address of current node, it's the same with above if current node is running as transactor
    pub secret_states: Vec<SecretState>,
}

/// Create RPC client for the transactor of given address.
async fn create_rpc_client_for_transactor(
    transport: &dyn TransportT,
    addr: &str,
) -> HttpClient {
    let transactor_account = transport
        .get_transactor_account(addr)
        .await
        .expect("Failed to fetch transactor account");

    HttpClientBuilder::default()
        .build(&transactor_account.endpoint)
        .expect("Failed to create RPC client")
}

impl Client {
    pub fn new(
        server_account: &TransactorAccount,
        init_account: &GameAccount,
        transport: Arc<dyn TransportT>,
    ) -> Self {
        let (input_tx, input_rx) = mpsc::channel(3);
        let (output_tx, output_rx) = watch::channel(EventFrame::Empty);
        let (closed_tx, closed_rx) = oneshot::channel();
        let server_addr = server_account.addr.clone();
        let transactor_addr = init_account
            .transactor_addr
            .clone()
            .expect("Game not served");
        let mode = if server_addr.eq(&transactor_addr) {
            ClientMode::Transactor
        } else {
            ClientMode::Validator
        };
        let ctx = Some(ClientContext {
            input_rx,
            output_tx,
            closed_tx,
            transport,
            mode,
            server_addr,
            transactor_addr,
            secret_states: Vec::new(),
        });
        Self {
            input_tx,
            output_rx,
            closed_rx,
            ctx,
        }
    }
}

async fn update_secret_state(
    client_context: &mut ClientContext,
    game_context: &GameContext,
) -> Result<()> {
    let random_states = game_context.list_random_states();
    let secret_states = &mut client_context.secret_states;
    if random_states.len() > secret_states.len() {
        for random_state in random_states.iter().skip(secret_states.len()) {
            let secret_state = SecretState::from_random_state(random_state, RandomMode::Shuffler);
            secret_states.push(secret_state);
        }
    }
    Ok(())
}

async fn randomize(client_context: &mut ClientContext, game_context: &GameContext) -> Result<()> {
    for random_state in game_context.list_random_states().iter() {
        match random_state.status {
            CipherStatus::Ready => (),
            CipherStatus::Locking(ref addr) => {
                // check if our operation is being requested
                if client_context.server_addr.eq(addr) {
                    let secret_state = client_context
                        .secret_states
                        .get_mut(random_state.id)
                        .expect("Failed to get secret state");

                    let origin = random_state
                        .ciphertexts
                        .iter()
                        .map(|c| c.ciphertext().to_owned())
                        .collect();

                    let locked = secret_state
                        .lock(origin)
                        .map_err(|e| Error::RandomizationError(e.to_string()))?;

                    let event = Event::Lock {
                        sender: client_context.server_addr.clone(),
                        random_id: random_state.id,
                        ciphertexts_and_digests: locked,
                    };

                    let event_frame = EventFrame::SendServerEvent {
                        addr: client_context.server_addr.clone(),
                        event,
                    };

                    client_context
                        .output_tx
                        .send(event_frame)
                        .map_err(|e| Error::InternalError(e.to_string()))?;
                }
            }
            CipherStatus::Masking(ref addr) => {
                // check if our operation is being requested
                if client_context.server_addr.eq(addr) {
                    let secret_state = client_context
                        .secret_states
                        .get_mut(random_state.id)
                        .expect("Failed to get secret state");

                    let origin = random_state
                        .ciphertexts
                        .iter()
                        .map(|c| c.ciphertext().to_owned())
                        .collect();

                    let masked = secret_state
                        .mask(origin)
                        .map_err(|e| Error::RandomizationError(e.to_string()))?;

                    let event = Event::Randomize {
                        sender: client_context.server_addr.clone(),
                        random_id: random_state.id,
                        ciphertexts: masked,
                    };

                    let event_frame = EventFrame::SendServerEvent {
                        addr: client_context.server_addr.clone(),
                        event,
                    };

                    client_context
                        .output_tx
                        .send(event_frame)
                        .map_err(|e| Error::InternalError(e.to_string()))?;
                }
            }
        }
    }
    Ok(())
}

async fn decrypt(_client_context: &mut ClientContext, _game_context: &GameContext) -> Result<()> {
    Ok(())
}

/// Send events to local event bus based on game context.
async fn run_as_transactor(client_context: &mut ClientContext) -> Result<()> {
    while let Some(event_frame) = client_context.input_rx.recv().await {
        match event_frame {
            EventFrame::ContextUpdated { ref context } => {
                update_secret_state(client_context, context).await?;
                randomize(client_context, context).await?;
                decrypt(client_context, context).await?;
            }
            EventFrame::Shutdown => break,
            _ => (),
        }
    }
    Ok(())
}

/// Read events from main transactor and validate.
async fn run_as_validator(client_context: &mut ClientContext) -> Result<()> {
    let _rpc_client = create_rpc_client_for_transactor(
        &*client_context.transport,
        &client_context.transactor_addr,
    )
    .await;

    while let Some(event_frame) = client_context.input_rx.recv().await {
        match event_frame {
            EventFrame::ContextUpdated { ref context } => {
                update_secret_state(client_context, context).await?;
                randomize(client_context, context).await?;
                decrypt(client_context, context).await?;
            }
            EventFrame::Shutdown => break,
            _ => (),
        }
    }
    Ok(())
}

impl Named for Client {
    fn name<'a>(&self) -> &'a str {
        "Client"
    }
}

impl Attachable for Client {
    fn input(&self) -> Option<mpsc::Sender<EventFrame>> {
        Some(self.input_tx.clone())
    }

    fn output(&self) -> Option<watch::Receiver<EventFrame>> {
        Some(self.output_rx.clone())
    }
}

impl Component<ClientContext> for Client {
    fn run(&mut self, ctx: ClientContext) {
        tokio::spawn(async move {
            let mut ctx = ctx;

            let res = match ctx.mode {
                ClientMode::Transactor => run_as_transactor(&mut ctx).await,
                ClientMode::Validator => run_as_validator(&mut ctx).await,
            };

            match res {
                Ok(()) => ctx
                    .closed_tx
                    .send(CloseReason::Complete)
                    .expect("Failed to send close reason"),
                Err(e) => ctx
                    .closed_tx
                    .send(CloseReason::Fault(e))
                    .expect("Failed to send close reason"),
            }
        });
    }

    fn borrow_mut_ctx(&mut self) -> &mut Option<ClientContext> {
        &mut self.ctx
    }

    fn closed(self) -> oneshot::Receiver<CloseReason> {
        self.closed_rx
    }
}

#[cfg(test)]
mod tests {
    use race_core_test::*;

    use super::*;

    #[test]
    fn test_mask() {
        let game_account = game_account_with_empty_data();
        let transactor_account = transactor_account();
        let transport = DummyTransport::default();

        let client = Client::new(&transactor_account, &game_account, Arc::new(transport));

    }
}
