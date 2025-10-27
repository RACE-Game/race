use race_transactor_frames::EventFrame;
use super::ComponentEnv;
use super::common::ProducerPorts;
use tokio::sync::mpsc;
use tracing::{info, error};
use async_trait::async_trait;
use crate::common::Component;
use crate::event_bus::CloseReason;
use std::sync::Arc;
use race_core::storage::StorageT;
use race_core::error::Error;
use race_core::types::GetCheckpointParams;

pub enum ReplayerControlFrame {
    Quit,
    Pause,
    Play,
    NextCheckpoint,
    PreviousCheckpoint,
    NextEvent,
    PreviousEvent,
}

pub struct ReplayerControl {
    input_tx: mpsc::Sender<ReplayerControlFrame>,
}

pub struct ReplayerControlContext {
    addr: String,
    input_rx: mpsc::Receiver<ReplayerControlFrame>,
    init_settle_version: u64,
    storage: Arc<dyn StorageT>,
}

impl ReplayerControl {
    pub fn init(addr: String, storage: Arc<dyn StorageT>, init_settle_version: u64) -> (Self, ReplayerControlContext) {
        let (input_tx, input_rx) = mpsc::channel(10);
        (
            Self {
                input_tx,
            },
            ReplayerControlContext {
                addr,
                input_rx,
                init_settle_version,
                storage,
            }
        )
    }

    pub fn get_input_tx(&self) -> mpsc::Sender<ReplayerControlFrame> {
        self.input_tx.clone()
    }
}

#[async_trait]
impl Component<ProducerPorts, ReplayerControlContext> for ReplayerControl {
    fn name() -> &'static str {
        "ReplayerControl"
    }

    async fn run(
        ports: ProducerPorts,
        ctx: ReplayerControlContext,
        env: ComponentEnv,
    ) -> CloseReason {
        let ReplayerControlContext { addr, mut input_rx, storage, init_settle_version } = ctx;

        // The focused checkpoint
        let mut current_settle_version = init_settle_version;

        let checkpoint_off_chain = match storage.get_checkpoint(GetCheckpointParams {
            game_addr: addr.clone(), settle_version: init_settle_version
        }).await {
            Err(e) => {
                error!("Failed to get checkpoint with address {} and settle version {}", addr, init_settle_version);
                return CloseReason::Fault(Error::MissingCheckpoint);
            }
            Ok(None) => {
                error!("No checkpoint with address {} and settle version {}", addr, init_settle_version);
                return CloseReason::Fault(Error::MissingCheckpoint);
            }
            Ok(Some(checkpoint_off_chain)) => {
                checkpoint_off_chain
            }
        };

        while let Some(frame) = input_rx.recv().await {
            match frame {
                ReplayerControlFrame::Quit => {
                    info!("Quit replayer");
                    ports.send(EventFrame::Shutdown).await;
                    break;
                }
                ReplayerControlFrame::Pause => {
                    info!("Pause replayer");
                }
                ReplayerControlFrame::Play => {}
                ReplayerControlFrame::NextCheckpoint => {}
                ReplayerControlFrame::PreviousCheckpoint => {}
                ReplayerControlFrame::NextEvent => {}
                ReplayerControlFrame::PreviousEvent => {}
            }
        }

        CloseReason::Complete
    }
}
