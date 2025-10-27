///! Replay session

use tracing::info;
use race_core::error::{Error, Result};
use race_core::storage::StorageT;
use race_event_record::EventRecords;
use race_components::{EventBus, PortsHandle, ReplayerControl};

pub struct Session {
    pub(crate) addr: String,
    pub(crate) bundle_addr: String,
    pub(crate) handles: Vec<PortsHandle>,
    pub(crate) event_bus: EventBus,
    pub(crate) replayer_control: ReplayerControl,
}

impl Session {
    pub async fn try_new(
        game_addr: String,
        bundle_addr: String,
        init_settle_version: u64,
        storage: Arc<dyn StorageT + Send + Sync>,
    ) -> Result<Self> {
        info!("Start replay session for {}", event_records.header.game_addr);

        let (replayer_control, replayer_control_context) = ReplayerControl::init(storage, init_settle_version);
        let mut replayer_control_handle = replayer_control.start(game_addr.clone(), replayer_control_ctx);

        let event_bus = EventBus::new(game_addr.clone());
        event_bus.attach(&mut replayer_control_handle).await;

        Ok(Self {
            addr: game_addr,
            bundle_addr,
            handles: vec![replayer_control_handle],
            event_bus,
            replayer_control
        })
    }
}
