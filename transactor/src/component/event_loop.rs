use std::time::{Duration, UNIX_EPOCH};

use async_trait::async_trait;
use race_core::context::GameContext;
use race_core::error::Error;
use race_core::event::Event;
use tokio::select;
use tracing::{info, warn};

use crate::component::common::{Component, PipelinePorts, Ports};
use crate::component::event_bus::CloseReason;
use crate::component::wrapped_handler::WrappedHandler;
use crate::frame::EventFrame;
use race_core::types::{ClientMode, GameAccount};

pub struct EventLoopContext {
    handler: WrappedHandler,
    game_context: GameContext,
    mode: ClientMode,
}

pub trait WrappedGameHandler: Send {
    fn init(&mut self, init_state: GameAccount) -> Result<(), Error>;

    fn handle_event(&mut self, event: EventFrame) -> Result<Vec<EventFrame>, Error>;
}

pub struct EventLoop {}

async fn handle(
    handler: &mut WrappedHandler,
    game_context: &mut GameContext,
    event: Event,
    ports: &PipelinePorts,
    mode: ClientMode,
) {
    info!("Handle event: {}", event);

    // if matches!(event, Event::RandomnessReady) {
    //     info!("random: {:?}", game_context.list_random_states())
    // }

    match handler.handle_event(game_context, &event) {
        Ok(effects) => {
            ports
                .send(EventFrame::Broadcast {
                    state: game_context.get_handler_state_raw().to_owned(),
                    event,
                    access_version: game_context.get_access_version(),
                    settle_version: game_context.get_settle_version(),
                    timestamp: game_context.get_timestamp(),
                })
                .await;

            ports
                .send(EventFrame::ContextUpdated {
                    context: game_context.clone(),
                })
                .await;

            if mode == ClientMode::Transactor {
                // We do optimistic updates here
                if let Some(settles) = effects.settles {
                    info!("Send settlements: {:?}", settles);
                    ports.send(EventFrame::Settle { settles }).await;

                    // The game should be restarted for next round.
                    ports
                        .send(EventFrame::SendServerEvent {
                            event: Event::GameStart {
                                access_version: game_context.get_access_version(),
                            },
                        })
                        .await;
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
    ports: &mut PipelinePorts,
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
        ports.recv().await
    } else if let Some(dispatch) = game_context.get_dispatch() {
        // If already passed
        if dispatch.timeout <= timestamp {
            let event = dispatch.event.clone();
            game_context.cancel_dispatch();
            return Some(EventFrame::SendServerEvent { event });
        }
        let to = tokio::time::sleep(Duration::from_millis(dispatch.timeout - timestamp));
        select! {
            ef = ports.recv() => {
                ef
            }
            _ = to => {
                let event = dispatch.event.clone();
                game_context.cancel_dispatch();
                Some(EventFrame::SendServerEvent { event })
            }
        }
    } else {
        ports.recv().await
    }
}

#[async_trait]
impl Component<PipelinePorts, EventLoopContext> for EventLoop {
    fn name(&self) -> &str {
        "Event Loop"
    }

    async fn run(mut ports: PipelinePorts, ctx: EventLoopContext) {
        let mut handler = ctx.handler;
        let mut game_context = ctx.game_context;

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
                &ports,
                ctx.mode,
            )
            .await;
        }

        // Read games from event bus
        while let Some(event_frame) = retrieve_event(&mut ports, &mut game_context, ctx.mode).await
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
                    handle(&mut handler, &mut game_context, event, &ports, ctx.mode).await;
                }
                EventFrame::PlayerLeaving { player_addr } => {
                    let event = Event::Leave { player_addr };
                    handle(&mut handler, &mut game_context, event, &ports, ctx.mode).await;
                }
                EventFrame::SendEvent { event } => {
                    handle(&mut handler, &mut game_context, event, &ports, ctx.mode).await;
                }
                EventFrame::SendServerEvent { event } => {
                    // Handle the shutdown event from game logic
                    if matches!(event, Event::Shutdown) {
                        ports.send(EventFrame::Shutdown).await;
                    } else {
                        handle(&mut handler, &mut game_context, event, &ports, ctx.mode).await;
                    }
                }
                EventFrame::Shutdown => {
                    warn!("Shutdown event loop");
                    ports.close(CloseReason::Complete);
                    break;
                }
                _ => (),
            }
        }
    }
}

impl EventLoop {
    pub fn init(
        handler: WrappedHandler,
        game_context: GameContext,
        mode: ClientMode,
    ) -> (Self, EventLoopContext) {
        (
            Self {},
            EventLoopContext {
                handler,
                game_context,
                mode,
            },
        )
    }
}
