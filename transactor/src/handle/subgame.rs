use std::sync::Arc;

use crate::component::{
    Broadcaster, Component, EventBridgeChild, EventBridgeParent, EventBus, EventLoop,
    LocalConnection, PortsHandle, WrappedClient, WrappedHandler,
};
use crate::frame::EventFrame;
use race_api::error::{Error, Result};
use race_core::context::GameContext;
use race_core::transport::TransportT;
use race_core::types::{ClientMode, ServerAccount, SubGameSpec};
use race_encryptor::Encryptor;

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
        spec: SubGameSpec,
        bridge_parent: EventBridgeParent,
        server_account: &ServerAccount,
        encryptor: Arc<Encryptor>,
        transport: Arc<dyn TransportT + Send + Sync>,
    ) -> Result<Self> {
        println!("Launch sub game, nodes: {:?}", spec.nodes);

        let game_addr = spec.game_addr.clone();
        let sub_id = spec.sub_id.clone();
        let addr = format!("{}:{}", game_addr, sub_id);
        let event_bus = EventBus::new(addr.to_string());

        let bundle_account = transport
            .get_game_bundle(&spec.bundle_addr)
            .await?
            .ok_or(Error::GameBundleNotFound)?;

        // Build an InitAccount
        let game_context = GameContext::try_new_with_sub_game_spec(&spec)?;
        let access_version = spec.access_version;
        let settle_version = spec.settle_version;

        let handler = WrappedHandler::load_by_bundle(&bundle_account, encryptor.clone()).await?;

        let (broadcaster, broadcaster_ctx) = Broadcaster::init(addr.clone());
        let mut broadcaster_handle = broadcaster.start(&addr, broadcaster_ctx);

        let (bridge, bridge_ctx) = bridge_parent.derive_child(sub_id.clone());
        let mut bridge_handle = bridge.start(&addr, bridge_ctx);

        let (event_loop, event_loop_ctx) =
            EventLoop::init(handler, game_context, ClientMode::Transactor);
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
        event_bus
            .send(EventFrame::InitState {
                init_account: spec.init_account,
                access_version,
                settle_version,
            })
            .await;

        Ok(Self {
            addr: format!("{}:{}", game_addr, sub_id),
            event_bus,
            handles: vec![broadcaster_handle, bridge_handle, event_loop_handle],
            broadcaster,
            bridge_child: bridge,
        })
    }
}
