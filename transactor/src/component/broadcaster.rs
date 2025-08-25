//! The broadcaster will broadcast events to all connected participants
//! The broadcast should also save

use std::collections::LinkedList;
use std::sync::Arc;

use borsh::{BorshSerialize, BorshDeserialize};
use async_trait::async_trait;
use race_api::event::Event;
use race_api::types::GameId;
use race_core::checkpoint::CheckpointOffChain;
use race_core::types::{BroadcastFrame, BroadcastSync, TxState};
use race_core::context::Node;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};

use crate::component::common::{Component, ConsumerPorts};
use crate::frame::EventFrame;

use super::{CloseReason, ComponentEnv};

/// Backup events in memeory, for new connected clients.  The
/// `settle_version` and `access_version` are the values at the time
/// we handle the events. The backups always start with a checkpoint
/// event which contains the initial handler state
#[derive(Debug)]
pub struct EventBackup {
    pub event: Event,
    pub timestamp: u64,
    pub state_sha: String,
}

#[derive(Debug)]
pub struct EventBackupGroup {
    pub state_sha: String,
    pub sync: BroadcastSync,
    pub events: LinkedList<EventBackup>,
    pub settle_version: u64,
    pub checkpoint_off_chain: Option<CheckpointOffChain>,
    pub nodes: Vec<Node>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct CheckpointBroadcastFrame {
    pub data: Vec<u8>,
    pub nodes: Vec<Node>,
}

impl EventBackupGroup {
    pub fn to_frames(&self) -> Vec<BroadcastFrame> {
        let mut frames = vec![];
        frames.push(BroadcastFrame::Sync {
            sync: self.sync.clone(),
        });
        for event in self.events.iter() {
            frames.push(BroadcastFrame::Event {
                event: event.event.clone(),
                timestamp: event.timestamp,
                state_sha: event.state_sha.clone(),
            });
        }
        frames
    }

    pub fn merge_sync(&mut self, sync: &BroadcastSync) {
        self.sync.merge(sync)
    }
}

pub struct BroadcasterContext {
    #[allow(unused)]
    id: String,
    game_id: GameId,
    event_backup_groups: Arc<RwLock<LinkedList<EventBackupGroup>>>,
    broadcast_tx: broadcast::Sender<BroadcastFrame>,
    checkpoint_tx: broadcast::Sender<CheckpointBroadcastFrame>,
}

/// A component that pushes event to clients.
pub struct Broadcaster {
    #[allow(unused)]
    id: String,
    #[allow(unused)]
    game_id: GameId,
    event_backup_groups: Arc<RwLock<LinkedList<EventBackupGroup>>>,
    broadcast_tx: broadcast::Sender<BroadcastFrame>,
    checkpoint_tx: broadcast::Sender<CheckpointBroadcastFrame>,
}

impl Broadcaster {
    pub fn init(id: String, game_id: GameId) -> (Self, BroadcasterContext) {
        let event_backup_groups = Arc::new(RwLock::new(LinkedList::new()));
        let (broadcast_tx, broadcast_rx) = broadcast::channel(10);
        let (checkpoint_tx, checkpoint_rx) = broadcast::channel(10);
        drop(broadcast_rx);
        drop(checkpoint_rx);
        (
            Self {
                id: id.clone(),
                game_id,
                event_backup_groups: event_backup_groups.clone(),
                broadcast_tx: broadcast_tx.clone(),
                checkpoint_tx: checkpoint_tx.clone(),
            },
            BroadcasterContext {
                id,
                game_id,
                event_backup_groups,
                broadcast_tx,
                checkpoint_tx,
            },
        )
    }

    pub fn get_checkpoint_rx(&self) -> broadcast::Receiver<CheckpointBroadcastFrame> {
        self.checkpoint_tx.subscribe()
    }

    pub fn get_broadcast_rx(&self) -> broadcast::Receiver<BroadcastFrame> {
        self.broadcast_tx.subscribe()
    }

    pub async fn get_latest_checkpoint_broadcast_frame(&self) -> Option<CheckpointBroadcastFrame> {
        let event_backup_groups = self.event_backup_groups.read().await;
        let latest_group = event_backup_groups.iter().last()?;
        let vd = latest_group.checkpoint_off_chain.as_ref()?.data.get(&self.game_id)?;
        return Some(CheckpointBroadcastFrame {
            data: vd.data.clone(),
            nodes: latest_group.nodes.clone(),
        });
    }

