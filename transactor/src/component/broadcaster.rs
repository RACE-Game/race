//! The broadcaster will broadcast events to all connected participants
//! The broadcast should also save

use std::collections::LinkedList;
use std::sync::Arc;

use async_trait::async_trait;
use race_api::event::Event;
use race_core::types::{BroadcastFrame, BroadcastSync, EventHistory, TxState};
use tokio::sync::{broadcast, Mutex};
use tracing::{debug, error, info};

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
    pub access_version: u64,
    pub settle_version: u64,
    pub state_sha: String,
}

#[derive(Debug)]
pub struct Checkpoint {
    pub access_version: u64,
    pub settle_version: u64,
}

#[derive(Debug)]
pub struct EventBackupGroup {
    pub sync: BroadcastSync,
    pub events: LinkedList<EventBackup>,
    pub checkpoint: Checkpoint,
    pub settle_version: u64,
    pub access_version: u64,
    pub state: Option<Vec<u8>>,
}

pub struct BroadcasterContext {
    id: String,
    event_backup_groups: Arc<Mutex<LinkedList<EventBackupGroup>>>,
    broadcast_tx: broadcast::Sender<BroadcastFrame>,
    #[allow(unused)]
    debug_mode: bool,
}

/// A component that pushes event to clients.
pub struct Broadcaster {
    id: String,
    event_backup_groups: Arc<Mutex<LinkedList<EventBackupGroup>>>,
    broadcast_tx: broadcast::Sender<BroadcastFrame>,
}

impl Broadcaster {
    pub fn init(id: String, debug_mode: bool) -> (Self, BroadcasterContext) {
        let event_backup_groups = Arc::new(Mutex::new(LinkedList::new()));
        let (broadcast_tx, broadcast_rx) = broadcast::channel(10);
        drop(broadcast_rx);
        (
            Self {
                id: id.clone(),
                event_backup_groups: event_backup_groups.clone(),
                broadcast_tx: broadcast_tx.clone(),
            },
            BroadcasterContext {
                id,
                debug_mode,
                event_backup_groups,
                broadcast_tx,
            },
        )
    }

    pub fn get_broadcast_rx(&self) -> broadcast::Receiver<BroadcastFrame> {
        self.broadcast_tx.subscribe()
    }

    pub async fn get_state(&self) -> Option<Vec<u8>> {
        let groups = self.event_backup_groups.lock().await;
        groups.back().and_then(|g| g.state.clone())
    }

    pub async fn retrieve_histories(&self, settle_version: u64) -> Vec<BroadcastFrame> {
        // info!(
        //     "{} Retrieve the histories, settle_version: {}",
        //     Self::name(),
        //     settle_version
        // );

        let mut frames: Vec<BroadcastFrame> = Vec::new();
        let event_backup_groups = self.event_backup_groups.lock().await;

        for group in event_backup_groups.iter() {
            let mut histories: Vec<EventHistory> = Vec::new();
            if group.settle_version >= settle_version {
                info!("Broadcast sync {:?}", group.sync);
                frames.push(BroadcastFrame::Sync {
                    sync: group.sync.clone(),
                });
                for event in group.events.iter() {
                    histories.push(EventHistory {
                        event: event.event.clone(),
                        timestamp: event.timestamp,
                        state_sha: event.state_sha.clone(),
                    });
                }
                frames.push(BroadcastFrame::EventHistories {
                    game_addr: self.id.clone(),
                    histories,
                })
            }
        }

        frames
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
                    let r = ctx.broadcast_tx.send(BroadcastFrame::Message {
                        game_addr: ctx.id.clone(),
                        message,
                    });

                    if let Err(e) = r {
                        // Usually it means no receivers
                        debug!("{} Failed to broadcast event: {:?}", env.log_prefix, e);
                    }
                }

                EventFrame::Checkpoint {
                    access_version,
                    settle_version,
                    ..
                }
                | EventFrame::InitState {
                    access_version,
                    settle_version,
                    ..
                } => {
                    info!(
                        "{} Create new history group with access_version = {}, settle_version = {}",
                        env.log_prefix, access_version, settle_version
                    );
                    let mut event_backup_groups = ctx.event_backup_groups.lock().await;

                    let checkpoint = Checkpoint {
                        access_version,
                        settle_version,
                    };

                    event_backup_groups.push_back(EventBackupGroup {
                        sync: BroadcastSync::new(access_version),
                        events: LinkedList::new(),
                        checkpoint,
                        access_version,
                        settle_version,
                        state: None,
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
                    access_version,
                    settle_version,
                    timestamp,
                    state_sha,
                    ..
                } => {
                    info!("{} Broadcaster receive event: {}", env.log_prefix, event);
                    let mut event_backup_groups = ctx.event_backup_groups.lock().await;

                    if let Some(current) = event_backup_groups.back_mut() {
                        current.events.push_back(EventBackup {
                            event: event.clone(),
                            settle_version,
                            access_version,
                            timestamp,
                            state_sha: state_sha.clone(),
                        });
                    } else {
                        error!("{} Received event without checkpoint", env.log_prefix);
                    }
                    // Keep 10 groups at most
                    if event_backup_groups.len() > 10 {
                        event_backup_groups.pop_front();
                    }
                    drop(event_backup_groups);

                    let r = ctx.broadcast_tx.send(BroadcastFrame::Event {
                        game_addr: ctx.id.clone(),
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
                    access_version,
                    transactor_addr,
                } => {
                    let sync = BroadcastSync {
                        new_players,
                        new_servers,
                        access_version,
                        transactor_addr,
                    };

                    let mut event_backup_groups = ctx.event_backup_groups.lock().await;
                    if let Some(current) = event_backup_groups.back_mut() {
                        info!("{} Merge sync: {:?}", env.log_prefix, sync);
                        current.sync.merge(&sync);
                    } else {
                        error!("{} Sync dropped", env.log_prefix);
                    }
                    drop(event_backup_groups);

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
    use race_core::types::PlayerJoin;
    use race_test::prelude::*;

    #[tokio::test]
    async fn test_broadcast_event() {
        let mut alice = TestClient::player("alice");
        let mut bob = TestClient::player("bob");
        let game_account = TestGameAccountBuilder::default()
            .add_player(&mut alice, 100)
            .add_player(&mut bob, 100)
            .build();

        let (broadcaster, ctx) = Broadcaster::init(game_account.addr.clone(), true);
        let handle = broadcaster.start("", ctx);
        let mut rx = broadcaster.get_broadcast_rx();

        // BroadcastFrame::Event
        {
            let event_frame = EventFrame::Broadcast {
                access_version: 10,
                settle_version: 10,
                timestamp: 0,
                event: Event::Custom {
                    sender: alice.id(),
                    raw: "CUSTOM EVENT".into(),
                },
                state_sha: "".into(),
                state: vec![],
            };

            let broadcast_frame = BroadcastFrame::Event {
                game_addr: game_account.addr,
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
                    balance: 100,
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
