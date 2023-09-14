//! The broadcaster will broadcast events to all connected participants
//! The broadcast should also save

use std::collections::LinkedList;
use std::sync::Arc;

use async_trait::async_trait;
use race_core::event::Event;
use race_core::types::{BroadcastFrame, GameAccount, TxState};
use tokio::sync::{broadcast, Mutex};
use tracing::debug;

use crate::component::common::{Component, ConsumerPorts};
use crate::frame::EventFrame;

use super::CloseReason;

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
}

#[derive(Debug)]
pub struct Checkpoint {
    pub state: Vec<u8>,
    pub access_version: u64,
    pub settle_version: u64,
}

#[derive(Debug)]
pub struct EventBackupGroup {
    pub events: LinkedList<EventBackup>,
    pub checkpoint: Checkpoint,
    pub settle_version: u64,
    pub access_version: u64,
}

pub struct BroadcasterContext {
    game_addr: String,
    event_backup_groups: Arc<Mutex<LinkedList<EventBackupGroup>>>,
    broadcast_tx: broadcast::Sender<BroadcastFrame>,
}

/// A component that pushes event to clients.
pub struct Broadcaster {
    game_addr: String,
    event_backup_groups: Arc<Mutex<LinkedList<EventBackupGroup>>>,
    broadcast_tx: broadcast::Sender<BroadcastFrame>,
}

impl Broadcaster {
    pub fn init(game_account: &GameAccount) -> (Self, BroadcasterContext) {
        let event_backup_groups = Arc::new(Mutex::new(LinkedList::new()));
        let (broadcast_tx, broadcast_rx) = broadcast::channel(10);
        drop(broadcast_rx);
        (
            Self {
                game_addr: game_account.addr.clone(),
                event_backup_groups: event_backup_groups.clone(),
                broadcast_tx: broadcast_tx.clone(),
            },
            BroadcasterContext {
                game_addr: game_account.addr.clone(),
                event_backup_groups,
                broadcast_tx,
            },
        )
    }

    pub fn get_broadcast_rx(&self) -> broadcast::Receiver<BroadcastFrame> {
        self.broadcast_tx.subscribe()
    }

    pub async fn retrieve_histories(&self, settle_version: u64) -> Vec<BroadcastFrame> {
        let event_backup_groups = self.event_backup_groups.lock().await;

        let mut histories: Vec<BroadcastFrame> = Vec::new();

        for group in event_backup_groups.iter() {
            if group.settle_version >= settle_version {
                // The first frame must be Init
                if histories.is_empty() {
                    histories.push(BroadcastFrame::Init {
                        game_addr: self.game_addr.clone(),
                        access_version: group.access_version,
                        settle_version: group.settle_version,
                        checkpoint_state: group.checkpoint.state.clone(),
                    });
                }
                // The rest frames must be Event
                for event in group.events.iter() {
                    histories.push(BroadcastFrame::Event {
                        game_addr: self.game_addr.clone(),
                        event: event.event.clone(),
                        timestamp: event.timestamp,
                    })
                }
            }
        }

        histories
    }
}

#[async_trait]
impl Component<ConsumerPorts, BroadcasterContext> for Broadcaster {
    fn name(&self) -> &str {
        "Broadcaster"
    }

    async fn run(mut ports: ConsumerPorts, ctx: BroadcasterContext) -> CloseReason {
        while let Some(event) = ports.recv().await {
            match event {
                EventFrame::SendMessage { message } => {
                    let r = ctx.broadcast_tx.send(BroadcastFrame::Message {
                        game_addr: ctx.game_addr.clone(),
                        message,
                    });

                    if let Err(e) = r {
                        // Usually it means no receivers
                        debug!("Failed to broadcast event: {:?}", e);
                    }
                }
                EventFrame::Checkpoint {
                    state,
                    access_version,
                    settle_version,
                } => {
                    let mut event_backup_groups = ctx.event_backup_groups.lock().await;

                    let checkpoint = Checkpoint {
                        state,
                        access_version,
                        settle_version,
                    };

                    event_backup_groups.push_back(EventBackupGroup {
                        events: LinkedList::new(),
                        checkpoint,
                        access_version,
                        settle_version,
                    });
                }
                EventFrame::TxState { tx_state } => match tx_state {
                    TxState::PlayerConfirming {
                        confirm_players,
                        access_version,
                    } => {
                        let tx_state = TxState::PlayerConfirming {
                            confirm_players,
                            access_version,
                        };

                        let r = ctx.broadcast_tx.send(BroadcastFrame::TxState { tx_state });

                        if let Err(e) = r {
                            debug!("Failed to broadcast event: {:?}", e);
                        }
                    }
                    TxState::PlayerConfirmingFailed(access_version) => {
                        let r = ctx.broadcast_tx.send(BroadcastFrame::TxState {
                            tx_state: TxState::PlayerConfirmingFailed(access_version),
                        });

                        if let Err(e) = r {
                            debug!("Failed to broadcast event: {:?}", e);
                        }
                    }
                },
                EventFrame::Broadcast {
                    event,
                    access_version,
                    settle_version,
                    timestamp,
                } => {
                    debug!("Broadcaster receive event: {}", event);
                    let mut event_backup_groups = ctx.event_backup_groups.lock().await;

                    if let Some(current) = event_backup_groups.back_mut() {
                        current.events.push_back(EventBackup {
                            event: event.clone(),
                            settle_version,
                            access_version,
                            timestamp,
                        });
                    }
                    // Keep 10 groups at most
                    if event_backup_groups.len() > 10 {
                        event_backup_groups.pop_front();
                    }

                    let r = ctx.broadcast_tx.send(BroadcastFrame::Event {
                        game_addr: ctx.game_addr.clone(),
                        event,
                        timestamp,
                    });

                    if let Err(e) = r {
                        // Usually it means no receivers
                        debug!("Failed to broadcast event: {:?}", e);
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
    use race_test::*;

    #[tokio::test]
    async fn test_broadcast_event() {
        let alice = TestClient::player("alice");
        let bob = TestClient::player("bob");
        let game_account = TestGameAccountBuilder::default()
            .add_player(&alice, 100)
            .add_player(&bob, 100)
            .build();
        let (broadcaster, ctx) = Broadcaster::init(&game_account);
        let handle = broadcaster.start(ctx);
        let mut rx = broadcaster.get_broadcast_rx();

        // BroadcastFrame::Event
        {
            let event_frame = EventFrame::Broadcast {
                access_version: 10,
                settle_version: 10,
                timestamp: 0,
                event: Event::Custom {
                    sender: "Alice".into(),
                    raw: "CUSTOM EVENT".into(),
                },
            };

            let broadcast_frame = BroadcastFrame::Event {
                game_addr: game_account.addr,
                timestamp: 0,
                event: Event::Custom {
                    sender: "Alice".into(),
                    raw: "CUSTOM EVENT".into(),
                },
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
                }],
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
