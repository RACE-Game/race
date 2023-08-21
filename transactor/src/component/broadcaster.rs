//! The broadcaster will broadcast event to all connected participants
//! The broadcast should also save

use std::collections::LinkedList;
use std::sync::Arc;

use async_trait::async_trait;
use race_core::event::Event;
use race_core::types::{BroadcastFrame, GameAccount};
use tokio::sync::{broadcast, Mutex};
use tracing::{debug, warn, info};

use crate::component::common::{Component, ConsumerPorts, Ports};
use crate::component::event_bus::CloseReason;
use crate::frame::EventFrame;

/// Backup events in memeory, for new connected client.  The
/// `settle_version` and `access_version` are the values at the time
/// we handle the events. The backups always start with a checkpoint
/// event which contains the initial handler state
pub struct EventBackup {
    pub event: Event,
    pub checkpoint_state: Option<Vec<u8>>,
    pub settle_version: u64,
    pub access_version: u64,
    pub timestamp: u64,
}

pub struct BroadcasterContext {
    game_addr: String,
    event_backups: Arc<Mutex<LinkedList<EventBackup>>>,
    broadcast_tx: broadcast::Sender<BroadcastFrame>,
}

/// A component that pushs event to clients.
pub struct Broadcaster {
    game_addr: String,
    event_backups: Arc<Mutex<LinkedList<EventBackup>>>,
    broadcast_tx: broadcast::Sender<BroadcastFrame>,
}

impl Broadcaster {
    pub fn init(game_account: &GameAccount) -> (Self, BroadcasterContext) {
        let event_backups = Arc::new(Mutex::new(LinkedList::new()));
        let (broadcast_tx, broadcast_rx) = broadcast::channel(10);
        drop(broadcast_rx);
        (
            Self {
                game_addr: game_account.addr.clone(),
                event_backups: event_backups.clone(),
                broadcast_tx: broadcast_tx.clone(),
            },
            BroadcasterContext {
                game_addr: game_account.addr.clone(),
                event_backups,
                broadcast_tx,
            },
        )
    }

    pub fn get_broadcast_rx(&self) -> broadcast::Receiver<BroadcastFrame> {
        self.broadcast_tx.subscribe()
    }

    pub async fn retrieve_histories(&self, _settle_version: u64) -> Vec<BroadcastFrame> {
        let event_backups = self.event_backups.lock().await;

        let mut histories: Vec<BroadcastFrame> = Vec::new();
        for event_backup in event_backups.iter() {
            if let Some(ref checkpoint_state) = event_backup.checkpoint_state {
                info!("checkpoint: {}", event_backup.event);
                histories.clear();
                histories.push(BroadcastFrame::Init {
                    game_addr: self.game_addr.clone(),
                    access_version: event_backup.access_version,
                    settle_version: event_backup.settle_version,
                    checkpoint_state: checkpoint_state.to_owned(),
                });
            } else {
                info!("event: {}", event_backup.event);
                histories.push(BroadcastFrame::Event {
                    game_addr: self.game_addr.clone(),
                    event: event_backup.event.clone(),
                    timestamp: event_backup.timestamp,
                })
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

    async fn run(mut ports: ConsumerPorts, ctx: BroadcasterContext) {
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
                EventFrame::Broadcast {
                    event,
                    checkpoint_state,
                    access_version,
                    settle_version,
                    timestamp,
                } => {
                    debug!("Broadcaster receive event: {}", event);
                    let mut event_backups = ctx.event_backups.lock().await;

                    // Remove old histories when we have new checkpoint.
                    // We keep at most two checkpoints at the same time.
                    if checkpoint_state.is_some()
                        && event_backups
                            .iter()
                            .filter(|e| e.checkpoint_state.is_some())
                            .count()
                            > 1
                    {
                        event_backups.pop_front();
                        while event_backups
                            .front()
                            .is_some_and(|e| e.checkpoint_state.is_none())
                        {
                            event_backups.pop_front();
                        }
                    }

                    event_backups.push_back(EventBackup {
                        event: event.clone(),
                        checkpoint_state: checkpoint_state.clone(),
                        settle_version,
                        access_version,
                        timestamp,
                    });

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
                    warn!("Shutdown broadcaster");
                    break;
                }
                _ => (),
            }
        }
        ports.close(CloseReason::Complete);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

        let event_frame = EventFrame::Broadcast {
            access_version: 10,
            settle_version: 10,
            timestamp: 0,
            event: Event::Custom {
                sender: "Alice".into(),
                raw: "CUSTOM EVENT".into(),
            },
            checkpoint_state: Some(vec![]),
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
}