    pub async fn get_latest_checkpoint(&self) -> Option<CheckpointOffChain> {
        let event_backup_groups = self.event_backup_groups.read().await;

        if let Some(latest_group) = event_backup_groups.iter().last() {
            return latest_group.checkpoint_off_chain.clone();
        }

        None
    }

    pub async fn get_checkpoint(&self, settle_version: u64) -> Option<CheckpointOffChain> {
        let event_backup_groups = self.event_backup_groups.read().await;
        info!("Get checkpoint with settle_version = {}", settle_version);

        for group in event_backup_groups.iter() {
            if group.settle_version == settle_version {
                return group.checkpoint_off_chain.clone();
            }
        }

        warn!("Missing the checkpoint for settle_version = {}, the client won't be able to join this game.", settle_version);
        let available_settle_versions: Vec<u64> = event_backup_groups
            .iter()
            .map(|g| g.settle_version)
            .collect();
        warn!("Available versions are: {:?}", available_settle_versions);
        None
    }

    /// Retrieve a list of event histories with a given
    /// `settle_version`.  All events happened after the
    /// `settle_version` will be returned.  If a zero `settle_version`
    /// is provided, just return the events after the latest
    /// checkpoint.
    pub async fn get_backlogs(&self, settle_version: u64) -> BroadcastFrame {
        let event_backup_groups = self.event_backup_groups.read().await;

        let mut checkpoint_off_chain: Option<CheckpointOffChain> = None;
        let mut backlogs: Vec<BroadcastFrame> = vec![];
        let mut state_sha = "".to_string();

        // By default, returns the histories with settle_version
        // greater than the given one
        if settle_version > 0 {
            for group in event_backup_groups.iter() {
                if group.settle_version == settle_version {
                    checkpoint_off_chain = group.checkpoint_off_chain.clone();
                    state_sha = group.state_sha.clone();
                }
                if group.settle_version >= settle_version {
                    backlogs.append(&mut group.to_frames());
                }
            }
        }

        if backlogs.is_empty() {
            if let Some(group) = event_backup_groups.iter().last() {
                checkpoint_off_chain = group.checkpoint_off_chain.clone();
                state_sha = group.state_sha.clone();
                backlogs.append(&mut group.to_frames());
            }
        }

        BroadcastFrame::Backlogs {
            checkpoint_off_chain,
            backlogs: Box::new(backlogs),
            state_sha,
        }
    }
}

#[async_trait]
impl Component<ConsumerPorts, BroadcasterContext> for Broadcaster {
    fn name() -> &'static str {
        "Broadcaster"
    }

