use std::sync::Arc;

use crate::component::{
    BridgeToParent, Broadcaster, Component, EventBridgeChild, EventBus, EventLoop, LocalConnection, PortsHandle, WrappedClient, WrappedHandler
};
use crate::frame::EventFrame;
use race_core::error::{Error, Result};
use race_core::context::{GameContext, SubGameInit};
use race_core::storage::StorageT;
use race_core::transport::TransportT;
use race_core::types::{ClientMode, GameMode, ServerAccount};
use race_encryptor::Encryptor;
use race_env::TransactorConfig;

#[allow(dead_code)]
pub struct SubGameHandle {
    pub(crate) addr: String,
    pub(crate) event_bus: EventBus,
    pub(crate) handles: Vec<PortsHandle>,
    pub(crate) broadcaster: Broadcaster,
    pub(crate) bridge_child: EventBridgeChild,
}

impl SubGameHandle {
    pub async fn try_new(
        sub_game_init: SubGameInit,
        bridge_to_parent: BridgeToParent,
        transport: Arc<dyn TransportT + Send + Sync>,
        encryptor: Arc<Encryptor>,
        _storage: Arc<dyn StorageT + Send + Sync>,
        server_account: &ServerAccount,
        _config: &TransactorConfig,
    ) -> Result<Self> {
        let game_addr = sub_game_init.spec.game_addr.clone();
        let game_id = sub_game_init.spec.game_id.clone();
        let addr = format!("{}:{}", game_addr, game_id);
        let event_bus = EventBus::new(addr.to_string());

        let bundle_account = transport
            .get_game_bundle(&sub_game_init.spec.bundle_addr)
            .await?
            .ok_or(Error::GameBundleNotFound)?;

        // Build an InitAccount
        let game_context = GameContext::try_new_with_sub_game_spec(sub_game_init)?;
        let access_version = game_context.access_version();
        let settle_version = game_context.settle_version();
        let checkpoint = game_context.checkpoint().clone();

        let handler = WrappedHandler::load_by_bundle(&bundle_account, encryptor.clone()).await?;

        let (broadcaster, broadcaster_ctx) = Broadcaster::init(addr.clone(), game_id);
        let mut broadcaster_handle = broadcaster.start(&addr, broadcaster_ctx);

        let (bridge, bridge_ctx) = EventBridgeChild::init(game_id, bridge_to_parent);
        let mut bridge_handle = bridge.start(&addr, bridge_ctx);

        let (event_loop, event_loop_ctx) =
            EventLoop::init(handler, game_context, ClientMode::Transactor, GameMode::Sub);
        let mut event_loop_handle = event_loop.start(&addr, event_loop_ctx);

        let mut connection = LocalConnection::new(encryptor.clone());

        event_bus.attach(&mut connection).await;
        let (client, client_ctx) = WrappedClient::init(
            server_account.addr.clone(),
            addr.clone(),
            ClientMode::Transactor,
            transport.clone(),
            encryptor,
            Arc::new(connection),
        );
        let mut client_handle = client.start(&addr, client_ctx);

        event_bus.attach(&mut client_handle).await;
        event_bus.attach(&mut bridge_handle).await;
        event_bus.attach(&mut broadcaster_handle).await;
        event_bus.attach(&mut event_loop_handle).await;

        let init_state = EventFrame::InitState {
            access_version,
            settle_version,
            checkpoint,
        };
        event_bus.send(init_state).await;

        Ok(Self {
            addr: format!("{}:{}", game_addr, game_id),
            event_bus,
            handles: vec![broadcaster_handle, bridge_handle, event_loop_handle],
            broadcaster,
            bridge_child: bridge,
        })
    }
}
