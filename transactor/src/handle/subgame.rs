use std::sync::Arc;

use race_transactor_frames::{EventFrame, BridgeToParent};
use race_transactor_components::{
    Broadcaster, Component, EventBridgeChild, EventBus, EventLoop, LocalConnection, PortsHandle, WrappedClient,
};
use race_core::error::Result;
use race_core::storage::StorageT;
use race_core::transport::TransportT;
use race_core::types::{ClientMode, GameMode, ServerAccount};
use race_core::checkpoint::ContextCheckpoint;
use race_encryptor::Encryptor;
use race_env::TransactorConfig;

#[allow(dead_code)]
pub struct SubGameHandle {
    pub(crate) addr: String,
    pub(crate) bundle_addr: String,
    pub(crate) event_bus: EventBus,
    pub(crate) handles: Vec<PortsHandle>,
    pub(crate) broadcaster: Broadcaster,
    pub(crate) bridge_child: EventBridgeChild,
}

impl SubGameHandle {
    pub async fn try_new(
        checkpoint: ContextCheckpoint,
        bridge_to_parent: BridgeToParent,
        transport: Arc<dyn TransportT + Send + Sync>,
        encryptor: Arc<Encryptor>,
        _storage: Arc<dyn StorageT + Send + Sync>,
        server_account: &ServerAccount,
        _config: &TransactorConfig,
    ) -> Result<Self> {
        let game_spec = &checkpoint.root_data().game_spec;
        let addr = format!("{}:{}", game_spec.game_addr, game_spec.game_id);
        let event_bus = EventBus::new(addr.to_string());

        let (broadcaster, broadcaster_ctx) = Broadcaster::init(addr.clone(), game_spec.game_id);
        let mut broadcaster_handle = broadcaster.start(&addr, broadcaster_ctx);

        let (bridge, bridge_ctx) = EventBridgeChild::init(game_spec.game_id, bridge_to_parent);
        let mut bridge_handle = bridge.start(&addr, bridge_ctx);

        let (event_loop, event_loop_ctx) =
            EventLoop::init(
                game_spec.clone(),
                encryptor.clone(),
                transport.clone(),
                ClientMode::Transactor,
                GameMode::Sub
            );

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

        let init_frame = EventFrame::RecoverCheckpointWithCredentials {
            checkpoint: checkpoint.clone()
        };

        event_bus.send(init_frame).await;

        Ok(Self {
            addr: format!("{}:{}", game_spec.game_addr, game_spec.game_id),
            bundle_addr: checkpoint.root_data().game_spec.bundle_addr.to_owned(),
            event_bus,
            handles: vec![broadcaster_handle, bridge_handle, event_loop_handle],
            broadcaster,
            bridge_child: bridge,
        })
    }
}
