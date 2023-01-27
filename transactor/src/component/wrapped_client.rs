//! This component will handle the events sharing between
//! transactors/validators.  Also it will handle the decryption for
//! hidden information when there are enough secrets available.
//! If the client is running as Validator mode, it will create the rpc client to
//! connect to the Transactor.
//! Following events will be handled by this component:
//! - ContextUpdated

use std::sync::Arc;

use crate::frame::EventFrame;
use race_client::Client;
use race_core::connection::ConnectionT;
use race_core::encryptor::EncryptorT;
use race_core::transport::TransportT;
use race_core::types::{ClientMode, GameAccount, ServerAccount};
use tokio::sync::{mpsc, oneshot};
use tracing::info;

use crate::component::traits::{Attachable, Component, Named};

use super::event_bus::CloseReason;

pub struct WrappedClient {
    pub input_tx: mpsc::Sender<EventFrame>,
    pub closed_rx: oneshot::Receiver<CloseReason>,
    pub ctx: Option<ClientContext>,
}

pub struct ClientContext {
    pub input_rx: mpsc::Receiver<EventFrame>,
    pub closed_tx: oneshot::Sender<CloseReason>,
    pub addr: String,
    pub game_addr: String,
    pub transport: Arc<dyn TransportT>,
    pub encryptor: Arc<dyn EncryptorT>,
    pub connection: Arc<dyn ConnectionT>,
    pub mode: ClientMode, // client running mode
}

impl WrappedClient {
    pub fn new(
        server_account: &ServerAccount,
        init_account: &GameAccount,
        transport: Arc<dyn TransportT>,
        encryptor: Arc<dyn EncryptorT>,
        connection: Arc<dyn ConnectionT>,
    ) -> Self {
        let (input_tx, input_rx) = mpsc::channel(3);
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
            closed_tx,
            transport,
            encryptor,
            connection,
            mode,
            addr: server_addr,
            game_addr: init_account.addr.to_owned(),
        });
        Self {
            input_tx,
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

    fn output(&mut self) -> Option<mpsc::Receiver<EventFrame>> {
        // let mut ret = None;
        // std::mem::swap(&mut ret, &mut self.output_rx);
        // ret
        None
    }
}

impl Component<ClientContext> for WrappedClient {
    fn run(&mut self, ctx: ClientContext) {
        tokio::spawn(async move {
            let ClientContext {
                mut input_rx,
                addr,
                game_addr,
                mode,
                transport,
                encryptor,
                connection,
                closed_tx,
            } = ctx;

            let mut client =
                Client::try_new(addr, game_addr, mode, transport, encryptor, connection)
                    .expect("Failed to create client");

            client
                .attach_game()
                .await
                .expect("Failed to attach to game");

            let mut res = Ok(());
            'outer: while let Some(event_frame) = input_rx.recv().await {
                match event_frame {
                    EventFrame::ContextUpdated { ref context } => {
                        match client.handle_updated_context(context) {
                            Ok(events) => {
                                for event in events.into_iter() {
                                    info!("Connection send event: {}", event);
                                    if let Err(_e) = client.submit_event(event).await {
                                        // TODO: Should vote for another transactor.
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

    use race_core::{context::GameContext, event::Event, random::ShuffledList};
    use race_encryptor::Encryptor;
    use race_test::*;

    use super::*;

    fn setup() -> (WrappedClient, GameContext, Arc<DummyConnection>) {
        let game_account = TestGameAccountBuilder::default()
            .add_players(2)
            .add_servers(1)
            .build();
        let encryptor = Arc::new(Encryptor::default());
        let transactor_account = transactor_account();
        let connection = Arc::new(DummyConnection::default());
        let transport = Arc::new(DummyTransport::default());
        let mut client = WrappedClient::new(
            &transactor_account,
            &game_account,
            transport,
            encryptor,
            connection.clone(),
        );
        client.start();
        let context = GameContext::try_new(&game_account).unwrap();
        (client, context, connection)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_lock() {
        let (mut client, mut ctx, connection) = setup();

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
        let event = connection.take().await.unwrap();
        match event {
            Event::Lock {
                sender,
                random_id,
                ciphertexts_and_digests,
            } => {
                assert_eq!(rid, random_id);
                assert_eq!(sender, transactor_account_addr());
                assert_eq!(3, ciphertexts_and_digests.len());
            }
            _ => panic!("Invalid event type"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_mask() {
        let (mut client, mut ctx, connection) = setup();

        let random = ShuffledList::new(vec!["a", "b", "c"]);
        let rid = ctx.init_random_state(&random);
        println!("random inited");

        println!("client created");
        client.start();

        let event_frame = EventFrame::ContextUpdated { context: ctx };
        client.input_tx.send(event_frame).await.unwrap();

        println!("before read event");
        let event = connection.take().await.unwrap();

        match event {
            Event::Mask {
                sender,
                random_id,
                ciphertexts,
            } => {
                assert_eq!(rid, random_id);
                assert_eq!(sender, transactor_account_addr());
                assert_eq!(3, ciphertexts.len());
            }
            _ => panic!("Invalid event type"),
        }
    }
}
