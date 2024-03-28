//! This component will handle the events sharing between
//! transactors/validators.  Also it will handle the decryption for
//! hidden information when there are enough secrets available.
//! If the client is running as Validator mode, it will create the rpc client to
//! connect to the Transactor.
//! Following events will be handled by this component:
//! - ContextUpdated

use std::sync::Arc;

use crate::component::common::{Component, ConsumerPorts};
use crate::frame::EventFrame;
use async_trait::async_trait;
use race_client::Client;
use race_core::connection::ConnectionT;
use race_core::encryptor::EncryptorT;
use race_core::transport::TransportT;
use race_core::types::ClientMode;
use tracing::{error, warn};

use super::ComponentEnv;
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
        addr: String,
        game_addr: String,
        mode: ClientMode,
        transport: Arc<dyn TransportT>,
        encryptor: Arc<dyn EncryptorT>,
        connection: Arc<dyn ConnectionT>,
    ) -> (Self, ClientContext) {
        (
            Self {},
            ClientContext {
                transport,
                encryptor,
                connection,
                mode,
                addr,
                game_addr,
            },
        )
    }
}

#[async_trait]
impl Component<ConsumerPorts, ClientContext> for WrappedClient {
    fn name() -> &'static str {
        "Client"
    }

    async fn run(mut ports: ConsumerPorts, ctx: ClientContext, env: ComponentEnv) -> CloseReason {
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
            warn!("{} Failed to attach to game due to error: {:?}", env.log_prefix, e);
        }

        let mut res = Ok(());
        'outer: while let Some(event_frame) = ports.recv().await {
            match event_frame {
                EventFrame::ContextUpdated { ref context } => {
                    match client.handle_updated_context(context) {
                        Ok(events) => {
                            for event in events.into_iter() {
                                // info!("Connection send event: {}", event);
                                if let Err(_e) = client.submit_event(event).await {
                                    break 'outer;
                                }
                            }
                            // if context.is_checkpoint() {
                            //     client.flush_secret_states();
                            // }
                        }
                        Err(e) => {
                            error!("{} Client error: {:?}", env.log_prefix, e);
                            res = Err(e);
                            break 'outer;
                        }
                    }
                }
                EventFrame::GameStart { .. } => {
                    client.flush_secret_states();
                }
                EventFrame::Shutdown => break,
                _ => (),
            }
        }

        return match res {
            Ok(()) => CloseReason::Complete,
            Err(e) => CloseReason::Fault(e),
        };
    }
}

#[cfg(test)]
mod tests {

    use race_api::prelude::*;
    use race_core::types::ServerAccount;
    use race_encryptor::Encryptor;
    use race_test::prelude::*;

    use crate::component::common::PortsHandle;

    use super::*;

    fn setup() -> (
        WrappedClient,
        GameContext,
        PortsHandle,
        Arc<DummyConnection>,
        TestClient,
    ) {
        let mut alice = TestClient::player("alice");
        let mut bob = TestClient::player("bob");
        let mut transactor = TestClient::transactor("transactor");
        let game_account = TestGameAccountBuilder::default()
            .add_player(&mut alice, 100)
            .add_player(&mut bob, 100)
            .set_transactor(&mut transactor)
            .build();
        let encryptor = Arc::new(Encryptor::default());
        let transactor_account = ServerAccount {
            addr: transactor.addr(),
            endpoint: "".into(),
        };
        let connection = Arc::new(DummyConnection::default());
        let transport = Arc::new(DummyTransport::default());
        let (client, client_ctx) = WrappedClient::init(
            transactor_account.addr.clone(),
            game_account.addr.clone(),
            ClientMode::Transactor,
            transport,
            encryptor,
            connection.clone(),
        );
        let handle = client.start(&game_account.addr, client_ctx);
        let mut context = GameContext::try_new(&game_account).unwrap();
        context.set_node_ready(game_account.access_version);
        (client, context, handle, connection, transactor)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_lock() {
        let (mut _client, mut ctx, handle, connection, tx) = setup();

        // Mask the random_state
        let random = RandomSpec::shuffled_list(vec!["a".into(), "b".into(), "c".into()]);
        let rid = ctx.init_random_state(random).unwrap();
        let random_state = ctx.get_random_state_mut(rid).unwrap();
        random_state
            .mask("transactor".to_string(), vec![vec![0], vec![0], vec![0]])
            .unwrap();

        let event_frame = EventFrame::ContextUpdated {
            context: Box::new(ctx),
        };
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
                assert_eq!(sender, tx.id());
                assert_eq!(3, ciphertexts_and_digests.len());
            }
            _ => panic!("Invalid event type"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_mask() {
        let (mut _client, mut ctx, handle, connection, tx) = setup();

        let random = RandomSpec::shuffled_list(vec!["a".into(), "b".into(), "c".into()]);
        println!("context: {:?}", ctx);
        let rid = ctx.init_random_state(random).unwrap();
        println!("random inited");

        let event_frame = EventFrame::ContextUpdated {
            context: Box::new(ctx),
        };
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
                assert_eq!(sender, tx.id());
                assert_eq!(3, ciphertexts.len());
            }
            _ => panic!("Invalid event type"),
        }
    }
}
