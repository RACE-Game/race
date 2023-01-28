use std::time::Duration;

use race_core::context::{DispatchEvent, GameContext};
use race_core::error::Error;
use race_core::event::Event;
use tokio::select;
use tokio::sync::{mpsc, oneshot};
use tracing::{info, warn};

use crate::component::event_bus::CloseReason;
use crate::component::traits::{Attachable, Component, Named};
use crate::component::wrapped_handler::WrappedHandler;
use crate::frame::EventFrame;
use race_core::types::GameAccount;

pub struct EventLoopContext {
    input_rx: mpsc::Receiver<EventFrame>,
    output_tx: mpsc::Sender<EventFrame>,
    closed_tx: oneshot::Sender<CloseReason>,
    handler: WrappedHandler,
    game_context: GameContext,
}

pub trait WrappedGameHandler: Send {
    fn init(&mut self, init_state: GameAccount) -> Result<(), Error>;

    fn handle_event(&mut self, event: EventFrame) -> Result<Vec<EventFrame>, Error>;
}

pub struct EventLoop {
    input_tx: mpsc::Sender<EventFrame>,
    output_rx: Option<mpsc::Receiver<EventFrame>>,
    closed_rx: oneshot::Receiver<CloseReason>,
    ctx: Option<EventLoopContext>,
}

impl Named for EventLoop {
    fn name<'a>(&self) -> &'a str {
        "EventLoop"
    }
}

impl Attachable for EventLoop {
    fn input(&self) -> Option<mpsc::Sender<EventFrame>> {
        Some(self.input_tx.clone())
    }

    fn output(&mut self) -> Option<mpsc::Receiver<EventFrame>> {
        let mut ret = None;
        std::mem::swap(&mut ret, &mut self.output_rx);
        ret
    }
}

async fn handle(
    handler: &mut WrappedHandler,
    game_context: &mut GameContext,
    event: Event,
    out: &mpsc::Sender<EventFrame>,
) {
    info!("Handle event: {}", event);
    match handler.handle_event(game_context, &event) {
        Ok(effects) => {
            info!("Send broadcast");
            out.send(EventFrame::Broadcast {
                state_json: game_context.get_handler_state_json().to_owned(),
                event,
                access_version: game_context.get_access_version(),
                settle_version: game_context.get_settle_version(),
            })
            .await
            .unwrap();

            info!("Send context updated");
            out.send(EventFrame::ContextUpdated {
                context: game_context.clone(),
            })
            .await
            .unwrap();

            // We do optimistic updates here
            if let Some(settles) = effects.settles {
                info!("Send settlements: {:?}", settles);
                out.send(EventFrame::Settle { settles }).await.unwrap();
            }
        }
        Err(e) => {
            warn!("Handle event error: {}", e.to_string());
            info!("Current context: {:?}", game_context);
        }
    }
}

async fn retrieve_event(
    input_rx: &mut mpsc::Receiver<EventFrame>,
    dispatch: &Option<DispatchEvent>,
) -> Option<EventFrame> {
    if let Some(dispatch) = dispatch {
        if dispatch.timeout == 0 {
            return Some(EventFrame::SendServerEvent {
                event: dispatch.event.clone(),
            });
        }
        let to = tokio::time::sleep(Duration::from_millis(dispatch.timeout));
        select! {
            ef = input_rx.recv() => {
                ef
            }
            _ = to => {
                Some(EventFrame::SendServerEvent {event: dispatch.event.clone()})
            }
        }
    } else {
        input_rx.recv().await
    }
}

impl Component<EventLoopContext> for EventLoop {
    fn run(&mut self, mut ctx: EventLoopContext) {
        tokio::spawn(async move {
            let mut handler = ctx.handler;
            let mut game_context = ctx.game_context;
            let output_tx = ctx.output_tx;
            while let Some(event_frame) =
                retrieve_event(&mut ctx.input_rx, game_context.get_dispatch()).await
            {
                match event_frame {
                    EventFrame::Sync {
                        new_players,
                        new_servers,
                        access_version,
                        transactor_addr,
                    } => {
                        let event = Event::Sync {
                            new_players,
                            new_servers,
                            access_version,
                            transactor_addr,
                        };
                        handle(&mut handler, &mut game_context, event, &output_tx).await;
                    }
                    EventFrame::PlayerLeaving { player_addr } => {
                        let event = Event::Leave { player_addr };
                        handle(&mut handler, &mut game_context, event, &output_tx).await;
                    }
                    EventFrame::SendEvent { event } => {
                        handle(&mut handler, &mut game_context, event, &output_tx).await;
                    }
                    EventFrame::SendServerEvent { event } => {
                        handle(&mut handler, &mut game_context, event, &output_tx).await;
                    }
                    EventFrame::Shutdown => {
                        ctx.closed_tx.send(CloseReason::Complete).unwrap();
                        break;
                    }
                    _ => (),
                }
            }
        });
    }

    fn borrow_mut_ctx(&mut self) -> &mut Option<EventLoopContext> {
        &mut self.ctx
    }

    fn closed(self) -> oneshot::Receiver<CloseReason> {
        self.closed_rx
    }
}

impl EventLoop {
    pub fn new(handler: WrappedHandler, game_context: GameContext) -> Self {
        let (input_tx, input_rx) = mpsc::channel(3);
        let (output_tx, output_rx) = mpsc::channel(3);
        let (closed_tx, closed_rx) = oneshot::channel();
        let ctx = Some(EventLoopContext {
            input_rx,
            output_tx,
            closed_tx,
            handler,
            game_context,
        });
        Self {
            input_tx,
            output_rx: Some(output_rx),
            closed_rx,
            ctx,
        }
    }
}
