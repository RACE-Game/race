///! The component to bridge two event buses, typically to be used
///! between the parent game and the sub games.
use async_trait::async_trait;
use tokio::sync::{broadcast, mpsc};

use crate::frame::{EventFrame, SignalFrame};

use super::{common::PipelinePorts, CloseReason, Component};

#[allow(dead_code)]
pub struct EventBridgeParentContext {
    tx: broadcast::Sender<EventFrame>,
    rx: mpsc::Receiver<EventFrame>,
    signal_tx: mpsc::Sender<SignalFrame>,
}

#[derive(Clone, Debug)]
pub struct EventBridgeParent {
    tx: mpsc::Sender<EventFrame>,
    bc: broadcast::Sender<EventFrame>,
}

pub struct EventBridgeChildContext {
    pub sub_id: usize,
    tx: mpsc::Sender<EventFrame>,
    rx: broadcast::Receiver<EventFrame>,
}

pub struct EventBridgeChild {
    pub sub_id: usize,
}

impl EventBridgeParent {
    pub fn init(signal_tx: mpsc::Sender<SignalFrame>) -> (Self, EventBridgeParentContext) {
        let (mpsc_tx, mpsc_rx) = mpsc::channel(10);
        let (bc_tx, _bc_rx) = broadcast::channel(10);
        (
            Self {
                tx: mpsc_tx,
                bc: bc_tx.clone(),
            },
            EventBridgeParentContext {
                tx: bc_tx,
                rx: mpsc_rx,
                signal_tx,
            },
        )
    }

    pub fn derive_child(&self, sub_id: usize) -> (EventBridgeChild, EventBridgeChildContext) {
        (
            EventBridgeChild {
                sub_id: sub_id.clone(),
            },
            EventBridgeChildContext {
                sub_id,
                tx: self.tx.clone(),
                rx: self.bc.subscribe(),
            },
        )
    }
}

impl EventBridgeParent {
    /// Read event from both the local event bus and the bridge.
    /// Return (true, event) when the event is from the bridge.
    /// Return None when bridge is closed.
    async fn read_event(
        ports: &mut PipelinePorts,
        rx: &mut mpsc::Receiver<EventFrame>,
    ) -> Option<(bool, EventFrame)> {
        tokio::select! {
            e = rx.recv() => {
                if let Some(e) = e {
                    Some((true, e))
                } else {
                    None
                }
            },
            e = ports.recv() => {
                if let Some(e) = e {
                    Some((false, e))
                } else {
                    None
                }
            },
        }
    }
}

#[async_trait]
impl Component<PipelinePorts, EventBridgeParentContext> for EventBridgeParent {
    fn name(&self) -> &str {
        "Event Bridge (Parent)"
    }

    async fn run(mut ports: PipelinePorts, mut ctx: EventBridgeParentContext) -> CloseReason {
        while let Some((from_bridge, event_frame)) = Self::read_event(&mut ports, &mut ctx.rx).await
        {
            if from_bridge {
                ports.send(event_frame).await;
            } else {
                match event_frame {
                    EventFrame::LaunchSubGame { spec } => {
                        let f = SignalFrame::LaunchSubGame { spec };
                        if let Err(e) = ctx.signal_tx.send(f).await {
                            println!("Failed to send: {}", e);
                        }
                    }
                    EventFrame::Shutdown => {
                        if let Err(e) = ctx.tx.send(event_frame) {
                            println!("Failed to send: {}", e);
                        }
                        break;
                    }
                    EventFrame::BridgeEvent { .. } | EventFrame::UpdateNodes { .. } => {
                        if let Err(e) = ctx.tx.send(event_frame) {
                            println!("Failed to send: {}", e);
                        }
                    }
                    _ => continue,
                }
            }
        }

        CloseReason::Complete
    }
}

impl EventBridgeChild {
    /// Read event from both the local event bus and the bridge.
    /// Return (true, event) when the event is from the bridge.
    /// Return None when bridge is closed.
    async fn read_event(
        ports: &mut PipelinePorts,
        rx: &mut broadcast::Receiver<EventFrame>,
    ) -> Option<(bool, EventFrame)> {
        tokio::select! {
            e = rx.recv() => {
                if let Ok(e) = e {
                    Some((true, e))
                } else {
                     None
                }
            },
            e = ports.recv() => {
                if let Some(e) = e {
                    Some((false, e))
                } else {
                    None
                }
            }
        }
    }
}

#[async_trait]
impl Component<PipelinePorts, EventBridgeChildContext> for EventBridgeChild {
    fn name(&self) -> &str {
        "Event Bridge (Child)"
    }

    async fn run(mut ports: PipelinePorts, mut ctx: EventBridgeChildContext) -> CloseReason {
        while let Some((from_bridge, event_frame)) = Self::read_event(&mut ports, &mut ctx.rx).await
        {
            if from_bridge {
                ports.send(event_frame).await;
            } else {
                match event_frame {
                    EventFrame::Shutdown => break,
                    EventFrame::BridgeEvent { .. } => {
                        if let Err(e) = ctx.tx.send(event_frame).await {
                            println!("Failed to send: {:?}", e);
                        }
                    }
                    _ => continue,
                }
            }
        }

        CloseReason::Complete
    }
}
