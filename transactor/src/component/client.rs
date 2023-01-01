//! This component will handle the events sharing between
//! transactors/validators.  Also it will handle the decryption for
//! hidden information when there are enough secrets available.
//! If the client is running as Validator mode, it will create the rpc client to
//! connect to the Transactor.

use std::collections::HashMap;
use std::sync::Arc;

use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee::RpcModule;
use race_core::context::GameContext;
use race_core::error::{Error, Result};
use race_core::random::RandomMode;
use race_core::transport::TransportT;
use race_core::types::{EventFrame, GameAccount, TransactorAccount};
use race_crypto::SecretState;
use race_env::Config;
use tokio::sync::{mpsc, oneshot, watch};

use crate::component::traits::{Attachable, Component, Named};

use super::event_bus::CloseReason;

pub enum ClientMode {
    Transactor,
    Validator,
}

pub struct Client {
    input_tx: mpsc::Sender<EventFrame>,
    output_rx: watch::Receiver<EventFrame>,
    closed_rx: oneshot::Receiver<CloseReason>,
    ctx: Option<ClientContext>,
}

pub struct ClientContext {
    input_rx: mpsc::Receiver<EventFrame>,
    output_tx: watch::Sender<EventFrame>,
    closed_tx: oneshot::Sender<CloseReason>,
    transport: Arc<dyn TransportT>,
    mode: ClientMode,        // client running mode
    transactor_addr: String, // address of current transactor
    server_addr: String, // address of current node, it's the same with above if current node is running as transactor
}

/// Create RPC client for the transactor of given address.
async fn create_rpc_client_for_transactor(transport: Arc<dyn TransportT>, addr: &str) -> HttpClient {
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
        config: Config,
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
        });
        Self {
            input_tx,
            output_rx,
            closed_rx,
            ctx,
        }
    }


    // pub async fn new(addr: &str, init_account: GameAccount, transport: &dyn TransportT) -> Result<Self> {
    //     if !init_account.served {
    //         return Err(Error::GameNotServed);
    //     }

    //     let curr_transactor_account = transport
    //         .get_transactor_account(addr)
    //         .await
    //         .ok_or(Error::InvalidTransactorAddress)?;

    //     // Find the first transactor and connect to it.
    //     let transactor_addr = init_account
    //         .transactors
    //         .iter()
    //         .flatten()
    //         .nth(0)
    //         .ok_or(Error::CantFindTransactor)?;

    //     let (transactor_account, mode) = if addr.eq(transactor_addr) {
    //         (curr_transactor_account, ClientMode::Transactor)
    //     } else {
    //         let main_transactor_account = transport
    //             .get_transactor_account(transactor_addr)
    //             .await
    //             .ok_or(Error::InvalidTransactorAddress)?;
    //         (main_transactor_account, ClientMode::Validator)
    //     };

    //     let rpc_client = HttpClientBuilder::default()
    //         .build(transactor_account.endpoint)
    //         .or(Err(Error::InitializeRpcClientError))?;

    //     Ok(Self {
    //         addr: addr.to_owned(),
    //         mode,
    //         rpc_client,
    //         secret_states: Default::default(),
    //     })
    // }

    // fn randomize_and_mask(&self, context: &GameContext, random_id: usize) -> Result<()> {
    //     match self.mode {
    //         ClientMode::Transactor => (),
    //         ClientMode::Validator => (),
    //     };
    //     Ok(())
    // }

    // fn lock(&self, context: &GameContext, random_id: usize) -> Result<()> {
    //     match self.mode {
    //         ClientMode::Transactor => (),
    //         ClientMode::Validator => (),
    //     };
    //     Ok(())
    // }

    // fn decrypt(&self, context: &GameContext) -> Result<()> {
    //     Ok(())
    // }

    // /// Handle context changes.
    // pub fn handle_context(&mut self, context: &mut GameContext) -> Result<()> {
    //     // Reset secret states when a new game starts.

    //     // Create corresponding secret state when new random state is created.
    //     let random_states = context.list_random_states();
    //     if random_states.len() > self.secret_states.len() {
    //         for i in random_states.len()..self.secret_states.len() {
    //             let rnd_st = &random_states[i];
    //             let secret_state = SecretState::from_random_state(rnd_st, RandomMode::Shuffler);
    //             self.secret_states.push(secret_state);
    //         }
    //     }

    //     // Randomization & Lock & Decryption
    //     for rnd_st in random_states.iter() {
    //         match rnd_st.status {
    //             race_core::random::CipherStatus::Ready => {
    //                 self.decrypt(context)?;
    //             }
    //             race_core::random::CipherStatus::Locking(ref lock_addr) => {
    //                 if lock_addr.eq(&self.addr) {
    //                     self.lock(context, rnd_st.id)?;
    //                 }
    //             }
    //             race_core::random::CipherStatus::Masking(ref mask_addr) => {
    //                 if mask_addr.eq(&self.addr) {
    //                     self.randomize_and_mask(context, rnd_st.id)?;
    //                 }
    //             }
    //         }
    //     }
    //     Ok(())
    // }
}


async fn run_as_transactor(ctx: ClientContext) {}

async fn run_as_validator(ctx: ClientContext) {
    let rpc_client = create_rpc_client_for_transactor(ctx.transport, &ctx.transactor_addr);
}


impl Named for Client {
    fn name<'a>(&self) -> &'a str {
        "Client"
    }
}

impl Attachable for Client {
    fn input(&self) -> Option<mpsc::Sender<race_core::types::EventFrame>> {
        Some(self.input_tx.clone())
    }

    fn output(&self) -> Option<watch::Receiver<race_core::types::EventFrame>> {
        Some(self.output_rx.clone())
    }
}

impl Component<ClientContext> for Client {
    fn run(&mut self, ctx: ClientContext) {
        tokio::spawn(async move {
            let ctx = ctx;

            match ctx.mode {
                ClientMode::Transactor => run_as_transactor(ctx).await,
                ClientMode::Validator => run_as_validator(ctx).await,
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
    use super::*;
}
