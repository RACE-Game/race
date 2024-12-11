use std::sync::Arc;

use crate::component::{
    Broadcaster, Component, EventBridgeParent, EventBus, EventLoop, GameSynchronizer, LocalConnection, PortsHandle, Refunder, Submitter, WrappedClient, WrappedHandler
};
use crate::frame::{EventFrame, SignalFrame};
use race_core::error::{Error, Result};
use race_core::types::{GetCheckpointParams, PlayerDeposit, PlayerJoin, ServerJoin};
use race_core::context::GameContext;
use race_core::storage::StorageT;
use race_core::transport::TransportT;
use race_core::types::{ClientMode, GameAccount, GameMode, ServerAccount};
use race_encryptor::Encryptor;
use race_env::TransactorConfig;
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
    let checkpoint_access_version = game_account
        .checkpoint_on_chain
        .as_ref()
        .map(|cp| cp.access_version)
        .unwrap_or_default();

    let new_players: Vec<PlayerJoin> = game_account
        .players
        .iter()
        .filter(|p| p.access_version > checkpoint_access_version)
        .cloned()
        .collect();

    let new_servers: Vec<ServerJoin> = game_account
        .servers
        .iter()
        .filter(|s| s.access_version > checkpoint_access_version)
        .cloned()
        .collect();

    let settle_version = game_account.settle_version;
    let new_deposits: Vec<PlayerDeposit> = game_account
        .deposits
        .iter()
        .filter(|d| d.settle_version == settle_version)
        .cloned()
        .collect();

    let transactor_addr = game_account
        .transactor_addr
        .clone()
        .ok_or(Error::GameNotServed)?;

    let init_sync = EventFrame::Sync {
        access_version: game_account.access_version,
        new_players,
        new_servers,
        new_deposits,
        transactor_addr,
    };

    Ok(init_sync)
}

impl TransactorHandle {
    pub async fn try_new(
        game_addr: &str,
        server_account: &ServerAccount,
        encryptor: Arc<Encryptor>,
        transport: Arc<dyn TransportT + Send + Sync>,
        storage: Arc<dyn StorageT + Send + Sync>,
        signal_tx: mpsc::Sender<SignalFrame>,
        config: &TransactorConfig,
    ) -> Result<Self> {
        info!(
            "Start game handle for {} with Transactor mode",
            game_addr
        );

        let Some(game_account) = transport.get_game_account(game_addr).await? else {
            return Err(Error::GameAccountNotFound);
        };

        let checkpoint_off_chain = storage
            .get_checkpoint(GetCheckpointParams {
                game_addr: game_addr.to_owned(),
                settle_version: game_account.settle_version,
            })
            .await?;

        let game_context = GameContext::try_new(&game_account, checkpoint_off_chain)?;
        let checkpoint = game_context.checkpoint().clone();

        info!("Use checkpoint: {}", !game_context.checkpoint_is_empty());

        let Some(bundle_account) = transport.get_game_bundle(&game_account.bundle_addr).await? else {
            return Err(Error::GameBundleNotFound);
        };

        let handler = WrappedHandler::load_by_bundle(&bundle_account, encryptor.clone()).await?;

        let event_bus = EventBus::new(game_account.addr.clone());

        let (broadcaster, broadcaster_ctx) = Broadcaster::init(game_account.addr.clone(), 0);
        let mut broadcaster_handle = broadcaster.start(&game_account.addr, broadcaster_ctx);

        let (bridge, bridge_ctx) = EventBridgeParent::init(signal_tx);
        let mut bridge_handle = bridge.start(&game_account.addr, bridge_ctx);

        let (event_loop, event_loop_ctx) = EventLoop::init(
            handler,
            game_context,
            ClientMode::Transactor,
            GameMode::Main,
        );
        let mut event_loop_handle = event_loop.start(&game_account.addr, event_loop_ctx);

        let (submitter, submitter_ctx) =
            Submitter::init(&game_account, transport.clone(), storage.clone(), config.submitter.as_ref());
        let mut submitter_handle = submitter.start(&game_account.addr, submitter_ctx);

        let (synchronizer, synchronizer_ctx) =
            GameSynchronizer::init(transport.clone(), &game_account);

        let (refunder, refunder_ctx) =
            Refunder::init(&game_account, transport.clone());
        let mut refunder_handle = refunder.start(&game_account.addr, refunder_ctx);

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
        let mut client_handle = client.start(&game_account.addr, client_ctx);

        event_bus.attach(&mut broadcaster_handle).await;
        event_bus.attach(&mut bridge_handle).await;
        event_bus.attach(&mut submitter_handle).await;
        event_bus.attach(&mut event_loop_handle).await;
        event_bus.attach(&mut client_handle).await;
        event_bus.attach(&mut refunder_handle).await;

        // Dispatch init state
        event_bus
            .send(EventFrame::InitState {
                access_version: game_account.access_version,
                settle_version: game_account.settle_version,
                checkpoint,
            })
            .await;
        let init_sync = create_init_sync(&game_account)?;
        event_bus.send(init_sync).await;

        let mut synchronizer_handle = synchronizer.start(&game_account.addr, synchronizer_ctx);
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
