use crate::frame::{EventFrame, SignalFrame};
use async_trait::async_trait;
use tokio::sync::{broadcast, mpsc};
use tracing::{info, log::error};

use super::{common::PipelinePorts, CloseReason, Component, ComponentEnv};

#[derive(Debug)]
pub struct BridgeToParent {
    tx_to_parent: mpsc::Sender<EventFrame>,
    rx_from_parent: broadcast::Receiver<EventFrame>,
}

#[allow(dead_code)]
pub struct EventBridgeParentContext {
    /// The sender to send to sub games.
    tx: broadcast::Sender<EventFrame>,
    /// The receiver to receive from sub games.
    rx: mpsc::Receiver<EventFrame>,
    /// The sender used to be cloned when launching sub games.
    sub_tx: mpsc::Sender<EventFrame>,
    signal_tx: mpsc::Sender<SignalFrame>,
}

#[derive(Clone, Debug)]
pub struct EventBridgeParent {
    #[allow(unused)]
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
                bc: bc_tx.clone(),
            },
            EventBridgeParentContext {
                tx: bc_tx,
                rx: mpsc_rx,
                sub_tx: mpsc_tx.clone(),
                signal_tx,
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
            if from_bridge {    // Bridge parent receives event from bridge child
                match event_frame {
                    EventFrame::SendBridgeEvent {
                        from,
                        dest,
                        event,
                        access_version,
                        settle_version,
                        checkpoint_state,
                    } => {
                        info!("{} Receives event: {}", env.log_prefix, event);
                        ports
                            .send(EventFrame::RecvBridgeEvent {
                                from,
                                dest,
                                event,
                                access_version,
                                settle_version,
                                checkpoint_state,
                            })
                            .await;
                    }

                    EventFrame::SubGameReady { game_id, .. } => {
                        info!("{} Receives subgame ready: {}", env.log_prefix, game_id);
                        ports.send(event_frame).await;
                    }
                    _ => (),
                }
            } else {            // Bridge parent receives event from event bus
                match event_frame {
                    EventFrame::LaunchSubGame { sub_game_init } => {
                        let f = SignalFrame::LaunchSubGame {
                            sub_game_init: *sub_game_init,
                            bridge_to_parent: BridgeToParent {
                                rx_from_parent: ctx.tx.subscribe(),
                                tx_to_parent: ctx.sub_tx.clone(),
                            },
                        };
                        if let Err(e) = ctx.signal_tx.send(f).await {
                            error!("{} Failed to send: {}", env.log_prefix, e);
                        }
                    }
                    EventFrame::Shutdown => {
                        info!("{} Sends Shutdown", env.log_prefix);
                        if let Err(e) = ctx.tx.send(event_frame) {
                            error!("{} Failed to send: {}", env.log_prefix, e);
                        }
                        info!("{} Stopped", env.log_prefix);
                        break;
                    }
                    EventFrame::SendBridgeEvent { dest, .. } if dest != 0 => {
                        info!("{} Sends event: {}", env.log_prefix, event_frame);
                        if let Err(e) = ctx.tx.send(event_frame) {
                            error!("{} Failed to send: {}", env.log_prefix, e);
                        }
                    }
                    EventFrame::Sync { new_players, new_servers, transactor_addr, .. } => {
                        if !ctx.tx.is_empty() {
                            let sub_sync = EventFrame::SubSync {
                                new_players, new_servers, transactor_addr
                            };
                            info!("{} Sends event: {}", env.log_prefix, sub_sync);
                            if let Err(e) = ctx.tx.send(sub_sync) {
                                error!("{} Failed to send: {}", env.log_prefix, e);
                            }
                        }
                    }
                    EventFrame::SubSync { .. } => {
                        if let Err(e) = ctx.tx.send(event_frame) {
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

impl EventBridgeChild {

    pub fn init(game_id: usize, bridge_to_parent: BridgeToParent) -> (EventBridgeChild, EventBridgeChildContext) {
        (
            EventBridgeChild {
                game_id: game_id.clone(),
            },
            EventBridgeChildContext {
                game_id,
                tx: bridge_to_parent.tx_to_parent,
                rx: bridge_to_parent.rx_from_parent,
            },
        )
    }

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
            if from_bridge { // Bridge child receives event from event parent
                match event_frame {
                    EventFrame::Shutdown => {
                        info!("{} Stopped", env.log_prefix);
                        ports.send(event_frame).await;
                        break;
                    }
                    EventFrame::Sync { .. } => {
                        info!("{} Receives {}", env.log_prefix, event_frame);
                        ports.send(event_frame).await;
                    }
                    EventFrame::SubSync { .. } => {
                        info!("{} Receives {}", env.log_prefix, event_frame);
                        ports.send(event_frame).await;
                    }
                    EventFrame::SendBridgeEvent {
                        from,
                        dest,
                        event,
                        access_version,
                        settle_version,
                        checkpoint_state,
                    } if dest == ctx.game_id => {
                        info!("{} Receives {}", env.log_prefix, event);
                        ports
                            .send(EventFrame::RecvBridgeEvent {
                                from,
                                dest,
                                event,
                                access_version,
                                settle_version,
                                checkpoint_state,
                            })
                            .await;
                    }
                    _ => {}
                }
            } else { // Bridge child receives event from event bus
                match event_frame {
                    EventFrame::Shutdown => break,

                    EventFrame::SubGameReady { .. } => {
                        info!("{} Send SubGameReady to parent", env.log_prefix);
                        if let Err(e) = ctx.tx.send(event_frame).await {
                            error!("{} Failed to send: {}", env.log_prefix, e);
                        }
                    }

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
