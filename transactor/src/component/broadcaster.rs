//! The broadcaster will broadcast event to all connected participants
//! The broadcast should also save
//

use std::collections::LinkedList;
use std::sync::Arc;

use async_trait::async_trait;
use race_core::event::Event;
use race_core::types::{BroadcastFrame, GameAccount};
use tokio::sync::{broadcast, Mutex};
use tracing::{debug, warn};

use crate::component::common::{Component, ConsumerPorts, Ports};
use crate::component::event_bus::CloseReason;
use crate::frame::EventFrame;

/// Backup events in memeory, for new connected client.
/// The `settle_version` and `access_version` are the values at the time we handle the events.
/// Old backups can be forgot once the `settle_version` and `access_version` is updated.
pub struct EventBackup {
    pub event: Event,
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

    /// Retrieve events those is handled with a specified `settle_version`.
    pub async fn retrieve_histories(&self, settle_version: u64) -> Vec<BroadcastFrame> {
        let event_backups = self.event_backups.lock().await;
        let mut versions: Option<(u64, u64)> = None;

        let mut histories: Vec<BroadcastFrame> = event_backups
            .iter()
            .filter_map(|event_backup| {
                if event_backup.settle_version >= settle_version {
                    if versions.is_none() {
                        versions = Some((event_backup.access_version, event_backup.settle_version));
                    }
                    Some(BroadcastFrame::Event {
                        game_addr: self.game_addr.clone(),
                        event: event_backup.event.clone(),
                        timestamp: event_backup.timestamp,
                    })
                } else {
                    None
                }
            })
            .collect();

        if let Some((access_version, settle_version)) = versions {
            histories.insert(
                0,
                BroadcastFrame::Init {
                    game_addr: self.game_addr.clone(),
                    access_version,
                    settle_version,
                },
            )
        } else if let Some(last) = event_backups.back() {
            histories.insert(
                0,
                BroadcastFrame::Init {
                    game_addr: self.game_addr.clone(),
                    access_version: last.access_version,
                    settle_version: last.settle_version + 1
                }
            )
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
                EventFrame::Broadcast {
                    event,
                    access_version,
                    settle_version,
                    timestamp,
                } => {
                    let mut event_backups = ctx.event_backups.lock().await;

                    event_backups.push_back(EventBackup {
                        event: event.clone(),
                        settle_version,
                        access_version,
                        timestamp,
                    });
                    // We keep at most 1000 backups
                    if event_backups.len() > 1000 {
                        event_backups.pop_front();
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
