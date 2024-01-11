use std::sync::Arc;

use crate::component::{
    Broadcaster, Component, EventBridgeParent, EventBus, EventLoop, GameSynchronizer,
    LocalConnection, PortsHandle, Submitter, WrappedClient, WrappedHandler,
};
use crate::frame::{EventFrame, SignalFrame};
use race_api::error::{Result, Error};
use race_api::types::{ServerJoin, PlayerJoin};
use race_core::context::GameContext;
use race_core::transport::TransportT;
use race_core::types::{ClientMode, GameAccount, GameBundle, ServerAccount};
use race_encryptor::Encryptor;
use tokio::sync::mpsc;
use tracing::info;

#[allow(dead_code)]
pub struct TransactorHandle {
    pub(crate) addr: String,
    pub(crate) handles: Vec<PortsHandle>,
    pub(crate) event_bus: EventBus,
    pub(crate) broadcaster: Broadcaster,
    pub(crate) bridge_parent: EventBridgeParent,
}

fn create_init_sync(game_account: &GameAccount) -> Result<EventFrame> {
    let new_players: Vec<PlayerJoin> = game_account
        .players
        .iter()
        .filter(|p| p.access_version > game_account.checkpoint_access_version)
        .cloned()
        .collect();

    let new_servers: Vec<ServerJoin> = game_account
        .servers
        .iter()
        .filter(|s| s.access_version > game_account.checkpoint_access_version)
        .cloned()
        .collect();

    let transactor_addr = game_account.transactor_addr.clone().ok_or(Error::GameNotServed)?;

    let init_sync = EventFrame::Sync {
        access_version: game_account.access_version,
        new_players,
        new_servers,
        transactor_addr,
    };

    Ok(init_sync)
}

impl TransactorHandle {
    pub async fn try_new(
        game_account: &GameAccount,
        server_account: &ServerAccount,
        bundle_account: &GameBundle,
        encryptor: Arc<Encryptor>,
        transport: Arc<dyn TransportT + Send + Sync>,
        signal_tx: mpsc::Sender<SignalFrame>,
    ) -> Result<Self> {
        info!(
            "Start game handle for {} with Transactor mode",
            game_account.addr
        );

        let game_context = GameContext::try_new(game_account)?;
        let handler = WrappedHandler::load_by_bundle(bundle_account, encryptor.clone()).await?;

        let event_bus = EventBus::default();

        let (broadcaster, broadcaster_ctx) = Broadcaster::init(game_account.addr.clone());
        let mut broadcaster_handle = broadcaster.start(broadcaster_ctx);

        let (bridge, bridge_ctx) = EventBridgeParent::init(signal_tx);
        let mut bridge_handle = bridge.start(bridge_ctx);

        let (event_loop, event_loop_ctx) =
            EventLoop::init(handler, game_context, ClientMode::Transactor);
        let mut event_loop_handle = event_loop.start(event_loop_ctx);

        let (submitter, submitter_ctx) = Submitter::init(game_account, transport.clone());
        let mut submitter_handle = submitter.start(submitter_ctx);

        let (synchronizer, synchronizer_ctx) =
            GameSynchronizer::init(transport.clone(), game_account);

        let mut connection = LocalConnection::new(encryptor.clone());

        event_bus.attach(&mut connection).await;
        let (client, client_ctx) = WrappedClient::init(
            server_account.addr.clone(),
            game_account.addr.clone(),
            ClientMode::Transactor,
            transport.clone(),
            encryptor,
            Arc::new(connection),
        );
        let mut client_handle = client.start(client_ctx);

        event_bus.attach(&mut broadcaster_handle).await;
        event_bus.attach(&mut bridge_handle).await;
        event_bus.attach(&mut submitter_handle).await;
        event_bus.attach(&mut event_loop_handle).await;
        event_bus.attach(&mut client_handle).await;

        // Dispatch init state
        let init_account = game_account.derive_checkpoint_init_account();
        info!("InitAccount: {:?}", init_account);
        event_bus.send(EventFrame::InitState { init_account }).await;
        event_bus.send(create_init_sync(game_account)?).await;

        let mut synchronizer_handle = synchronizer.start(synchronizer_ctx);
        event_bus.attach(&mut synchronizer_handle).await;

        Ok(Self {
            addr: game_account.addr.clone(),
            event_bus,
            handles: vec![
                broadcaster_handle,
                submitter_handle,
                event_loop_handle,
                client_handle,
                synchronizer_handle,
            ],
            broadcaster,
            bridge_parent: bridge,
        })
    }
}
