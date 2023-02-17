//! The broadcaster will broadcast event to all connected participants
//! The broadcast should also save
//

use std::collections::LinkedList;
use std::sync::Arc;

use async_trait::async_trait;
use race_core::event::Event;
use race_core::types::{BroadcastFrame, GameAccount};
use tokio::sync::{broadcast, Mutex};
use tracing::warn;

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
    snapshot: Arc<Mutex<String>>,
    event_backups: Arc<Mutex<LinkedList<EventBackup>>>,
    latest_access_version: u64,
    latest_settle_version: u64,
    broadcast_tx: broadcast::Sender<BroadcastFrame>,
}

/// A component that pushs event to clients.
pub struct Broadcaster {
    game_addr: String,
    snapshot: Arc<Mutex<String>>,
    event_backups: Arc<Mutex<LinkedList<EventBackup>>>,
    broadcast_tx: broadcast::Sender<BroadcastFrame>,
}

impl Broadcaster {
    pub fn init(game_account: &GameAccount, init_snapshot: String) -> (Self, BroadcasterContext) {
        let snapshot = Arc::new(Mutex::new(init_snapshot));
        let event_backups = Arc::new(Mutex::new(LinkedList::new()));
        let (broadcast_tx, broadcast_rx) = broadcast::channel(10);
        drop(broadcast_rx);
        (
            Self {
                game_addr: game_account.addr.clone(),
                snapshot: snapshot.clone(),
                event_backups: event_backups.clone(),
                broadcast_tx: broadcast_tx.clone(),
            },
            BroadcasterContext {
                game_addr: game_account.addr.clone(),
                snapshot,
                event_backups,
                latest_access_version: game_account.access_version,
                latest_settle_version: game_account.settle_version,
                broadcast_tx,
            },
        )
    }

    pub async fn get_snapshot(&self) -> String {
        self.snapshot.lock().await.to_owned()
    }

    pub fn get_broadcast_rx(&self) -> broadcast::Receiver<BroadcastFrame> {
        self.broadcast_tx.subscribe()
    }

    /// Retrieve events those is handled with a specified `settle_version`.
    pub async fn retrieve_histories(&self, settle_version: u64) -> Vec<BroadcastFrame> {
        self.event_backups
            .lock()
            .await
            .iter()
            .filter_map(|event_backup| {
                if event_backup.settle_version >= settle_version {
                    Some(BroadcastFrame {
                        game_addr: self.game_addr.clone(),
                        event: event_backup.event.clone(),
                        timestamp: event_backup.timestamp,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

#[async_trait]
impl Component<ConsumerPorts, BroadcasterContext> for Broadcaster {
    fn name(&self) -> &str {
        "Broadcaster"
    }

    async fn run(mut ports: ConsumerPorts, mut ctx: BroadcasterContext) {
        while let Some(event) = ports.recv().await {
            match event {
                EventFrame::Broadcast {
                    event,
                    state_json,
                    access_version,
                    settle_version,
                    timestamp,
                } => {
                    // info!("Broadcaster broadcast event: {:?}", event);
                    let mut snapshot = ctx.snapshot.lock().await;
                    let mut event_backups = ctx.event_backups.lock().await;
                    ctx.latest_access_version = access_version;
                    ctx.latest_settle_version = settle_version;
                    *snapshot = state_json.clone();
                    event_backups.push_back(EventBackup {
                        event: event.clone(),
                        settle_version,
                        access_version,
                        timestamp,
                    });
                    // We keep at most 100 backups
                    if event_backups.len() > 100 {
                        event_backups.pop_front();
                    }

                    let r = ctx.broadcast_tx.send(BroadcastFrame {
                        game_addr: ctx.game_addr.clone(),
                        event,
                        timestamp,
                    });
                    if let Err(e) = r {
                        warn!("Failed to broadcast event: {:?}", e);
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
        let game_account = TestGameAccountBuilder::default().add_players(2).build();
        let (broadcaster, ctx) = Broadcaster::init(&game_account, "{}".into());
        let handle = broadcaster.start(ctx);
        let mut rx = broadcaster.get_broadcast_rx();

        let event_frame = EventFrame::Broadcast {
            access_version: 10,
            settle_version: 10,
            timestamp: 0,
            state_json: "STATE JSON".into(),
            event: Event::Custom {
                sender: "Alice".into(),
                raw: "CUSTOM EVENT".into(),
            },
        };

        let broadcast_frame = BroadcastFrame {
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