    async fn run(
        mut ports: ConsumerPorts,
        ctx: BroadcasterContext,
        env: ComponentEnv,
    ) -> CloseReason {
        while let Some(event) = ports.recv().await {
            match event {
                EventFrame::SendMessage { message } => {
                    let r = ctx.broadcast_tx.send(BroadcastFrame::Message { message });

                    if let Err(e) = r {
                        // Usually it means no receivers
                        debug!("{} Failed to broadcast event: {:?}", env.log_prefix, e);
                    }
                }

                EventFrame::Checkpoint {
                    access_version,
                    settle_version,
                    checkpoint,
                    state_sha,
                    nodes,
                } => {
                    info!(
                        "{} Create new history group (via Checkpoint) with access_version = {}, settle_version = {}",
                        env.log_prefix, access_version, settle_version
                    );
                    let mut event_backup_groups = ctx.event_backup_groups.write().await;
                    let checkpoint_off_chain = checkpoint.derive_offchain_part();

                    if let Some(vd) = checkpoint_off_chain.data.get(&ctx.game_id) {
                        let r = ctx.checkpoint_tx.send(CheckpointBroadcastFrame {
                            nodes: nodes.clone(),
                            data: vd.data.to_owned(),
                        });
                        if let Err(e) = r {
                            // Usually it means no receivers
                            debug!("{} Failed to broadcast checkpoint: {:?}", env.log_prefix, e);
                        }
                    }


                    event_backup_groups.push_back(EventBackupGroup {
                        sync: BroadcastSync::new(access_version),
                        state_sha,
                        events: LinkedList::new(),
                        settle_version,
                        checkpoint_off_chain: Some(checkpoint_off_chain),
                        nodes,
                    });


                }

                EventFrame::InitState {
                    access_version,
                    settle_version,
                    checkpoint,
                    ..
                } => {
                    info!(
                        "{} Create new history group (via InitState) with access_version = {}, settle_version = {}",
                        env.log_prefix, access_version, settle_version
                    );
                    let mut event_backup_groups = ctx.event_backup_groups.write().await;

                    event_backup_groups.push_back(EventBackupGroup {
                        sync: BroadcastSync::new(access_version),
                        events: LinkedList::new(),
                        settle_version,
                        checkpoint_off_chain: Some(checkpoint.derive_offchain_part()),
                        state_sha: "".into(),
                        nodes: Vec::default(),
                    });
                }

                EventFrame::TxState { tx_state } => match tx_state {
                    TxState::SettleSucceed { .. } => {
                        let r = ctx.broadcast_tx.send(BroadcastFrame::TxState { tx_state });

                        if let Err(e) = r {
                            debug!("{} Failed to broadcast event: {:?}", env.log_prefix, e);
                        }
                    }
                    TxState::PlayerConfirming { .. } => {
                        let r = ctx.broadcast_tx.send(BroadcastFrame::TxState { tx_state });

                        if let Err(e) = r {
                            debug!("{} Failed to broadcast event: {:?}", env.log_prefix, e);
                        }
                    }
                    TxState::PlayerConfirmingFailed(_) => {
                        let r = ctx.broadcast_tx.send(BroadcastFrame::TxState { tx_state });

                        if let Err(e) = r {
                            debug!("{} Failed to broadcast event: {:?}", env.log_prefix, e);
                        }
                    }
                },

                EventFrame::Broadcast {
                    event,
                    timestamp,
                    state_sha,
                    ..
                } => {
                    // info!("{} Broadcaster receive event: {}", env.log_prefix, event);
                    let mut event_backup_groups = ctx.event_backup_groups.write().await;

                    if let Some(current) = event_backup_groups.back_mut() {
                        current.events.push_back(EventBackup {
                            event: event.clone(),
                            timestamp,
                            state_sha: state_sha.clone(),
                        });
                    } else {
                        error!("{} Received event without checkpoint", env.log_prefix);
                    }
                    // Keep 200 groups at most
                    if event_backup_groups.len() > 200 {
                        event_backup_groups.pop_front();
                    }
                    drop(event_backup_groups);

                    let r = ctx.broadcast_tx.send(BroadcastFrame::Event {
                        event,
                        timestamp,
                        state_sha,
                    });

                    if let Err(e) = r {
                        // Usually it means no receivers
                        debug!("{} Failed to broadcast event: {:?}", env.log_prefix, e);
                    }
                }

                EventFrame::Sync {
                    new_servers,
                    new_players,
                    new_deposits,
                    access_version,
                    transactor_addr,
                } => {
                    let sync = BroadcastSync {
                        new_players,
                        new_servers,
                        new_deposits,
                        access_version,
                        transactor_addr,
                    };

                    ctx.event_backup_groups
                        .write()
                        .await
                        .back_mut()
                        .map(|g| g.merge_sync(&sync));

                    let broadcast_frame = BroadcastFrame::Sync { sync };
                    let r = ctx.broadcast_tx.send(broadcast_frame);

                    if let Err(e) = r {
                        debug!(
                            "{} Failed to broadcast node updates: {:?}",
                            env.log_prefix, e
                        );
                    }
                }
                EventFrame::SubSync {
                    access_version,
                    new_players,
                    new_servers,
                    transactor_addr,
                } => {
                    let sync = BroadcastSync {
                        new_players,
                        new_servers,
                        new_deposits: vec![],
                        access_version,
                        transactor_addr,
                    };

                    ctx.event_backup_groups
                        .write()
                        .await
                        .back_mut()
                        .map(|g| g.merge_sync(&sync));

                    let broadcast_frame = BroadcastFrame::Sync { sync };
                    let r = ctx.broadcast_tx.send(broadcast_frame);

                    if let Err(e) = r {
                        debug!(
                            "{} Failed to broadcast node updates: {:?}",
                            env.log_prefix, e
                        );
                    }
                }
                EventFrame::Shutdown => {
                    info!("{} Stopped", env.log_prefix);
                    break;
                }
                _ => (),
            }
        }

        CloseReason::Complete
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use race_core::{checkpoint::Checkpoint, types::PlayerJoin};
    use race_test::prelude::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 3)]
    async fn test_event_reorder() {
        // Test how the events are grouped when the checkpoint is delayed due to settle_lock.
        // Assume we have the following case, where in event handler, things happen in this order:
        // InitState, CustomEvent, Checkpoint, CustomEvent
        // Due to the settle lock, the emitted frames are in this order:
        // InitState, CustomEvent, CustomEvent, Checkpoint

        tracing_subscriber::fmt::init();

        let mut alice = TestClient::player("alice");
        let mut bob = TestClient::player("bob");
        let game_account = TestGameAccountBuilder::default()
            .add_player(&mut alice, 100)
            .add_player(&mut bob, 100)
            .build();

        let (broadcaster, ctx) = Broadcaster::init(game_account.addr.clone(), 0);
        let handle = broadcaster.start("", ctx);

        {
            let event_frame = EventFrame::InitState {
                access_version: 0,
                settle_version: 1,
                checkpoint: Checkpoint::default(),
            };
            handle.send_unchecked(event_frame).await;

            let event_frame = EventFrame::Broadcast {
                event: Event::Custom {
                    sender: 1,
                    raw: vec![],
                },
                state_sha: "1".into(),
                timestamp: 1,
            };
            handle.send_unchecked(event_frame).await;

            let event_frame = EventFrame::Checkpoint {
                checkpoint: Checkpoint::default(),
                access_version: 1,
                settle_version: 2,
                state_sha: "2".into(),
                nodes: vec![],
            };
            handle.send_unchecked(event_frame).await;

            let event_frame = EventFrame::Broadcast {
                event: Event::Custom {
                    sender: 2,
                    raw: vec![],
                },
                state_sha: "3".into(),
                timestamp: 1,
            };
            handle.send_unchecked(event_frame).await;
        }

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let broadcast_frame = broadcaster.get_backlogs(0).await;
        assert_eq!(
            broadcast_frame,
            BroadcastFrame::Backlogs {
                checkpoint_off_chain: Some(Checkpoint::default().derive_offchain_part()),
                backlogs: Box::new(vec![BroadcastFrame::Sync {
                    sync: BroadcastSync {
                        new_players: vec![],
                        new_servers: vec![],
                        new_deposits: vec![],
                        transactor_addr: "".into(),
                        access_version: 1
                    }
                }]),
                state_sha: "2".into(),
            }
        );
    }

    #[tokio::test]
    async fn test_broadcast_event() {
        let mut alice = TestClient::player("alice");
        let mut bob = TestClient::player("bob");
        let game_account = TestGameAccountBuilder::default()
            .add_player(&mut alice, 100)
            .add_player(&mut bob, 100)
            .build();

        let (broadcaster, ctx) = Broadcaster::init(game_account.addr.clone(), 0);
        let handle = broadcaster.start("", ctx);
        let mut rx = broadcaster.get_broadcast_rx();

        // BroadcastFrame::Event
        {
            let event_frame = EventFrame::Broadcast {
                timestamp: 0,
                event: Event::Custom {
                    sender: alice.id(),
                    raw: "CUSTOM EVENT".into(),
                },
                state_sha: "".into(),
            };

            let broadcast_frame = BroadcastFrame::Event {
                timestamp: 0,
                event: Event::Custom {
                    sender: alice.id(),
                    raw: "CUSTOM EVENT".into(),
                },
                state_sha: "".into(),
            };

            handle.send_unchecked(event_frame).await;
            let received = rx.recv().await.unwrap();
            assert_eq!(received, broadcast_frame);
        }

        // BroadcastFrame::Confirming
        {
            let tx_state = TxState::PlayerConfirming {
                confirm_players: vec![PlayerJoin {
                    addr: "Alice".into(),
                    position: 0,
                    access_version: 10,
                    verify_key: "alice".into(),
                }
                .into()],
                access_version: 10,
            };
            let event_frame = EventFrame::TxState {
                tx_state: tx_state.clone(),
            };

            let broadcast_frame = BroadcastFrame::TxState { tx_state };
            handle.send_unchecked(event_frame).await;
            let received = rx.recv().await.unwrap();
            assert_eq!(received, broadcast_frame);
        }
    }
}
