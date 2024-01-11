use std::sync::Arc;

use crate::component::{
    Component, EventBridgeParent, EventBus, EventLoop, PortsHandle, RemoteConnection, Subscriber,
    Voter, WrappedClient, WrappedHandler, WrappedTransport,
};
use crate::frame::{EventFrame, SignalFrame};
use race_api::error::{Error, Result};
use race_core::context::GameContext;
use race_core::transport::TransportT;
use race_core::types::{ClientMode, GameAccount, GameBundle, ServerAccount};
use race_encryptor::Encryptor;
use tokio::sync::mpsc;
use tracing::info;

#[allow(dead_code)]
pub struct ValidatorHandle {
    pub(crate) addr: String,
    pub(crate) event_bus: EventBus,
    pub(crate) handles: Vec<PortsHandle>,
    pub(crate) bridge_parent: EventBridgeParent,
}

impl ValidatorHandle {
    pub async fn try_new(
        game_account: &GameAccount,
        server_account: &ServerAccount,
        bundle_account: &GameBundle,
        encryptor: Arc<Encryptor>,
        transport: Arc<WrappedTransport>,
        signal_tx: mpsc::Sender<SignalFrame>,
    ) -> Result<Self> {
        info!(
            "Start game handle for {} with Validator mode",
            game_account.addr
        );
        let game_context = GameContext::try_new(game_account)?;
        let handler = WrappedHandler::load_by_bundle(bundle_account, encryptor.clone()).await?;

        let transactor_addr = game_account
            .transactor_addr
            .as_ref()
            .ok_or(Error::GameNotServed)?;
        let transactor_account = transport
            .get_server_account(transactor_addr)
            .await?
            .ok_or(Error::CantFindTransactor)?;

        info!("Creating components");
        let event_bus = EventBus::default();

        let (bridge, bridge_ctx) = EventBridgeParent::init(signal_tx);
        let mut bridge_handle = bridge.start(bridge_ctx);

        let (event_loop, event_loop_ctx) =
            EventLoop::init(handler, game_context, ClientMode::Validator);
        let mut event_loop_handle = event_loop.start(event_loop_ctx);

        let connection = Arc::new(
            RemoteConnection::try_new(
                &server_account.addr,
                &transactor_account.endpoint,
                encryptor.clone(),
            )
            .await?,
        );
        let (subscriber, subscriber_context) =
            Subscriber::init(game_account, server_account, connection.clone());
        let mut subscriber_handle = subscriber.start(subscriber_context);

        let (client, client_ctx) = WrappedClient::init(
            server_account.addr.clone(),
            game_account.addr.clone(),
            ClientMode::Validator,
            transport.clone(),
            encryptor,
            connection,
        );
        let mut client_handle = client.start(client_ctx);

        let (voter, voter_ctx) = Voter::init(game_account, server_account, transport.clone());
        let mut voter_handle = voter.start(voter_ctx);

        event_bus.attach(&mut bridge_handle).await;
        event_bus.attach(&mut event_loop_handle).await;
        event_bus.attach(&mut voter_handle).await;
        event_bus.attach(&mut client_handle).await;

        let init_account = game_account.derive_checkpoint_init_account();
        info!("InitAccount: {:?}", init_account);

        // Dispatch init state
        event_bus.send(EventFrame::InitState { init_account }).await;

        event_bus.attach(&mut subscriber_handle).await;
        Ok(Self {
            addr: game_account.addr.clone(),
            event_bus,
            handles: vec![
                subscriber_handle,
                client_handle,
                event_loop_handle,
                voter_handle,
            ],
            bridge_parent: bridge,
        })
    }
}
