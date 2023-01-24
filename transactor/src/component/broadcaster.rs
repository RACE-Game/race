//! The broadcaster will broadcast event to all connected participants
//! The broadcast should also save
//

use std::sync::Arc;
use std::collections::LinkedList;

use race_core::event::Event;
use race_core::types::{BroadcastFrame, GameAccount};
use tokio::sync::{broadcast, mpsc, oneshot, Mutex};
use tracing::info;

use crate::component::event_bus::CloseReason;
use crate::component::traits::{Attachable, Component, Named};
use crate::frame::EventFrame;

/// Backup events in memeory, for new connected client.
/// The `settle_version` and `access_version` are the values at the time we handle the events.
/// Old backups can be forgot once the `settle_version` and `access_version` is updated.
pub struct EventBackup {
    pub event: Event,
    pub settle_version: u64,
    pub access_version: u64,
}

pub struct BroadcasterContext {
    game_addr: String,
    input_rx: mpsc::Receiver<EventFrame>,
    closed_tx: oneshot::Sender<CloseReason>,
    broadcast_tx: broadcast::Sender<BroadcastFrame>,
    latest_access_version: u64,
    latest_settle_version: u64,
}

/// A component that pushs event to clients.
pub struct Broadcaster {
    input_tx: mpsc::Sender<EventFrame>,
    closed_rx: oneshot::Receiver<CloseReason>,
    snapshot: Arc<Mutex<String>>,
    event_backups: Arc<Mutex<LinkedList<EventBackup>>>,
    ctx: Option<BroadcasterContext>,
    broadcast_tx: broadcast::Sender<BroadcastFrame>,
}

impl Named for Broadcaster {
    fn name<'a>(&self) -> &'a str {
        "Broadcaster"
    }
}

impl Attachable for Broadcaster {
    fn input(&self) -> Option<mpsc::Sender<EventFrame>> {
        Some(self.input_tx.clone())
    }

    fn output(&mut self) -> Option<mpsc::Receiver<EventFrame>> {
        None
    }
}

impl Component<BroadcasterContext> for Broadcaster {
    fn run(&mut self, mut ctx: BroadcasterContext) {
        let snapshot = self.snapshot.clone();
        let event_backups = self.event_backups.clone();
        tokio::spawn(async move {
            loop {
                if let Some(event) = ctx.input_rx.recv().await {
                    match event {
                        EventFrame::Broadcast {
                            event,
                            state_json,
                            access_version,
                            settle_version,
                        } => {
                            info!("Broadcaster broadcast event: {:?}", event);
                            let mut snapshot = snapshot.lock().await;
                            let mut event_backups = event_backups.lock().await;
                            ctx.latest_access_version = access_version;
                            ctx.latest_settle_version = settle_version;
                            *snapshot = state_json.clone();
                            event_backups.push_back(EventBackup {
                                event: event.clone(),
                                settle_version,
                                access_version,
                            });
                            // We keep at most 100 backups
                            if event_backups.len() > 100 {
                                event_backups.pop_front();
                            }
                            ctx.broadcast_tx
                                .send(BroadcastFrame {
                                    game_addr: ctx.game_addr.clone(),
                                    event,
                                })
                                .unwrap();
                        }
                        _ => (),
                    }
                } else {
                    ctx.closed_tx.send(CloseReason::Complete).unwrap();
                    break;
                }
            }
        });
    }

    fn borrow_mut_ctx(&mut self) -> &mut Option<BroadcasterContext> {
        &mut self.ctx
    }

    fn closed(self) -> oneshot::Receiver<CloseReason> {
        self.closed_rx
    }
}

impl Broadcaster {
    pub fn new(init_state: &GameAccount, init_snapshot: String) -> Self {
        let snapshot = Arc::new(Mutex::new(init_snapshot));
        let event_backups = Arc::new(Mutex::new(LinkedList::new()));
        let (input_tx, input_rx) = mpsc::channel(3);
        let (closed_tx, closed_rx) = oneshot::channel();
        let (broadcast_tx, broadcast_rx) = broadcast::channel(3);
        drop(broadcast_rx);
        let ctx = Some(BroadcasterContext {
            game_addr: init_state.addr.clone(),
            closed_tx,
            input_rx,
            broadcast_tx: broadcast_tx.clone(),
            latest_access_version: init_state.access_version,
            latest_settle_version: init_state.settle_version,
        });
        Self {
            input_tx,
            closed_rx,
            snapshot,
            ctx,
            broadcast_tx,
            event_backups,
        }
    }

    pub async fn get_snapshot(&self) -> String {
        self.snapshot.lock().await.to_owned()
    }

    pub fn get_broadcast_rx(&self) -> broadcast::Receiver<BroadcastFrame> {
        self.broadcast_tx.subscribe()
    }

    /// Retrieve events those is handled with a specified `settle_version`.
    pub async fn retrieve_events(&self, settle_version: u64) -> Vec<Event> {
        self.event_backups
            .lock()
            .await
            .iter()
            .filter_map(|event_backup| {
                if event_backup.settle_version <= settle_version {
                    Some(event_backup.event.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use race_core::event::Event;
    use race_test::*;

    use super::*;

    #[tokio::test]
    async fn test_broadcast_event() {
        let game_account = TestGameAccountBuilder::default().add_players(2).build();
        let mut broadcaster = Broadcaster::new(&game_account, "{}".into());
        let mut rx = broadcaster.get_broadcast_rx();
        let event_frame = EventFrame::Broadcast {
            access_version: 10,
            settle_version: 10,
            state_json: "STATE JSON".into(),
            event: Event::Custom {
                sender: "Alice".into(),
                raw: "CUSTOM EVENT".into(),
            },
        };
        let broadcast_frame = BroadcastFrame {
            game_addr: game_account.addr,
            event: Event::Custom {
                sender: "Alice".into(),
                raw: "CUSTOM EVENT".into(),
            },
        };
        broadcaster.start();
        broadcaster.input_tx.send(event_frame).await.unwrap();
        let frame = rx.recv().await.expect("Failed to receive event");
        assert_eq!(frame, broadcast_frame);
    }
}
