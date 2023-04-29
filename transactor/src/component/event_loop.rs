use std::time::{Duration, UNIX_EPOCH};

use async_trait::async_trait;
use race_core::context::GameContext;
use race_core::error::Error;
use race_core::event::Event;
use tokio::select;
use tracing::{error, info, warn};

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

    #[allow(unused)] mode: ClientMode,
) {
    info!("Handle event: {}", event);

    let access_version = game_context.get_access_version();
    let settle_version = game_context.get_settle_version();

    match handler.handle_event(game_context, &event) {
        Ok(effects) => {
            ports
                .send(EventFrame::Broadcast {
                    event,
                    access_version,
                    settle_version,
                    timestamp: game_context.get_timestamp(),
                })
                .await;

            ports
                .send(EventFrame::ContextUpdated {
                    context: game_context.clone(),
                })
                .await;

            // We do optimistic updates here
            if let Some(settles) = effects.settles {
                info!("Send settlements: {:?}", settles);
                ports.send(EventFrame::Settle { settles }).await;
            }
        }
        Err(e) => {
            warn!("Handle event error: {}", e.to_string());
            // info!("Current context: {:?}", game_context);
        }
    }
}

/// Take the event from clients or the pending dispatched event.
/// Transactor will retrieve events from both dispatching event and
/// ports, while Validator will retrieve events from only ports.
async fn retrieve_event(
    ports: &mut PipelinePorts,
    game_context: &mut GameContext,
    mode: ClientMode,
) -> Option<EventFrame> {
    let timestamp = current_timestamp();
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

        // Read games from event bus
        while let Some(event_frame) = retrieve_event(&mut ports, &mut game_context, ctx.mode).await
        {
            // Set timestamp to current time
            game_context.set_timestamp(current_timestamp());

            match event_frame {
                EventFrame::InitState { init_account } => {
                    if let Err(e) = game_context
                        .apply_checkpoint(init_account.access_version, init_account.settle_version)
                    {
                        error!("Failed to apply checkpoint: {:?}", e);
                        ports.close(CloseReason::Fault(e));
                        return;
                    }

                    if let Err(e) = handler.init_state(&mut game_context, &init_account) {
                        error!("Failed to initiaze state: {:?}", e);
                        ports.close(CloseReason::Fault(e));
                        return;
                    }

                    info!(
                        "Initialize game state for {}, access_version = {}, settle_version = {}",
                        init_account.addr, init_account.access_version, init_account.settle_version
                    );

                    if game_context.get_dispatch().is_none() {
                        game_context.dispatch_safe(Event::Ready, 0);
                    }
                }
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

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
