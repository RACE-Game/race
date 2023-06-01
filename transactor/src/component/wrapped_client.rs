//! This component will handle the events sharing between
//! transactors/validators.  Also it will handle the decryption for
//! hidden information when there are enough secrets available.
//! If the client is running as Validator mode, it will create the rpc client to
//! connect to the Transactor.
//! Following events will be handled by this component:
//! - ContextUpdated

use std::sync::Arc;

use crate::component::common::{Component, ConsumerPorts, Ports};
use crate::frame::EventFrame;
use async_trait::async_trait;
use race_client::Client;
use race_core::connection::ConnectionT;
use race_core::encryptor::EncryptorT;
use race_core::event::Event;
use race_core::transport::TransportT;
use race_core::types::{ClientMode, GameAccount, ServerAccount};
use tracing::{info, warn};

use super::event_bus::CloseReason;

pub struct WrappedClient {}

pub struct ClientContext {
    pub addr: String,
    pub game_addr: String,
    pub transport: Arc<dyn TransportT>,
    pub encryptor: Arc<dyn EncryptorT>,
    pub connection: Arc<dyn ConnectionT>,
    pub mode: ClientMode, // client running mode
}

impl WrappedClient {
    pub fn init(
        server_account: &ServerAccount,
        init_account: &GameAccount,
        transport: Arc<dyn TransportT>,
        encryptor: Arc<dyn EncryptorT>,
        connection: Arc<dyn ConnectionT>,
    ) -> (Self, ClientContext) {
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

        (
            Self {},
            ClientContext {
                transport,
                encryptor,
                connection,
                mode,
                addr: server_addr,
                game_addr: init_account.addr.to_owned(),
            },
        )
    }
}

#[async_trait]
impl Component<ConsumerPorts, ClientContext> for WrappedClient {
    fn name(&self) -> &str {
        "Client"
    }

    async fn run(mut ports: ConsumerPorts, ctx: ClientContext) {
        let ClientContext {
            addr,
            game_addr,
            mode,
            transport,
            encryptor,
            connection,
        } = ctx;

        let mut client = Client::new(addr, game_addr, mode, transport, encryptor, connection);

        if let Err(e) = client.attach_game().await {
            warn!("Failed to attach to game due to error: {:?}", e);
        }

        let mut res = Ok(());
        'outer: while let Some(event_frame) = ports.recv().await {
            // info!("Client receives event frame: {}", event_frame);
            match event_frame {
                EventFrame::Broadcast { event, .. } => {
                    if matches!(event, Event::GameStart { access_version: _ }) {
                        client.flush_secret_states();
                    }
                }
                EventFrame::ContextUpdated { ref context } => {
                    match client.handle_updated_context(context) {
                        Ok(events) => {
                            info!("{} events generated", events.len());
                            for event in events.into_iter() {
                                info!("Connection send event: {}", event);
                                if let Err(_e) = client.submit_event(event).await {
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

        warn!("Shutdown client, result: {:?}", res);
        match res {
            Ok(()) => ports.close(CloseReason::Complete),
            Err(e) => ports.close(CloseReason::Fault(e)),
        };
    }
}

#[cfg(test)]
mod tests {

    use race_core::{context::GameContext, event::Event, random::RandomSpec};
    use race_encryptor::Encryptor;
    use race_test::*;

    use crate::component::common::PortsHandle;

    use super::*;

    fn setup() -> (
        WrappedClient,
        GameContext,
        PortsHandle,
        Arc<DummyConnection>,
    ) {
        let alice = TestClient::player("alice");
        let bob = TestClient::player("bob");
        let transactor = TestClient::transactor("transactor");
        let game_account = TestGameAccountBuilder::default()
            .add_player(&alice, 100)
            .add_player(&bob, 100)
            .set_transactor(&transactor)
            .build();
        let encryptor = Arc::new(Encryptor::default());
        let transactor_account = ServerAccount {
            addr: transactor.get_addr(),
            endpoint: "".into(),
        };
        let connection = Arc::new(DummyConnection::default());
        let transport = Arc::new(DummyTransport::default());
        let (client, client_ctx) = WrappedClient::init(
            &transactor_account,
            &game_account,
            transport,
            encryptor,
            connection.clone(),
        );
        let handle = client.start(client_ctx);
        let mut context = GameContext::try_new(&game_account).unwrap();
        context.set_node_ready(game_account.access_version);
        (client, context, handle, connection)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_lock() {
        let (mut _client, mut ctx, handle, connection) = setup();

        // Mask the random_state
        let random = RandomSpec::shuffled_list(vec!["a".into(), "b".into(), "c".into()]);
        let rid = ctx.init_random_state(random).unwrap();
        let random_state = ctx.get_random_state_mut(rid).unwrap();
        random_state
            .mask("transactor".to_string(), vec![vec![0], vec![0], vec![0]])
            .unwrap();

        let event_frame = EventFrame::ContextUpdated { context: ctx };
        handle.send_unchecked(event_frame).await;

        println!("before read event");
        let event = connection.take().await.unwrap();
        match event {
            Event::Lock {
                sender,
                random_id,
                ciphertexts_and_digests,
            } => {
                assert_eq!(rid, random_id);
                assert_eq!(sender, "transactor".to_string());
                assert_eq!(3, ciphertexts_and_digests.len());
            }
            _ => panic!("Invalid event type"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_mask() {
        let (mut _client, mut ctx, handle, connection) = setup();

        let random = RandomSpec::shuffled_list(vec!["a".into(), "b".into(), "c".into()]);
        println!("context: {:?}", ctx);
        let rid = ctx.init_random_state(random).unwrap();
        println!("random inited");

        let event_frame = EventFrame::ContextUpdated { context: ctx };
        handle.send_unchecked(event_frame).await;

        println!("before read event");
        let event = connection.take().await.unwrap();

        match event {
            Event::Mask {
                sender,
                random_id,
                ciphertexts,
            } => {
                assert_eq!(rid, random_id);
                assert_eq!(sender, "transactor".to_string());
                assert_eq!(3, ciphertexts.len());
            }
            _ => panic!("Invalid event type"),
        }
    }
}
