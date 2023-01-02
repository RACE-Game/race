//! This component will handle the events sharing between
//! transactors/validators.  Also it will handle the decryption for
//! hidden information when there are enough secrets available.
//! If the client is running as Validator mode, it will create the rpc client to
//! connect to the Transactor.

use std::collections::HashMap;
use std::sync::Arc;

use crate::frame::EventFrame;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee::RpcModule;
use race_core::context::GameContext;
use race_core::error::{Error, Result};
use race_core::random::RandomMode;
use race_core::transport::TransportT;
use race_core::types::{GameAccount, TransactorAccount};
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
async fn create_rpc_client_for_transactor(
    transport: Arc<dyn TransportT>,
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
}

/// Send events to local event bus based on game context.
async fn run_as_transactor(ctx: ClientContext) {}

/// Read events from main transactor and validate.
async fn run_as_validator(ctx: ClientContext) {
    let rpc_client = create_rpc_client_for_transactor(ctx.transport, &ctx.transactor_addr);
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
