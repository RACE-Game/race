//! This component will handle the events sharing between
//! transactors/validators.  Also it will handle the decryption for
//! hidden information when there are enough secrets available.
//! If the client is running as Validator mode, it will create the rpc client to
//! connect to the Transactor.
//! Following events will be handled by this component:
//! - ContextUpdated

use std::rc::Rc;
use std::sync::Arc;

use crate::frame::EventFrame;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use race_core::client::Client;
use race_core::error::Error;
use race_core::transport::TransportT;
use race_core::types::{ClientMode, GameAccount, TransactorAccount};
use race_encryptor::Encryptor;
use tokio::sync::{mpsc, oneshot, watch};

use crate::component::traits::{Attachable, Component, Named};

use super::event_bus::CloseReason;

pub struct WrappedClient {
    pub input_tx: mpsc::Sender<EventFrame>,
    pub output_rx: watch::Receiver<EventFrame>,
    pub closed_rx: oneshot::Receiver<CloseReason>,
    pub ctx: Option<ClientContext>,
}

pub struct ClientContext {
    pub input_rx: mpsc::Receiver<EventFrame>,
    pub output_tx: watch::Sender<EventFrame>,
    pub closed_tx: oneshot::Sender<CloseReason>,
    pub addr: String,
    pub transport: Arc<dyn TransportT>,
    pub mode: ClientMode, // client running mode
}

/// Create RPC client for the transactor of given address.
async fn create_rpc_client_for_transactor(transport: &dyn TransportT, addr: &str) -> HttpClient {
    let transactor_account = transport
        .get_transactor_account(addr)
        .await
        .expect("Failed to fetch transactor account");

    HttpClientBuilder::default()
        .build(&transactor_account.endpoint)
        .expect("Failed to create RPC client")
}

impl WrappedClient {
    pub fn new(
        server_account: &TransactorAccount,
        init_account: &GameAccount,
        transport: Arc<dyn TransportT>,
    ) -> Self {
        let (input_tx, input_rx) = mpsc::channel(3);
        let (output_tx, output_rx) = watch::channel(EventFrame::Empty);
        let (closed_tx, closed_rx) = oneshot::channel();

        // Detect our client mode by check if our address is the transactor address
        let server_addr = server_account.addr.clone();
        let mode = if server_addr.eq(init_account
            .transactor_addr
            .as_ref()
            .expect("Game is not served"))
        {
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
            addr: server_addr,
        });
        Self {
            input_tx,
            output_rx,
            closed_rx,
            ctx,
        }
    }
}

impl Named for WrappedClient {
    fn name<'a>(&self) -> &'a str {
        "Client"
    }
}

impl Attachable for WrappedClient {
    fn input(&self) -> Option<mpsc::Sender<EventFrame>> {
        Some(self.input_tx.clone())
    }

    fn output(&self) -> Option<watch::Receiver<EventFrame>> {
        Some(self.output_rx.clone())
    }
}

impl Component<ClientContext> for WrappedClient {
    fn run(&mut self, ctx: ClientContext) {
        tokio::spawn(async move {
            let ClientContext {
                mut input_rx,
                addr,
                mode,
                transport,
                closed_tx,
                output_tx,
            } = ctx;
            let encryptor =  Rc::new(Encryptor::default());
            let mut client = Client::new(addr, mode, transport, encryptor).expect("Failed to create client");
            let mut res = Ok(());
            'outer: while let Some(event_frame) = input_rx.recv().await {
                match event_frame {
                    EventFrame::ContextUpdated { ref context } => {
                        match client.handle_updated_context(context) {
                            Ok(events) => {
                                for event in events.into_iter() {
                                    if let Err(e) =
                                        output_tx.send(EventFrame::SendServerEvent { event })
                                    {
                                        res = Err(Error::InternalError(e.to_string()));
                                        break 'outer;
                                    }
                                }
                            }
                            Err(e) => {
                                res = Err(e);
                                break 'outer;
                            }
                        }
                    }
                    EventFrame::Shutdown => break,
                    _ => (),
                }
            }

            match res {
                Ok(()) => closed_tx
                    .send(CloseReason::Complete)
                    .expect("Failed to send close reason"),
                Err(e) => closed_tx
                    .send(CloseReason::Fault(e))
                    .expect("Fail to send close reason"),
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

    use race_core::{event::Event, random::ShuffledList, context::GameContext};
    use race_test::*;

    use super::*;

    fn setup() -> (WrappedClient, GameContext) {
        let game_account = TestGameAccountBuilder::default()
            .add_players(2)
            .add_servers(1)
            .build();
        let transactor_account = transactor_account();
        let transport = DummyTransport::default();
        let mut client =
            WrappedClient::new(&transactor_account, &game_account, Arc::new(transport));
        client.start();
        let context = GameContext::new(&game_account).unwrap();
        (client, context)
    }

    #[tokio::test]
    async fn test_lock() {
        let (mut client, mut ctx) = setup();

        // Mask the random_state
        let random = ShuffledList::new(vec!["a", "b", "c"]);
        let rid = ctx.init_random_state(&random);
        let random_state = ctx.get_random_state_mut(rid).unwrap();
        random_state
            .mask(transactor_account_addr(), vec![vec![0], vec![0], vec![0]])
            .unwrap();

        println!("client created");
        client.start();

        let event_frame = EventFrame::ContextUpdated { context: ctx };
        client.input_tx.send(event_frame).await.unwrap();

        println!("before read event");
        client.output_rx.changed().await.unwrap();
        let event_frame = client.output_rx.borrow();

        match &*event_frame {
            EventFrame::SendServerEvent { ref event } => match event {
                Event::Lock {
                    sender,
                    random_id,
                    ciphertexts_and_digests,
                } => {
                    assert_eq!(rid, *random_id);
                    assert_eq!(sender, &transactor_account_addr());
                    assert_eq!(3, ciphertexts_and_digests.len());
                }
                _ => panic!("invalid event type"),
            },
            _ => panic!("invalid event frame"),
        }
    }

    #[tokio::test]
    async fn test_mask() {
        let (mut client, mut ctx) = setup();

        let random = ShuffledList::new(vec!["a", "b", "c"]);
        let rid = ctx.init_random_state(&random);
        println!("random inited");

        println!("client created");
        client.start();

        let event_frame = EventFrame::ContextUpdated { context: ctx };
        client.input_tx.send(event_frame).await.unwrap();

        println!("before read event");
        client.output_rx.changed().await.unwrap();
        let event_frame = client.output_rx.borrow();

        match &*event_frame {
            EventFrame::SendServerEvent { ref event } => match event {
                Event::Mask {
                    sender,
                    random_id,
                    ciphertexts,
                } => {
                    assert_eq!(rid, *random_id);
                    assert_eq!(sender, &transactor_account_addr());
                    assert_eq!(3, ciphertexts.len());
                }
                _ => panic!("invalid event type"),
            },
            _ => panic!("invalid event frame"),
        }
    }
}
