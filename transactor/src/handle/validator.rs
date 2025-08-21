use std::sync::Arc;

use crate::component::{
    Component, EventBridgeParent, EventBus, EventLoop, PortsHandle, RemoteConnection, Subscriber,
    Voter, WrappedClient, WrappedHandler,
};
use crate::frame::{EventFrame, SignalFrame};
use race_core::context::GameContext;
use race_core::error::{Error, Result};
use race_core::storage::StorageT;
use race_core::transport::TransportT;
use race_core::types::{CheckpointParams, ClientMode, GameMode, ServerAccount};
use race_encryptor::Encryptor;
use race_env::TransactorConfig;
use tokio::sync::mpsc;
use tracing::info;

#[allow(dead_code)]
pub struct ValidatorHandle {
    pub(crate) addr: String,
    pub(crate) bundle_addr: String,
    pub(crate) event_bus: EventBus,
    pub(crate) handles: Vec<PortsHandle>,
    pub(crate) bridge_parent: EventBridgeParent,
}

impl ValidatorHandle {
    pub async fn try_new(
        game_addr: &str,
        server_account: &ServerAccount,
        encryptor: Arc<Encryptor>,
        transport: Arc<dyn TransportT + Send + Sync>,
        _storage: Arc<dyn StorageT + Send + Sync>,
        signal_tx: mpsc::Sender<SignalFrame>,
        _config: &TransactorConfig,
    ) -> Result<Self> {
        info!("Start game handle for {} with Validator mode", game_addr,);
        let Some(game_account) = transport.get_game_account(game_addr).await? else {
            return Err(Error::GameAccountNotFound);
        };

        let transactor_addr = game_account
            .transactor_addr
            .as_ref()
            .ok_or(Error::GameNotServed)?;
        let transactor_account = transport
            .get_server_account(transactor_addr)
            .await?
            .ok_or(Error::CantFindTransactor)?;

        let connection = Arc::new(
            RemoteConnection::try_new(
                &server_account.addr,
                &transactor_account.endpoint,
                encryptor.clone(),
            )
            .await?,
        );

        let checkpoint_off_chain = connection.get_checkpoint_off_chain(
            game_addr,
            CheckpointParams {
                settle_version: game_account.settle_version,
            },
        ).await?;

        // let checkpoint_off_chain = storage
        //     .get_checkpoint(GetCheckpointParams {
        //         game_addr: game_addr.to_owned(),
        //         settle_version: game_account.settle_version,
        //     })
        //     .await?;

        let game_context = GameContext::try_new(&game_account, checkpoint_off_chain)?;
        let checkpoint = game_context.checkpoint().clone();

        let Some(bundle_account) = transport.get_game_bundle(&game_account.bundle_addr).await?
        else {
            return Err(Error::GameBundleNotFound);
        };

        let handler = Box::new(WrappedHandler::load_by_bundle(&bundle_account, encryptor.clone()).await?);

        info!("Creating components");
        let event_bus = EventBus::new(game_account.addr.clone());

        let (bridge, bridge_ctx) = EventBridgeParent::init(signal_tx);
        let mut bridge_handle = bridge.start(&game_account.addr, bridge_ctx);

        let (event_loop, event_loop_ctx) =
            EventLoop::init(handler, game_context, ClientMode::Validator, GameMode::Main);
        let mut event_loop_handle = event_loop.start(&game_account.addr, event_loop_ctx);

        let (subscriber, subscriber_context) =
            Subscriber::init(&game_account, server_account, connection.clone());
        let mut subscriber_handle = subscriber.start(&game_account.addr, subscriber_context);

        let (client, client_ctx) = WrappedClient::init(
            server_account.addr.clone(),
            game_account.addr.clone(),
            ClientMode::Validator,
            transport.clone(),
            encryptor,
            connection,
        );
        let mut client_handle = client.start(&game_account.addr, client_ctx);

        let (voter, voter_ctx) = Voter::init(&game_account, server_account, transport.clone());
        let mut voter_handle = voter.start(&game_account.addr, voter_ctx);

        event_bus.attach(&mut bridge_handle).await;
        event_bus.attach(&mut event_loop_handle).await;
        event_bus.attach(&mut voter_handle).await;
        event_bus.attach(&mut client_handle).await;

        // Dispatch init state
        event_bus
            .send(EventFrame::InitState {
                access_version: game_account.access_version,
                settle_version: game_account.settle_version,
                checkpoint,
            })
            .await;

        event_bus.attach(&mut subscriber_handle).await;
        Ok(Self {
            addr: game_account.addr.clone(),
            bundle_addr: game_account.bundle_addr.clone(),
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
