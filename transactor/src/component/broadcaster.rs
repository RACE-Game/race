use std::sync::Arc;

use race_core::types::{EventFrame, GameAccount};
use tokio::sync::{broadcast, mpsc, oneshot, watch, Mutex};

use crate::component::event_bus::CloseReason;
use crate::component::traits::{Attachable, Component, Named};

pub struct BroadcasterContext {
    input_rx: mpsc::Receiver<EventFrame>,
    closed_tx: oneshot::Sender<CloseReason>,
    broadcast_tx: broadcast::Sender<EventFrame>,
}

/// A component that pushs event to clients.
pub struct Broadcaster {
    input_tx: mpsc::Sender<EventFrame>,
    closed_rx: oneshot::Receiver<CloseReason>,
    snapshot: Arc<Mutex<String>>,
    ctx: Option<BroadcasterContext>,
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

    fn output(&self) -> Option<watch::Receiver<EventFrame>> {
        None
    }
}

impl Component<BroadcasterContext> for Broadcaster {
    fn run(&mut self, mut ctx: BroadcasterContext) {
        let snapshot = self.snapshot.clone();
        tokio::spawn(async move {
            loop {
                if let Some(event) = ctx.input_rx.recv().await {
                    match event {
                        EventFrame::Broadcast {
                            addr,
                            event,
                            state_json,
                        } => {
                            let mut snapshot = snapshot.lock().await;
                            *snapshot = state_json;
                            // TODO, broad cast event
                            ctx.broadcast_tx.send(EventFrame::Broadcast { addr: addr.to_owned(), state_json: snapshot.clone(), event }).unwrap();
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
    pub fn new(init_state: GameAccount, broadcast_tx: broadcast::Sender<EventFrame>) -> Self {
        let snapshot = Arc::new(Mutex::new("".into()));
        let (input_tx, input_rx) = mpsc::channel(3);
        let (closed_tx, closed_rx) = oneshot::channel();
        let ctx = Some(BroadcasterContext {
            closed_tx,
            input_rx,
            broadcast_tx,
        });
        Self {
            input_tx,
            closed_rx,
            snapshot,
            ctx,
        }
    }
}
