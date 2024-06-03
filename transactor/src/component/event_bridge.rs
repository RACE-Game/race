use crate::frame::{EventFrame, SignalFrame};
use async_trait::async_trait;
use tokio::sync::{broadcast, mpsc};
use tracing::{info, log::error};

use super::{common::PipelinePorts, CloseReason, Component, ComponentEnv};

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
    pub game_id: usize,
    tx: mpsc::Sender<EventFrame>,
    rx: broadcast::Receiver<EventFrame>,
}

pub struct EventBridgeChild {
    pub game_id: usize,
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

    pub fn derive_child(&self, game_id: usize) -> (EventBridgeChild, EventBridgeChildContext) {
        (
            EventBridgeChild {
                game_id: game_id.clone(),
            },
            EventBridgeChildContext {
                game_id,
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
    fn name() -> &'static str {
        "Event Bridge (Parent)"
    }

    async fn run(
        mut ports: PipelinePorts,
        mut ctx: EventBridgeParentContext,
        env: ComponentEnv,
    ) -> CloseReason {
        while let Some((from_bridge, event_frame)) = Self::read_event(&mut ports, &mut ctx.rx).await
        {
            if from_bridge {
                match event_frame {
                    EventFrame::SendBridgeEvent {
                        from,
                        dest,
                        event,
                        access_version,
                        settle_version,
                        checkpoint,
                    } => {
                        info!("{} Receives event: {}", env.log_prefix, event);
                        ports
                            .send(EventFrame::RecvBridgeEvent {
                                from,
                                dest,
                                event,
                                access_version,
                                settle_version,
                                checkpoint,
                            })
                            .await;
                    }
                    _ => (),
                }
            } else {
                match event_frame {
                    EventFrame::LaunchSubGame { spec } => {
                        let f = SignalFrame::LaunchSubGame { spec: *spec };
                        if let Err(e) = ctx.signal_tx.send(f).await {
                            error!("{} Failed to send: {}", env.log_prefix, e);
                        }
                    }
                    EventFrame::Shutdown => {
                        if !ctx.tx.is_empty() {
                            info!("{} Sends Shutdown", env.log_prefix);
                            if let Err(e) = ctx.tx.send(event_frame) {
                                error!("{} Failed to send: {}", env.log_prefix, e);
                            }
                        }
                        break;
                    }
                    EventFrame::SendBridgeEvent { dest, .. } if dest != 0 => {
                        info!("{} Sends event: {}", env.log_prefix, event_frame);
                        if let Err(e) = ctx.tx.send(event_frame) {
                            error!("{} Failed to send: {}", env.log_prefix, e);
                        }
                    }
                    EventFrame::Sync { .. } => {
                        if !ctx.tx.is_empty() {
                            info!("{} Sends event: {}", env.log_prefix, event_frame);
                            if let Err(e) = ctx.tx.send(event_frame) {
                                error!("{} Failed to send: {}", env.log_prefix, e);
                            }
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
    fn name() -> &'static str {
        "Event Bridge (Child)"
    }

    async fn run(
        mut ports: PipelinePorts,
        mut ctx: EventBridgeChildContext,
        env: ComponentEnv,
    ) -> CloseReason {
        while let Some((from_bridge, event_frame)) = Self::read_event(&mut ports, &mut ctx.rx).await
        {
            if from_bridge {
                match event_frame {
                    EventFrame::Shutdown => {
                        info!("{} Receives Shutdown, quit", env.log_prefix);
                        ports.send(event_frame).await;
                        break;
                    }
                    EventFrame::Sync { .. } => {
                        info!("{} Receives event: {}", env.log_prefix, event_frame);
                        ports.send(event_frame).await;
                    }
                    EventFrame::SendBridgeEvent {
                        from,
                        dest,
                        event,
                        access_version,
                        settle_version,
                        checkpoint,
                    } if dest == ctx.game_id => {
                        info!("{} Receives event: {}", env.log_prefix, event);
                        ports
                            .send(EventFrame::RecvBridgeEvent {
                                from,
                                dest,
                                event,
                                access_version,
                                settle_version,
                                checkpoint,
                            })
                            .await;
                    }
                    _ => {}
                }
            } else {
                match event_frame {
                    EventFrame::Shutdown => break,
                    EventFrame::SendBridgeEvent { dest, .. } if dest != ctx.game_id => {
                        info!("{} Sends event: {}", env.log_prefix, event_frame);
                        if let Err(e) = ctx.tx.send(event_frame).await {
                            error!("{} Failed to send: {}", env.log_prefix, e);
                        }
                    }
                    _ => continue,
                }
            }
        }

        CloseReason::Complete
    }
}
