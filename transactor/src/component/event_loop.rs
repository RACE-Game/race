use std::time::{Duration, UNIX_EPOCH};

use race_core::context::GameContext;
use race_core::error::Error;
use race_core::event::Event;
use tokio::select;
use tokio::sync::{mpsc, oneshot};
use tracing::{info, warn};

use crate::component::event_bus::CloseReason;
use crate::component::traits::{Attachable, Component, Named};
use crate::component::wrapped_handler::WrappedHandler;
use crate::frame::EventFrame;
use race_core::types::{ClientMode, GameAccount};

pub struct EventLoopContext {
    input_rx: mpsc::Receiver<EventFrame>,
    output_tx: mpsc::Sender<EventFrame>,
    closed_tx: oneshot::Sender<CloseReason>,
    handler: WrappedHandler,
    game_context: GameContext,
    mode: ClientMode,
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
    mode: ClientMode,
) {
    info!("Handle event: {}", event);

    // if matches!(event, Event::RandomnessReady) {
    //     info!("random: {:?}", game_context.list_random_states())
    // }

    match handler.handle_event(game_context, &event) {
        Ok(effects) => {
            out.send(EventFrame::Broadcast {
                state_json: game_context.get_handler_state_json().to_owned(),
                event,
                access_version: game_context.get_access_version(),
                settle_version: game_context.get_settle_version(),
                timestamp: game_context.get_timestamp(),
            })
            .await
            .unwrap();

            out.send(EventFrame::ContextUpdated {
                context: game_context.clone(),
            })
            .await
            .unwrap();

            if mode == ClientMode::Transactor {
                // We do optimistic updates here
                if let Some(settles) = effects.settles {
                    info!("Send settlements: {:?}", settles);
                    out.send(EventFrame::Settle { settles }).await.unwrap();

                    // The game should be restarted for next round.
                    out.send(EventFrame::SendServerEvent {
                        event: Event::GameStart {
                            access_version: game_context.get_access_version(),
                        },
                    })
                    .await
                    .unwrap();
                }
            }
        }
        Err(e) => {
            warn!("Handle event error: {}", e.to_string());
            // info!("Current context: {:?}", game_context);
        }
    }
}

/// Take the event from clients or the pending dispatched event.
async fn retrieve_event(
    input_rx: &mut mpsc::Receiver<EventFrame>,
    game_context: &mut GameContext,
    mode: ClientMode,
) -> Option<EventFrame> {
    // Set timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    game_context.set_timestamp(timestamp);

    if mode != ClientMode::Transactor {
        input_rx.recv().await
    } else if let Some(dispatch) = game_context.get_dispatch() {
        // If already passed
        if dispatch.timeout <= timestamp {
            let event = dispatch.event.clone();
            game_context.cancel_dispatch();
            return Some(EventFrame::SendServerEvent { event });
        }
        let to = tokio::time::sleep(Duration::from_millis(dispatch.timeout - timestamp));
        select! {
            ef = input_rx.recv() => {
                ef
            }
            _ = to => {
                let event = dispatch.event.clone();
                game_context.cancel_dispatch();
                Some(EventFrame::SendServerEvent { event })
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

            if ctx.mode == ClientMode::Transactor {
                // Send the very first event to game handler
                // This event doesn't have to be succeed.
                let first_event = Event::GameStart {
                    access_version: game_context.get_access_version(),
                };
                handle(
                    &mut handler,
                    &mut game_context,
                    first_event,
                    &output_tx,
                    ctx.mode,
                )
                .await;
            }

            // Read games from event bus
            while let Some(event_frame) =
                retrieve_event(&mut ctx.input_rx, &mut game_context, ctx.mode).await
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
                        handle(&mut handler, &mut game_context, event, &output_tx, ctx.mode).await;
                    }
                    EventFrame::PlayerLeaving { player_addr } => {
                        let event = Event::Leave { player_addr };
                        handle(&mut handler, &mut game_context, event, &output_tx, ctx.mode).await;
                    }
                    EventFrame::SendEvent { event } => {
                        handle(&mut handler, &mut game_context, event, &output_tx, ctx.mode).await;
                    }
                    EventFrame::SendServerEvent { event } => {
                        handle(&mut handler, &mut game_context, event, &output_tx, ctx.mode).await;
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
    pub fn new(handler: WrappedHandler, game_context: GameContext, mode: ClientMode) -> Self {
        let (input_tx, input_rx) = mpsc::channel(3);
        let (output_tx, output_rx) = mpsc::channel(3);
        let (closed_tx, closed_rx) = oneshot::channel();
        let ctx = Some(EventLoopContext {
            input_rx,
            output_tx,
            closed_tx,
            handler,
            game_context,
            mode,
        });
        Self {
            input_tx,
            output_rx: Some(output_rx),
            closed_rx,
            ctx,
        }
    }
}
